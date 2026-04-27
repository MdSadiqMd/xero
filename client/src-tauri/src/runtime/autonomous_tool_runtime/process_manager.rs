use std::{
    collections::BTreeMap,
    env,
    io::Read,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use super::{
    policy::{process_manager_policy_trace, CommandPolicyDecision, PreparedCommandRequest},
    process::{apply_sanitized_command_environment, SAFE_COMMAND_ENV_KEYS},
    repo_scope::{display_relative_or_root, normalize_relative_path},
    AutonomousCommandPolicyOutcome, AutonomousCommandPolicyTrace, AutonomousProcessActionRiskLevel,
    AutonomousProcessCommandMetadata, AutonomousProcessLifecycleContract,
    AutonomousProcessManagerAction, AutonomousProcessManagerContract,
    AutonomousProcessManagerOutput, AutonomousProcessManagerPolicyTrace,
    AutonomousProcessManagerRequest, AutonomousProcessMetadata, AutonomousProcessOutputChunk,
    AutonomousProcessOutputLimits, AutonomousProcessOutputStream, AutonomousProcessOwner,
    AutonomousProcessOwnershipScope, AutonomousProcessPersistenceContract,
    AutonomousProcessReadinessState, AutonomousProcessStatus, AutonomousToolOutput,
    AutonomousToolResult, AutonomousToolRuntime, AUTONOMOUS_TOOL_PROCESS_MANAGER,
};
use crate::{
    auth::now_timestamp,
    commands::{validate_non_empty, CommandError, CommandResult},
    runtime::{
        cancelled_error,
        process_tree::{
            cleanup_process_group_after_root_exit, configure_process_tree_root,
            terminate_process_tree,
        },
        redaction::{
            find_prohibited_persistence_content, redact_command_argv_for_persistence,
            render_command_for_persistence,
        },
    },
};

const PROCESS_MANAGER_PHASE: &str = "phase_1_owned_mvp";
const PROCESS_MANAGER_INITIAL_DRAIN: Duration = Duration::from_millis(150);
const MAX_OWNED_PROCESSES: usize = 8;
const RECENT_OUTPUT_RING_BYTES: usize = 1024 * 1024;
const RECENT_OUTPUT_RING_CHUNKS: usize = 512;
const FULL_OUTPUT_ARTIFACT_THRESHOLD_BYTES: usize = 1024 * 1024;
const PROCESS_OUTPUT_EXCERPT_BYTES: usize = 16 * 1024;
const MAX_PROCESS_OUTPUT_READ_BYTES: usize = 64 * 1024;
const REDACTED_PROCESS_OUTPUT_SUMMARY: &str =
    "Process output was redacted before durable persistence.";

#[derive(Debug, Default)]
pub(super) struct OwnedProcessRegistry {
    processes: Mutex<BTreeMap<String, Arc<OwnedProcess>>>,
    next_id: AtomicU64,
}

impl OwnedProcessRegistry {
    fn next_process_id(&self) -> String {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        format!("owned-process-{id}")
    }

    fn ensure_capacity(&self) -> CommandResult<()> {
        let processes = self.processes.lock().map_err(process_registry_lock_error)?;
        if processes.len() >= MAX_OWNED_PROCESSES {
            return Err(CommandError::user_fixable(
                "autonomous_tool_process_manager_limit_reached",
                format!(
                    "Cadence limits the process manager to {MAX_OWNED_PROCESSES} concurrent owned process(es). Kill an existing process before starting another."
                ),
            ));
        }
        Ok(())
    }

    fn insert(&self, process: Arc<OwnedProcess>) -> CommandResult<()> {
        let mut processes = self.processes.lock().map_err(process_registry_lock_error)?;
        if processes.len() >= MAX_OWNED_PROCESSES {
            return Err(CommandError::user_fixable(
                "autonomous_tool_process_manager_limit_reached",
                format!(
                    "Cadence limits the process manager to {MAX_OWNED_PROCESSES} concurrent owned process(es). Kill an existing process before starting another."
                ),
            ));
        }
        processes.insert(process.process_id.clone(), process);
        Ok(())
    }

    fn get(&self, process_id: &str) -> CommandResult<Arc<OwnedProcess>> {
        let processes = self.processes.lock().map_err(process_registry_lock_error)?;
        processes.get(process_id).cloned().ok_or_else(|| {
            CommandError::user_fixable(
                "autonomous_tool_process_manager_not_found",
                format!("Cadence could not find owned process `{process_id}`."),
            )
        })
    }

    fn list(&self) -> CommandResult<Vec<Arc<OwnedProcess>>> {
        let processes = self.processes.lock().map_err(process_registry_lock_error)?;
        Ok(processes.values().cloned().collect())
    }

    fn remove(&self, process_id: &str) -> CommandResult<Arc<OwnedProcess>> {
        let mut processes = self.processes.lock().map_err(process_registry_lock_error)?;
        processes.remove(process_id).ok_or_else(|| {
            CommandError::user_fixable(
                "autonomous_tool_process_manager_not_found",
                format!("Cadence could not find owned process `{process_id}`."),
            )
        })
    }
}

impl Drop for OwnedProcessRegistry {
    fn drop(&mut self) {
        if let Ok(processes) = self.processes.get_mut() {
            for process in processes.values() {
                let _ = process.kill();
            }
        }
    }
}

fn process_registry_lock_error(_error: std::sync::PoisonError<impl Sized>) -> CommandError {
    CommandError::system_fault(
        "autonomous_tool_process_manager_lock_failed",
        "Cadence could not lock the owned process registry.",
    )
}

#[derive(Debug)]
struct OwnedProcess {
    process_id: String,
    pid: u32,
    label: Option<String>,
    process_type: Option<String>,
    group: Option<String>,
    owner: AutonomousProcessOwner,
    command: AutonomousProcessCommandMetadata,
    child: Mutex<Option<Child>>,
    status: Mutex<AutonomousProcessStatus>,
    started_at: String,
    exited_at: Mutex<Option<String>>,
    exit_code: Mutex<Option<i32>>,
    chunks: Mutex<Vec<AutonomousProcessOutputChunk>>,
    next_cursor: AtomicU64,
}

impl OwnedProcess {
    #[allow(clippy::too_many_arguments)]
    fn new(
        process_id: String,
        prepared: &PreparedCommandRequest,
        child: Child,
        label: Option<String>,
        process_type: Option<String>,
        group: Option<String>,
    ) -> Self {
        let pid = child.id();
        Self {
            process_id,
            pid,
            label,
            process_type,
            group,
            owner: AutonomousProcessOwner {
                thread_id: None,
                session_id: None,
                repo_id: None,
                user_id: None,
                scope: AutonomousProcessOwnershipScope::CadenceOwned,
            },
            command: AutonomousProcessCommandMetadata {
                argv: redact_command_argv_for_persistence(&prepared.argv),
                shell_mode: false,
                cwd: display_relative_or_root(&prepared.cwd, &prepared.cwd),
                sanitized_env: sanitized_env_summary(),
            },
            child: Mutex::new(Some(child)),
            status: Mutex::new(AutonomousProcessStatus::Running),
            started_at: now_timestamp(),
            exited_at: Mutex::new(None),
            exit_code: Mutex::new(None),
            chunks: Mutex::new(Vec::new()),
            next_cursor: AtomicU64::new(1),
        }
    }

    fn set_display_cwd(&mut self, cwd: String) {
        self.command.cwd = cwd;
    }

    fn push_chunk(
        &self,
        stream: AutonomousProcessOutputStream,
        capture: SanitizedProcessOutput,
    ) -> CommandResult<()> {
        let cursor = self.next_cursor.fetch_add(1, Ordering::Relaxed);
        let mut chunks = self.chunks.lock().map_err(process_output_lock_error)?;
        chunks.push(AutonomousProcessOutputChunk {
            cursor,
            stream,
            text: capture.text,
            truncated: capture.truncated,
            redacted: capture.redacted,
            captured_at: Some(now_timestamp()),
        });
        prune_process_output_chunks(&mut chunks);
        Ok(())
    }

    fn read_chunks_after(
        &self,
        after_cursor: u64,
        max_bytes: usize,
    ) -> CommandResult<Vec<AutonomousProcessOutputChunk>> {
        let chunks = self.chunks.lock().map_err(process_output_lock_error)?;
        let mut selected = Vec::new();
        let mut bytes = 0_usize;
        for chunk in chunks
            .iter()
            .filter(|chunk| chunk.cursor > after_cursor)
            .cloned()
        {
            let chunk_bytes = chunk.text.as_deref().map(str::len).unwrap_or_default();
            if !selected.is_empty() && bytes.saturating_add(chunk_bytes) > max_bytes {
                break;
            }
            bytes = bytes.saturating_add(chunk_bytes);
            selected.push(chunk);
            if bytes >= max_bytes {
                break;
            }
        }
        Ok(selected)
    }

    fn next_cursor_value(&self) -> u64 {
        self.next_cursor.load(Ordering::Relaxed)
    }

    fn poll_exit(&self) -> CommandResult<Option<i32>> {
        if let Some(exit_code) = *self.exit_code.lock().map_err(process_exit_lock_error)? {
            return Ok(Some(exit_code));
        }

        let mut child = self.child.lock().map_err(process_state_lock_error)?;
        let Some(child_ref) = child.as_mut() else {
            return Ok(*self.exit_code.lock().map_err(process_exit_lock_error)?);
        };

        match child_ref.try_wait() {
            Ok(Some(status)) => {
                let exit_code = status.code();
                cleanup_process_group_after_root_exit(child_ref.id());
                *self.exit_code.lock().map_err(process_exit_lock_error)? = exit_code;
                *self.exited_at.lock().map_err(process_exit_lock_error)? = Some(now_timestamp());
                *self.status.lock().map_err(process_status_lock_error)? =
                    AutonomousProcessStatus::Exited;
                *child = None;
                Ok(exit_code)
            }
            Ok(None) => {
                *self.status.lock().map_err(process_status_lock_error)? =
                    AutonomousProcessStatus::Running;
                Ok(None)
            }
            Err(error) => Err(CommandError::retryable(
                "autonomous_tool_process_manager_wait_failed",
                format!(
                    "Cadence could not observe owned process `{}`: {error}",
                    self.process_id
                ),
            )),
        }
    }

    fn kill(&self) -> CommandResult<Option<i32>> {
        let mut child = self.child.lock().map_err(process_state_lock_error)?;
        let Some(child_ref) = child.as_mut() else {
            return Ok(*self.exit_code.lock().map_err(process_exit_lock_error)?);
        };

        match child_ref.try_wait() {
            Ok(Some(status)) => {
                let exit_code = status.code();
                cleanup_process_group_after_root_exit(child_ref.id());
                *self.exit_code.lock().map_err(process_exit_lock_error)? = exit_code;
                *self.exited_at.lock().map_err(process_exit_lock_error)? = Some(now_timestamp());
                *self.status.lock().map_err(process_status_lock_error)? =
                    AutonomousProcessStatus::Exited;
                *child = None;
                Ok(exit_code)
            }
            Ok(None) => {
                *self.status.lock().map_err(process_status_lock_error)? =
                    AutonomousProcessStatus::Killing;
                let status = terminate_process_tree(child_ref).map_err(|error| {
                    CommandError::retryable(
                        "autonomous_tool_process_manager_kill_failed",
                        format!(
                            "Cadence could not kill owned process `{}`: {error}",
                            self.process_id
                        ),
                    )
                })?;
                let exit_code = status.code();
                *self.exit_code.lock().map_err(process_exit_lock_error)? = exit_code;
                *self.exited_at.lock().map_err(process_exit_lock_error)? = Some(now_timestamp());
                *self.status.lock().map_err(process_status_lock_error)? =
                    AutonomousProcessStatus::Killed;
                *child = None;
                Ok(exit_code)
            }
            Err(error) => Err(CommandError::retryable(
                "autonomous_tool_process_manager_wait_failed",
                format!(
                    "Cadence could not observe owned process `{}` before killing it: {error}",
                    self.process_id
                ),
            )),
        }
    }

    fn metadata(&self) -> CommandResult<AutonomousProcessMetadata> {
        let exit_code = *self.exit_code.lock().map_err(process_exit_lock_error)?;
        let status = *self.status.lock().map_err(process_status_lock_error)?;
        let exited_at = self
            .exited_at
            .lock()
            .map_err(process_exit_lock_error)?
            .clone();
        Ok(AutonomousProcessMetadata {
            process_id: self.process_id.clone(),
            pid: Some(self.pid),
            process_group_id: Some(self.pid as i64),
            label: self.label.clone(),
            process_type: self.process_type.clone(),
            group: self.group.clone(),
            owner: self.owner.clone(),
            command: self.command.clone(),
            status,
            started_at: Some(self.started_at.clone()),
            exited_at,
            exit_code,
            output_cursor: Some(self.next_cursor_value().saturating_sub(1)),
            detected_urls: Vec::new(),
            detected_ports: Vec::new(),
            recent_errors: Vec::new(),
            recent_warnings: Vec::new(),
            readiness: AutonomousProcessReadinessState {
                ready: false,
                detector: None,
                matched: None,
            },
            restart_count: 0,
        })
    }
}

fn process_state_lock_error(_error: std::sync::PoisonError<impl Sized>) -> CommandError {
    CommandError::system_fault(
        "autonomous_tool_process_manager_lock_failed",
        "Cadence could not lock owned process state.",
    )
}

fn process_status_lock_error(_error: std::sync::PoisonError<impl Sized>) -> CommandError {
    CommandError::system_fault(
        "autonomous_tool_process_manager_lock_failed",
        "Cadence could not lock owned process status.",
    )
}

fn process_exit_lock_error(_error: std::sync::PoisonError<impl Sized>) -> CommandError {
    CommandError::system_fault(
        "autonomous_tool_process_manager_lock_failed",
        "Cadence could not lock owned process exit state.",
    )
}

fn process_output_lock_error(_error: std::sync::PoisonError<impl Sized>) -> CommandError {
    CommandError::system_fault(
        "autonomous_tool_process_manager_lock_failed",
        "Cadence could not lock owned process output.",
    )
}

impl AutonomousToolRuntime {
    pub fn process_manager(
        &self,
        request: AutonomousProcessManagerRequest,
    ) -> CommandResult<AutonomousToolResult> {
        self.process_manager_with_approval(request, false)
    }

    pub fn process_manager_with_operator_approval(
        &self,
        request: AutonomousProcessManagerRequest,
    ) -> CommandResult<AutonomousToolResult> {
        self.process_manager_with_approval(request, true)
    }

    fn process_manager_with_approval(
        &self,
        request: AutonomousProcessManagerRequest,
        operator_approved: bool,
    ) -> CommandResult<AutonomousToolResult> {
        validate_process_manager_request(&request)?;
        validate_phase_1_scope(&request)?;

        match request.action {
            AutonomousProcessManagerAction::Start => {
                self.process_manager_start(request, operator_approved)
            }
            AutonomousProcessManagerAction::List => self.process_manager_list(request),
            AutonomousProcessManagerAction::Status => self.process_manager_status(request),
            AutonomousProcessManagerAction::Output => self.process_manager_output(request),
            AutonomousProcessManagerAction::Kill => self.process_manager_kill(request),
            action => Err(unsupported_phase_1_action(action)),
        }
    }

    fn process_manager_start(
        &self,
        request: AutonomousProcessManagerRequest,
        operator_approved: bool,
    ) -> CommandResult<AutonomousToolResult> {
        let prepared_request = super::AutonomousCommandRequest {
            argv: request.argv.clone(),
            cwd: request.cwd.clone(),
            timeout_ms: request.timeout_ms,
        };
        let decision =
            self.evaluate_command_policy(self.prepare_command_request(prepared_request)?)?;

        match decision {
            CommandPolicyDecision::Allow { prepared, policy } => {
                self.spawn_owned_process(request, prepared, process_policy_from_command(policy))
            }
            CommandPolicyDecision::Escalate { prepared, policy } if operator_approved => {
                let policy = operator_approved_command_policy(policy, &prepared.argv);
                self.spawn_owned_process(request, prepared, process_policy_from_command(policy))
            }
            CommandPolicyDecision::Escalate { prepared, policy } => {
                self.unspawned_process_manager_approval_result(request, prepared, policy)
            }
        }
    }

    fn spawn_owned_process(
        &self,
        request: AutonomousProcessManagerRequest,
        prepared: PreparedCommandRequest,
        policy: AutonomousProcessManagerPolicyTrace,
    ) -> CommandResult<AutonomousToolResult> {
        self.owned_processes.ensure_capacity()?;
        self.check_cancelled()?;

        let mut command = Command::new(&prepared.argv[0]);
        command
            .args(prepared.argv.iter().skip(1))
            .current_dir(&prepared.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        configure_process_tree_root(&mut command);
        apply_sanitized_command_environment(&mut command);

        let mut child = command.spawn().map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => CommandError::user_fixable(
                "autonomous_tool_process_manager_not_found",
                format!("Cadence could not find command `{}`.", prepared.argv[0]),
            ),
            _ => CommandError::system_fault(
                "autonomous_tool_process_manager_spawn_failed",
                format!(
                    "Cadence could not launch owned process `{}`: {error}",
                    prepared.argv[0]
                ),
            ),
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            let _ = terminate_process_tree(&mut child);
            CommandError::system_fault(
                "autonomous_tool_process_manager_stdout_missing",
                "Cadence could not capture owned process stdout.",
            )
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            let _ = terminate_process_tree(&mut child);
            CommandError::system_fault(
                "autonomous_tool_process_manager_stderr_missing",
                "Cadence could not capture owned process stderr.",
            )
        })?;

        let process_id = self.owned_processes.next_process_id();
        let cwd = display_relative_or_root(&self.repo_root, &prepared.cwd);
        let mut owned_process = OwnedProcess::new(
            process_id.clone(),
            &prepared,
            child,
            clean_optional_string(request.label.as_deref()),
            clean_optional_string(request.process_type.as_deref()),
            clean_optional_string(request.group.as_deref()),
        );
        owned_process.set_display_cwd(cwd.clone());
        let process = Arc::new(owned_process);

        spawn_owned_process_reader(
            Arc::clone(&process),
            stdout,
            AutonomousProcessOutputStream::Stdout,
        );
        spawn_owned_process_reader(
            Arc::clone(&process),
            stderr,
            AutonomousProcessOutputStream::Stderr,
        );

        if let Err(error) = self.owned_processes.insert(Arc::clone(&process)) {
            let _ = process.kill();
            return Err(error);
        }

        thread::sleep(PROCESS_MANAGER_INITIAL_DRAIN);
        if self.is_cancelled() {
            let _ = self.owned_processes.remove(&process_id);
            let _ = process.kill();
            return Err(cancelled_error());
        }

        let exit_code = process.poll_exit()?;
        let chunks = process.read_chunks_after(0, default_process_output_read_bytes())?;
        let metadata = process.metadata()?;
        let running = exit_code.is_none();
        let message = if running {
            format!(
                "Started owned process `{process_id}` for `{}` in `{cwd}`.",
                render_command_for_summary(&prepared.argv)
            )
        } else {
            format!(
                "Owned process `{process_id}` for `{}` exited during startup.",
                render_command_for_summary(&prepared.argv)
            )
        };

        Ok(process_manager_result(ProcessManagerResultInput {
            action: AutonomousProcessManagerAction::Start,
            spawned: true,
            process_id: Some(process_id),
            processes: vec![metadata],
            chunks,
            next_cursor: Some(process.next_cursor_value()),
            policy,
            message,
        }))
    }

    fn unspawned_process_manager_approval_result(
        &self,
        request: AutonomousProcessManagerRequest,
        prepared: PreparedCommandRequest,
        command_policy: AutonomousCommandPolicyTrace,
    ) -> CommandResult<AutonomousToolResult> {
        let cwd = prepared
            .cwd_relative
            .as_ref()
            .map(|path| display_relative_or_root(&self.repo_root, &self.repo_root.join(path)))
            .unwrap_or_else(|| ".".into());
        let policy = process_policy_requiring_command_approval(command_policy);
        let message = format!(
            "Owned process `{}` requires operator review before Cadence can start it.",
            render_command_for_summary(&prepared.argv)
        );
        Ok(process_manager_result(ProcessManagerResultInput {
            action: AutonomousProcessManagerAction::Start,
            spawned: false,
            process_id: Some("unstarted".into()),
            processes: vec![unstarted_process_metadata(
                &prepared.argv,
                cwd,
                request.label,
                request.process_type,
                request.group,
            )],
            chunks: Vec::new(),
            next_cursor: Some(0),
            policy,
            message,
        }))
    }

    fn process_manager_list(
        &self,
        request: AutonomousProcessManagerRequest,
    ) -> CommandResult<AutonomousToolResult> {
        let processes = self.owned_processes.list()?;
        let mut metadata = Vec::with_capacity(processes.len());
        for process in processes {
            let _ = process.poll_exit()?;
            metadata.push(process.metadata()?);
        }
        let message = format!("Listed {} Cadence-owned process(es).", metadata.len());
        Ok(process_manager_result(ProcessManagerResultInput {
            action: AutonomousProcessManagerAction::List,
            spawned: true,
            process_id: None,
            processes: metadata,
            chunks: Vec::new(),
            next_cursor: request.after_cursor,
            policy: process_manager_policy_trace(
                AutonomousProcessManagerAction::List,
                request.target_ownership,
                false,
            ),
            message,
        }))
    }

    fn process_manager_status(
        &self,
        request: AutonomousProcessManagerRequest,
    ) -> CommandResult<AutonomousToolResult> {
        let process_id = normalized_process_id(&request)?;
        let process = self.owned_processes.get(&process_id)?;
        let _ = process.poll_exit()?;
        let metadata = process.metadata()?;
        let message = format!("Returned status for owned process `{process_id}`.");
        Ok(process_manager_result(ProcessManagerResultInput {
            action: AutonomousProcessManagerAction::Status,
            spawned: true,
            process_id: Some(process_id),
            processes: vec![metadata],
            chunks: Vec::new(),
            next_cursor: request.after_cursor,
            policy: process_manager_policy_trace(
                AutonomousProcessManagerAction::Status,
                request.target_ownership,
                false,
            ),
            message,
        }))
    }

    fn process_manager_output(
        &self,
        request: AutonomousProcessManagerRequest,
    ) -> CommandResult<AutonomousToolResult> {
        let process_id = normalized_process_id(&request)?;
        let process = self.owned_processes.get(&process_id)?;
        let _ = process.poll_exit()?;
        let max_bytes = request
            .max_bytes
            .unwrap_or_else(default_process_output_read_bytes)
            .clamp(1, MAX_PROCESS_OUTPUT_READ_BYTES);
        let chunks = process.read_chunks_after(request.after_cursor.unwrap_or(0), max_bytes)?;
        let metadata = process.metadata()?;
        let message = format!(
            "Read {} output chunk(s) from owned process `{process_id}`.",
            chunks.len()
        );
        Ok(process_manager_result(ProcessManagerResultInput {
            action: AutonomousProcessManagerAction::Output,
            spawned: true,
            process_id: Some(process_id),
            processes: vec![metadata],
            chunks,
            next_cursor: Some(process.next_cursor_value()),
            policy: process_manager_policy_trace(
                AutonomousProcessManagerAction::Output,
                request.target_ownership,
                false,
            ),
            message,
        }))
    }

    fn process_manager_kill(
        &self,
        request: AutonomousProcessManagerRequest,
    ) -> CommandResult<AutonomousToolResult> {
        let process_id = normalized_process_id(&request)?;
        let process = self.owned_processes.remove(&process_id)?;
        let _ = process.kill()?;
        thread::sleep(PROCESS_MANAGER_INITIAL_DRAIN);
        let chunks = process.read_chunks_after(
            request.after_cursor.unwrap_or(0),
            MAX_PROCESS_OUTPUT_READ_BYTES,
        )?;
        let metadata = process.metadata()?;
        let message = format!("Killed owned process `{process_id}`.");
        Ok(process_manager_result(ProcessManagerResultInput {
            action: AutonomousProcessManagerAction::Kill,
            spawned: true,
            process_id: Some(process_id),
            processes: vec![metadata],
            chunks,
            next_cursor: Some(process.next_cursor_value()),
            policy: process_manager_policy_trace(
                AutonomousProcessManagerAction::Kill,
                request.target_ownership,
                false,
            ),
            message,
        }))
    }
}

