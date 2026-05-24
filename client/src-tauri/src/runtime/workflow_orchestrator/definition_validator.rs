use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value as JsonValue;

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
    let mut artifact_contracts_by_ref = BTreeMap::new();
    for (index, node) in definition.nodes.iter().enumerate() {
        let id = node.id();
        if !node_ids.insert(id.to_string()) {
            diagnostics.push(error(
                "duplicate_node_id",
                format!("nodes.{index}.id"),
                format!("Node id `{id}` is duplicated."),
            ));
        }
        if let Some(artifact_type) = node.produced_artifact_type() {
            let artifact_ref = format!("{id}.{artifact_type}");
            produced_artifacts.insert(artifact_ref.clone());
            if let Some(contract) = node.output_contract() {
                let artifact_contract = definition.artifact_contracts.iter().find(|candidate| {
                    candidate.artifact_type == contract.artifact_type
                        && candidate.schema_version == contract.schema_version
                });
                if artifact_contract.is_none()
                    && contract.extraction
                        != crate::commands::contracts::workflows::WorkflowOutputExtractionDto::GenericText
                {
                    diagnostics.push(error(
                        "artifact_contract_missing",
                        format!("nodes.{index}.outputContract"),
                        format!(
                            "JSON artifact `{}` v{} must declare an artifact contract.",
                            contract.artifact_type, contract.schema_version
                        ),
                    ));
                }
                if let Some(artifact_contract) = artifact_contract {
                    artifact_contracts_by_ref.insert(artifact_ref, artifact_contract);
                    if let (Some(render_text_path), Some(json_schema)) = (
                        contract.render_text_path.as_deref(),
                        artifact_contract.json_schema.as_ref(),
                    ) {
                        if !json_schema_allows_path(json_schema, render_text_path) {
                            diagnostics.push(error(
                                "render_text_path_not_in_schema",
                                format!("nodes.{index}.outputContract.renderTextPath"),
                                format!(
                                    "Render path `{render_text_path}` is not allowed by the `{}` artifact schema.",
                                    contract.artifact_type
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }

    let mut subgraph_ids = BTreeSet::new();
    for (index, subgraph) in definition.subgraphs.iter().enumerate() {
        if !subgraph_ids.insert(subgraph.id.clone()) {
            diagnostics.push(error(
                "duplicate_subgraph_id",
                format!("subgraphs.{index}.id"),
                format!("Subgraph id `{}` is duplicated.", subgraph.id),
            ));
        }
        if subgraph.nodes.is_empty() {
            diagnostics.push(error(
                "subgraph_nodes_empty",
                format!("subgraphs.{index}.nodes"),
                format!("Subgraph `{}` must contain at least one node.", subgraph.id),
            ));
            continue;
        }
        let subgraph_node_ids = subgraph
            .nodes
            .iter()
            .map(|node| node.id().to_string())
            .collect::<BTreeSet<_>>();
        if !subgraph_node_ids.contains(&subgraph.start_node_id) {
            diagnostics.push(error(
                "subgraph_start_node_missing",
                format!("subgraphs.{index}.startNodeId"),
                format!(
                    "Subgraph `{}` references a missing start node.",
                    subgraph.id
                ),
            ));
        }
        for (edge_index, edge) in subgraph.edges.iter().enumerate() {
            if !subgraph_node_ids.contains(&edge.from_node_id) {
                diagnostics.push(error(
                    "subgraph_edge_source_missing",
                    format!("subgraphs.{index}.edges.{edge_index}.fromNodeId"),
                    format!(
                        "Subgraph edge `{}` references a missing source node.",
                        edge.id
                    ),
                ));
            }
            if !subgraph_node_ids.contains(&edge.to_node_id) {
                diagnostics.push(error(
                    "subgraph_edge_target_missing",
                    format!("subgraphs.{index}.edges.{edge_index}.toNodeId"),
                    format!(
                        "Subgraph edge `{}` references a missing target node.",
                        edge.id
                    ),
                ));
            }
            validate_condition_shape(
                &edge.condition,
                format!("subgraphs.{index}.edges.{edge_index}.condition"),
                &mut diagnostics,
            );
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
    let mut outgoing_defaults: BTreeMap<String, &str> = BTreeMap::new();
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
            let buckets = default_edge_buckets(edge.r#type);
            let conflicts = buckets.iter().any(|bucket| {
                if *bucket == "all" {
                    outgoing_defaults
                        .keys()
                        .any(|key| key.starts_with(&format!("{}:", edge.from_node_id)))
                } else {
                    outgoing_defaults.contains_key(&format!("{}:all", edge.from_node_id))
                        || outgoing_defaults
                            .contains_key(&format!("{}:{bucket}", edge.from_node_id))
                }
            });
            if conflicts {
                diagnostics.push(error(
                    "duplicate_default_edge",
                    format!("edges.{index}.condition"),
                    format!(
                        "Node `{}` has more than one default else edge.",
                        edge.from_node_id
                    ),
                ));
            } else {
                for bucket in buckets {
                    outgoing_defaults
                        .insert(format!("{}:{bucket}", edge.from_node_id), edge.id.as_str());
                }
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
        for (artifact_ref, json_path) in condition_artifact_field_refs(&edge.condition) {
            if let Some(contract) = artifact_contracts_by_ref.get(&artifact_ref) {
                if let Some(json_schema) = contract.json_schema.as_ref() {
                    if !json_schema_allows_path(json_schema, &json_path) {
                        diagnostics.push(error(
                            "condition_artifact_path_not_in_schema",
                            format!("edges.{index}.condition"),
                            format!(
                                "Condition references `{artifact_ref}{json_path}`, but that field is not allowed by the artifact schema."
                            ),
                        ));
                    }
                }
            }
        }
        for state_ref in condition_state_refs(&edge.condition) {
            if !produced_artifacts.contains(&state_ref) {
                diagnostics.push(error(
                    "condition_state_ref_missing",
                    format!("edges.{index}.condition"),
                    format!("Condition references missing state value `{state_ref}`."),
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
        if let Some(input_bindings) = node_input_bindings(node) {
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
                if let WorkflowInputBindingDto::State { state_ref, .. } = binding {
                    if !produced_artifacts.contains(state_ref) {
                        diagnostics.push(error(
                            "state_ref_missing",
                            format!("nodes.{index}.inputBindings.{binding_index}.stateRef"),
                            format!(
                                "State reference `{state_ref}` is not produced by any state-capable node."
                            ),
                        ));
                    }
                }
            }
        }
        match node {
            WorkflowNodeDto::StateRead { query, .. }
            | WorkflowNodeDto::StateQuery { query, .. } => {
                validate_state_query(query, format!("nodes.{index}.query"), &mut diagnostics);
            }
            WorkflowNodeDto::StateWrite { operation, .. }
            | WorkflowNodeDto::StatePatch { operation, .. } => {
                validate_state_write_operation(
                    operation,
                    format!("nodes.{index}.operation"),
                    &mut diagnostics,
                    true,
                );
            }
            WorkflowNodeDto::CollectionLoop {
                collection,
                sort_key,
                max_item_count,
                ..
            } => {
                validate_state_query(
                    collection,
                    format!("nodes.{index}.collection"),
                    &mut diagnostics,
                );
                if *max_item_count == 0 {
                    diagnostics.push(error(
                        "collection_loop_max_item_count_invalid",
                        format!("nodes.{index}.maxItemCount"),
                        "Collection loops must allow at least one item.",
                    ));
                }
                if sort_key
                    .as_deref()
                    .is_some_and(|path| !path.trim().starts_with('$'))
                {
                    diagnostics.push(error(
                        "collection_loop_sort_path_invalid",
                        format!("nodes.{index}.sortKey"),
                        "Collection loop sort keys must use a JSON path that starts with `$`.",
                    ));
                }
            }
            WorkflowNodeDto::Subgraph { subgraph_id, .. } => {
                if !subgraph_ids.contains(subgraph_id) {
                    diagnostics.push(error(
                        "subgraph_ref_missing",
                        format!("nodes.{index}.subgraphId"),
                        format!("Subgraph node references missing subgraph `{subgraph_id}`."),
                    ));
                }
            }
            WorkflowNodeDto::Command {
                command,
                allowed_commands,
                timeout_seconds,
                ..
            } => {
                if command.trim().is_empty() {
                    diagnostics.push(error(
                        "command_empty",
                        format!("nodes.{index}.command"),
                        "Command nodes must declare a command.",
                    ));
                }
                if *timeout_seconds == 0 {
                    diagnostics.push(error(
                        "command_timeout_invalid",
                        format!("nodes.{index}.timeoutSeconds"),
                        "Command node timeout must be at least one second.",
                    ));
                }
                if allowed_commands.is_empty() {
                    diagnostics.push(error(
                        "command_allowlist_empty",
                        format!("nodes.{index}.allowedCommands"),
                        "Command nodes must declare an allowlist.",
                    ));
                } else if !allowed_commands.iter().any(|allowed| allowed == command) {
                    diagnostics.push(error(
                        "command_not_in_allowlist",
                        format!("nodes.{index}.allowedCommands"),
                        format!("Command `{command}` must appear in the command node allowlist."),
                    ));
                }
            }
            WorkflowNodeDto::HumanCheckpoint {
                decision_options,
                resume_payload_schema,
                state_updates,
                ..
            } => {
                let mut seen = BTreeSet::new();
                for (option_index, option) in decision_options.iter().enumerate() {
                    let option = option.trim();
                    if option.is_empty() {
                        diagnostics.push(error(
                            "checkpoint_decision_empty",
                            format!("nodes.{index}.decisionOptions.{option_index}"),
                            "Human checkpoint decision options cannot be blank.",
                        ));
                    } else if !seen.insert(option.to_string()) {
                        diagnostics.push(error(
                            "checkpoint_decision_duplicate",
                            format!("nodes.{index}.decisionOptions.{option_index}"),
                            format!("Human checkpoint decision `{option}` is duplicated."),
                        ));
                    }
                }
                if resume_payload_schema
                    .as_ref()
                    .is_some_and(|schema| !schema.is_object())
                {
                    diagnostics.push(error(
                        "checkpoint_payload_schema_invalid",
                        format!("nodes.{index}.resumePayloadSchema"),
                        "Human checkpoint resume payload schemas must be JSON Schema objects.",
                    ));
                }
                for (update_index, operation) in state_updates.iter().enumerate() {
                    validate_state_write_operation(
                        operation,
                        format!("nodes.{index}.stateUpdates.{update_index}"),
                        &mut diagnostics,
                        false,
                    );
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

fn validate_state_query(
    query: &crate::commands::contracts::workflows::WorkflowStateQueryDto,
    path: String,
    diagnostics: &mut Vec<WorkflowValidationDiagnosticDto>,
) {
    for (index, filter) in query.filters.iter().enumerate() {
        if !filter.path.trim().starts_with('$') {
            diagnostics.push(error(
                "state_query_filter_path_invalid",
                format!("{path}.filters.{index}.path"),
                "State query filter paths must use a JSON path that starts with `$`.",
            ));
        }
    }
    if query
        .order_by
        .as_deref()
        .is_some_and(|order_by| !order_by.trim().starts_with('$'))
    {
        diagnostics.push(error(
            "state_query_order_path_invalid",
            format!("{path}.orderBy"),
            "State query order paths must use a JSON path that starts with `$`.",
        ));
    }
}

fn validate_state_write_operation(
    operation: &crate::commands::contracts::workflows::WorkflowStateWriteOperationDto,
    path: String,
    diagnostics: &mut Vec<WorkflowValidationDiagnosticDto>,
    require_output_artifact: bool,
) {
    if require_output_artifact && operation.output_artifact_type.trim().is_empty() {
        diagnostics.push(error(
            "state_write_output_artifact_empty",
            format!("{path}.outputArtifactType"),
            "State write nodes must name their output artifact.",
        ));
    }
    if operation
        .idempotency_key
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        diagnostics.push(error(
            "state_write_idempotency_key_empty",
            format!("{path}.idempotencyKey"),
            "State write idempotency keys cannot be blank.",
        ));
    }
    if operation
        .target_id
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        diagnostics.push(error(
            "state_write_target_id_empty",
            format!("{path}.targetId"),
            "State write target ids cannot be blank.",
        ));
    }
}

fn node_input_bindings(node: &WorkflowNodeDto) -> Option<&Vec<WorkflowInputBindingDto>> {
    match node {
        WorkflowNodeDto::Agent { input_bindings, .. }
        | WorkflowNodeDto::StateWrite { input_bindings, .. }
        | WorkflowNodeDto::StatePatch { input_bindings, .. }
        | WorkflowNodeDto::Subgraph { input_bindings, .. } => Some(input_bindings),
        _ => None,
    }
}

fn default_edge_buckets(edge_type: WorkflowEdgeTypeDto) -> Vec<&'static str> {
    match edge_type {
        WorkflowEdgeTypeDto::Success => vec!["success"],
        WorkflowEdgeTypeDto::Failure | WorkflowEdgeTypeDto::Recovery => vec!["failure"],
        WorkflowEdgeTypeDto::Conditional
        | WorkflowEdgeTypeDto::Loop
        | WorkflowEdgeTypeDto::ManualOverride => vec!["all"],
    }
}

fn condition_artifact_field_refs(condition: &WorkflowConditionDto) -> Vec<(String, String)> {
    match condition {
        WorkflowConditionDto::ArtifactFieldEquals {
            artifact_ref, path, ..
        }
        | WorkflowConditionDto::ArtifactFieldIn {
            artifact_ref, path, ..
        }
        | WorkflowConditionDto::ArtifactFieldNumberCompare {
            artifact_ref, path, ..
        } => vec![(artifact_ref.clone(), path.clone())],
        WorkflowConditionDto::All { conditions } | WorkflowConditionDto::Any { conditions } => {
            conditions
                .iter()
                .flat_map(condition_artifact_field_refs)
                .collect()
        }
        WorkflowConditionDto::Not { condition } => condition_artifact_field_refs(condition),
        _ => Vec::new(),
    }
}

fn json_schema_allows_path(schema: &JsonValue, path: &str) -> bool {
    if path == "$" {
        return true;
    }
    let Some(remainder) = path.strip_prefix("$.") else {
        return false;
    };
    let mut cursor = schema;
    for raw_segment in remainder.split('.') {
        let Some((field, indexes)) = parse_schema_path_segment(raw_segment) else {
            return false;
        };
        if !schema_type_allows_object(cursor) {
            return false;
        }
        let Some(properties) = cursor.get("properties").and_then(JsonValue::as_object) else {
            return false;
        };
        let Some(next) = properties.get(field) else {
            return false;
        };
        cursor = next;
        for _ in 0..indexes {
            let Some(items) = cursor.get("items") else {
                return false;
            };
            cursor = items;
        }
    }
    true
}

fn parse_schema_path_segment(segment: &str) -> Option<(&str, usize)> {
    let field_end = segment.find('[').unwrap_or(segment.len());
    let field = &segment[..field_end];
    if field.is_empty() {
        return None;
    }
    let mut indexes = 0;
    let mut rest = &segment[field_end..];
    while !rest.is_empty() {
        let inner = rest.strip_prefix('[')?;
        let close = inner.find(']')?;
        inner[..close].parse::<usize>().ok()?;
        indexes += 1;
        rest = &inner[close + 1..];
    }
    Some((field, indexes))
}

fn schema_type_allows_object(schema: &JsonValue) -> bool {
    match schema.get("type") {
        Some(JsonValue::String(value)) => value == "object",
        Some(JsonValue::Array(values)) => values.iter().any(|value| value == "object"),
        _ => true,
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
        }
        | WorkflowConditionDto::StateFieldEquals {
            path: json_path, ..
        } => {
            if !json_path.starts_with('$') {
                diagnostics.push(error(
                    "condition_json_path_invalid",
                    path,
                    "Workflow field conditions must use a JSON path that starts with `$`.",
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

fn condition_state_refs(condition: &WorkflowConditionDto) -> Vec<String> {
    match condition {
        WorkflowConditionDto::StateFieldEquals { state_ref, .. }
        | WorkflowConditionDto::StateCollectionCountCompare { state_ref, .. } => {
            vec![state_ref.clone()]
        }
        WorkflowConditionDto::All { conditions } | WorkflowConditionDto::Any { conditions } => {
            conditions.iter().flat_map(condition_state_refs).collect()
        }
        WorkflowConditionDto::Not { condition } => condition_state_refs(condition),
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
            WorkflowArtifactContractDto, WorkflowEdgeDto, WorkflowEdgeTypeDto,
            WorkflowInputBindingDto, WorkflowNodeDto, WorkflowOutputContractDto,
            WorkflowOutputExtractionDto, WorkflowRunPolicyDto, WorkflowTerminalStatusDto,
            WorkflowValidationStatusDto,
        },
    };
    use serde_json::json;

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
            subgraphs: Vec::new(),
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
            resume_payload_schema: None,
            state_updates: Vec::new(),
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

    #[test]
    fn validator_rejects_condition_path_outside_artifact_schema() {
        let mut definition = linear_definition();
        if let WorkflowNodeDto::Agent {
            output_contract, ..
        } = &mut definition.nodes[0]
        {
            output_contract.artifact_type = "verification_result".into();
            output_contract.extraction = WorkflowOutputExtractionDto::JsonObject;
        }
        if let WorkflowNodeDto::Agent { input_bindings, .. } = &mut definition.nodes[1] {
            *input_bindings = vec![WorkflowInputBindingDto::Artifact {
                name: "handoff".into(),
                required: true,
                artifact_ref: "agent-a.verification_result".into(),
                path: None,
                prompt_label: None,
            }];
        }
        definition
            .artifact_contracts
            .push(WorkflowArtifactContractDto {
                artifact_type: "verification_result".into(),
                schema_version: 1,
                json_schema: Some(json!({
                    "type": "object",
                    "required": ["status"],
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["passed", "gaps_found", "human_needed"]
                        }
                    },
                    "additionalProperties": false
                })),
                display_name: "Verification result".into(),
                description: String::new(),
            });
        definition.edges[0].condition = WorkflowConditionDto::ArtifactFieldEquals {
            artifact_ref: "agent-a.verification_result".into(),
            path: "$.missing".into(),
            value: json!("passed"),
        };

        let report = validate_workflow_definition(&definition);

        assert_eq!(report.status, WorkflowValidationStatusDto::Invalid);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "condition_artifact_path_not_in_schema"));
    }

    #[test]
    fn validator_accepts_condition_path_declared_by_artifact_schema() {
        let mut definition = linear_definition();
        if let WorkflowNodeDto::Agent {
            output_contract, ..
        } = &mut definition.nodes[0]
        {
            output_contract.artifact_type = "verification_result".into();
            output_contract.extraction = WorkflowOutputExtractionDto::JsonObject;
        }
        if let WorkflowNodeDto::Agent { input_bindings, .. } = &mut definition.nodes[1] {
            *input_bindings = vec![WorkflowInputBindingDto::Artifact {
                name: "handoff".into(),
                required: true,
                artifact_ref: "agent-a.verification_result".into(),
                path: None,
                prompt_label: None,
            }];
        }
        definition
            .artifact_contracts
            .push(WorkflowArtifactContractDto {
                artifact_type: "verification_result".into(),
                schema_version: 1,
                json_schema: Some(json!({
                    "type": "object",
                    "required": ["status"],
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["passed", "gaps_found", "human_needed"]
                        }
                    },
                    "additionalProperties": false
                })),
                display_name: "Verification result".into(),
                description: String::new(),
            });
        definition.edges[0].condition = WorkflowConditionDto::ArtifactFieldEquals {
            artifact_ref: "agent-a.verification_result".into(),
            path: "$.status".into(),
            value: json!("passed"),
        };

        let report = validate_workflow_definition(&definition);

        assert_eq!(report.status, WorkflowValidationStatusDto::Valid);
    }
}
