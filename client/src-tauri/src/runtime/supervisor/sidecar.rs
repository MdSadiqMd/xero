use std::{
    net::TcpListener,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};

use crate::{
    auth::now_timestamp,
    commands::{validate_non_empty, CommandError},
    db::project_store::{
        self, RuntimeRunRecord, RuntimeRunSnapshotRecord, RuntimeRunStatus,
        RuntimeRunTransportLiveness, RuntimeRunTransportRecord, RuntimeRunUpsertRecord,
    },
};

use super::{
    control::spawn_control_listener, diagnostic_live_event, emit_interactive_boundary_if_detected,
    emit_normalized_events, protocol_diagnostic_into_record, write_json_line, PtyEventNormalizer,
    RuntimeSupervisorSidecarArgs, SharedPtyWriter, SidecarSharedState, SupervisorEventHub,
};
use super::{HEARTBEAT_INTERVAL, TERMINAL_ATTACH_GRACE_PERIOD};
use crate::runtime::protocol::{
    SupervisorProcessStatus, SupervisorProtocolDiagnostic, SupervisorStartupMessage,
    SUPERVISOR_KIND_DETACHED_PTY, SUPERVISOR_PROTOCOL_VERSION, SUPERVISOR_TRANSPORT_KIND_TCP,
};

pub(super) fn run_supervisor_sidecar_from_env() -> Result<(), CommandError> {
    let args = parse_sidecar_args(std::env::args().skip(1))?;
    run_supervisor_sidecar(args)
}

fn parse_sidecar_args(
    args: impl IntoIterator<Item = String>,
) -> Result<RuntimeSupervisorSidecarArgs, CommandError> {
    let mut project_id = None;
    let mut repo_root = None;
    let mut runtime_kind = None;
    let mut run_id = None;
    let mut session_id = None;
    let mut flow_id = None;
    let mut program = None;
    let mut command_args = Vec::new();

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project-id" => project_id = args.next(),
            "--repo-root" => repo_root = args.next().map(PathBuf::from),
            "--runtime-kind" => runtime_kind = args.next(),
            "--run-id" => run_id = args.next(),
            "--session-id" => session_id = args.next(),
            "--flow-id" => flow_id = args.next(),
            "--program" => program = args.next(),
            "--command-arg" => {
                let Some(value) = args.next() else {
                    return Err(CommandError::user_fixable(
                        "runtime_supervisor_request_invalid",
                        "Cadence received a detached supervisor command arg flag without a value.",
                    ));
                };
                command_args.push(value);
            }
            other => {
                return Err(CommandError::user_fixable(
                    "runtime_supervisor_request_invalid",
                    format!("Cadence received unsupported detached supervisor argument `{other}`."),
                ))
            }
        }
    }

    let args = RuntimeSupervisorSidecarArgs {
        project_id: project_id.ok_or_else(|| CommandError::invalid_request("projectId"))?,
        repo_root: repo_root.ok_or_else(|| CommandError::invalid_request("repoRoot"))?,
        runtime_kind: runtime_kind.ok_or_else(|| CommandError::invalid_request("runtimeKind"))?,
        run_id: run_id.ok_or_else(|| CommandError::invalid_request("runId"))?,
        session_id: session_id.ok_or_else(|| CommandError::invalid_request("sessionId"))?,
        flow_id,
        program: program.ok_or_else(|| CommandError::invalid_request("program"))?,
        args: command_args,
    };

    validate_non_empty(&args.project_id, "projectId")?;
    validate_non_empty(&args.runtime_kind, "runtimeKind")?;
    validate_non_empty(&args.run_id, "runId")?;
    validate_non_empty(&args.session_id, "sessionId")?;
    if let Some(flow_id) = args.flow_id.as_deref() {
        validate_non_empty(flow_id, "flowId")?;
    }
    validate_non_empty(&args.program, "program")?;

    Ok(args)
}

