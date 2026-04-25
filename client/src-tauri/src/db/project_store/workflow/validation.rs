use super::*;

pub(crate) fn validate_workflow_graph_upsert_payload(
    graph: &WorkflowGraphUpsertRecord,
) -> Result<(), CommandError> {
    use std::collections::BTreeSet;

    let mut node_ids = BTreeSet::new();
    let mut phase_ids = BTreeSet::new();
    let mut sort_orders = BTreeSet::new();

    for node in &graph.nodes {
        validate_non_empty_text(&node.node_id, "node_id", "workflow_graph_request_invalid")?;
        validate_non_empty_text(&node.name, "name", "workflow_graph_request_invalid")?;

        if node.completed_tasks > node.task_count {
            return Err(CommandError::user_fixable(
                "workflow_graph_request_invalid",
                format!(
                    "Workflow node `{}` has completed_tasks ({}) greater than task_count ({}).",
                    node.node_id, node.completed_tasks, node.task_count
                ),
            ));
        }

        if !node_ids.insert(node.node_id.as_str()) {
            return Err(CommandError::user_fixable(
                "workflow_graph_request_invalid",
                format!(
                    "Workflow graph contains duplicate node id `{}`.",
                    node.node_id
                ),
            ));
        }

        if !phase_ids.insert(node.phase_id) {
            return Err(CommandError::user_fixable(
                "workflow_graph_request_invalid",
                format!(
                    "Workflow graph contains duplicate phase id `{}`.",
                    node.phase_id
                ),
            ));
        }

        if !sort_orders.insert(node.sort_order) {
            return Err(CommandError::user_fixable(
                "workflow_graph_request_invalid",
                format!(
                    "Workflow graph contains duplicate sort order `{}`.",
                    node.sort_order
                ),
            ));
        }
    }

    for edge in &graph.edges {
        validate_non_empty_text(
            &edge.from_node_id,
            "from_node_id",
            "workflow_graph_request_invalid",
        )?;
        validate_non_empty_text(
            &edge.to_node_id,
            "to_node_id",
            "workflow_graph_request_invalid",
        )?;
        validate_non_empty_text(
            &edge.transition_kind,
            "transition_kind",
            "workflow_graph_request_invalid",
        )?;

        if !node_ids.contains(edge.from_node_id.as_str())
            || !node_ids.contains(edge.to_node_id.as_str())
        {
            return Err(CommandError::user_fixable(
                "workflow_graph_request_invalid",
                format!(
                    "Workflow edge `{}` -> `{}` references unknown node ids.",
                    edge.from_node_id, edge.to_node_id
                ),
            ));
        }
    }

    for gate in &graph.gates {
        validate_non_empty_text(&gate.node_id, "node_id", "workflow_graph_request_invalid")?;
        validate_non_empty_text(&gate.gate_key, "gate_key", "workflow_graph_request_invalid")?;

        if !node_ids.contains(gate.node_id.as_str()) {
            return Err(CommandError::user_fixable(
                "workflow_graph_request_invalid",
                format!(
                    "Workflow gate `{}` references unknown node `{}`.",
                    gate.gate_key, gate.node_id
                ),
            ));
        }

        if matches!(
            gate.gate_state,
            WorkflowGateState::Pending | WorkflowGateState::Blocked
        ) && (gate.action_type.is_none() || gate.title.is_none() || gate.detail.is_none())
        {
            return Err(CommandError::user_fixable(
                "workflow_graph_request_invalid",
                format!(
                    "Workflow gate `{}` for node `{}` requires action_type/title/detail when pending or blocked.",
                    gate.gate_key, gate.node_id
                ),
            ));
        }
    }

    Ok(())
}

