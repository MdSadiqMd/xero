use std::collections::{BTreeMap, BTreeSet};

use crate::commands::contracts::workflows::{
    WorkflowConditionDto, WorkflowDefinitionDto, WorkflowEdgeDto, WorkflowEdgeTypeDto,
    WorkflowInputBindingDto, WorkflowNodeDto, WorkflowValidationDiagnosticDto,
    WorkflowValidationReportDto, WorkflowValidationSeverityDto, WorkflowValidationStatusDto,
};

pub fn validate_workflow_definition(
    definition: &WorkflowDefinitionDto,
) -> WorkflowValidationReportDto {
    let mut diagnostics = Vec::new();
    validate_required_fields(definition, &mut diagnostics);

    let mut node_ids = BTreeSet::new();
    let mut produced_artifacts = BTreeSet::new();
    for (index, node) in definition.nodes.iter().enumerate() {
        let id = node.id();
        if !node_ids.insert(id.to_string()) {
            diagnostics.push(error(
                "duplicate_node_id",
                format!("nodes.{index}.id"),
                format!("Node id `{id}` is duplicated."),
            ));
        }
        if let Some(contract) = node.output_contract() {
            produced_artifacts.insert(format!("{id}.{}", contract.artifact_type));
        }
    }

    if !node_ids.contains(&definition.start_node_id) {
        diagnostics.push(error(
            "start_node_missing",
            "startNodeId",
            "The start node must exist.",
        ));
    }

    let mut edge_ids = BTreeSet::new();
    let mut outgoing_defaults: BTreeMap<&str, &str> = BTreeMap::new();
    let mut outgoing_edges: BTreeMap<&str, Vec<&WorkflowEdgeDto>> = BTreeMap::new();
    for (index, edge) in definition.edges.iter().enumerate() {
        if !edge_ids.insert(edge.id.clone()) {
            diagnostics.push(error(
                "duplicate_edge_id",
                format!("edges.{index}.id"),
                format!("Edge id `{}` is duplicated.", edge.id),
            ));
        }
        if !node_ids.contains(&edge.from_node_id) {
            diagnostics.push(error(
                "edge_source_missing",
                format!("edges.{index}.fromNodeId"),
                format!("Edge `{}` references a missing source node.", edge.id),
            ));
        }
        if !node_ids.contains(&edge.to_node_id) {
            diagnostics.push(error(
                "edge_target_missing",
                format!("edges.{index}.toNodeId"),
                format!("Edge `{}` references a missing target node.", edge.id),
            ));
        }
        if matches!(edge.condition, WorkflowConditionDto::Always) {
            if outgoing_defaults
                .insert(edge.from_node_id.as_str(), edge.id.as_str())
                .is_some()
            {
                diagnostics.push(error(
                    "duplicate_default_edge",
                    format!("edges.{index}.condition"),
                    format!(
                        "Node `{}` has more than one default else edge.",
                        edge.from_node_id
                    ),
                ));
            }
        }
        if matches!(edge.r#type, WorkflowEdgeTypeDto::Loop) || edge.loop_policy.is_some() {
            match edge.loop_policy.as_ref() {
                Some(policy) => {
                    if policy.max_attempts == 0 {
                        diagnostics.push(error(
                            "loop_max_attempts_invalid",
                            format!("edges.{index}.loopPolicy.maxAttempts"),
                            format!("Loop edge `{}` must allow at least one attempt.", edge.id),
                        ));
                    }
                    if !node_ids.contains(&policy.on_exhausted) {
                        diagnostics.push(error(
                            "loop_exhaustion_target_missing",
                            format!("edges.{index}.loopPolicy.onExhausted"),
                            format!(
                                "Loop edge `{}` must route exhaustion to an existing node.",
                                edge.id
                            ),
                        ));
                    }
                }
                None => diagnostics.push(error(
                    "loop_policy_missing",
                    format!("edges.{index}.loopPolicy"),
                    format!("Loop edge `{}` must declare a loop policy.", edge.id),
                )),
            }
        }
        validate_condition_shape(
            &edge.condition,
            format!("edges.{index}.condition"),
            &mut diagnostics,
        );
        for artifact_ref in condition_artifact_refs(&edge.condition) {
            if !produced_artifacts.contains(&artifact_ref) {
                diagnostics.push(error(
                    "condition_artifact_ref_missing",
                    format!("edges.{index}.condition"),
                    format!("Condition references missing artifact `{artifact_ref}`."),
                ));
            }
        }
        for node_ref in condition_node_refs(&edge.condition) {
            if !node_ids.contains(&node_ref) {
                diagnostics.push(error(
                    "condition_node_ref_missing",
                    format!("edges.{index}.condition"),
                    format!("Condition references missing node `{node_ref}`."),
                ));
            }
        }

        outgoing_edges
            .entry(edge.from_node_id.as_str())
            .or_default()
            .push(edge);
    }

    for (index, node) in definition.nodes.iter().enumerate() {
        match node {
            WorkflowNodeDto::Agent { input_bindings, .. } => {
                for (binding_index, binding) in input_bindings.iter().enumerate() {
                    if let WorkflowInputBindingDto::Artifact { artifact_ref, .. } = binding {
                        if !produced_artifacts.contains(artifact_ref) {
                            diagnostics.push(error(
                                "artifact_ref_missing",
                                format!("nodes.{index}.inputBindings.{binding_index}.artifactRef"),
                                format!(
                                    "Artifact reference `{artifact_ref}` is not produced by any agent node."
                                ),
                            ));
                        }
                    }
                }
            }
            WorkflowNodeDto::Merge {
                wait_policy,
                quorum,
                ..
            } => {
                if wait_policy
                    == &crate::commands::contracts::workflows::WorkflowMergeWaitPolicyDto::Quorum
                    && quorum.unwrap_or(0) == 0
                {
                    diagnostics.push(error(
                        "merge_quorum_missing",
                        format!("nodes.{index}.quorum"),
                        "Quorum merge nodes must declare a quorum.",
                    ));
                }
            }
            _ => {}
        }
    }

    diagnostics.extend(detect_unbounded_cycles(definition, &outgoing_edges));
    WorkflowValidationReportDto {
        status: if diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == WorkflowValidationSeverityDto::Error)
        {
            WorkflowValidationStatusDto::Invalid
        } else {
            WorkflowValidationStatusDto::Valid
        },
        diagnostics,
    }
}