struct ProcessManagerResultInput {
    action: AutonomousProcessManagerAction,
    spawned: bool,
    process_id: Option<String>,
    processes: Vec<AutonomousProcessMetadata>,
    chunks: Vec<AutonomousProcessOutputChunk>,
    next_cursor: Option<u64>,
    policy: AutonomousProcessManagerPolicyTrace,
    message: String,
}

fn process_manager_result(input: ProcessManagerResultInput) -> AutonomousToolResult {
    AutonomousToolResult {
        tool_name: AUTONOMOUS_TOOL_PROCESS_MANAGER.into(),
        summary: input.message.clone(),
        command_result: None,
        output: AutonomousToolOutput::ProcessManager(AutonomousProcessManagerOutput {
            action: input.action,
            phase: PROCESS_MANAGER_PHASE.into(),
            spawned: input.spawned,
            process_id: input.process_id,
            processes: input.processes,
            chunks: input.chunks,
            next_cursor: input.next_cursor,
            policy: input.policy,
            contract: process_manager_contract(),
            message: input.message,
        }),
    }
}

fn validate_process_manager_request(
    request: &AutonomousProcessManagerRequest,
) -> CommandResult<()> {
    match request.action {
        AutonomousProcessManagerAction::Start => {
            if request.argv.is_empty() || request.argv[0].trim().is_empty() {
                return Err(CommandError::user_fixable(
                    "autonomous_tool_process_manager_start_invalid",
                    "Cadence requires process_manager start requests to include a non-empty argv[0].",
                ));
            }
            validate_argv_contract(&request.argv)?;
        }
        AutonomousProcessManagerAction::Status
        | AutonomousProcessManagerAction::Output
        | AutonomousProcessManagerAction::Digest
        | AutonomousProcessManagerAction::WaitForReady
        | AutonomousProcessManagerAction::Signal
        | AutonomousProcessManagerAction::Kill
        | AutonomousProcessManagerAction::Restart => {
            validate_non_empty(
                request.process_id.as_deref().unwrap_or_default(),
                "processId",
            )?;
        }
        AutonomousProcessManagerAction::Send | AutonomousProcessManagerAction::SendAndWait => {
            validate_non_empty(
                request.process_id.as_deref().unwrap_or_default(),
                "processId",
            )?;
            validate_non_empty(request.input.as_deref().unwrap_or_default(), "input")?;
        }
        AutonomousProcessManagerAction::GroupStatus => {
            validate_non_empty(request.group.as_deref().unwrap_or_default(), "group")?;
        }
        AutonomousProcessManagerAction::List => {}
    }

    if let Some(cwd) = request.cwd.as_deref() {
        normalize_relative_path(cwd, "cwd")?;
    }
    if let Some(label) = request.label.as_deref() {
        validate_non_empty(label, "label")?;
    }
    if let Some(process_type) = request.process_type.as_deref() {
        validate_non_empty(process_type, "processType")?;
    }
    if let Some(signal) = request.signal.as_deref() {
        validate_non_empty(signal, "signal")?;
    }
    if let Some(wait_pattern) = request.wait_pattern.as_deref() {
        validate_non_empty(wait_pattern, "waitPattern")?;
    }
    if let Some(wait_url) = request.wait_url.as_deref() {
        validate_non_empty(wait_url, "waitUrl")?;
    }

    Ok(())
}

