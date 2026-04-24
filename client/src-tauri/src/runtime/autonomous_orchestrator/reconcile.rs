use crate::{
    auth::now_timestamp,
    db::project_store::{
        AutonomousRunRecord, AutonomousRunSnapshotRecord, AutonomousRunStatus,
        AutonomousRunUpsertRecord, AutonomousUnitAttemptRecord, AutonomousUnitKind,
        AutonomousUnitRecord, AutonomousUnitStatus, RuntimeRunDiagnosticRecord,
        RuntimeRunSnapshotRecord, RuntimeRunStatus,
    },
    runtime::autonomous_run_state::{
        autonomous_unit_status_for_run, clone_current_attempt_artifacts,
        generate_autonomous_child_session_id,
    },
};

use super::AutonomousRuntimeReconcileIntent;

const AUTONOMOUS_DUPLICATE_START_REASON: &str =
    "Cadence reused the already-active autonomous run for this project instead of launching a duplicate supervisor.";
const AUTONOMOUS_CANCEL_REASON_CODE: &str = "autonomous_run_cancelled";
const AUTONOMOUS_CANCEL_REASON_MESSAGE: &str =
    "Operator cancelled the autonomous run from the desktop shell.";
pub(super) const AUTONOMOUS_BOUNDARY_PAUSE_CODE: &str = "autonomous_operator_action_required";

