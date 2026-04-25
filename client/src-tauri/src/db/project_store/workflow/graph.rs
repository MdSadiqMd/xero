use super::*;

pub fn load_workflow_graph(
    repo_root: &Path,
    expected_project_id: &str,
) -> Result<WorkflowGraphRecord, CommandError> {
    let database_path = database_path_for_repo(repo_root);
    let connection = open_project_database(repo_root, &database_path)?;
    read_project_row(&connection, &database_path, repo_root, expected_project_id)?;

    let nodes = read_workflow_graph_nodes(&connection, &database_path, expected_project_id)?;
    let edges = read_workflow_graph_edges(&connection, &database_path, expected_project_id)?;
    let gates = read_workflow_gate_metadata(&connection, &database_path, expected_project_id)?;

    Ok(WorkflowGraphRecord {
        nodes,
        edges,
        gates,
    })
}

pub fn upsert_workflow_graph(
    repo_root: &Path,
    expected_project_id: &str,
    graph: &WorkflowGraphUpsertRecord,
) -> Result<WorkflowGraphRecord, CommandError> {
    validate_workflow_graph_upsert_payload(graph)?;

    let database_path = database_path_for_repo(repo_root);
    let connection = open_project_database(repo_root, &database_path)?;
    read_project_row(&connection, &database_path, repo_root, expected_project_id)?;

    let transaction = connection.unchecked_transaction().map_err(|error| {
        map_workflow_graph_transaction_error(
            "workflow_graph_transaction_failed",
            &database_path,
            error,
            "Cadence could not start the workflow-graph upsert transaction.",
        )
    })?;

    transaction
        .execute(
            "DELETE FROM workflow_graph_edges WHERE project_id = ?1",
            params![expected_project_id],
        )
        .map_err(|error| {
            map_workflow_graph_write_error(
                "workflow_graph_clear_failed",
                &database_path,
                error,
                "Cadence could not clear previous workflow edges.",
            )
        })?;

    transaction
        .execute(
            "DELETE FROM workflow_gate_metadata WHERE project_id = ?1",
            params![expected_project_id],
        )
        .map_err(|error| {
            map_workflow_graph_write_error(
                "workflow_graph_clear_failed",
                &database_path,
                error,
                "Cadence could not clear previous workflow gate metadata.",
            )
        })?;

    transaction
        .execute(
            "DELETE FROM workflow_graph_nodes WHERE project_id = ?1",
            params![expected_project_id],
        )
        .map_err(|error| {
            map_workflow_graph_write_error(
                "workflow_graph_clear_failed",
                &database_path,
                error,
                "Cadence could not clear previous workflow graph nodes.",
            )
        })?;

    for node in &graph.nodes {
        transaction
            .execute(
                r#"
                INSERT INTO workflow_graph_nodes (
                    project_id,
                    node_id,
                    phase_id,
                    sort_order,
                    name,
                    description,
                    status,
                    current_step,
                    task_count,
                    completed_tasks,
                    summary,
                    updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                "#,
                params![
                    expected_project_id,
                    node.node_id.as_str(),
                    i64::from(node.phase_id),
                    i64::from(node.sort_order),
                    node.name.as_str(),
                    node.description.as_str(),
                    phase_status_sql_value(&node.status),
                    node.current_step.as_ref().map(phase_step_sql_value),
                    i64::from(node.task_count),
                    i64::from(node.completed_tasks),
                    node.summary.as_deref(),
                ],
            )
            .map_err(|error| {
                map_workflow_graph_write_error(
                    "workflow_graph_node_upsert_failed",
                    &database_path,
                    error,
                    "Cadence could not persist a workflow graph node.",
                )
            })?;
    }

    for edge in &graph.edges {
        transaction
            .execute(
                r#"
                INSERT INTO workflow_graph_edges (
                    project_id,
                    from_node_id,
                    to_node_id,
                    transition_kind,
                    gate_requirement,
                    updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                "#,
                params![
                    expected_project_id,
                    edge.from_node_id.as_str(),
                    edge.to_node_id.as_str(),
                    edge.transition_kind.as_str(),
                    edge.gate_requirement.as_deref(),
                ],
            )
            .map_err(|error| {
                map_workflow_graph_write_error(
                    "workflow_graph_edge_upsert_failed",
                    &database_path,
                    error,
                    "Cadence could not persist a workflow graph edge.",
                )
            })?;
    }

    for gate in &graph.gates {
        transaction
            .execute(
                r#"
                INSERT INTO workflow_gate_metadata (
                    project_id,
                    node_id,
                    gate_key,
                    gate_state,
                    action_type,
                    title,
                    detail,
                    decision_context,
                    updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                "#,
                params![
                    expected_project_id,
                    gate.node_id.as_str(),
                    gate.gate_key.as_str(),
                    workflow_gate_state_sql_value(&gate.gate_state),
                    gate.action_type.as_deref(),
                    gate.title.as_deref(),
                    gate.detail.as_deref(),
                    gate.decision_context.as_deref(),
                ],
            )
            .map_err(|error| {
                map_workflow_graph_write_error(
                    "workflow_gate_upsert_failed",
                    &database_path,
                    error,
                    "Cadence could not persist workflow gate metadata.",
                )
            })?;
    }

    transaction.commit().map_err(|error| {
        map_workflow_graph_commit_error(
            "workflow_graph_commit_failed",
            &database_path,
            error,
            "Cadence could not commit the workflow-graph upsert transaction.",
        )
    })?;

    load_workflow_graph(repo_root, expected_project_id)
}

pub fn apply_workflow_transition(
    repo_root: &Path,
    expected_project_id: &str,
    transition: &ApplyWorkflowTransitionRecord,
) -> Result<ApplyWorkflowTransitionResult, CommandError> {
    validate_workflow_transition_payload(transition)?;

    let database_path = database_path_for_repo(repo_root);
    let mut connection = open_project_database(repo_root, &database_path)?;
    read_project_row(&connection, &database_path, repo_root, expected_project_id)?;

    let transition_event = if let Some(existing) = read_transition_event_by_transition_id(
        &connection,
        &database_path,
        expected_project_id,
        &transition.transition_id,
    )? {
        existing
    } else {
        let transaction = connection.unchecked_transaction().map_err(|error| {
            map_workflow_graph_transaction_error(
                "workflow_transition_transaction_failed",
                &database_path,
                error,
                "Cadence could not start the workflow-transition transaction.",
            )
        })?;

        let transition_mutation = build_transition_mutation_record(transition);
        let mutation_outcome = apply_workflow_transition_mutation(
            &transaction,
            &database_path,
            expected_project_id,
            &transition_mutation,
            &WORKFLOW_TRANSITION_COMMAND_MUTATION_ERROR_PROFILE,
            map_workflow_graph_write_error,
        )?;

        match mutation_outcome {
            WorkflowTransitionMutationApplyOutcome::Replayed(transition_event) => transition_event,
            WorkflowTransitionMutationApplyOutcome::Applied => {
                transaction.commit().map_err(|error| {
                    map_workflow_graph_commit_error(
                        "workflow_transition_commit_failed",
                        &database_path,
                        error,
                        "Cadence could not commit the workflow transition transaction.",
                    )
                })?;

                read_transition_event_by_transition_id(
                    &connection,
                    &database_path,
                    expected_project_id,
                    &transition.transition_id,
                )?
                .ok_or_else(|| {
                    CommandError::system_fault(
                        "workflow_transition_event_missing_after_persist",
                        format!(
                            "Cadence persisted transition `{}` in {} but could not read it back.",
                            transition.transition_id,
                            database_path.display()
                        ),
                    )
                })?
            }
        }
    };

    let automatic_dispatch = attempt_automatic_dispatch_after_transition(
        &mut connection,
        &database_path,
        expected_project_id,
        &transition_event,
    );

    let phases = read_phase_summaries(&connection, &database_path, expected_project_id)?;

    Ok(ApplyWorkflowTransitionResult {
        transition_event,
        automatic_dispatch,
        phases,
    })
}

pub fn load_workflow_transition_event(
    repo_root: &Path,
    expected_project_id: &str,
    transition_id: &str,
) -> Result<Option<WorkflowTransitionEventRecord>, CommandError> {
    validate_non_empty_text(
        transition_id,
        "transition_id",
        "workflow_transition_request_invalid",
    )?;

    let database_path = database_path_for_repo(repo_root);
    let connection = open_project_database(repo_root, &database_path)?;
    read_project_row(&connection, &database_path, repo_root, expected_project_id)?;

    read_transition_event_by_transition_id(
        &connection,
        &database_path,
        expected_project_id,
        transition_id,
    )
}

pub fn load_recent_workflow_transition_events(
    repo_root: &Path,
    expected_project_id: &str,
    limit: Option<u32>,
) -> Result<Vec<WorkflowTransitionEventRecord>, CommandError> {
    let database_path = database_path_for_repo(repo_root);
    let connection = open_project_database(repo_root, &database_path)?;
    read_project_row(&connection, &database_path, repo_root, expected_project_id)?;

    read_transition_events(
        &connection,
        &database_path,
        expected_project_id,
        limit.map(i64::from),
    )
}