fn run_supervisor_sidecar(args: RuntimeSupervisorSidecarArgs) -> Result<(), CommandError> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).map_err(|_| {
        CommandError::retryable(
            "runtime_supervisor_bind_failed",
            "Cadence could not bind the detached PTY supervisor control listener.",
        )
    })?;
    listener.set_nonblocking(true).map_err(|_| {
        CommandError::retryable(
            "runtime_supervisor_bind_failed",
            "Cadence could not configure the detached PTY supervisor control listener.",
        )
    })?;
    let endpoint = listener
        .local_addr()
        .map_err(|_| {
            CommandError::retryable(
                "runtime_supervisor_bind_failed",
                "Cadence could not read the detached PTY supervisor control listener address.",
            )
        })?
        .to_string();

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize::default()).map_err(|_| {
        CommandError::retryable(
            "runtime_supervisor_pty_failed",
            "Cadence could not allocate a PTY for the detached supervisor.",
        )
    })?;
    let mut builder = CommandBuilder::new(&args.program);
    builder.args(&args.args);
    builder.cwd(&args.repo_root);

    let mut child = match pair.slave.spawn_command(builder) {
        Ok(child) => child,
        Err(_) => {
            emit_startup_message(&SupervisorStartupMessage::Error {
                protocol_version: SUPERVISOR_PROTOCOL_VERSION,
                code: "runtime_supervisor_pty_failed".into(),
                message:
                    "Cadence could not spawn the requested command inside the detached PTY supervisor."
                        .into(),
                retryable: true,
            })?;
            return Ok(());
        }
    };

    let writer = match pair.master.take_writer() {
        Ok(writer) => writer,
        Err(_) => {
            let _ = child.kill();
            emit_startup_message(&SupervisorStartupMessage::Error {
                protocol_version: SUPERVISOR_PROTOCOL_VERSION,
                code: "runtime_supervisor_writer_unavailable".into(),
                message: "Cadence could not take exclusive ownership of the detached PTY writer."
                    .into(),
                retryable: true,
            })?;
            return Ok(());
        }
    };
    let writer: SharedPtyWriter = Arc::new(Mutex::new(writer));

    let child_pid = child.process_id();
    let mut killer = child.clone_killer();
    let started_at = now_timestamp();

    let initial_snapshot = project_store::upsert_runtime_run(
        &args.repo_root,
        &RuntimeRunUpsertRecord {
            run: RuntimeRunRecord {
                project_id: args.project_id.clone(),
                run_id: args.run_id.clone(),
                runtime_kind: args.runtime_kind.clone(),
                supervisor_kind: SUPERVISOR_KIND_DETACHED_PTY.into(),
                status: RuntimeRunStatus::Running,
                transport: RuntimeRunTransportRecord {
                    kind: SUPERVISOR_TRANSPORT_KIND_TCP.into(),
                    endpoint: endpoint.clone(),
                    liveness: RuntimeRunTransportLiveness::Reachable,
                },
                started_at: started_at.clone(),
                last_heartbeat_at: Some(started_at.clone()),
                stopped_at: None,
                last_error: None,
                updated_at: started_at.clone(),
            },
            checkpoint: None,
        },
    )
    .map_err(|_| {
        let _ = killer.kill();
        CommandError::retryable(
            "runtime_supervisor_persist_failed",
            "Cadence could not persist detached supervisor startup metadata.",
        )
    })?;

    let shared = Arc::new(Mutex::new(SidecarSharedState {
        project_id: args.project_id.clone(),
        run_id: args.run_id.clone(),
        runtime_kind: args.runtime_kind.clone(),
        session_id: args.session_id.clone(),
        flow_id: args.flow_id.clone(),
        endpoint: endpoint.clone(),
        started_at: initial_snapshot.run.started_at.clone(),
        child_pid,
        status: SupervisorProcessStatus::Running,
        stop_requested: false,
        last_heartbeat_at: initial_snapshot.run.last_heartbeat_at.clone(),
        last_checkpoint_sequence: initial_snapshot.last_checkpoint_sequence,
        last_checkpoint_at: initial_snapshot.last_checkpoint_at.clone(),
        last_error: None,
        stopped_at: None,
        next_boundary_serial: 0,
        active_boundary: None,
    }));
    let event_hub = Arc::new(Mutex::new(SupervisorEventHub::default()));
    let persistence_lock = Arc::new(Mutex::new(()));
    let shutdown = Arc::new(AtomicBool::new(false));

    let control_thread = spawn_control_listener(
        listener,
        shared.clone(),
        event_hub.clone(),
        writer.clone(),
        shutdown.clone(),
        killer,
    );

    emit_startup_message(&SupervisorStartupMessage::Ready {
        protocol_version: SUPERVISOR_PROTOCOL_VERSION,
        project_id: args.project_id.clone(),
        run_id: args.run_id.clone(),
        supervisor_kind: SUPERVISOR_KIND_DETACHED_PTY.into(),
        transport_kind: SUPERVISOR_TRANSPORT_KIND_TCP.into(),
        endpoint: endpoint.clone(),
        started_at: initial_snapshot.run.started_at.clone(),
        supervisor_pid: std::process::id(),
        child_pid,
        status: SupervisorProcessStatus::Running,
    })?;

    let reader_thread = spawn_pty_reader(
        pair.master.try_clone_reader().map_err(|_| {
            CommandError::retryable(
                "runtime_supervisor_pty_failed",
                "Cadence could not clone the detached PTY supervisor reader.",
            )
        })?,
        args.repo_root.clone(),
        shared.clone(),
        event_hub.clone(),
        persistence_lock.clone(),
        shutdown.clone(),
    );
    let heartbeat_thread = spawn_heartbeat_loop(
        args.repo_root.clone(),
        shared.clone(),
        persistence_lock.clone(),
        shutdown.clone(),
    );

    let exit_status = child.wait().map_err(|_| {
        shutdown.store(true, Ordering::SeqCst);
        CommandError::retryable(
            "runtime_supervisor_wait_failed",
            "Cadence lost the detached PTY child before it returned an exit status.",
        )
    })?;

    persist_sidecar_exit(&args.repo_root, &shared, &persistence_lock, exit_status)?;
    thread::sleep(TERMINAL_ATTACH_GRACE_PERIOD);
    shutdown.store(true, Ordering::SeqCst);

    let _ = control_thread.join();
    let _ = reader_thread.join();
    let _ = heartbeat_thread.join();

    Ok(())
}