fn validate_phase_1_scope(request: &AutonomousProcessManagerRequest) -> CommandResult<()> {
    if request.target_ownership == Some(AutonomousProcessOwnershipScope::External) {
        return Err(CommandError::user_fixable(
            "autonomous_tool_process_manager_external_unsupported",
            "Cadence phase 1 process_manager only controls Cadence-owned processes.",
        ));
    }
    if request.shell_mode {
        return Err(CommandError::user_fixable(
            "autonomous_tool_process_manager_shell_mode_unsupported",
            "Cadence phase 1 process_manager starts explicit argv commands only; shellMode is not supported yet.",
        ));
    }
    if request.persistent {
        return Err(CommandError::user_fixable(
            "autonomous_tool_process_manager_persistent_unsupported",
            "Cadence phase 1 process_manager does not support persistent background processes yet.",
        ));
    }
    match request.action {
        AutonomousProcessManagerAction::Start
        | AutonomousProcessManagerAction::List
        | AutonomousProcessManagerAction::Status
        | AutonomousProcessManagerAction::Output
        | AutonomousProcessManagerAction::Kill => Ok(()),
        action => Err(unsupported_phase_1_action(action)),
    }
}

fn validate_argv_contract(argv: &[String]) -> CommandResult<()> {
    if argv.iter().any(|argument| argument.contains('\0')) {
        return Err(CommandError::user_fixable(
            "autonomous_tool_process_manager_start_invalid",
            "Cadence refused a process_manager command that contained a NUL byte.",
        ));
    }

    let _redacted = redact_command_argv_for_persistence(argv);
    let mut probe = Command::new(&argv[0]);
    apply_sanitized_command_environment(&mut probe);
    Ok(())
}

