#[path = "autonomous_fixture_parity/support.rs"]
mod support;

#[path = "autonomous_fixture_parity/rollover_resume.rs"]
mod rollover_resume;

#[path = "autonomous_fixture_parity/recovery_parity.rs"]
mod recovery_parity;

#[path = "autonomous_fixture_parity/write_lock.rs"]
mod write_lock;

// Keep this parity target on its documented serialized rerun path when detached-runtime
// fixture reload timing is under load; preserve the behavior contract instead of adding sleeps.

#[test]
fn autonomous_fixture_repo_parity_proves_stage_rollover_boundary_resume_and_reload_identity() {
    rollover_resume::autonomous_fixture_repo_parity_proves_stage_rollover_boundary_resume_and_reload_identity();
}

#[test]
fn autonomous_fixture_repo_parity_binds_openrouter_truth_and_replays_tool_skill_recovery_after_reload(
) {
    recovery_parity::autonomous_fixture_repo_parity_binds_openrouter_truth_and_replays_tool_skill_recovery_after_reload();
}

#[test]
fn autonomous_fixture_repo_parity_replays_fixture_driven_skill_lifecycle_after_reload() {
    recovery_parity::autonomous_fixture_repo_parity_replays_fixture_driven_skill_lifecycle_after_reload();
}

#[test]
fn get_autonomous_run_returns_transient_state_when_initial_persist_hits_write_lock() {
    write_lock::get_autonomous_run_returns_transient_state_when_initial_persist_hits_write_lock();
}

#[test]
fn get_autonomous_run_reuses_unchanged_snapshot_without_write_lock_contention() {
    write_lock::get_autonomous_run_reuses_unchanged_snapshot_without_write_lock_contention();
}
