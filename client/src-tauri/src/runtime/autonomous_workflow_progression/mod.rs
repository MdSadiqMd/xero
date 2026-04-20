use std::path::Path;

use crate::{
    commands::CommandError,
    db::project_store::{
        self, AutonomousRunSnapshotRecord, AutonomousRunStatus, AutonomousRunUpsertRecord,
    },
};

mod linkage;
mod payload;
mod progression;

const MAX_PROGRESSION_STEPS: usize = 16;

pub fn persist_autonomous_workflow_progression(
    repo_root: &Path,
    project_id: &str,
    existing: Option<&AutonomousRunSnapshotRecord>,
    payload: AutonomousRunUpsertRecord,
) -> Result<AutonomousRunSnapshotRecord, CommandError> {
    if !matches!(
        payload.run.status,
        AutonomousRunStatus::Starting | AutonomousRunStatus::Running
    ) {
        return payload::persist_autonomous_run_if_changed(repo_root, existing, &payload);
    }

    let graph = project_store::load_workflow_graph(repo_root, project_id)?;
    if graph.nodes.is_empty() {
        return payload::persist_autonomous_run_if_changed(repo_root, existing, &payload);
    }

    let existing_linkage = existing
        .and_then(|snapshot| snapshot.unit.as_ref())
        .and_then(|unit| unit.workflow_linkage.as_ref());

    let active_node = linkage::resolve_active_node(&graph.nodes)?;
    if let Some(linkage) = existing_linkage {
        if linkage.workflow_node_id != active_node.node_id {
            return Err(CommandError::user_fixable(
                "autonomous_workflow_linkage_stage_conflict",
                format!(
                    "Cadence refused to advance autonomous workflow progression because the durable autonomous linkage points at workflow node `{}` while the active workflow node is `{}`.",
                    linkage.workflow_node_id, active_node.node_id
                ),
            ));
        }
    }

    let progression_states = progression::collect_progression_states(
        repo_root,
        project_id,
        &payload.run.run_id,
        existing_linkage,
    )?;

    if progression_states.is_empty() {
        return payload::persist_autonomous_run_if_changed(repo_root, existing, &payload);
    }

    let mut persisted = existing.cloned();
    let mut working_payload = payload;
    for progression in progression_states {
        working_payload = linkage::reconcile_payload_with_progression_stage(
            persisted.as_ref(),
            working_payload,
            &progression,
        );
        persisted = Some(project_store::upsert_autonomous_run(
            repo_root,
            &working_payload,
        )?);
    }

    persisted.ok_or_else(|| {
        CommandError::system_fault(
            "autonomous_workflow_progression_missing",
            format!(
                "Cadence progressed autonomous workflow state for project `{project_id}` but could not read back the durable autonomous snapshot."
            ),
        )
    })
}