fn unsupported_phase_1_action(action: AutonomousProcessManagerAction) -> CommandError {
    CommandError::user_fixable(
        "autonomous_tool_process_manager_action_unsupported",
        format!(
            "Cadence phase 1 process_manager supports start, list, status, output, and kill; `{}` is planned for a later phase.",
            process_manager_action_label(action)
        ),
    )
}

pub(super) fn process_manager_contract() -> AutonomousProcessManagerContract {
    AutonomousProcessManagerContract {
        phase: PROCESS_MANAGER_PHASE.into(),
        supported_actions: vec![
            AutonomousProcessManagerAction::Start,
            AutonomousProcessManagerAction::List,
            AutonomousProcessManagerAction::Status,
            AutonomousProcessManagerAction::Output,
            AutonomousProcessManagerAction::Kill,
        ],
        ownership_fields: vec![
            "threadId".into(),
            "sessionId".into(),
            "repoId".into(),
            "userId".into(),
            "scope".into(),
        ],
        risk_levels: vec![
            AutonomousProcessActionRiskLevel::Observe,
            AutonomousProcessActionRiskLevel::RunOwned,
            AutonomousProcessActionRiskLevel::SignalOwned,
            AutonomousProcessActionRiskLevel::SignalExternal,
            AutonomousProcessActionRiskLevel::PersistentBackground,
            AutonomousProcessActionRiskLevel::SystemRead,
            AutonomousProcessActionRiskLevel::OsAutomation,
        ],
        output_limits: AutonomousProcessOutputLimits {
            recent_output_ring_bytes: RECENT_OUTPUT_RING_BYTES,
            recent_output_ring_chunks: RECENT_OUTPUT_RING_CHUNKS,
            full_output_artifact_threshold_bytes: FULL_OUTPUT_ARTIFACT_THRESHOLD_BYTES,
            excerpt_bytes: PROCESS_OUTPUT_EXCERPT_BYTES,
            cursor_kind: "monotonic_output_cursor".into(),
        },
        persistence: AutonomousProcessPersistenceContract {
            persist_metadata: true,
            persist_output_chunks: true,
            redact_before_persistence: true,
            persist_policy_trace: true,
            full_output_artifacts: false,
        },
        lifecycle: AutonomousProcessLifecycleContract {
            app_shutdown: "terminate_non_persistent_cadence_owned_process_trees".into(),
            thread_switch: "reinject_owned_process_digest_without_granting_new_control".into(),
            session_compaction: "persist_metadata_and_reinject_digest_with_output_cursors".into(),
            crash_recovery: "owned_processes_marked_unknown_until_reobserved".into(),
        },
    }
}

