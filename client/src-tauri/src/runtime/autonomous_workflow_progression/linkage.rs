use crate::{
    commands::{CommandError, PhaseStatus, PhaseStep, PlanningLifecycleStageKindDto},
    db::project_store::{
        AutonomousRunSnapshotRecord, AutonomousRunUpsertRecord, AutonomousUnitAttemptRecord,
        AutonomousUnitKind, AutonomousUnitRecord, AutonomousUnitStatus,
        AutonomousWorkflowLinkageRecord, WorkflowGraphNodeRecord, WorkflowHandoffPackageRecord,
        WorkflowTransitionEventRecord,
    },
    runtime::autonomous_run_state::{
        autonomous_unit_status_for_run, generate_autonomous_child_session_id,
    },
};

#[derive(Debug, Clone)]
pub(super) struct StableProgressionState {
    pub(super) unit_kind: AutonomousUnitKind,
    pub(super) workflow_linkage: Option<AutonomousWorkflowLinkageRecord>,
    pub(super) unit_summary: String,
}

pub(super) fn reconcile_payload_with_progression_stage(
    existing: Option<&AutonomousRunSnapshotRecord>,
    mut payload: AutonomousRunUpsertRecord,
    progression: &StableProgressionState,
) -> AutonomousRunUpsertRecord {
    let Some(current_unit) = payload.unit.as_ref() else {
        return payload;
    };
    let Some(current_attempt) = payload.attempt.as_ref() else {
        return payload;
    };

    if should_reuse_current_identity(current_unit, current_attempt, progression) {
        payload.run.active_unit_sequence = Some(current_unit.sequence);

        if let Some(unit) = payload.unit.as_mut() {
            unit.kind = progression.unit_kind.clone();
            unit.workflow_linkage = progression.workflow_linkage.clone();
            if unit.boundary_id.is_none()
                && !matches!(
                    unit.status,
                    AutonomousUnitStatus::Blocked | AutonomousUnitStatus::Paused
                )
            {
                unit.summary = progression.unit_summary.clone();
            }
        }

        if let Some(attempt) = payload.attempt.as_mut() {
            attempt.workflow_linkage = progression.workflow_linkage.clone();
        }

        return payload;
    }

    let next_sequence = next_unit_sequence(existing, &payload);
    let next_attempt_number = next_attempt_number(existing, &payload);
    let timestamp = payload.run.updated_at.clone();
    let unit_status = autonomous_unit_status_for_run(&payload.run.status);
    let unit_id = format!("{}:unit:{}", payload.run.run_id, next_sequence);
    let attempt_id = format!("{unit_id}:attempt:{next_attempt_number}");

    payload.run.active_unit_sequence = Some(next_sequence);
    payload.unit = Some(AutonomousUnitRecord {
        project_id: payload.run.project_id.clone(),
        run_id: payload.run.run_id.clone(),
        unit_id: unit_id.clone(),
        sequence: next_sequence,
        kind: progression.unit_kind.clone(),
        status: unit_status.clone(),
        summary: progression.unit_summary.clone(),
        boundary_id: None,
        workflow_linkage: progression.workflow_linkage.clone(),
        started_at: timestamp.clone(),
        finished_at: None,
        updated_at: timestamp.clone(),
        last_error: payload.run.last_error.clone(),
    });
    payload.attempt = Some(AutonomousUnitAttemptRecord {
        project_id: payload.run.project_id.clone(),
        run_id: payload.run.run_id.clone(),
        unit_id,
        attempt_id,
        attempt_number: next_attempt_number,
        child_session_id: generate_autonomous_child_session_id(),
        status: unit_status,
        boundary_id: None,
        workflow_linkage: progression.workflow_linkage.clone(),
        started_at: timestamp.clone(),
        finished_at: None,
        updated_at: timestamp,
        last_error: payload.run.last_error.clone(),
    });
    payload.artifacts = Vec::new();

    payload
}

pub(super) fn stage_state_from_transition(
    nodes: &[WorkflowGraphNodeRecord],
    transition_event: &WorkflowTransitionEventRecord,
    package: &WorkflowHandoffPackageRecord,
) -> Result<StableProgressionState, CommandError> {
    let node = resolve_target_node(nodes, &transition_event.to_node_id)?;
    let unit_kind = map_node_to_unit_kind(&node)?;
    Ok(StableProgressionState {
        unit_summary: linked_unit_summary(&unit_kind, &node.name, package),
        unit_kind,
        workflow_linkage: Some(linkage_from_transition_and_package(
            transition_event,
            package,
        )),
    })
}

