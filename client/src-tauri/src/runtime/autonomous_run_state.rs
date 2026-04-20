use rand::RngCore;

use crate::db::project_store::{
    AutonomousRunSnapshotRecord, AutonomousRunStatus, AutonomousUnitArtifactRecord,
    AutonomousUnitStatus,
};

pub(crate) fn current_attempt_artifacts(
    existing: &AutonomousRunSnapshotRecord,
) -> &[AutonomousUnitArtifactRecord] {
    let Some(attempt_id) = existing
        .attempt
        .as_ref()
        .map(|attempt| attempt.attempt_id.as_str())
    else {
        return &[];
    };

    existing
        .history
        .iter()
        .find(|entry| {
            entry
                .latest_attempt
                .as_ref()
                .is_some_and(|attempt| attempt.attempt_id == attempt_id)
        })
        .map(|entry| entry.artifacts.as_slice())
        .unwrap_or(&[])
}

pub(crate) fn clone_current_attempt_artifacts(
    existing: Option<&AutonomousRunSnapshotRecord>,
) -> Vec<AutonomousUnitArtifactRecord> {
    match existing {
        Some(existing) => current_attempt_artifacts(existing).to_vec(),
        None => Vec::new(),
    }
}

pub(crate) fn autonomous_unit_status_for_run(status: &AutonomousRunStatus) -> AutonomousUnitStatus {
    match status {
        AutonomousRunStatus::Starting
        | AutonomousRunStatus::Running
        | AutonomousRunStatus::Stale
        | AutonomousRunStatus::Cancelling => AutonomousUnitStatus::Active,
        AutonomousRunStatus::Paused => AutonomousUnitStatus::Paused,
        AutonomousRunStatus::Cancelled => AutonomousUnitStatus::Cancelled,
        AutonomousRunStatus::Stopped | AutonomousRunStatus::Completed => {
            AutonomousUnitStatus::Completed
        }
        AutonomousRunStatus::Failed | AutonomousRunStatus::Crashed => AutonomousUnitStatus::Failed,
    }
}

pub(crate) fn generate_autonomous_child_session_id() -> String {
    let mut bytes = [0_u8; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!(
        "child-{}",
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}