fn process_manager_action_label(action: AutonomousProcessManagerAction) -> &'static str {
    match action {
        AutonomousProcessManagerAction::Start => "start",
        AutonomousProcessManagerAction::List => "list",
        AutonomousProcessManagerAction::Status => "status",
        AutonomousProcessManagerAction::Output => "output",
        AutonomousProcessManagerAction::Digest => "digest",
        AutonomousProcessManagerAction::WaitForReady => "wait_for_ready",
        AutonomousProcessManagerAction::Send => "send",
        AutonomousProcessManagerAction::SendAndWait => "send_and_wait",
        AutonomousProcessManagerAction::Signal => "signal",
        AutonomousProcessManagerAction::Kill => "kill",
        AutonomousProcessManagerAction::Restart => "restart",
        AutonomousProcessManagerAction::GroupStatus => "group_status",
    }
}

fn normalized_process_id(request: &AutonomousProcessManagerRequest) -> CommandResult<String> {
    let process_id = request.process_id.as_deref().unwrap_or_default().trim();
    validate_non_empty(process_id, "processId")?;
    Ok(process_id.to_owned())
}

fn clean_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn sanitized_env_summary() -> Vec<String> {
    let mut keys = SAFE_COMMAND_ENV_KEYS
        .iter()
        .filter(|key| env::var_os(key).is_some())
        .map(|key| (*key).to_owned())
        .collect::<Vec<_>>();
    if env::var_os("PATH").is_none() {
        keys.push("PATH".into());
    }
    keys.push("CADENCE_AGENT_SANITIZED_ENV".into());
    keys.sort();
    keys.dedup();
    keys
}