fn emit_startup_message(message: &SupervisorStartupMessage) -> Result<(), CommandError> {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    write_json_line(&mut stdout, message).map_err(|_| {
        CommandError::retryable(
            "runtime_supervisor_handshake_write_failed",
            "Cadence could not emit the detached PTY supervisor startup handshake.",
        )
    })
}

fn spawn_pty_reader(
    mut reader: Box<dyn std::io::Read + Send>,
    repo_root: PathBuf,
    shared: Arc<Mutex<SidecarSharedState>>,
    event_hub: Arc<Mutex<SupervisorEventHub>>,
    persistence_lock: Arc<Mutex<()>>,
    shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        let mut normalizer = PtyEventNormalizer::default();

        while !shutdown.load(Ordering::SeqCst) {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    emit_normalized_events(
                        &repo_root,
                        &shared,
                        &event_hub,
                        &persistence_lock,
                        normalizer.finish(),
                    );
                    break;
                }
                Ok(bytes_read) => {
                    emit_normalized_events(
                        &repo_root,
                        &shared,
                        &event_hub,
                        &persistence_lock,
                        normalizer.push_chunk(&buffer[..bytes_read]),
                    );
                    emit_interactive_boundary_if_detected(
                        &repo_root,
                        &shared,
                        &event_hub,
                        &persistence_lock,
                        &mut normalizer,
                    );
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    emit_normalized_events(
                        &repo_root,
                        &shared,
                        &event_hub,
                        &persistence_lock,
                        vec![diagnostic_live_event(
                            "runtime_supervisor_reader_failed",
                            "Runtime stream read failed",
                            "Cadence lost the detached PTY reader before the child exited.",
                        )],
                    );
                    let _ = persist_sidecar_runtime_error(
                        &repo_root,
                        &shared,
                        &persistence_lock,
                        "runtime_supervisor_reader_failed",
                        "Cadence lost the detached PTY reader before the child exited.",
                    );
                    break;
                }
            }
        }
    })
}

fn spawn_heartbeat_loop(
    repo_root: PathBuf,
    shared: Arc<Mutex<SidecarSharedState>>,
    persistence_lock: Arc<Mutex<()>>,
    shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        while !shutdown.load(Ordering::SeqCst) {
            thread::sleep(HEARTBEAT_INTERVAL);
            if shutdown.load(Ordering::SeqCst) {
                break;
            }

            {
                let mut snapshot = shared.lock().expect("sidecar state lock poisoned");
                if matches!(
                    snapshot.status,
                    SupervisorProcessStatus::Stopped | SupervisorProcessStatus::Failed
                ) {
                    break;
                }
                snapshot.last_heartbeat_at = Some(now_timestamp());
            }

            let _ = persist_runtime_row_from_shared(&repo_root, &shared, &persistence_lock);
        }
    })
}

