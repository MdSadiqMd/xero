use super::*;

#[derive(Debug)]
struct RawGraphNodeRow {
    node_id: String,
    phase_id: i64,
    sort_order: i64,
    name: String,
    description: String,
    status: String,
    current_step: Option<String>,
    task_count: i64,
    completed_tasks: i64,
    summary: Option<String>,
}

#[derive(Debug)]
struct RawGraphEdgeRow {
    from_node_id: String,
    to_node_id: String,
    transition_kind: String,
    gate_requirement: Option<String>,
}

#[derive(Debug)]
struct RawGateMetadataRow {
    node_id: String,
    gate_key: String,
    gate_state: String,
    action_type: Option<String>,
    title: Option<String>,
    detail: Option<String>,
    decision_context: Option<String>,
}

#[derive(Debug)]
struct RawTransitionEventRow {
    id: i64,
    transition_id: String,
    causal_transition_id: Option<String>,
    from_node_id: String,
    to_node_id: String,
    transition_kind: String,
    gate_decision: String,
    gate_decision_context: Option<String>,
    created_at: String,
}

#[derive(Debug)]
struct RawWorkflowHandoffPackageRow {
    id: i64,
    project_id: String,
    handoff_transition_id: String,
    causal_transition_id: Option<String>,
    from_node_id: String,
    to_node_id: String,
    transition_kind: String,
    package_payload: String,
    package_hash: String,
    created_at: String,
}

pub(crate) fn read_project_row(
    connection: &Connection,
    database_path: &Path,
    repo_root: &Path,
    expected_project_id: &str,
) -> Result<ProjectSummaryRow, CommandError> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                name,
                description,
                milestone,
                branch,
                runtime
            FROM projects
            WHERE id = ?1
            "#,
            [expected_project_id],
            |row| {
                Ok(ProjectSummaryRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    milestone: row.get(3)?,
                    branch: row.get(4)?,
                    runtime: row.get(5)?,
                })
            },
        )
        .map_err(|error| {
            map_project_query_error(error, database_path, repo_root, expected_project_id)
        })
}