fn default_process_output_read_bytes() -> usize {
    PROCESS_OUTPUT_EXCERPT_BYTES
}

fn process_policy_from_command(
    command_policy: AutonomousCommandPolicyTrace,
) -> AutonomousProcessManagerPolicyTrace {
    AutonomousProcessManagerPolicyTrace {
        risk_level: AutonomousProcessActionRiskLevel::RunOwned,
        approval_required: command_policy.outcome != AutonomousCommandPolicyOutcome::Allowed,
        code: command_policy.code,
        reason: command_policy.reason,
    }
}

fn process_policy_requiring_command_approval(
    command_policy: AutonomousCommandPolicyTrace,
) -> AutonomousProcessManagerPolicyTrace {
    AutonomousProcessManagerPolicyTrace {
        risk_level: AutonomousProcessActionRiskLevel::RunOwned,
        approval_required: true,
        code: command_policy.code,
        reason: command_policy.reason,
    }
}

fn operator_approved_command_policy(
    mut policy: AutonomousCommandPolicyTrace,
    argv: &[String],
) -> AutonomousCommandPolicyTrace {
    policy.outcome = AutonomousCommandPolicyOutcome::Allowed;
    policy.code = "policy_allowed_after_operator_approval".into();
    policy.reason = format!(
        "Operator approval allowed previously escalated command `{}` to run.",
        render_command_for_summary(argv)
    );
    policy
}