pub(super) fn resolve_active_node(
    nodes: &[WorkflowGraphNodeRecord],
) -> Result<WorkflowGraphNodeRecord, CommandError> {
    let mut active_nodes = nodes
        .iter()
        .filter(|node| node.status == PhaseStatus::Active)
        .cloned()
        .collect::<Vec<_>>();

    active_nodes.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.node_id.cmp(&right.node_id))
    });

    match active_nodes.len() {
        0 => Err(CommandError::retryable(
            "autonomous_workflow_active_node_missing",
            "Cadence cannot advance autonomous workflow progression because the workflow graph has no active node.".to_string(),
        )),
        1 => Ok(active_nodes.remove(0)),
        _ => Err(CommandError::user_fixable(
            "autonomous_workflow_active_node_ambiguous",
            format!(
                "Cadence cannot advance autonomous workflow progression because the workflow graph exposes multiple active nodes ({}).",
                active_nodes
                    .iter()
                    .map(|node| node.node_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
    }
}

pub(super) fn map_node_to_unit_kind(
    node: &WorkflowGraphNodeRecord,
) -> Result<AutonomousUnitKind, CommandError> {
    if let Some(stage) = classify_stage(node) {
        return Ok(match stage {
            PlanningLifecycleStageKindDto::Discussion | PlanningLifecycleStageKindDto::Research => {
                AutonomousUnitKind::Researcher
            }
            PlanningLifecycleStageKindDto::Requirements
            | PlanningLifecycleStageKindDto::Roadmap => AutonomousUnitKind::Planner,
        });
    }

    match node.current_step {
        Some(PhaseStep::Discuss) => Ok(AutonomousUnitKind::Researcher),
        Some(PhaseStep::Plan) => Ok(AutonomousUnitKind::Planner),
        Some(PhaseStep::Execute) => Ok(AutonomousUnitKind::Executor),
        Some(PhaseStep::Verify | PhaseStep::Ship) => Ok(AutonomousUnitKind::Verifier),
        None => Err(CommandError::user_fixable(
            "autonomous_workflow_unit_mapping_invalid",
            format!(
                "Cadence cannot map workflow node `{}` onto an autonomous unit kind because the node does not expose a recognized lifecycle stage or step.",
                node.node_id
            ),
        )),
    }
}

pub(super) fn classify_stage(
    node: &WorkflowGraphNodeRecord,
) -> Option<PlanningLifecycleStageKindDto> {
    let normalized = node.node_id.trim().to_ascii_lowercase().replace('_', "-");
    match normalized.as_str() {
        "discussion"
        | "discuss"
        | "plan-discussion"
        | "planning-discussion"
        | "workflow-discussion"
        | "lifecycle-discussion" => Some(PlanningLifecycleStageKindDto::Discussion),
        "research" | "plan-research" | "planning-research" | "workflow-research"
        | "lifecycle-research" => Some(PlanningLifecycleStageKindDto::Research),
        "requirements"
        | "requirement"
        | "plan-requirements"
        | "planning-requirements"
        | "workflow-requirements"
        | "lifecycle-requirements" => Some(PlanningLifecycleStageKindDto::Requirements),
        "roadmap" | "plan-roadmap" | "planning-roadmap" | "workflow-roadmap"
        | "lifecycle-roadmap" => Some(PlanningLifecycleStageKindDto::Roadmap),
        _ => None,
    }
}

pub(super) fn linkage_from_transition_and_package(
    transition_event: &WorkflowTransitionEventRecord,
    package: &WorkflowHandoffPackageRecord,
) -> AutonomousWorkflowLinkageRecord {
    AutonomousWorkflowLinkageRecord {
        workflow_node_id: transition_event.to_node_id.clone(),
        transition_id: transition_event.transition_id.clone(),
        causal_transition_id: transition_event.causal_transition_id.clone(),
        handoff_transition_id: package.handoff_transition_id.clone(),
        handoff_package_hash: package.package_hash.clone(),
    }
}

fn should_reuse_current_identity(
    unit: &AutonomousUnitRecord,
    attempt: &AutonomousUnitAttemptRecord,
    progression: &StableProgressionState,
) -> bool {
    let target_linkage = progression.workflow_linkage.as_ref();
    unit.workflow_linkage.as_ref() == target_linkage
        || attempt.workflow_linkage.as_ref() == target_linkage
        || (unit.workflow_linkage.is_none() && attempt.workflow_linkage.is_none())
}

fn next_unit_sequence(
    existing: Option<&AutonomousRunSnapshotRecord>,
    payload: &AutonomousRunUpsertRecord,
) -> u32 {
    existing
        .map(|snapshot| {
            snapshot
                .history
                .iter()
                .map(|entry| entry.unit.sequence)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or_else(|| payload.unit.as_ref().map(|unit| unit.sequence).unwrap_or(0))
        + 1
}

fn next_attempt_number(
    existing: Option<&AutonomousRunSnapshotRecord>,
    payload: &AutonomousRunUpsertRecord,
) -> u32 {
    existing
        .map(|snapshot| {
            snapshot
                .history
                .iter()
                .filter_map(|entry| {
                    entry
                        .latest_attempt
                        .as_ref()
                        .map(|attempt| attempt.attempt_number)
                })
                .max()
                .unwrap_or(0)
        })
        .unwrap_or_else(|| {
            payload
                .attempt
                .as_ref()
                .map(|attempt| attempt.attempt_number)
                .unwrap_or(0)
        })
        + 1
}

fn resolve_target_node(
    nodes: &[WorkflowGraphNodeRecord],
    node_id: &str,
) -> Result<WorkflowGraphNodeRecord, CommandError> {
    nodes.iter()
        .find(|node| node.node_id == node_id)
        .cloned()
        .ok_or_else(|| {
            CommandError::retryable(
                "autonomous_workflow_target_node_missing",
                format!(
                    "Cadence could not resolve workflow node `{node_id}` while reconciling autonomous workflow progression."
                ),
            )
        })
}

fn linked_unit_summary(
    unit_kind: &AutonomousUnitKind,
    node_name: &str,
    package: &WorkflowHandoffPackageRecord,
) -> String {
    format!(
        "{} child session is using persisted workflow handoff `{}` for stage `{}`.",
        autonomous_unit_kind_label(unit_kind),
        package.handoff_transition_id,
        node_name
    )
}

fn autonomous_unit_kind_label(kind: &AutonomousUnitKind) -> &'static str {
    match kind {
        AutonomousUnitKind::Researcher => "Researcher",
        AutonomousUnitKind::Planner => "Planner",
        AutonomousUnitKind::Executor => "Executor",
        AutonomousUnitKind::Verifier => "Verifier",
    }
}