pub(crate) fn validate_workflow_transition_payload(
    transition: &ApplyWorkflowTransitionRecord,
) -> Result<(), CommandError> {
    validate_non_empty_text(
        &transition.transition_id,
        "transition_id",
        "workflow_transition_request_invalid",
    )?;
    validate_non_empty_text(
        &transition.from_node_id,
        "from_node_id",
        "workflow_transition_request_invalid",
    )?;
    validate_non_empty_text(
        &transition.to_node_id,
        "to_node_id",
        "workflow_transition_request_invalid",
    )?;
    validate_non_empty_text(
        &transition.transition_kind,
        "transition_kind",
        "workflow_transition_request_invalid",
    )?;
    validate_non_empty_text(
        &transition.occurred_at,
        "occurred_at",
        "workflow_transition_request_invalid",
    )?;

    if transition.from_node_id == transition.to_node_id {
        return Err(CommandError::user_fixable(
            "workflow_transition_request_invalid",
            "Workflow transitions must change node ids.",
        ));
    }

    for gate_update in &transition.gate_updates {
        validate_non_empty_text(
            &gate_update.gate_key,
            "gate_key",
            "workflow_transition_request_invalid",
        )?;
    }

    if let Some(secret_hint) = transition
        .gate_decision_context
        .as_deref()
        .and_then(find_prohibited_transition_diagnostic_content)
    {
        return Err(CommandError::user_fixable(
            "workflow_transition_request_invalid",
            format!(
                "Workflow transition diagnostics must not include {secret_hint}. Remove secret-bearing transcript/tool/auth payload content before retrying."
            ),
        ));
    }

    for gate_update in &transition.gate_updates {
        if let Some(secret_hint) = gate_update
            .decision_context
            .as_deref()
            .and_then(find_prohibited_transition_diagnostic_content)
        {
            return Err(CommandError::user_fixable(
                "workflow_transition_request_invalid",
                format!(
                    "Workflow gate diagnostics for `{}` must not include {secret_hint}. Remove secret-bearing transcript/tool/auth payload content before retrying.",
                    gate_update.gate_key
                ),
            ));
        }
    }

    Ok(())
}

pub(crate) fn validate_workflow_handoff_package_payload(
    payload: &WorkflowHandoffPackageUpsertRecord,
) -> Result<(), CommandError> {
    validate_non_empty_text(
        &payload.project_id,
        "project_id",
        "workflow_handoff_request_invalid",
    )?;
    validate_non_empty_text(
        &payload.handoff_transition_id,
        "handoff_transition_id",
        "workflow_handoff_request_invalid",
    )?;
    if let Some(causal_transition_id) = payload.causal_transition_id.as_deref() {
        validate_non_empty_text(
            causal_transition_id,
            "causal_transition_id",
            "workflow_handoff_request_invalid",
        )?;
    }
    validate_non_empty_text(
        &payload.from_node_id,
        "from_node_id",
        "workflow_handoff_request_invalid",
    )?;
    validate_non_empty_text(
        &payload.to_node_id,
        "to_node_id",
        "workflow_handoff_request_invalid",
    )?;
    validate_non_empty_text(
        &payload.transition_kind,
        "transition_kind",
        "workflow_handoff_request_invalid",
    )?;
    validate_non_empty_text(
        &payload.package_payload,
        "package_payload",
        "workflow_handoff_request_invalid",
    )?;
    validate_non_empty_text(
        &payload.created_at,
        "created_at",
        "workflow_handoff_request_invalid",
    )?;
    validate_rfc3339_timestamp(
        &payload.created_at,
        "created_at",
        None,
        "workflow_handoff_request_invalid",
    )?;

    if let Some(secret_hint) = find_prohibited_workflow_handoff_content(&payload.package_payload) {
        return Err(CommandError::user_fixable(
            "workflow_handoff_request_invalid",
            format!(
                "Workflow handoff packages must not include {secret_hint}. Remove secret-bearing transcript/tool/auth payload content before retrying."
            ),
        ));
    }

    canonicalize_workflow_handoff_package_payload(
        &payload.package_payload,
        None,
        "workflow_handoff_request_invalid",
    )?;

    Ok(())
}

