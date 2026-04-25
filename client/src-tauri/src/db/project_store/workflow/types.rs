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
pub(crate) struct RuntimeOperatorResumeTarget {
    pub(crate) run_id: String,
    pub(crate) boundary_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolveOperatorAnswerRequirement {
    GateLinked,
    RuntimeResumable,
}