pub fn reconcile_runtime_snapshot(
    existing: Option<&AutonomousRunSnapshotRecord>,
    runtime_snapshot: &RuntimeRunSnapshotRecord,
    intent: AutonomousRuntimeReconcileIntent,
) -> AutonomousRunUpsertRecord {
    let is_same_run =
        existing.is_some_and(|existing| existing.run.run_id == runtime_snapshot.run.run_id);
    let existing_run = is_same_run.then(|| existing.expect("checked same-run autonomous snapshot"));
    let existing_unit = existing_run.and_then(|snapshot| snapshot.unit.as_ref());
    let existing_attempt = existing_run.and_then(|snapshot| snapshot.attempt.as_ref());

    let duplicate_start_detected =
        matches!(intent, AutonomousRuntimeReconcileIntent::DuplicateStart)
            || existing_run
                .map(|snapshot| snapshot.run.duplicate_start_detected)
                .unwrap_or(false);
    let duplicate_start_run_id =
        duplicate_start_detected.then(|| runtime_snapshot.run.run_id.clone());
    let duplicate_start_reason =
        duplicate_start_detected.then_some(AUTONOMOUS_DUPLICATE_START_REASON.to_string());

    let base_updated_at = if matches!(intent, AutonomousRuntimeReconcileIntent::DuplicateStart) {
        now_timestamp()
    } else {
        runtime_snapshot.run.updated_at.clone()
    };

    let last_error = runtime_snapshot.run.last_error.clone();
    let existing_blocked_boundary = existing_attempt
        .and_then(|attempt| attempt.boundary_id.clone())
        .filter(|boundary_id| !boundary_id.trim().is_empty())
        .zip(existing_run)
        .filter(|(_, snapshot)| {
            matches!(
                snapshot.run.status,
                AutonomousRunStatus::Paused
                    | AutonomousRunStatus::Running
                    | AutonomousRunStatus::Stale
            ) && matches!(
                snapshot.attempt.as_ref().map(|attempt| &attempt.status),
                Some(AutonomousUnitStatus::Blocked | AutonomousUnitStatus::Paused)
            )
        })
        .map(|(boundary_id, _)| boundary_id);

    let (status, cancelled_at, cancel_reason) = match runtime_snapshot.run.status {
        RuntimeRunStatus::Stopped
            if matches!(intent, AutonomousRuntimeReconcileIntent::CancelRequested) =>
        {
            (
                AutonomousRunStatus::Cancelled,
                runtime_snapshot
                    .run
                    .stopped_at
                    .clone()
                    .or_else(|| Some(now_timestamp())),
                Some(RuntimeRunDiagnosticRecord {
                    code: AUTONOMOUS_CANCEL_REASON_CODE.into(),
                    message: AUTONOMOUS_CANCEL_REASON_MESSAGE.into(),
                }),
            )
        }
        RuntimeRunStatus::Stopped => (
            existing_run
                .map(|snapshot| snapshot.run.status.clone())
                .filter(|status| {
                    matches!(
                        status,
                        AutonomousRunStatus::Cancelled | AutonomousRunStatus::Completed
                    )
                })
                .unwrap_or(AutonomousRunStatus::Stopped),
            existing_run.and_then(|snapshot| snapshot.run.cancelled_at.clone()),
            existing_run.and_then(|snapshot| snapshot.run.cancel_reason.clone()),
        ),
        RuntimeRunStatus::Starting | RuntimeRunStatus::Running
            if existing_blocked_boundary.is_some() =>
        {
            (
                AutonomousRunStatus::Paused,
                existing_run.and_then(|snapshot| snapshot.run.cancelled_at.clone()),
                existing_run.and_then(|snapshot| snapshot.run.cancel_reason.clone()),
            )
        }
        RuntimeRunStatus::Starting => (AutonomousRunStatus::Starting, None, None),
        RuntimeRunStatus::Running => (AutonomousRunStatus::Running, None, None),
        RuntimeRunStatus::Stale => (AutonomousRunStatus::Stale, None, None),
        RuntimeRunStatus::Failed => (AutonomousRunStatus::Failed, None, None),
    };

    let (crashed_at, crash_reason) = match runtime_snapshot.run.status {
        RuntimeRunStatus::Stale | RuntimeRunStatus::Failed => (
            existing_run
                .and_then(|snapshot| snapshot.run.crashed_at.clone())
                .or_else(|| Some(runtime_snapshot.run.updated_at.clone())),
            last_error.clone(),
        ),
        _ => (None, None),
    };

    let completed_at = existing_run
        .and_then(|snapshot| snapshot.run.completed_at.clone())
        .filter(|_| matches!(status, AutonomousRunStatus::Completed));
    let paused_at = if matches!(status, AutonomousRunStatus::Paused) {
        existing_run
            .and_then(|snapshot| snapshot.run.paused_at.clone())
            .or_else(|| Some(base_updated_at.clone()))
    } else {
        None
    };
    let pause_reason = if matches!(status, AutonomousRunStatus::Paused) {
        existing_run
            .and_then(|snapshot| snapshot.run.pause_reason.clone())
            .or_else(|| {
                Some(RuntimeRunDiagnosticRecord {
                    code: AUTONOMOUS_BOUNDARY_PAUSE_CODE.into(),
                    message: "Cadence paused the active autonomous attempt until the operator resolves the pending boundary.".into(),
                })
            })
    } else {
        None
    };

    let sequence = existing_unit.map(|unit| unit.sequence).unwrap_or(1);
    let unit_id = existing_unit
        .map(|unit| unit.unit_id.clone())
        .unwrap_or_else(|| format!("{}:unit:{}", runtime_snapshot.run.run_id, sequence));
    let attempt_number = existing_attempt
        .map(|attempt| attempt.attempt_number)
        .unwrap_or(1);
    let attempt_id = existing_attempt
        .map(|attempt| attempt.attempt_id.clone())
        .unwrap_or_else(|| format!("{unit_id}:attempt:{attempt_number}"));
    let child_session_id = existing_attempt
        .map(|attempt| attempt.child_session_id.clone())
        .unwrap_or_else(generate_autonomous_child_session_id);
    let unit_summary = existing_unit
        .map(|unit| unit.summary.clone())
        .or_else(|| {
            runtime_snapshot
                .checkpoints
                .last()
                .map(|checkpoint| checkpoint.summary.clone())
        })
        .unwrap_or_else(|| "Researcher child session launched.".to_string());

    let blocked_by_boundary = existing_blocked_boundary.is_some();
    let unit_status = if blocked_by_boundary
        && matches!(
            runtime_snapshot.run.status,
            RuntimeRunStatus::Starting | RuntimeRunStatus::Running | RuntimeRunStatus::Stale
        ) {
        AutonomousUnitStatus::Blocked
    } else {
        autonomous_unit_status_for_run(&status)
    };
    let finished_at = match unit_status {
        AutonomousUnitStatus::Completed
        | AutonomousUnitStatus::Cancelled
        | AutonomousUnitStatus::Failed => Some(base_updated_at.clone()),
        _ => None,
    };

    let boundary_id = existing_attempt
        .and_then(|attempt| attempt.boundary_id.clone())
        .filter(|_| blocked_by_boundary);
    let unit = AutonomousUnitRecord {
        project_id: runtime_snapshot.run.project_id.clone(),
        run_id: runtime_snapshot.run.run_id.clone(),
        unit_id: unit_id.clone(),
        sequence,
        kind: existing_unit
            .map(|unit| unit.kind.clone())
            .unwrap_or(AutonomousUnitKind::Researcher),
        status: unit_status.clone(),
        summary: unit_summary,
        boundary_id: boundary_id.clone(),
        workflow_linkage: existing_unit.and_then(|unit| unit.workflow_linkage.clone()),
        started_at: existing_unit
            .map(|unit| unit.started_at.clone())
            .unwrap_or_else(|| runtime_snapshot.run.started_at.clone()),
        finished_at: finished_at.clone(),
        updated_at: base_updated_at.clone(),
        last_error: last_error.clone(),
    };
    let attempt = AutonomousUnitAttemptRecord {
        project_id: runtime_snapshot.run.project_id.clone(),
        run_id: runtime_snapshot.run.run_id.clone(),
        unit_id: unit_id.clone(),
        attempt_id: attempt_id.clone(),
        attempt_number,
        child_session_id,
        status: unit_status,
        boundary_id,
        workflow_linkage: existing_attempt.and_then(|attempt| attempt.workflow_linkage.clone()),
        started_at: existing_attempt
            .map(|attempt| attempt.started_at.clone())
            .unwrap_or_else(|| runtime_snapshot.run.started_at.clone()),
        finished_at,
        updated_at: base_updated_at.clone(),
        last_error: last_error.clone(),
    };

    AutonomousRunUpsertRecord {
        run: AutonomousRunRecord {
            project_id: runtime_snapshot.run.project_id.clone(),
            agent_session_id: runtime_snapshot.run.agent_session_id.clone(),
            run_id: runtime_snapshot.run.run_id.clone(),
            runtime_kind: runtime_snapshot.run.runtime_kind.clone(),
            provider_id: runtime_snapshot.run.provider_id.clone(),
            supervisor_kind: runtime_snapshot.run.supervisor_kind.clone(),
            status,
            active_unit_sequence: Some(sequence),
            duplicate_start_detected,
            duplicate_start_run_id,
            duplicate_start_reason,
            started_at: runtime_snapshot.run.started_at.clone(),
            last_heartbeat_at: runtime_snapshot.run.last_heartbeat_at.clone(),
            last_checkpoint_at: runtime_snapshot.last_checkpoint_at.clone(),
            paused_at,
            cancelled_at,
            completed_at,
            crashed_at,
            stopped_at: runtime_snapshot.run.stopped_at.clone(),
            pause_reason,
            cancel_reason,
            crash_reason,
            last_error,
            updated_at: base_updated_at,
        },
        unit: Some(unit),
        attempt: Some(attempt),
        artifacts: clone_current_attempt_artifacts(existing_run),
    }
}