pub(crate) fn validate_workflow_handoff_transition_metadata(
    payload: &WorkflowHandoffPackageUpsertRecord,
    transition_event: &WorkflowTransitionEventRecord,
) -> Result<(), CommandError> {
    if payload.from_node_id != transition_event.from_node_id {
        return Err(CommandError::user_fixable(
            "workflow_handoff_request_invalid",
            format!(
                "Workflow handoff package source node `{}` does not match transition `{}` source node `{}`.",
                payload.from_node_id,
                payload.handoff_transition_id,
                transition_event.from_node_id
            ),
        ));
    }

    if payload.to_node_id != transition_event.to_node_id {
        return Err(CommandError::user_fixable(
            "workflow_handoff_request_invalid",
            format!(
                "Workflow handoff package target node `{}` does not match transition `{}` target node `{}`.",
                payload.to_node_id,
                payload.handoff_transition_id,
                transition_event.to_node_id
            ),
        ));
    }

    if payload.transition_kind != transition_event.transition_kind {
        return Err(CommandError::user_fixable(
            "workflow_handoff_request_invalid",
            format!(
                "Workflow handoff package transition kind `{}` does not match transition `{}` kind `{}`.",
                payload.transition_kind,
                payload.handoff_transition_id,
                transition_event.transition_kind
            ),
        ));
    }

    if let Some(causal_transition_id) = payload.causal_transition_id.as_deref() {
        if transition_event.causal_transition_id.as_deref() != Some(causal_transition_id) {
            return Err(CommandError::user_fixable(
                "workflow_handoff_request_invalid",
                format!(
                    "Workflow handoff package causal transition `{}` does not match transition `{}` causal linkage `{:?}`.",
                    causal_transition_id,
                    payload.handoff_transition_id,
                    transition_event.causal_transition_id
                ),
            ));
        }
    }

    Ok(())
}

pub(crate) fn canonicalize_workflow_handoff_package_payload(
    value: &str,
    database_path: Option<&Path>,
    code: &str,
) -> Result<String, CommandError> {
    let parsed: serde_json::Value = serde_json::from_str(value).map_err(|error| {
        map_workflow_handoff_payload_error(
            code,
            database_path,
            format!("Field `package_payload` must be valid JSON text: {error}"),
        )
    })?;

    if !parsed.is_object() {
        return Err(map_workflow_handoff_payload_error(
            code,
            database_path,
            "Field `package_payload` must be a JSON object with redacted context metadata.".into(),
        ));
    }

    let canonical = canonicalize_workflow_handoff_json_value(parsed);
    serde_json::to_string(&canonical).map_err(|error| {
        map_workflow_handoff_payload_error(
            code,
            database_path,
            format!("Field `package_payload` could not be canonicalized: {error}"),
        )
    })
}

pub(crate) fn canonicalize_workflow_handoff_json_value(
    value: serde_json::Value,
) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut sorted = std::collections::BTreeMap::new();
            for (key, nested) in map {
                sorted.insert(key, canonicalize_workflow_handoff_json_value(nested));
            }

            serde_json::Value::Object(sorted.into_iter().collect())
        }
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items
                .into_iter()
                .map(canonicalize_workflow_handoff_json_value)
                .collect(),
        ),
        other => other,
    }
}

pub(crate) fn compute_workflow_handoff_package_hash(canonical_payload: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_payload.as_bytes());
    let digest = hasher.finalize();

    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn validate_workflow_handoff_package_hash(
    value: &str,
    field: &str,
    database_path: &Path,
    code: &str,
) -> Result<(), CommandError> {
    if value.len() != 64 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(map_snapshot_decode_error(
            code,
            database_path,
            format!("Field `{field}` must be a 64-character hexadecimal hash."),
        ));
    }

    if value
        .chars()
        .any(|character| character.is_ascii_uppercase())
    {
        return Err(map_snapshot_decode_error(
            code,
            database_path,
            format!("Field `{field}` must use lowercase hexadecimal characters."),
        ));
    }

    Ok(())
}

pub(crate) fn validate_rfc3339_timestamp(
    value: &str,
    field: &str,
    database_path: Option<&Path>,
    code: &str,
) -> Result<(), CommandError> {
    OffsetDateTime::parse(value, &Rfc3339).map_err(|error| {
        map_workflow_handoff_payload_error(
            code,
            database_path,
            format!("Field `{field}` must be RFC3339 text: {error}"),
        )
    })?;

    Ok(())
}

fn map_workflow_handoff_payload_error(
    code: &str,
    database_path: Option<&Path>,
    details: String,
) -> CommandError {
    match database_path {
        Some(database_path) => map_snapshot_decode_error(code, database_path, details),
        None => CommandError::user_fixable(code, details),
    }
}
