use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowGateState {
    Pending,
    Satisfied,
    Blocked,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowTransitionGateDecision {
    Approved,
    Rejected,
    Blocked,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowGraphNodeRecord {
    pub node_id: String,
    pub phase_id: u32,
    pub sort_order: u32,
    pub name: String,
    pub description: String,
    pub status: PhaseStatus,
    pub current_step: Option<PhaseStep>,
    pub task_count: u32,
    pub completed_tasks: u32,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowGraphEdgeRecord {
    pub from_node_id: String,
    pub to_node_id: String,
    pub transition_kind: String,
    pub gate_requirement: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowGateMetadataRecord {
    pub node_id: String,
    pub gate_key: String,
    pub gate_state: WorkflowGateState,
    pub action_type: Option<String>,
    pub title: Option<String>,
    pub detail: Option<String>,
    pub decision_context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowTransitionEventRecord {
    pub id: i64,
    pub transition_id: String,
    pub causal_transition_id: Option<String>,
    pub from_node_id: String,
    pub to_node_id: String,
    pub transition_kind: String,
    pub gate_decision: WorkflowTransitionGateDecision,
    pub gate_decision_context: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowHandoffPackageRecord {
    pub id: i64,
    pub project_id: String,
    pub handoff_transition_id: String,
    pub causal_transition_id: Option<String>,
    pub from_node_id: String,
    pub to_node_id: String,
    pub transition_kind: String,
    pub package_payload: String,
    pub package_hash: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowHandoffPackageUpsertRecord {
    pub project_id: String,
    pub handoff_transition_id: String,
    pub causal_transition_id: Option<String>,
    pub from_node_id: String,
    pub to_node_id: String,
    pub transition_kind: String,
    pub package_payload: String,
    pub created_at: String,
}

#[allow(dead_code)]
pub(crate) fn map_workflow_handoff_package_record(
    record: WorkflowHandoffPackageRecord,
) -> WorkflowHandoffPackageDto {
    WorkflowHandoffPackageDto {
        id: record.id,
        project_id: record.project_id,
        handoff_transition_id: record.handoff_transition_id,
        causal_transition_id: record.causal_transition_id,
        from_node_id: record.from_node_id,
        to_node_id: record.to_node_id,
        transition_kind: record.transition_kind,
        package_payload: record.package_payload,
        package_hash: record.package_hash,
        created_at: record.created_at,
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowHandoffPackagePayload {
    pub schema_version: u32,
    pub trigger_transition: WorkflowHandoffTriggerTransitionPayload,
    pub destination_state: WorkflowHandoffDestinationStatePayload,
    pub lifecycle_projection: WorkflowHandoffLifecycleProjectionPayload,
    pub operator_continuity: WorkflowHandoffOperatorContinuityPayload,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowHandoffTriggerTransitionPayload {
    pub transition_id: String,
    pub causal_transition_id: Option<String>,
    pub from_node_id: String,
    pub to_node_id: String,
    pub transition_kind: String,
    pub gate_decision: String,
    pub gate_decision_context_present: bool,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowHandoffDestinationStatePayload {
    pub node_id: String,
    pub phase_id: u32,
    pub sort_order: u32,
    pub name: String,
    pub description: String,
    pub status: PhaseStatus,
    pub current_step: Option<PhaseStep>,
    pub task_count: u32,
    pub completed_tasks: u32,
    pub pending_gate_count: u32,
    pub gates: Vec<WorkflowHandoffDestinationGatePayload>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowHandoffDestinationGatePayload {
    pub gate_key: String,
    pub gate_state: String,
    pub action_type: Option<String>,
    pub detail_present: bool,
    pub decision_context_present: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowHandoffLifecycleProjectionPayload {
    pub stages: Vec<PlanningLifecycleStageDto>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowHandoffOperatorContinuityPayload {
    pub pending_gate_action_count: u32,
    pub pending_gate_actions: Vec<WorkflowHandoffPendingGateActionPayload>,
    pub latest_resume: Option<WorkflowHandoffLatestResumePayload>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowHandoffPendingGateActionPayload {
    pub action_id: String,
    pub action_type: String,
    pub gate_node_id: String,
    pub gate_key: String,
    pub transition_from_node_id: String,
    pub transition_to_node_id: String,
    pub transition_kind: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowHandoffLatestResumePayload {
    pub source_action_id: Option<String>,
    pub status: ResumeHistoryStatus,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowAutomaticDispatchPackageOutcome {
    Persisted {
        package: WorkflowHandoffPackageRecord,
    },
    Replayed {
        package: WorkflowHandoffPackageRecord,
    },
    Skipped {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowAutomaticDispatchOutcome {
    NoContinuation,
    Applied {
        transition_event: WorkflowTransitionEventRecord,
        handoff_package: WorkflowAutomaticDispatchPackageOutcome,
    },
    Replayed {
        transition_event: WorkflowTransitionEventRecord,
        handoff_package: WorkflowAutomaticDispatchPackageOutcome,
    },
    Skipped {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowGraphRecord {
    pub nodes: Vec<WorkflowGraphNodeRecord>,
    pub edges: Vec<WorkflowGraphEdgeRecord>,
    pub gates: Vec<WorkflowGateMetadataRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowGraphUpsertRecord {
    pub nodes: Vec<WorkflowGraphNodeRecord>,
    pub edges: Vec<WorkflowGraphEdgeRecord>,
    pub gates: Vec<WorkflowGateMetadataRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowGateDecisionUpdate {
    pub gate_key: String,
    pub gate_state: WorkflowGateState,
    pub decision_context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyWorkflowTransitionRecord {
    pub transition_id: String,
    pub causal_transition_id: Option<String>,
    pub from_node_id: String,
    pub to_node_id: String,
    pub transition_kind: String,
    pub gate_decision: WorkflowTransitionGateDecision,
    pub gate_decision_context: Option<String>,
    pub gate_updates: Vec<WorkflowGateDecisionUpdate>,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyWorkflowTransitionResult {
    pub transition_event: WorkflowTransitionEventRecord,
    pub automatic_dispatch: WorkflowAutomaticDispatchOutcome,
    pub phases: Vec<PhaseSummaryDto>,
}

#[derive(Debug, Clone)]
pub(crate) struct OperatorApprovalGateLink {
    pub(crate) gate_node_id: String,
    pub(crate) gate_key: String,
    pub(crate) transition_from_node_id: String,
    pub(crate) transition_to_node_id: String,
    pub(crate) transition_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorApprovalGateLinkInput {
    pub gate_node_id: String,
    pub gate_key: String,
    pub transition_from_node_id: String,
    pub transition_to_node_id: String,
    pub transition_kind: String,
}

#[derive(Debug, Clone)]
pub(crate) struct OperatorResumeTransitionContext {
    pub(crate) gate_node_id: String,
    pub(crate) gate_key: String,
    pub(crate) transition_from_node_id: String,
    pub(crate) transition_to_node_id: String,
    pub(crate) transition_kind: String,
    pub(crate) user_answer: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeOperatorResumeTarget {
    pub(crate) run_id: String,
    pub(crate) boundary_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolveOperatorAnswerRequirement {
    GateLinked,
    RuntimeResumable,
}

pub(crate) type WorkflowTransitionSqlErrorMapper = fn(&str, &Path, SqlError, &str) -> CommandError;

#[derive(Debug, Clone)]
pub(crate) struct WorkflowTransitionGateMutationRecord {
    pub(crate) node_id: String,
    pub(crate) gate_key: String,
    pub(crate) gate_state: WorkflowGateState,
    pub(crate) decision_context: Option<String>,
    pub(crate) require_pending_or_blocked: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkflowTransitionMutationRecord {
    pub(crate) transition_id: String,
    pub(crate) causal_transition_id: Option<String>,
    pub(crate) from_node_id: String,
    pub(crate) to_node_id: String,
    pub(crate) transition_kind: String,
    pub(crate) gate_decision: WorkflowTransitionGateDecision,
    pub(crate) gate_decision_context: Option<String>,
    pub(crate) gate_updates: Vec<WorkflowTransitionGateMutationRecord>,
    pub(crate) required_gate_requirement: Option<String>,
    pub(crate) occurred_at: String,
}

pub(crate) enum WorkflowTransitionMutationApplyOutcome {
    Applied,
    Replayed(WorkflowTransitionEventRecord),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WorkflowTransitionMutationErrorProfile {
    pub(crate) edge_check_failed_code: &'static str,
    pub(crate) edge_check_failed_message: &'static str,
    pub(crate) gate_update_failed_code: &'static str,
    pub(crate) gate_update_failed_message: &'static str,
    pub(crate) gate_check_failed_code: &'static str,
    pub(crate) gate_check_failed_message: &'static str,
    pub(crate) source_update_failed_code: &'static str,
    pub(crate) source_update_failed_message: &'static str,
    pub(crate) target_update_failed_code: &'static str,
    pub(crate) target_update_failed_message: &'static str,
    pub(crate) event_persist_failed_code: &'static str,
    pub(crate) event_persist_failed_message: &'static str,
}

pub(crate) const WORKFLOW_TRANSITION_COMMAND_MUTATION_ERROR_PROFILE:
    WorkflowTransitionMutationErrorProfile = WorkflowTransitionMutationErrorProfile {
    edge_check_failed_code: "workflow_transition_edge_check_failed",
    edge_check_failed_message: "Cadence could not verify workflow transition edge legality.",
    gate_update_failed_code: "workflow_transition_gate_update_failed",
    gate_update_failed_message: "Cadence could not persist workflow gate decisions.",
    gate_check_failed_code: "workflow_transition_gate_check_failed",
    gate_check_failed_message: "Cadence could not verify workflow gate state before transition.",
    source_update_failed_code: "workflow_transition_source_update_failed",
    source_update_failed_message: "Cadence could not update workflow source-node state.",
    target_update_failed_code: "workflow_transition_target_update_failed",
    target_update_failed_message: "Cadence could not update workflow target-node state.",
    event_persist_failed_code: "workflow_transition_event_persist_failed",
    event_persist_failed_message: "Cadence could not persist the workflow transition event.",
};

pub(crate) const OPERATOR_RESUME_MUTATION_ERROR_PROFILE: WorkflowTransitionMutationErrorProfile =
    WorkflowTransitionMutationErrorProfile {
        edge_check_failed_code: "operator_resume_transition_edge_check_failed",
        edge_check_failed_message:
            "Cadence could not verify gate-linked resume transition legality.",
        gate_update_failed_code: "operator_resume_gate_update_failed",
        gate_update_failed_message:
            "Cadence could not persist the approved gate decision during resume.",
        gate_check_failed_code: "operator_resume_gate_check_failed",
        gate_check_failed_message:
            "Cadence could not verify workflow gate state before resume transition.",
        source_update_failed_code: "operator_resume_source_update_failed",
        source_update_failed_message:
            "Cadence could not update workflow source-node state during resume.",
        target_update_failed_code: "operator_resume_target_update_failed",
        target_update_failed_message:
            "Cadence could not update workflow target-node state during resume.",
        event_persist_failed_code: "operator_resume_transition_event_persist_failed",
        event_persist_failed_message:
            "Cadence could not persist the resume-caused workflow transition event.",
    };

pub(crate) const WORKFLOW_AUTOMATIC_DISPATCH_MUTATION_ERROR_PROFILE:
    WorkflowTransitionMutationErrorProfile = WorkflowTransitionMutationErrorProfile {
    edge_check_failed_code: "workflow_transition_auto_dispatch_edge_check_failed",
    edge_check_failed_message:
        "Cadence could not verify automatic workflow dispatch edge legality.",
    gate_update_failed_code: "workflow_transition_auto_dispatch_gate_update_failed",
    gate_update_failed_message: "Cadence could not persist automatic workflow gate updates.",
    gate_check_failed_code: "workflow_transition_auto_dispatch_gate_check_failed",
    gate_check_failed_message:
        "Cadence could not verify workflow gate state before automatic dispatch.",
    source_update_failed_code: "workflow_transition_auto_dispatch_source_update_failed",
    source_update_failed_message: "Cadence could not update automatic-dispatch source node state.",
    target_update_failed_code: "workflow_transition_auto_dispatch_target_update_failed",
    target_update_failed_message: "Cadence could not update automatic-dispatch target node state.",
    event_persist_failed_code: "workflow_transition_auto_dispatch_event_persist_failed",
    event_persist_failed_message:
        "Cadence could not persist the automatic workflow transition event.",
};