fn validate_required_fields(
    definition: &WorkflowDefinitionDto,
    diagnostics: &mut Vec<WorkflowValidationDiagnosticDto>,
) {
    if definition.schema != "xero.workflow_definition.v1" {
        diagnostics.push(error(
            "schema_unsupported",
            "schema",
            "Workflow definitions must use schema `xero.workflow_definition.v1`.",
        ));
    }
    for (field, value) in [
        ("id", definition.id.as_str()),
        ("projectId", definition.project_id.as_str()),
        ("name", definition.name.as_str()),
        ("startNodeId", definition.start_node_id.as_str()),
    ] {
        if value.trim().is_empty() {
            diagnostics.push(error(
                "required_field_empty",
                field,
                format!("Workflow field `{field}` cannot be empty."),
            ));
        }
    }
    if definition.nodes.is_empty() {
        diagnostics.push(error(
            "nodes_empty",
            "nodes",
            "A Workflow must contain at least one node.",
        ));
    }
    if definition.run_policy.concurrency_limit == 0 {
        diagnostics.push(error(
            "concurrency_limit_invalid",
            "runPolicy.concurrencyLimit",
            "Workflow concurrency limit must be at least 1.",
        ));
    }
}

fn validate_condition_shape(
    condition: &WorkflowConditionDto,
    path: String,
    diagnostics: &mut Vec<WorkflowValidationDiagnosticDto>,
) {
    match condition {
        WorkflowConditionDto::All { conditions } | WorkflowConditionDto::Any { conditions } => {
            if conditions.is_empty() {
                diagnostics.push(error(
                    "condition_children_empty",
                    path.clone(),
                    "Composite Workflow conditions must contain at least one child condition.",
                ));
            }
            for (index, child) in conditions.iter().enumerate() {
                validate_condition_shape(child, format!("{path}.conditions.{index}"), diagnostics);
            }
        }
        WorkflowConditionDto::Not { condition } => {
            validate_condition_shape(condition, format!("{path}.condition"), diagnostics);
        }
        WorkflowConditionDto::ArtifactFieldEquals {
            path: json_path, ..
        }
        | WorkflowConditionDto::ArtifactFieldIn {
            path: json_path, ..
        }
        | WorkflowConditionDto::ArtifactFieldNumberCompare {
            path: json_path, ..
        } => {
            if !json_path.starts_with('$') {
                diagnostics.push(error(
                    "condition_json_path_invalid",
                    path,
                    "Artifact field conditions must use a JSON path that starts with `$`.",
                ));
            }
        }
        _ => {}
    }
}

fn detect_unbounded_cycles(
    definition: &WorkflowDefinitionDto,
    outgoing_edges: &BTreeMap<&str, Vec<&WorkflowEdgeDto>>,
) -> Vec<WorkflowValidationDiagnosticDto> {
    let mut detector = CycleDetector {
        outgoing_edges,
        visiting: BTreeSet::new(),
        visited: BTreeSet::new(),
        stack: Vec::new(),
        reported_cycles: BTreeSet::new(),
        diagnostics: Vec::new(),
    };
    if definition
        .nodes
        .iter()
        .any(|node| node.id() == definition.start_node_id)
    {
        detector.visit(&definition.start_node_id);
    }
    detector.diagnostics
}