fn render_command_for_summary(argv: &[String]) -> String {
    render_command_for_persistence(argv)
}

fn unstarted_process_metadata(
    argv: &[String],
    cwd: String,
    label: Option<String>,
    process_type: Option<String>,
    group: Option<String>,
) -> AutonomousProcessMetadata {
    AutonomousProcessMetadata {
        process_id: "unstarted".into(),
        pid: None,
        process_group_id: None,
        label: clean_optional_string(label.as_deref()),
        process_type: clean_optional_string(process_type.as_deref()),
        group: clean_optional_string(group.as_deref()),
        owner: AutonomousProcessOwner {
            thread_id: None,
            session_id: None,
            repo_id: None,
            user_id: None,
            scope: AutonomousProcessOwnershipScope::CadenceOwned,
        },
        command: AutonomousProcessCommandMetadata {
            argv: redact_command_argv_for_persistence(argv),
            shell_mode: false,
            cwd,
            sanitized_env: sanitized_env_summary(),
        },
        status: AutonomousProcessStatus::Starting,
        started_at: None,
        exited_at: None,
        exit_code: None,
        output_cursor: Some(0),
        detected_urls: Vec::new(),
        detected_ports: Vec::new(),
        recent_errors: Vec::new(),
        recent_warnings: Vec::new(),
        readiness: AutonomousProcessReadinessState {
            ready: false,
            detector: None,
            matched: None,
        },
        restart_count: 0,
    }
}

fn prune_process_output_chunks(chunks: &mut Vec<AutonomousProcessOutputChunk>) {
    let mut total_bytes = chunks.iter().map(process_output_chunk_bytes).sum::<usize>();
    let mut drop_count = 0;
    while chunks.len().saturating_sub(drop_count) > RECENT_OUTPUT_RING_CHUNKS
        || total_bytes > RECENT_OUTPUT_RING_BYTES
    {
        let Some(chunk) = chunks.get(drop_count) else {
            break;
        };
        total_bytes = total_bytes.saturating_sub(process_output_chunk_bytes(chunk));
        drop_count += 1;
    }

    if drop_count > 0 {
        chunks.drain(0..drop_count);
    }
}

fn process_output_chunk_bytes(chunk: &AutonomousProcessOutputChunk) -> usize {
    chunk.text.as_deref().map(str::len).unwrap_or_default()
}

#[derive(Debug)]
struct SanitizedProcessOutput {
    text: Option<String>,
    truncated: bool,
    redacted: bool,
}

fn sanitize_process_output(bytes: &[u8], truncated: bool) -> SanitizedProcessOutput {
    if bytes.is_empty() {
        return SanitizedProcessOutput {
            text: None,
            truncated,
            redacted: false,
        };
    }

    let decoded = String::from_utf8_lossy(bytes).into_owned();
    if find_prohibited_persistence_content(&decoded).is_some() {
        return SanitizedProcessOutput {
            text: Some(REDACTED_PROCESS_OUTPUT_SUMMARY.into()),
            truncated,
            redacted: true,
        };
    }

    let collapsed = decoded.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = collapsed.trim();
    if trimmed.is_empty() {
        return SanitizedProcessOutput {
            text: None,
            truncated,
            redacted: false,
        };
    }

    SanitizedProcessOutput {
        text: Some(truncate_chars(trimmed, PROCESS_OUTPUT_EXCERPT_BYTES)),
        truncated,
        redacted: false,
    }
}

fn truncate_chars(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }

    let truncated = value
        .chars()
        .take(limit.saturating_sub(1))
        .collect::<String>();
    format!("{truncated}…")
}