pub(crate) fn read_workflow_graph_nodes(
    connection: &Connection,
    database_path: &Path,
    expected_project_id: &str,
) -> Result<Vec<WorkflowGraphNodeRecord>, CommandError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                node_id,
                phase_id,
                sort_order,
                name,
                description,
                status,
                current_step,
                task_count,
                completed_tasks,
                summary
            FROM workflow_graph_nodes
            WHERE project_id = ?1
            ORDER BY sort_order ASC, phase_id ASC
            "#,
        )
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_graph_query_failed",
                format!(
                    "Cadence could not prepare workflow-graph node rows from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let raw_rows = statement
        .query_map([expected_project_id], |row| {
            Ok(RawGraphNodeRow {
                node_id: row.get(0)?,
                phase_id: row.get(1)?,
                sort_order: row.get(2)?,
                name: row.get(3)?,
                description: row.get(4)?,
                status: row.get(5)?,
                current_step: row.get(6)?,
                task_count: row.get(7)?,
                completed_tasks: row.get(8)?,
                summary: row.get(9)?,
            })
        })
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_graph_query_failed",
                format!(
                    "Cadence could not query workflow-graph node rows from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    raw_rows
        .map(|raw_row| {
            raw_row
                .map_err(|error| {
                    CommandError::system_fault(
                        "workflow_graph_decode_failed",
                        format!(
                            "Cadence could not decode workflow-graph node rows from {}: {error}",
                            database_path.display()
                        ),
                    )
                })
                .and_then(|raw_row| decode_workflow_graph_node_row(raw_row, database_path))
        })
        .collect()
}

pub(crate) fn read_workflow_graph_edges(
    connection: &Connection,
    database_path: &Path,
    expected_project_id: &str,
) -> Result<Vec<WorkflowGraphEdgeRecord>, CommandError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                from_node_id,
                to_node_id,
                transition_kind,
                gate_requirement
            FROM workflow_graph_edges
            WHERE project_id = ?1
            ORDER BY from_node_id ASC, to_node_id ASC
            "#,
        )
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_graph_query_failed",
                format!(
                    "Cadence could not prepare workflow-graph edge rows from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let raw_rows = statement
        .query_map([expected_project_id], |row| {
            Ok(RawGraphEdgeRow {
                from_node_id: row.get(0)?,
                to_node_id: row.get(1)?,
                transition_kind: row.get(2)?,
                gate_requirement: row.get(3)?,
            })
        })
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_graph_query_failed",
                format!(
                    "Cadence could not query workflow-graph edge rows from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    raw_rows
        .map(|raw_row| {
            raw_row
                .map_err(|error| {
                    CommandError::system_fault(
                        "workflow_graph_decode_failed",
                        format!(
                            "Cadence could not decode workflow-graph edge rows from {}: {error}",
                            database_path.display()
                        ),
                    )
                })
                .and_then(|raw_row| decode_workflow_graph_edge_row(raw_row, database_path))
        })
        .collect()
}

pub(crate) fn read_workflow_gate_metadata(
    connection: &Connection,
    database_path: &Path,
    expected_project_id: &str,
) -> Result<Vec<WorkflowGateMetadataRecord>, CommandError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                node_id,
                gate_key,
                gate_state,
                action_type,
                title,
                detail,
                decision_context
            FROM workflow_gate_metadata
            WHERE project_id = ?1
            ORDER BY node_id ASC, gate_key ASC
            "#,
        )
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_graph_query_failed",
                format!(
                    "Cadence could not prepare workflow gate rows from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let raw_rows = statement
        .query_map([expected_project_id], |row| {
            Ok(RawGateMetadataRow {
                node_id: row.get(0)?,
                gate_key: row.get(1)?,
                gate_state: row.get(2)?,
                action_type: row.get(3)?,
                title: row.get(4)?,
                detail: row.get(5)?,
                decision_context: row.get(6)?,
            })
        })
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_graph_query_failed",
                format!(
                    "Cadence could not query workflow gate rows from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    raw_rows
        .map(|raw_row| {
            raw_row
                .map_err(|error| {
                    CommandError::system_fault(
                        "workflow_graph_decode_failed",
                        format!(
                            "Cadence could not decode workflow gate rows from {}: {error}",
                            database_path.display()
                        ),
                    )
                })
                .and_then(|raw_row| decode_workflow_gate_metadata_row(raw_row, database_path))
        })
        .collect()
}

pub(crate) fn read_transition_events(
    connection: &Connection,
    database_path: &Path,
    expected_project_id: &str,
    limit_override: Option<i64>,
) -> Result<Vec<WorkflowTransitionEventRecord>, CommandError> {
    let limit = limit_override
        .unwrap_or(MAX_WORKFLOW_TRANSITION_EVENT_ROWS)
        .max(1);

    let mut statement = connection
        .prepare(
            r#"
            SELECT
                id,
                transition_id,
                causal_transition_id,
                from_node_id,
                to_node_id,
                transition_kind,
                gate_decision,
                gate_decision_context,
                created_at
            FROM workflow_transition_events
            WHERE project_id = ?1
            ORDER BY created_at DESC, id DESC
            LIMIT ?2
            "#,
        )
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_transition_query_failed",
                format!(
                    "Cadence could not prepare workflow transition-event rows from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let raw_rows = statement
        .query_map(params![expected_project_id, limit], |row| {
            Ok(RawTransitionEventRow {
                id: row.get(0)?,
                transition_id: row.get(1)?,
                causal_transition_id: row.get(2)?,
                from_node_id: row.get(3)?,
                to_node_id: row.get(4)?,
                transition_kind: row.get(5)?,
                gate_decision: row.get(6)?,
                gate_decision_context: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_transition_query_failed",
                format!(
                    "Cadence could not query workflow transition-event rows from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    raw_rows
        .map(|raw_row| {
            raw_row
                .map_err(|error| {
                    CommandError::system_fault(
                        "workflow_transition_decode_failed",
                        format!(
                            "Cadence could not decode workflow transition-event rows from {}: {error}",
                            database_path.display()
                        ),
                    )
                })
                .and_then(|raw_row| decode_workflow_transition_event_row(raw_row, database_path))
        })
        .collect()
}

pub(crate) fn read_transition_event_by_transition_id(
    connection: &Connection,
    database_path: &Path,
    project_id: &str,
    transition_id: &str,
) -> Result<Option<WorkflowTransitionEventRecord>, CommandError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                id,
                transition_id,
                causal_transition_id,
                from_node_id,
                to_node_id,
                transition_kind,
                gate_decision,
                gate_decision_context,
                created_at
            FROM workflow_transition_events
            WHERE project_id = ?1
              AND transition_id = ?2
            LIMIT 1
            "#,
        )
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_transition_query_failed",
                format!(
                    "Cadence could not prepare transition-event lookup from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let mut rows = statement
        .query(params![project_id, transition_id])
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_transition_query_failed",
                format!(
                    "Cadence could not query transition-event lookup from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let Some(row) = rows.next().map_err(|error| {
        CommandError::system_fault(
            "workflow_transition_query_failed",
            format!(
                "Cadence could not read transition-event lookup rows from {}: {error}",
                database_path.display()
            ),
        )
    })?
    else {
        return Ok(None);
    };

    decode_workflow_transition_event_row(
        RawTransitionEventRow {
            id: row.get(0).map_err(|error| {
                CommandError::system_fault(
                    "workflow_transition_decode_failed",
                    format!(
                        "Cadence could not decode transition-event lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            transition_id: row.get(1).map_err(|error| {
                CommandError::system_fault(
                    "workflow_transition_decode_failed",
                    format!(
                        "Cadence could not decode transition-event lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            causal_transition_id: row.get(2).map_err(|error| {
                CommandError::system_fault(
                    "workflow_transition_decode_failed",
                    format!(
                        "Cadence could not decode transition-event lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            from_node_id: row.get(3).map_err(|error| {
                CommandError::system_fault(
                    "workflow_transition_decode_failed",
                    format!(
                        "Cadence could not decode transition-event lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            to_node_id: row.get(4).map_err(|error| {
                CommandError::system_fault(
                    "workflow_transition_decode_failed",
                    format!(
                        "Cadence could not decode transition-event lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            transition_kind: row.get(5).map_err(|error| {
                CommandError::system_fault(
                    "workflow_transition_decode_failed",
                    format!(
                        "Cadence could not decode transition-event lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            gate_decision: row.get(6).map_err(|error| {
                CommandError::system_fault(
                    "workflow_transition_decode_failed",
                    format!(
                        "Cadence could not decode transition-event lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            gate_decision_context: row.get(7).map_err(|error| {
                CommandError::system_fault(
                    "workflow_transition_decode_failed",
                    format!(
                        "Cadence could not decode transition-event lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            created_at: row.get(8).map_err(|error| {
                CommandError::system_fault(
                    "workflow_transition_decode_failed",
                    format!(
                        "Cadence could not decode transition-event lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
        },
        database_path,
    )
    .map(Some)
}

pub(crate) fn read_workflow_handoff_packages(
    connection: &Connection,
    database_path: &Path,
    expected_project_id: &str,
    limit_override: Option<i64>,
) -> Result<Vec<WorkflowHandoffPackageRecord>, CommandError> {
    let limit = limit_override
        .unwrap_or(MAX_WORKFLOW_HANDOFF_PACKAGE_ROWS)
        .max(1);

    let mut statement = connection
        .prepare(
            r#"
            SELECT
                id,
                project_id,
                handoff_transition_id,
                causal_transition_id,
                from_node_id,
                to_node_id,
                transition_kind,
                package_payload,
                package_hash,
                created_at
            FROM workflow_handoff_packages
            WHERE project_id = ?1
            ORDER BY created_at DESC, id DESC
            LIMIT ?2
            "#,
        )
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_handoff_query_failed",
                format!(
                    "Cadence could not prepare workflow handoff-package rows from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let raw_rows = statement
        .query_map(params![expected_project_id, limit], |row| {
            Ok(RawWorkflowHandoffPackageRow {
                id: row.get(0)?,
                project_id: row.get(1)?,
                handoff_transition_id: row.get(2)?,
                causal_transition_id: row.get(3)?,
                from_node_id: row.get(4)?,
                to_node_id: row.get(5)?,
                transition_kind: row.get(6)?,
                package_payload: row.get(7)?,
                package_hash: row.get(8)?,
                created_at: row.get(9)?,
            })
        })
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_handoff_query_failed",
                format!(
                    "Cadence could not query workflow handoff-package rows from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    raw_rows
        .map(|raw_row| {
            raw_row
                .map_err(|error| {
                    CommandError::system_fault(
                        "workflow_handoff_decode_failed",
                        format!(
                            "Cadence could not decode workflow handoff-package rows from {}: {error}",
                            database_path.display()
                        ),
                    )
                })
                .and_then(|raw_row| decode_workflow_handoff_package_row(raw_row, database_path))
        })
        .collect()
}

pub(crate) fn read_workflow_handoff_package_by_transition_id(
    connection: &Connection,
    database_path: &Path,
    project_id: &str,
    handoff_transition_id: &str,
) -> Result<Option<WorkflowHandoffPackageRecord>, CommandError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                id,
                project_id,
                handoff_transition_id,
                causal_transition_id,
                from_node_id,
                to_node_id,
                transition_kind,
                package_payload,
                package_hash,
                created_at
            FROM workflow_handoff_packages
            WHERE project_id = ?1
              AND handoff_transition_id = ?2
            LIMIT 1
            "#,
        )
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_handoff_query_failed",
                format!(
                    "Cadence could not prepare workflow handoff-package lookup from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let mut rows = statement
        .query(params![project_id, handoff_transition_id])
        .map_err(|error| {
            CommandError::system_fault(
                "workflow_handoff_query_failed",
                format!(
                    "Cadence could not query workflow handoff-package lookup from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let Some(row) = rows.next().map_err(|error| {
        CommandError::system_fault(
            "workflow_handoff_query_failed",
            format!(
                "Cadence could not read workflow handoff-package lookup rows from {}: {error}",
                database_path.display()
            ),
        )
    })?
    else {
        return Ok(None);
    };

    decode_workflow_handoff_package_row(
        RawWorkflowHandoffPackageRow {
            id: row.get(0).map_err(|error| {
                CommandError::system_fault(
                    "workflow_handoff_decode_failed",
                    format!(
                        "Cadence could not decode workflow handoff-package lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            project_id: row.get(1).map_err(|error| {
                CommandError::system_fault(
                    "workflow_handoff_decode_failed",
                    format!(
                        "Cadence could not decode workflow handoff-package lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            handoff_transition_id: row.get(2).map_err(|error| {
                CommandError::system_fault(
                    "workflow_handoff_decode_failed",
                    format!(
                        "Cadence could not decode workflow handoff-package lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            causal_transition_id: row.get(3).map_err(|error| {
                CommandError::system_fault(
                    "workflow_handoff_decode_failed",
                    format!(
                        "Cadence could not decode workflow handoff-package lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            from_node_id: row.get(4).map_err(|error| {
                CommandError::system_fault(
                    "workflow_handoff_decode_failed",
                    format!(
                        "Cadence could not decode workflow handoff-package lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            to_node_id: row.get(5).map_err(|error| {
                CommandError::system_fault(
                    "workflow_handoff_decode_failed",
                    format!(
                        "Cadence could not decode workflow handoff-package lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            transition_kind: row.get(6).map_err(|error| {
                CommandError::system_fault(
                    "workflow_handoff_decode_failed",
                    format!(
                        "Cadence could not decode workflow handoff-package lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            package_payload: row.get(7).map_err(|error| {
                CommandError::system_fault(
                    "workflow_handoff_decode_failed",
                    format!(
                        "Cadence could not decode workflow handoff-package lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            package_hash: row.get(8).map_err(|error| {
                CommandError::system_fault(
                    "workflow_handoff_decode_failed",
                    format!(
                        "Cadence could not decode workflow handoff-package lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            created_at: row.get(9).map_err(|error| {
                CommandError::system_fault(
                    "workflow_handoff_decode_failed",
                    format!(
                        "Cadence could not decode workflow handoff-package lookup rows from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
        },
        database_path,
    )
    .map(Some)
}

fn decode_workflow_graph_node_row(
    raw_row: RawGraphNodeRow,
    database_path: &Path,
) -> Result<WorkflowGraphNodeRecord, CommandError> {
    let phase_id = decode_snapshot_row_id(
        raw_row.phase_id,
        "phase_id",
        database_path,
        "workflow_graph_decode_failed",
    )?;
    let sort_order = decode_snapshot_row_id(
        raw_row.sort_order,
        "sort_order",
        database_path,
        "workflow_graph_decode_failed",
    )?;
    let task_count = decode_snapshot_row_id(
        raw_row.task_count,
        "task_count",
        database_path,
        "workflow_graph_decode_failed",
    )?;
    let completed_tasks = decode_snapshot_row_id(
        raw_row.completed_tasks,
        "completed_tasks",
        database_path,
        "workflow_graph_decode_failed",
    )?;

    if completed_tasks > task_count {
        return Err(map_snapshot_decode_error(
            "workflow_graph_decode_failed",
            database_path,
            format!(
                "Field `completed_tasks` cannot exceed `task_count` ({} > {}).",
                completed_tasks, task_count
            ),
        ));
    }

    Ok(WorkflowGraphNodeRecord {
        node_id: require_non_empty_owned(
            raw_row.node_id,
            "node_id",
            database_path,
            "workflow_graph_decode_failed",
        )?,
        phase_id,
        sort_order,
        name: require_non_empty_owned(
            raw_row.name,
            "name",
            database_path,
            "workflow_graph_decode_failed",
        )?,
        description: raw_row.description,
        status: parse_phase_status(&raw_row.status).map_err(|details| {
            map_snapshot_decode_error("workflow_graph_decode_failed", database_path, details)
        })?,
        current_step: raw_row
            .current_step
            .as_deref()
            .map(parse_phase_step)
            .transpose()
            .map_err(|details| {
                map_snapshot_decode_error("workflow_graph_decode_failed", database_path, details)
            })?,
        task_count,
        completed_tasks,
        summary: raw_row.summary,
    })
}

fn decode_workflow_graph_edge_row(
    raw_row: RawGraphEdgeRow,
    database_path: &Path,
) -> Result<WorkflowGraphEdgeRecord, CommandError> {
    Ok(WorkflowGraphEdgeRecord {
        from_node_id: require_non_empty_owned(
            raw_row.from_node_id,
            "from_node_id",
            database_path,
            "workflow_graph_decode_failed",
        )?,
        to_node_id: require_non_empty_owned(
            raw_row.to_node_id,
            "to_node_id",
            database_path,
            "workflow_graph_decode_failed",
        )?,
        transition_kind: require_non_empty_owned(
            raw_row.transition_kind,
            "transition_kind",
            database_path,
            "workflow_graph_decode_failed",
        )?,
        gate_requirement: decode_optional_non_empty_text(
            raw_row.gate_requirement,
            "gate_requirement",
            database_path,
            "workflow_graph_decode_failed",
        )?,
    })
}

fn decode_workflow_gate_metadata_row(
    raw_row: RawGateMetadataRow,
    database_path: &Path,
) -> Result<WorkflowGateMetadataRecord, CommandError> {
    let gate_state = parse_workflow_gate_state(&raw_row.gate_state).map_err(|details| {
        map_snapshot_decode_error("workflow_graph_decode_failed", database_path, details)
    })?;

    let action_type = decode_optional_non_empty_text(
        raw_row.action_type,
        "action_type",
        database_path,
        "workflow_graph_decode_failed",
    )?;
    let title = decode_optional_non_empty_text(
        raw_row.title,
        "title",
        database_path,
        "workflow_graph_decode_failed",
    )?;
    let detail = decode_optional_non_empty_text(
        raw_row.detail,
        "detail",
        database_path,
        "workflow_graph_decode_failed",
    )?;

    if matches!(
        gate_state,
        WorkflowGateState::Pending | WorkflowGateState::Blocked
    ) && (action_type.is_none() || title.is_none() || detail.is_none())
    {
        return Err(map_snapshot_decode_error(
            "workflow_graph_decode_failed",
            database_path,
            "Pending or blocked workflow gates must include action_type, title, and detail.".into(),
        ));
    }

    Ok(WorkflowGateMetadataRecord {
        node_id: require_non_empty_owned(
            raw_row.node_id,
            "node_id",
            database_path,
            "workflow_graph_decode_failed",
        )?,
        gate_key: require_non_empty_owned(
            raw_row.gate_key,
            "gate_key",
            database_path,
            "workflow_graph_decode_failed",
        )?,
        gate_state,
        action_type,
        title,
        detail,
        decision_context: decode_optional_non_empty_text(
            raw_row.decision_context,
            "decision_context",
            database_path,
            "workflow_graph_decode_failed",
        )?,
    })
}

fn decode_workflow_transition_event_row(
    raw_row: RawTransitionEventRow,
    database_path: &Path,
) -> Result<WorkflowTransitionEventRecord, CommandError> {
    Ok(WorkflowTransitionEventRecord {
        id: raw_row.id,
        transition_id: require_non_empty_owned(
            raw_row.transition_id,
            "transition_id",
            database_path,
            "workflow_transition_decode_failed",
        )?,
        causal_transition_id: decode_optional_non_empty_text(
            raw_row.causal_transition_id,
            "causal_transition_id",
            database_path,
            "workflow_transition_decode_failed",
        )?,
        from_node_id: require_non_empty_owned(
            raw_row.from_node_id,
            "from_node_id",
            database_path,
            "workflow_transition_decode_failed",
        )?,
        to_node_id: require_non_empty_owned(
            raw_row.to_node_id,
            "to_node_id",
            database_path,
            "workflow_transition_decode_failed",
        )?,
        transition_kind: require_non_empty_owned(
            raw_row.transition_kind,
            "transition_kind",
            database_path,
            "workflow_transition_decode_failed",
        )?,
        gate_decision: parse_workflow_transition_gate_decision(&raw_row.gate_decision).map_err(
            |details| {
                map_snapshot_decode_error(
                    "workflow_transition_decode_failed",
                    database_path,
                    details,
                )
            },
        )?,
        gate_decision_context: decode_optional_non_empty_text(
            raw_row.gate_decision_context,
            "gate_decision_context",
            database_path,
            "workflow_transition_decode_failed",
        )?,
        created_at: require_non_empty_owned(
            raw_row.created_at,
            "created_at",
            database_path,
            "workflow_transition_decode_failed",
        )?,
    })
}

fn decode_workflow_handoff_package_row(
    raw_row: RawWorkflowHandoffPackageRow,
    database_path: &Path,
) -> Result<WorkflowHandoffPackageRecord, CommandError> {
    let package_payload = require_non_empty_owned(
        raw_row.package_payload,
        "package_payload",
        database_path,
        "workflow_handoff_decode_failed",
    )?;
    let canonical_payload = canonicalize_workflow_handoff_package_payload(
        &package_payload,
        Some(database_path),
        "workflow_handoff_decode_failed",
    )?;
    if canonical_payload != package_payload {
        return Err(map_snapshot_decode_error(
            "workflow_handoff_decode_failed",
            database_path,
            "Field `package_payload` must use canonical JSON key ordering for deterministic hashing."
                .into(),
        ));
    }

    if let Some(secret_hint) = find_prohibited_workflow_handoff_content(&package_payload) {
        return Err(map_snapshot_decode_error(
            "workflow_handoff_decode_failed",
            database_path,
            format!(
                "Field `package_payload` must not include {secret_hint}; persisted handoff packages are redacted-only."
            ),
        ));
    }

    let package_hash = require_non_empty_owned(
        raw_row.package_hash,
        "package_hash",
        database_path,
        "workflow_handoff_decode_failed",
    )?;
    validate_workflow_handoff_package_hash(
        &package_hash,
        "package_hash",
        database_path,
        "workflow_handoff_decode_failed",
    )?;

    let expected_hash = compute_workflow_handoff_package_hash(&canonical_payload);
    if package_hash != expected_hash {
        return Err(map_snapshot_decode_error(
            "workflow_handoff_decode_failed",
            database_path,
            format!(
                "Field `package_hash` must match the deterministic hash of `package_payload` (expected `{expected_hash}`, found `{package_hash}`)."
            ),
        ));
    }

    let created_at = require_non_empty_owned(
        raw_row.created_at,
        "created_at",
        database_path,
        "workflow_handoff_decode_failed",
    )?;
    validate_rfc3339_timestamp(
        &created_at,
        "created_at",
        Some(database_path),
        "workflow_handoff_decode_failed",
    )?;

    Ok(WorkflowHandoffPackageRecord {
        id: raw_row.id,
        project_id: require_non_empty_owned(
            raw_row.project_id,
            "project_id",
            database_path,
            "workflow_handoff_decode_failed",
        )?,
        handoff_transition_id: require_non_empty_owned(
            raw_row.handoff_transition_id,
            "handoff_transition_id",
            database_path,
            "workflow_handoff_decode_failed",
        )?,
        causal_transition_id: decode_optional_non_empty_text(
            raw_row.causal_transition_id,
            "causal_transition_id",
            database_path,
            "workflow_handoff_decode_failed",
        )?,
        from_node_id: require_non_empty_owned(
            raw_row.from_node_id,
            "from_node_id",
            database_path,
            "workflow_handoff_decode_failed",
        )?,
        to_node_id: require_non_empty_owned(
            raw_row.to_node_id,
            "to_node_id",
            database_path,
            "workflow_handoff_decode_failed",
        )?,
        transition_kind: require_non_empty_owned(
            raw_row.transition_kind,
            "transition_kind",
            database_path,
            "workflow_handoff_decode_failed",
        )?,
        package_payload,
        package_hash,
        created_at,
    })
}
