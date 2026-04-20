use std::path::Path;

use crate::{
    commands::CommandError,
    db::project_store::{self, AutonomousRunSnapshotRecord, AutonomousRunUpsertRecord},
    runtime::autonomous_run_state::current_attempt_artifacts,
};

pub(super) fn persist_autonomous_run_if_changed(
    repo_root: &Path,
    existing: Option<&AutonomousRunSnapshotRecord>,
    payload: &AutonomousRunUpsertRecord,
) -> Result<AutonomousRunSnapshotRecord, CommandError> {
    if autonomous_run_payload_matches_existing(existing, payload) {
        return Ok(existing
            .expect("matching autonomous payload requires an existing snapshot")
            .clone());
    }

    project_store::upsert_autonomous_run(repo_root, payload)
}

fn autonomous_run_payload_matches_existing(
    existing: Option<&AutonomousRunSnapshotRecord>,
    payload: &AutonomousRunUpsertRecord,
) -> bool {
    let Some(existing) = existing else {
        return false;
    };

    existing.run == payload.run
        && existing.unit == payload.unit
        && existing.attempt == payload.attempt
        && current_attempt_artifacts(existing) == payload.artifacts.as_slice()
}