fn spawn_owned_process_reader(
    process: Arc<OwnedProcess>,
    mut reader: impl Read + Send + 'static,
    stream: AutonomousProcessOutputStream,
) {
    thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let capture = sanitize_process_output(&buffer[..read], false);
                    let _ = process.push_chunk(stream, capture);
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => {
                    let capture = SanitizedProcessOutput {
                        text: Some(format!("Owned process output read failed: {error}")),
                        truncated: false,
                        redacted: false,
                    };
                    let _ = process.push_chunk(stream, capture);
                    break;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use std::fs;
    use std::{thread, time::Duration};

    use super::*;
    use crate::{
        commands::{
            RuntimeRunActiveControlSnapshotDto, RuntimeRunApprovalModeDto,
            RuntimeRunControlStateDto,
        },
        runtime::AutonomousToolRequest,
    };

    #[test]
    fn owned_process_can_start_output_list_status_and_kill() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let runtime = test_runtime(tempdir.path());

        let start = runtime
            .execute(AutonomousToolRequest::ProcessManager(start_request(
                long_running_output_command(),
            )))
            .expect("start process");
        let start_output = process_manager_output(start);
        assert!(start_output.spawned);
        let process_id = start_output.process_id.clone().expect("process id");

        let output = wait_for_process_output(&runtime, &process_id, "ready");
        assert!(
            output
                .chunks
                .iter()
                .filter_map(|chunk| chunk.text.as_deref())
                .any(|text| text.contains("ready")),
            "expected ready output chunk"
        );

        let list = process_manager_output(
            runtime
                .execute(AutonomousToolRequest::ProcessManager(base_request(
                    AutonomousProcessManagerAction::List,
                )))
                .expect("list processes"),
        );
        assert!(
            list.processes
                .iter()
                .any(|process| process.process_id == process_id),
            "started process should appear in list"
        );

        let mut status_request = base_request(AutonomousProcessManagerAction::Status);
        status_request.process_id = Some(process_id.clone());
        let status = process_manager_output(
            runtime
                .execute(AutonomousToolRequest::ProcessManager(status_request))
                .expect("status process"),
        );
        assert_eq!(status.processes[0].process_id, process_id);
        assert!(matches!(
            status.processes[0].status,
            AutonomousProcessStatus::Running | AutonomousProcessStatus::Exited
        ));

        let mut kill_request = base_request(AutonomousProcessManagerAction::Kill);
        kill_request.process_id = Some(process_id.clone());
        let kill = process_manager_output(
            runtime
                .execute(AutonomousToolRequest::ProcessManager(kill_request))
                .expect("kill process"),
        );
        assert_eq!(kill.processes[0].process_id, process_id);
        assert!(matches!(
            kill.processes[0].status,
            AutonomousProcessStatus::Killed | AutonomousProcessStatus::Exited
        ));

        let list_after_kill = process_manager_output(
            runtime
                .execute(AutonomousToolRequest::ProcessManager(base_request(
                    AutonomousProcessManagerAction::List,
                )))
                .expect("list after kill"),
        );
        assert!(
            list_after_kill
                .processes
                .iter()
                .all(|process| process.process_id != process_id),
            "killed process should be removed from registry"
        );
    }

    #[cfg(unix)]
    #[test]
    fn killing_owned_process_terminates_child_tree() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let runtime = test_runtime(tempdir.path());
        let pid_file = tempdir.path().join("child.pid");
        let command = vec![
            "sh".into(),
            "-c".into(),
            "sleep 30 & echo $! > child.pid; wait".into(),
        ];

        let start = runtime
            .execute_approved(AutonomousToolRequest::ProcessManager(start_request(
                command,
            )))
            .expect("start process tree");
        let start_output = process_manager_output(start);
        let process_id = start_output.process_id.clone().expect("process id");
        let child_pid = wait_for_child_pid(&pid_file);
        assert!(process_exists(child_pid), "child process should be alive");

        let mut kill_request = base_request(AutonomousProcessManagerAction::Kill);
        kill_request.process_id = Some(process_id);
        let _ = runtime
            .execute(AutonomousToolRequest::ProcessManager(kill_request))
            .expect("kill process tree");

        for _ in 0..20 {
            if !process_exists(child_pid) {
                return;
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("child process {child_pid} survived process-manager kill");
    }

    fn test_runtime(repo_root: &std::path::Path) -> AutonomousToolRuntime {
        AutonomousToolRuntime::new(repo_root)
            .expect("runtime")
            .with_runtime_run_controls(RuntimeRunControlStateDto {
                active: RuntimeRunActiveControlSnapshotDto {
                    provider_profile_id: None,
                    model_id: "test-model".into(),
                    thinking_effort: None,
                    approval_mode: RuntimeRunApprovalModeDto::Yolo,
                    plan_mode_required: false,
                    revision: 1,
                    applied_at: now_timestamp(),
                },
                pending: None,
            })
    }

    fn long_running_output_command() -> Vec<String> {
        #[cfg(unix)]
        {
            vec![
                "sh".into(),
                "-c".into(),
                "printf 'ready\\n'; sleep 30".into(),
            ]
        }
        #[cfg(windows)]
        {
            vec![
                "cmd".into(),
                "/C".into(),
                "echo ready && timeout /T 30 > NUL".into(),
            ]
        }
    }

    fn start_request(argv: Vec<String>) -> AutonomousProcessManagerRequest {
        let mut request = base_request(AutonomousProcessManagerAction::Start);
        request.argv = argv;
        request
    }

    fn base_request(action: AutonomousProcessManagerAction) -> AutonomousProcessManagerRequest {
        AutonomousProcessManagerRequest {
            action,
            process_id: None,
            group: None,
            label: None,
            process_type: None,
            argv: Vec::new(),
            cwd: None,
            shell_mode: false,
            target_ownership: None,
            persistent: false,
            timeout_ms: None,
            after_cursor: None,
            max_bytes: None,
            input: None,
            wait_pattern: None,
            wait_port: None,
            wait_url: None,
            signal: None,
        }
    }

    fn wait_for_process_output(
        runtime: &AutonomousToolRuntime,
        process_id: &str,
        needle: &str,
    ) -> AutonomousProcessManagerOutput {
        let mut last = None;
        for _ in 0..20 {
            let mut request = base_request(AutonomousProcessManagerAction::Output);
            request.process_id = Some(process_id.into());
            let output = process_manager_output(
                runtime
                    .execute(AutonomousToolRequest::ProcessManager(request))
                    .expect("read output"),
            );
            if output
                .chunks
                .iter()
                .filter_map(|chunk| chunk.text.as_deref())
                .any(|text| text.contains(needle))
            {
                return output;
            }
            last = Some(output);
            thread::sleep(Duration::from_millis(50));
        }
        last.expect("output attempts")
    }

    fn process_manager_output(result: AutonomousToolResult) -> AutonomousProcessManagerOutput {
        match result.output {
            AutonomousToolOutput::ProcessManager(output) => output,
            other => panic!("expected process manager output, got {other:?}"),
        }
    }

    #[cfg(unix)]
    fn wait_for_child_pid(path: &std::path::Path) -> i32 {
        for _ in 0..20 {
            if let Ok(value) = fs::read_to_string(path) {
                if let Ok(pid) = value.trim().parse::<i32>() {
                    return pid;
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
        panic!("child pid file was not written")
    }

    #[cfg(unix)]
    fn process_exists(pid: i32) -> bool {
        let result = unsafe { libc::kill(pid, 0) };
        if result == 0 {
            return true;
        }
        std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
    }
}