fn persist_sidecar_exit(
    repo_root: &Path,
    shared: &Arc<Mutex<SidecarSharedState>>,
    persistence_lock: &Arc<Mutex<()>>,
    exit_status: portable_pty::ExitStatus,
) -> Result<(), CommandError> {
    let stop_requested = shared
        .lock()
        .expect("sidecar state lock poisoned")
        .stop_requested;

    let (status, last_error, summary): (
        SupervisorProcessStatus,
        Option<SupervisorProtocolDiagnostic>,
        String,
    ) = if stop_requested {
        (
            SupervisorProcessStatus::Stopped,
            None,
            "PTY child stopped by supervisor request.".to_string(),
        )
    } else if exit_status.success() {
        (
            SupervisorProcessStatus::Stopped,
            None,
            "PTY child exited cleanly.".to_string(),
        )
    } else {
        (
            SupervisorProcessStatus::Failed,
            Some(SupervisorProtocolDiagnostic {
                code: "runtime_supervisor_exit_nonzero".into(),
                message: format!("PTY child exited with status {exit_status}."),
            }),
            format!("PTY child exited with status {exit_status}."),
        )
    };

    {
        let mut snapshot = shared.lock().expect("sidecar state lock poisoned");
        snapshot.status = status.clone();
        snapshot.last_error = last_error;
        snapshot.stopped_at = Some(now_timestamp());
        snapshot.last_heartbeat_at = Some(now_timestamp());
    }

    persist_runtime_row_from_shared(repo_root, shared, persistence_lock)?;
    persist_sidecar_checkpoint(
        repo_root,
        shared,
        persistence_lock,
        match status {
            SupervisorProcessStatus::Stopped => RuntimeRunStatus::Stopped,
            SupervisorProcessStatus::Failed => RuntimeRunStatus::Failed,
            SupervisorProcessStatus::Starting => RuntimeRunStatus::Starting,
            SupervisorProcessStatus::Running => RuntimeRunStatus::Running,
        },
        project_store::RuntimeRunCheckpointKind::State,
        summary,
    )?;

    Ok(())
}

pub(super) fn persist_sidecar_runtime_error(
    repo_root: &Path,
    shared: &Arc<Mutex<SidecarSharedState>>,
    persistence_lock: &Arc<Mutex<()>>,
    code: &str,
    message: &str,
) -> Result<(), CommandError> {
    {
        let mut snapshot = shared.lock().expect("sidecar state lock poisoned");
        snapshot.last_error = Some(SupervisorProtocolDiagnostic {
            code: code.into(),
            message: message.into(),
        });
    }

    persist_runtime_row_from_shared(repo_root, shared, persistence_lock).map(|_| ())
}

