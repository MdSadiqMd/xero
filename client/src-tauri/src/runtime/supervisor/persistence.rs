use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{
    auth::now_timestamp,
    commands::CommandError,
    db::project_store::{
        self, RuntimeRunDiagnosticRecord, RuntimeRunRecord, RuntimeRunSnapshotRecord,
        RuntimeRunStatus, RuntimeRunTransportLiveness, RuntimeRunTransportRecord,
        RuntimeRunUpsertRecord,
    },
};

use super::SidecarSharedState;
use crate::runtime::protocol::{
    SupervisorProcessStatus, SupervisorProtocolDiagnostic, SUPERVISOR_KIND_DETACHED_PTY,
    SUPERVISOR_TRANSPORT_KIND_TCP,
};

pub(super) fn protocol_diagnostic_into_record(
    diagnostic: SupervisorProtocolDiagnostic,
) -> RuntimeRunDiagnosticRecord {
    RuntimeRunDiagnosticRecord {
        code: diagnostic.code,
        message: diagnostic.message,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AppliedPendingControlTransition {
    pub revision: u32,
    pub applied_at: String,
    pub prompt_consumed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PendingControlApplyOutcome {
    NoPending,
    Applied(AppliedPendingControlTransition),
    PersistFailed { code: String, message: String },
}

pub(super) fn apply_pending_controls_at_boundary(
    repo_root: &Path,
    shared: &Arc<Mutex<SidecarSharedState>>,
    persistence_lock: &Arc<Mutex<()>>,
) -> PendingControlApplyOutcome {
    const APPLY_FAILED_CODE: &str = "runtime_run_controls_apply_failed";
    const APPLY_FAILED_MESSAGE: &str = "Cadence kept queued runtime-run controls pending because it could not persist the model-call boundary transition.";

    let snapshot = shared.lock().expect("sidecar state lock poisoned").clone();
    let Some(pending) = snapshot.control_state.pending.clone() else {
        return PendingControlApplyOutcome::NoPending;
    };

    let applied_at = now_timestamp();
    let prompt_consumed = pending.queued_prompt.is_some();
    {
        let mut state = shared.lock().expect("sidecar state lock poisoned");
        state.control_state.active = project_store::RuntimeRunActiveControlSnapshotRecord {
            model_id: pending.model_id.clone(),
            thinking_effort: pending.thinking_effort.clone(),
            approval_mode: pending.approval_mode.clone(),
            revision: pending.revision,
            applied_at: applied_at.clone(),
        };
        state.control_state.pending = None;
        state.last_error = None;
    }

    match persist_runtime_row_from_shared(repo_root, shared, persistence_lock) {
        Ok(_) => PendingControlApplyOutcome::Applied(AppliedPendingControlTransition {
            revision: pending.revision,
            applied_at,
            prompt_consumed,
        }),
        Err(_) => {
            {
                let mut state = shared.lock().expect("sidecar state lock poisoned");
                state.control_state = snapshot.control_state.clone();
                state.last_error = Some(SupervisorProtocolDiagnostic {
                    code: APPLY_FAILED_CODE.into(),
                    message: APPLY_FAILED_MESSAGE.into(),
                });
            }
            let _ = persist_runtime_row_from_shared(repo_root, shared, persistence_lock);
            PendingControlApplyOutcome::PersistFailed {
                code: APPLY_FAILED_CODE.into(),
                message: APPLY_FAILED_MESSAGE.into(),
            }
        }
    }
}

pub(super) fn persist_sidecar_exit(
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
        provider_id,
        started_at,
        endpoint,
        heartbeat_at,
        stopped_at,
        next_sequence,
        last_error,
        control_state,
    ) = {
        let mut snapshot = shared.lock().expect("sidecar state lock poisoned");
        snapshot.last_checkpoint_sequence = snapshot.last_checkpoint_sequence.saturating_add(1);
        snapshot.last_checkpoint_at = Some(now_timestamp());
        (
            snapshot.project_id.clone(),
            snapshot.run_id.clone(),
            snapshot.runtime_kind.clone(),
            snapshot.provider_id.clone(),
            snapshot.started_at.clone(),
            snapshot.endpoint.clone(),
            snapshot.last_heartbeat_at.clone(),
            snapshot.stopped_at.clone(),
            snapshot.last_checkpoint_sequence,
            snapshot
                .last_error
                .clone()
                .map(protocol_diagnostic_into_record),
            snapshot.control_state.clone(),
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
                    provider_id: provider_id.clone(),
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
                control_state: Some(control_state.clone()),
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
                        provider_id: shared
                            .lock()
                            .expect("sidecar state lock poisoned")
                            .provider_id
                            .clone(),
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
                    control_state: Some(
                        shared
                            .lock()
                            .expect("sidecar state lock poisoned")
                            .control_state
                            .clone(),
                    ),
                },
            )
        }
        Err(error) => Err(error),
    }
}

pub(super) fn persist_runtime_row_from_shared(
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
                provider_id: snapshot.provider_id,
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
            control_state: Some(snapshot.control_state),
        },
    )
}