struct CycleDetector<'a> {
    outgoing_edges: &'a BTreeMap<&'a str, Vec<&'a WorkflowEdgeDto>>,
    visiting: BTreeSet<String>,
    visited: BTreeSet<String>,
    stack: Vec<&'a WorkflowEdgeDto>,
    reported_cycles: BTreeSet<String>,
    diagnostics: Vec<WorkflowValidationDiagnosticDto>,
}

impl<'a> CycleDetector<'a> {
    fn visit(&mut self, node_id: &str) {
        if self.visiting.contains(node_id) {
            let start_index = self
                .stack
                .iter()
                .position(|edge| edge.from_node_id == node_id)
                .unwrap_or(0);
            let cycle = &self.stack[start_index..];
            let cycle_key = cycle
                .iter()
                .map(|edge| edge.id.as_str())
                .collect::<Vec<_>>()
                .join(" -> ");
            if !cycle.iter().any(|edge| {
                matches!(edge.r#type, WorkflowEdgeTypeDto::Loop) && edge.loop_policy.is_some()
            }) && self.reported_cycles.insert(cycle_key.clone())
            {
                self.diagnostics.push(error(
                    "cycle_without_loop_policy",
                    "edges",
                    format!("Cycle `{cycle_key}` must include an explicit bounded loop edge."),
                ));
            }
            return;
        }
        if self.visited.contains(node_id) {
            return;
        }

        self.visiting.insert(node_id.to_string());
        if let Some(edges) = self.outgoing_edges.get(node_id) {
            for edge in edges {
                self.stack.push(edge);
                self.visit(&edge.to_node_id);
                self.stack.pop();
            }
        }
        self.visiting.remove(node_id);
        self.visited.insert(node_id.to_string());
    }
}

fn condition_artifact_refs(condition: &WorkflowConditionDto) -> Vec<String> {
    match condition {
        WorkflowConditionDto::ArtifactExists { artifact_ref }
        | WorkflowConditionDto::ArtifactFieldEquals { artifact_ref, .. }
        | WorkflowConditionDto::ArtifactFieldIn { artifact_ref, .. }
        | WorkflowConditionDto::ArtifactFieldNumberCompare { artifact_ref, .. } => {
            vec![artifact_ref.clone()]
        }
        WorkflowConditionDto::All { conditions } | WorkflowConditionDto::Any { conditions } => {
            conditions
                .iter()
                .flat_map(condition_artifact_refs)
                .collect()
        }
        WorkflowConditionDto::Not { condition } => condition_artifact_refs(condition),
        _ => Vec::new(),
    }
}

fn condition_node_refs(condition: &WorkflowConditionDto) -> Vec<String> {
    match condition {
        WorkflowConditionDto::NodeStatus { node_id, .. } => vec![node_id.clone()],
        WorkflowConditionDto::FailureClassIs {
            node_id: Some(node_id),
            ..
        } => vec![node_id.clone()],
        WorkflowConditionDto::HumanDecisionIs {
            checkpoint_node_id, ..
        } => vec![checkpoint_node_id.clone()],
        WorkflowConditionDto::All { conditions } | WorkflowConditionDto::Any { conditions } => {
            conditions.iter().flat_map(condition_node_refs).collect()
        }
        WorkflowConditionDto::Not { condition } => condition_node_refs(condition),
        _ => Vec::new(),
    }
}

fn error(
    code: impl Into<String>,
    path: impl Into<String>,
    message: impl Into<String>,
) -> WorkflowValidationDiagnosticDto {
    WorkflowValidationDiagnosticDto {
        severity: WorkflowValidationSeverityDto::Error,
        code: code.into(),
        path: path.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::contracts::{
        runtime::RuntimeAgentIdDto,
        workflow_agents::AgentRefDto,
        workflows::{
            WorkflowEdgeDto, WorkflowEdgeTypeDto, WorkflowNodeDto, WorkflowOutputContractDto,
            WorkflowRunPolicyDto, WorkflowTerminalStatusDto, WorkflowValidationStatusDto,
        },
    };

    fn linear_definition() -> WorkflowDefinitionDto {
        WorkflowDefinitionDto {
            schema: "xero.workflow_definition.v1".into(),
            id: "workflow-linear".into(),
            project_id: "project-1".into(),
            name: "Linear".into(),
            description: String::new(),
            version: 1,
            start_node_id: "agent-a".into(),
            nodes: vec![
                WorkflowNodeDto::Agent {
                    id: "agent-a".into(),
                    title: "Agent A".into(),
                    description: String::new(),
                    position: Default::default(),
                    agent_ref: AgentRefDto::BuiltIn {
                        runtime_agent_id: RuntimeAgentIdDto::Engineer,
                        version: 2,
                    },
                    display_label: None,
                    input_bindings: Vec::new(),
                    output_contract: WorkflowOutputContractDto::default(),
                    run_overrides: None,
                    resource_scopes: Vec::new(),
                    failure_policy: Default::default(),
                },
                WorkflowNodeDto::Agent {
                    id: "agent-b".into(),
                    title: "Agent B".into(),
                    description: String::new(),
                    position: Default::default(),
                    agent_ref: AgentRefDto::Custom {
                        definition_id: "custom-work".into(),
                        version: 1,
                    },
                    display_label: None,
                    input_bindings: vec![WorkflowInputBindingDto::Artifact {
                        name: "handoff".into(),
                        required: true,
                        artifact_ref: "agent-a.text_output".into(),
                        path: None,
                        prompt_label: None,
                    }],
                    output_contract: WorkflowOutputContractDto {
                        artifact_type: "implementation_summary".into(),
                        ..WorkflowOutputContractDto::default()
                    },
                    run_overrides: None,
                    resource_scopes: Vec::new(),
                    failure_policy: Default::default(),
                },
                WorkflowNodeDto::Terminal {
                    id: "done".into(),
                    title: "Done".into(),
                    description: String::new(),
                    position: Default::default(),
                    terminal_status: WorkflowTerminalStatusDto::Success,
                },
            ],
            edges: vec![
                WorkflowEdgeDto {
                    id: "edge-a-b".into(),
                    from_node_id: "agent-a".into(),
                    to_node_id: "agent-b".into(),
                    r#type: WorkflowEdgeTypeDto::Success,
                    label: String::new(),
                    priority: 10,
                    condition: WorkflowConditionDto::NodeStatus {
                        node_id: "agent-a".into(),
                        status: crate::commands::contracts::workflows::WorkflowNodeRunStatusDto::Succeeded,
                    },
                    loop_policy: None,
                },
                WorkflowEdgeDto {
                    id: "edge-b-done".into(),
                    from_node_id: "agent-b".into(),
                    to_node_id: "done".into(),
                    r#type: WorkflowEdgeTypeDto::Success,
                    label: String::new(),
                    priority: 10,
                    condition: WorkflowConditionDto::Always,
                    loop_policy: None,
                },
            ],
            artifact_contracts: Vec::new(),
            run_policy: WorkflowRunPolicyDto::default(),
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn validator_accepts_linear_custom_agent_workflow() {
        let report = validate_workflow_definition(&linear_definition());

        assert_eq!(report.status, WorkflowValidationStatusDto::Valid);
    }

    #[test]
    fn validator_rejects_cycle_without_loop_policy() {
        let mut definition = linear_definition();
        definition.edges.push(WorkflowEdgeDto {
            id: "edge-b-a".into(),
            from_node_id: "agent-b".into(),
            to_node_id: "agent-a".into(),
            r#type: WorkflowEdgeTypeDto::Conditional,
            label: "retry".into(),
            priority: 20,
            condition: WorkflowConditionDto::Always,
            loop_policy: None,
        });

        let report = validate_workflow_definition(&definition);

        assert_eq!(report.status, WorkflowValidationStatusDto::Invalid);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cycle_without_loop_policy"));
    }

    #[test]
    fn validator_accepts_bounded_loop_with_exhaustion_route() {
        let mut definition = linear_definition();
        definition.nodes.push(WorkflowNodeDto::HumanCheckpoint {
            id: "human".into(),
            title: "Human".into(),
            description: String::new(),
            position: Default::default(),
            checkpoint_type:
                crate::commands::contracts::workflows::WorkflowHumanCheckpointTypeDto::Decision,
            prompt: "Choose a route.".into(),
            decision_options: vec!["retry".into(), "stop".into()],
        });
        definition.edges.push(WorkflowEdgeDto {
            id: "edge-b-a".into(),
            from_node_id: "agent-b".into(),
            to_node_id: "agent-a".into(),
            r#type: WorkflowEdgeTypeDto::Loop,
            label: "retry".into(),
            priority: 20,
            condition: WorkflowConditionDto::LoopAttemptLt {
                loop_key: "retry".into(),
                value: 2,
            },
            loop_policy: Some(
                crate::commands::contracts::workflows::WorkflowLoopPolicyDto {
                    loop_key: "retry".into(),
                    max_attempts: 2,
                    attempt_scope: Default::default(),
                    carryover_policy: Default::default(),
                    selected_artifact_refs: Vec::new(),
                    reset_policy: Default::default(),
                    stall_detector: None,
                    on_exhausted: "human".into(),
                },
            ),
        });

        let report = validate_workflow_definition(&definition);

        assert_eq!(report.status, WorkflowValidationStatusDto::Valid);
    }
}