pub(super) fn persist_sidecar_checkpoint(
    repo_root: &Path,
    shared: &Arc<Mutex<SidecarSharedState>>,
    persistence_lock: &Arc<Mutex<()>>,
    status: RuntimeRunStatus,
    checkpoint_kind: project_store::RuntimeRunCheckpointKind,
    summary: String,
) -> Result<RuntimeRunSnapshotRecord, CommandError> {
    let (
        project_id,
        run_id,
        runtime_kind,
        started_at,
        endpoint,
        heartbeat_at,
        stopped_at,
        next_sequence,
        last_error,
    ) = {
        let mut snapshot = shared.lock().expect("sidecar state lock poisoned");
        snapshot.last_checkpoint_sequence = snapshot.last_checkpoint_sequence.saturating_add(1);
        snapshot.last_checkpoint_at = Some(now_timestamp());
        (
            snapshot.project_id.clone(),
            snapshot.run_id.clone(),
            snapshot.runtime_kind.clone(),
            snapshot.started_at.clone(),
            snapshot.endpoint.clone(),
            snapshot.last_heartbeat_at.clone(),
            snapshot.stopped_at.clone(),
            snapshot.last_checkpoint_sequence,
            snapshot
                .last_error
                .clone()
                .map(protocol_diagnostic_into_record),
        )
    };

    let attempt = {
        let _guard = persistence_lock
            .lock()
            .expect("runtime supervisor persistence lock poisoned");
        project_store::upsert_runtime_run(
            repo_root,
            &RuntimeRunUpsertRecord {
                run: RuntimeRunRecord {
                    project_id: project_id.clone(),
                    run_id: run_id.clone(),
                    runtime_kind: runtime_kind.clone(),
                    supervisor_kind: SUPERVISOR_KIND_DETACHED_PTY.into(),
                    status: status.clone(),
                    transport: RuntimeRunTransportRecord {
                        kind: SUPERVISOR_TRANSPORT_KIND_TCP.into(),
                        endpoint,
                        liveness: RuntimeRunTransportLiveness::Reachable,
                    },
                    started_at,
                    last_heartbeat_at: heartbeat_at,
                    stopped_at,
                    last_error,
                    updated_at: now_timestamp(),
                },
                checkpoint: Some(project_store::RuntimeRunCheckpointRecord {
                    project_id: project_id.clone(),
                    run_id: run_id.clone(),
                    sequence: next_sequence,
                    kind: checkpoint_kind.clone(),
                    summary: summary.clone(),
                    created_at: now_timestamp(),
                }),
            },
        )
    };

    match attempt {
        Ok(snapshot) => Ok(snapshot),
        Err(error)
            if matches!(
                error.code.as_str(),
                "runtime_run_checkpoint_invalid" | "runtime_run_request_invalid"
            ) =>
        {
            let fallback_summary = match checkpoint_kind {
                project_store::RuntimeRunCheckpointKind::ActionRequired => {
                    super::INTERACTIVE_BOUNDARY_CHECKPOINT_SUMMARY.into()
                }
                _ => super::REDACTED_SHELL_OUTPUT_SUMMARY.into(),
            };
            let _guard = persistence_lock
                .lock()
                .expect("runtime supervisor persistence lock poisoned");
            project_store::upsert_runtime_run(
                repo_root,
                &RuntimeRunUpsertRecord {
                    run: RuntimeRunRecord {
                        project_id: project_id.clone(),
                        run_id: run_id.clone(),
                        runtime_kind,
                        supervisor_kind: SUPERVISOR_KIND_DETACHED_PTY.into(),
                        status,
                        transport: RuntimeRunTransportRecord {
                            kind: SUPERVISOR_TRANSPORT_KIND_TCP.into(),
                            endpoint: shared
                                .lock()
                                .expect("sidecar state lock poisoned")
                                .endpoint
                                .clone(),
                            liveness: RuntimeRunTransportLiveness::Reachable,
                        },
                        started_at: shared
                            .lock()
                            .expect("sidecar state lock poisoned")
                            .started_at
                            .clone(),
                        last_heartbeat_at: shared
                            .lock()
                            .expect("sidecar state lock poisoned")
                            .last_heartbeat_at
                            .clone(),
                        stopped_at: shared
                            .lock()
                            .expect("sidecar state lock poisoned")
                            .stopped_at
                            .clone(),
                        last_error: shared
                            .lock()
                            .expect("sidecar state lock poisoned")
                            .last_error
                            .clone()
                            .map(protocol_diagnostic_into_record),
                        updated_at: now_timestamp(),
                    },
                    checkpoint: Some(project_store::RuntimeRunCheckpointRecord {
                        project_id,
                        run_id,
                        sequence: next_sequence,
                        kind: checkpoint_kind,
                        summary: fallback_summary,
                        created_at: now_timestamp(),
                    }),
                },
            )
        }
        Err(error) => Err(error),
    }
}

fn persist_runtime_row_from_shared(
    repo_root: &Path,
    shared: &Arc<Mutex<SidecarSharedState>>,
    persistence_lock: &Arc<Mutex<()>>,
) -> Result<RuntimeRunSnapshotRecord, CommandError> {
    let snapshot = shared.lock().expect("sidecar state lock poisoned").clone();
    let _guard = persistence_lock
        .lock()
        .expect("runtime supervisor persistence lock poisoned");
    project_store::upsert_runtime_run(
        repo_root,
        &RuntimeRunUpsertRecord {
            run: RuntimeRunRecord {
                project_id: snapshot.project_id,
                run_id: snapshot.run_id,
                runtime_kind: snapshot.runtime_kind,
                supervisor_kind: SUPERVISOR_KIND_DETACHED_PTY.into(),
                status: match snapshot.status {
                    SupervisorProcessStatus::Starting => RuntimeRunStatus::Starting,
                    SupervisorProcessStatus::Running => RuntimeRunStatus::Running,
                    SupervisorProcessStatus::Stopped => RuntimeRunStatus::Stopped,
                    SupervisorProcessStatus::Failed => RuntimeRunStatus::Failed,
                },
                transport: RuntimeRunTransportRecord {
                    kind: SUPERVISOR_TRANSPORT_KIND_TCP.into(),
                    endpoint: snapshot.endpoint,
                    liveness: RuntimeRunTransportLiveness::Reachable,
                },
                started_at: snapshot.started_at,
                last_heartbeat_at: snapshot.last_heartbeat_at,
                stopped_at: snapshot.stopped_at,
                last_error: snapshot.last_error.map(protocol_diagnostic_into_record),
                updated_at: now_timestamp(),
            },
            checkpoint: None,
        },
    )
}
