use super::*;

#[derive(Debug, Clone)]
struct OperatorApprovalGateCandidate {
    node_id: String,
    gate_key: String,
    title: String,
    detail: String,
}

pub(crate) fn resolve_operator_approval_gate_link(
    transaction: &Transaction<'_>,
    database_path: &Path,
    project_id: &str,
    action_type: &str,
    title: &str,
    detail: &str,
) -> Result<Option<OperatorApprovalGateLink>, CommandError> {
    let mut statement = transaction
        .prepare(
            r#"
            SELECT
                node_id,
                gate_key,
                title,
                detail
            FROM workflow_gate_metadata
            WHERE project_id = ?1
              AND gate_state IN ('pending', 'blocked')
              AND action_type = ?2
            ORDER BY node_id ASC, gate_key ASC
            "#,
        )
        .map_err(|error| {
            map_operator_loop_write_error(
                "operator_approval_gate_lookup_failed",
                database_path,
                error,
                "Cadence could not load unresolved workflow gates for operator approval persistence.",
            )
        })?;

    let gate_candidates = statement
        .query_map(params![project_id, action_type], |row| {
            Ok(OperatorApprovalGateCandidate {
                node_id: row.get(0)?,
                gate_key: row.get(1)?,
                title: row.get(2)?,
                detail: row.get(3)?,
            })
        })
        .map_err(|error| {
            map_operator_loop_write_error(
                "operator_approval_gate_lookup_failed",
                database_path,
                error,
                "Cadence could not query unresolved workflow gates for operator approval persistence.",
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CommandError::system_fault(
                "operator_approval_gate_decode_failed",
                format!(
                    "Cadence could not decode unresolved workflow gate rows from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    if gate_candidates.is_empty() {
        return Ok(None);
    }

    let mut filtered_candidates: Vec<OperatorApprovalGateCandidate> = gate_candidates
        .iter()
        .filter(|candidate| candidate.title == title && candidate.detail == detail)
        .cloned()
        .collect();

    if filtered_candidates.is_empty() {
        filtered_candidates = gate_candidates;
    }

    if filtered_candidates.len() > 1 {
        let mut active_node_statement = transaction
            .prepare(
                r#"
                SELECT node_id
                FROM workflow_graph_nodes
                WHERE project_id = ?1
                  AND status = 'active'
                ORDER BY sort_order ASC, node_id ASC
                LIMIT 1
                "#,
            )
            .map_err(|error| {
                map_operator_loop_write_error(
                    "operator_approval_gate_lookup_failed",
                    database_path,
                    error,
                    "Cadence could not load active workflow-node context for gate-link disambiguation.",
                )
            })?;

        let active_node_id: Option<String> = active_node_statement
            .query_row(params![project_id], |row| row.get(0))
            .optional()
            .map_err(|error| {
                map_operator_loop_write_error(
                    "operator_approval_gate_lookup_failed",
                    database_path,
                    error,
                    "Cadence could not query active workflow-node context for gate-link disambiguation.",
                )
            })?;

        if let Some(active_node_id) = active_node_id {
            let active_candidates: Vec<OperatorApprovalGateCandidate> = filtered_candidates
                .iter()
                .filter(|candidate| candidate.node_id == active_node_id)
                .cloned()
                .collect();

            if !active_candidates.is_empty() {
                filtered_candidates = active_candidates;
            }
        }
    }

    if filtered_candidates.len() != 1 {
        let candidates = filtered_candidates
            .iter()
            .map(|candidate| format!("{}:{}", candidate.node_id, candidate.gate_key))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(CommandError::user_fixable(
            "operator_approval_gate_ambiguous",
            format!(
                "Cadence cannot persist action-required item `{action_type}` because it matches multiple unresolved workflow gates ({candidates})."
            ),
        ));
    }

    let selected = &filtered_candidates[0];

    let mut edge_statement = transaction
        .prepare(
            r#"
            SELECT
                from_node_id,
                to_node_id,
                transition_kind
            FROM workflow_graph_edges
            WHERE project_id = ?1
              AND to_node_id = ?2
              AND gate_requirement = ?3
            ORDER BY from_node_id ASC, to_node_id ASC, transition_kind ASC
            "#,
        )
        .map_err(|error| {
            map_operator_loop_write_error(
                "operator_approval_transition_lookup_failed",
                database_path,
                error,
                "Cadence could not load workflow continuation edges for gate-linked operator approval.",
            )
        })?;

    let transitions = edge_statement
        .query_map(
            params![project_id, selected.node_id.as_str(), selected.gate_key.as_str()],
            |row| {
                Ok(OperatorApprovalGateLink {
                    gate_node_id: selected.node_id.clone(),
                    gate_key: selected.gate_key.clone(),
                    transition_from_node_id: row.get(0)?,
                    transition_to_node_id: row.get(1)?,
                    transition_kind: row.get(2)?,
                })
            },
        )
        .map_err(|error| {
            map_operator_loop_write_error(
                "operator_approval_transition_lookup_failed",
                database_path,
                error,
                "Cadence could not query workflow continuation edges for gate-linked operator approval.",
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            CommandError::system_fault(
                "operator_approval_transition_decode_failed",
                format!(
                    "Cadence could not decode workflow continuation edges from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    match transitions.as_slice() {
        [] => Err(CommandError::user_fixable(
            "operator_approval_transition_missing",
            format!(
                "Cadence cannot persist gate-linked operator request `{action_type}` because gate `{}` on node `{}` has no legal continuation edge.",
                selected.gate_key, selected.node_id
            ),
        )),
        [transition] => Ok(Some(transition.clone())),
        _ => {
            let candidates = transitions
                .iter()
                .map(|transition| {
                    format!(
                        "{}->{}:{}",
                        transition.transition_from_node_id,
                        transition.transition_to_node_id,
                        transition.transition_kind
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            Err(CommandError::user_fixable(
                "operator_approval_transition_ambiguous",
                format!(
                    "Cadence cannot persist gate-linked operator request `{action_type}` because gate `{}` on node `{}` maps to multiple continuation edges ({candidates}).",
                    selected.gate_key, selected.node_id
                ),
            ))
        }
    }
}

pub(crate) fn decode_operator_resume_transition_context(
    approval_request: &OperatorApprovalDto,
    action_id: &str,
) -> Result<Option<OperatorResumeTransitionContext>, CommandError> {
    let gate_fields_populated =
        approval_request.gate_node_id.is_some() || approval_request.gate_key.is_some();
    let transition_fields_populated = approval_request.transition_from_node_id.is_some()
        || approval_request.transition_to_node_id.is_some()
        || approval_request.transition_kind.is_some();

    if !gate_fields_populated && !transition_fields_populated {
        return Ok(None);
    }

    let gate_node_id = approval_request
        .gate_node_id
        .as_deref()
        .ok_or_else(|| {
            CommandError::retryable(
                "operator_resume_gate_link_missing",
                format!(
                    "Cadence cannot resume gate-linked operator request `{action_id}` because `gateNodeId` is missing."
                ),
            )
        })?
        .to_string();
    let gate_key = approval_request
        .gate_key
        .as_deref()
        .ok_or_else(|| {
            CommandError::retryable(
                "operator_resume_gate_link_missing",
                format!(
                    "Cadence cannot resume gate-linked operator request `{action_id}` because `gateKey` is missing."
                ),
            )
        })?
        .to_string();
    let transition_from_node_id = approval_request
        .transition_from_node_id
        .as_deref()
        .ok_or_else(|| {
            CommandError::retryable(
                "operator_resume_transition_context_missing",
                format!(
                    "Cadence cannot resume gate-linked operator request `{action_id}` because `transitionFromNodeId` is missing."
                ),
            )
        })?
        .to_string();
    let transition_to_node_id = approval_request
        .transition_to_node_id
        .as_deref()
        .ok_or_else(|| {
            CommandError::retryable(
                "operator_resume_transition_context_missing",
                format!(
                    "Cadence cannot resume gate-linked operator request `{action_id}` because `transitionToNodeId` is missing."
                ),
            )
        })?
        .to_string();
    let transition_kind = approval_request
        .transition_kind
        .as_deref()
        .ok_or_else(|| {
            CommandError::retryable(
                "operator_resume_transition_context_missing",
                format!(
                    "Cadence cannot resume gate-linked operator request `{action_id}` because `transitionKind` is missing."
                ),
            )
        })?
        .to_string();

    if gate_node_id != transition_to_node_id {
        return Err(CommandError::retryable(
            "operator_resume_transition_context_invalid",
            format!(
                "Cadence cannot resume gate-linked operator request `{action_id}` because gate node `{gate_node_id}` does not match transition target `{transition_to_node_id}`."
            ),
        ));
    }

    let user_answer = approval_request.user_answer.as_deref().ok_or_else(|| {
        CommandError::user_fixable(
            "operator_resume_answer_missing",
            format!(
                "Cadence cannot resume gate-linked operator request `{action_id}` because no user answer was recorded with the approval."
            ),
        )
    })?;

    if let Some(secret_hint) = find_prohibited_transition_diagnostic_content(user_answer) {
        return Err(CommandError::user_fixable(
            "operator_resume_decision_payload_invalid",
            format!(
                "Operator decision payload for `{action_id}` must not include {secret_hint}. Remove secret-bearing transcript/tool/auth material before retrying."
            ),
        ));
    }

    Ok(Some(OperatorResumeTransitionContext {
        gate_node_id,
        gate_key,
        transition_from_node_id,
        transition_to_node_id,
        transition_kind,
        user_answer: user_answer.to_string(),
    }))
}

pub(crate) fn read_latest_transition_id(
    transaction: &Transaction<'_>,
    database_path: &Path,
    project_id: &str,
) -> Result<Option<String>, CommandError> {
    transaction
        .query_row(
            r#"
            SELECT transition_id
            FROM workflow_transition_events
            WHERE project_id = ?1
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
            params![project_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| {
            map_operator_loop_write_error(
                "operator_resume_transition_lookup_failed",
                database_path,
                error,
                "Cadence could not load prior workflow transition context for resume causality.",
            )
        })
}

pub(crate) fn build_transition_mutation_record(
    transition: &ApplyWorkflowTransitionRecord,
) -> WorkflowTransitionMutationRecord {
    WorkflowTransitionMutationRecord {
        transition_id: transition.transition_id.clone(),
        causal_transition_id: transition.causal_transition_id.clone(),
        from_node_id: transition.from_node_id.clone(),
        to_node_id: transition.to_node_id.clone(),
        transition_kind: transition.transition_kind.clone(),
        gate_decision: transition.gate_decision.clone(),
        gate_decision_context: transition.gate_decision_context.clone(),
        gate_updates: transition
            .gate_updates
            .iter()
            .map(|gate_update| WorkflowTransitionGateMutationRecord {
                node_id: transition.to_node_id.clone(),
                gate_key: gate_update.gate_key.clone(),
                gate_state: gate_update.gate_state.clone(),
                decision_context: gate_update.decision_context.clone(),
                require_pending_or_blocked: false,
            })
            .collect(),
        required_gate_requirement: None,
        occurred_at: transition.occurred_at.clone(),
    }
}

pub(crate) fn derive_resume_transition_id(
    action_id: &str,
    context: &OperatorResumeTransitionContext,
) -> String {
    let suffix = stable_transition_id_suffix(&[
        "resume",
        action_id.trim(),
        context.transition_from_node_id.as_str(),
        context.transition_to_node_id.as_str(),
        context.transition_kind.as_str(),
        context.gate_key.as_str(),
    ]);

    format!("resume:{}:{suffix}", action_id.trim())
}

pub(crate) fn stable_transition_id_suffix(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(b"\n");
    }

    let digest = hasher.finalize();
    digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(crate) fn apply_workflow_transition_mutation(
    transaction: &Transaction<'_>,
    database_path: &Path,
    project_id: &str,
    transition: &WorkflowTransitionMutationRecord,
    error_profile: &WorkflowTransitionMutationErrorProfile,
    map_sql_error: WorkflowTransitionSqlErrorMapper,
) -> Result<WorkflowTransitionMutationApplyOutcome, CommandError> {
    if let Some(existing) = read_transition_event_by_transition_id(
        transaction,
        database_path,
        project_id,
        &transition.transition_id,
    )? {
        return Ok(WorkflowTransitionMutationApplyOutcome::Replayed(existing));
    }

    assert_transition_edge_exists(
        transaction,
        database_path,
        project_id,
        &transition.from_node_id,
        &transition.to_node_id,
        &transition.transition_kind,
        transition.required_gate_requirement.as_deref(),
        error_profile,
        map_sql_error,
    )?;

    for gate_update in &transition.gate_updates {
        validate_non_empty_text(
            &gate_update.gate_key,
            "gate_key",
            "workflow_transition_request_invalid",
        )?;

        let update_statement = if gate_update.require_pending_or_blocked {
            r#"
            UPDATE workflow_gate_metadata
            SET gate_state = ?4,
                decision_context = ?5,
                updated_at = ?6
            WHERE project_id = ?1
              AND node_id = ?2
              AND gate_key = ?3
              AND gate_state IN ('pending', 'blocked')
            "#
        } else {
            r#"
            UPDATE workflow_gate_metadata
            SET gate_state = ?4,
                decision_context = ?5,
                updated_at = ?6
            WHERE project_id = ?1
              AND node_id = ?2
              AND gate_key = ?3
            "#
        };

        let updated = transaction
            .execute(
                update_statement,
                params![
                    project_id,
                    gate_update.node_id.as_str(),
                    gate_update.gate_key.as_str(),
                    workflow_gate_state_sql_value(&gate_update.gate_state),
                    gate_update.decision_context.as_deref(),
                    transition.occurred_at.as_str(),
                ],
            )
            .map_err(|error| {
                map_sql_error(
                    error_profile.gate_update_failed_code,
                    database_path,
                    error,
                    error_profile.gate_update_failed_message,
                )
            })?;

        if updated == 0 {
            let gate_missing_detail = if gate_update.require_pending_or_blocked {
                format!(
                    "gate `{}` is not defined for workflow node `{}` in a pending or blocked state",
                    gate_update.gate_key, gate_update.node_id
                )
            } else {
                format!(
                    "gate `{}` is not defined for workflow node `{}`",
                    gate_update.gate_key, gate_update.node_id
                )
            };

            return Err(CommandError::user_fixable(
                "workflow_transition_gate_not_found",
                format!(
                    "Cadence could not apply transition `{}` because {gate_missing_detail}.",
                    transition.transition_id
                ),
            ));
        }
    }

    let mut gate_state_statement = transaction
        .prepare(
            r#"
            SELECT gate_state
            FROM workflow_gate_metadata
            WHERE project_id = ?1
              AND node_id = ?2
            "#,
        )
        .map_err(|error| {
            map_sql_error(
                error_profile.gate_check_failed_code,
                database_path,
                error,
                error_profile.gate_check_failed_message,
            )
        })?;

    let gate_states = gate_state_statement
        .query_map(params![project_id, transition.to_node_id.as_str()], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|error| {
            map_sql_error(
                error_profile.gate_check_failed_code,
                database_path,
                error,
                error_profile.gate_check_failed_message,
            )
        })?;

    let mut unresolved_gate_count = 0_i64;
    for gate_state_row in gate_states {
        let raw_gate_state = gate_state_row.map_err(|error| {
            map_sql_error(
                error_profile.gate_check_failed_code,
                database_path,
                error,
                error_profile.gate_check_failed_message,
            )
        })?;

        let parsed_gate_state = parse_workflow_gate_state(raw_gate_state.trim()).map_err(|reason| {
            CommandError::system_fault(
                error_profile.gate_check_failed_code,
                format!(
                    "Cadence found malformed workflow gate metadata while applying transition `{}`: {reason}",
                    transition.transition_id
                ),
            )
        })?;

        if matches!(
            parsed_gate_state,
            WorkflowGateState::Pending | WorkflowGateState::Blocked
        ) {
            unresolved_gate_count += 1;
        }
    }

    if unresolved_gate_count > 0 {
        return Err(CommandError::user_fixable(
            "workflow_transition_gate_unmet",
            format!(
                "Cadence cannot transition from `{}` to `{}` because {unresolved_gate_count} required gate(s) are still pending or blocked.",
                transition.from_node_id, transition.to_node_id
            ),
        ));
    }

    let source_updated = transaction
        .execute(
            r#"
            UPDATE workflow_graph_nodes
            SET status = 'complete',
                updated_at = ?3
            WHERE project_id = ?1
              AND node_id = ?2
            "#,
            params![
                project_id,
                transition.from_node_id.as_str(),
                transition.occurred_at.as_str(),
            ],
        )
        .map_err(|error| {
            map_sql_error(
                error_profile.source_update_failed_code,
                database_path,
                error,
                error_profile.source_update_failed_message,
            )
        })?;

    if source_updated == 0 {
        return Err(CommandError::user_fixable(
            "workflow_transition_source_missing",
            format!(
                "Cadence cannot apply transition `{}` because source node `{}` does not exist.",
                transition.transition_id, transition.from_node_id
            ),
        ));
    }

    let target_updated = transaction
        .execute(
            r#"
            UPDATE workflow_graph_nodes
            SET status = 'active',
                updated_at = ?3
            WHERE project_id = ?1
              AND node_id = ?2
            "#,
            params![
                project_id,
                transition.to_node_id.as_str(),
                transition.occurred_at.as_str(),
            ],
        )
        .map_err(|error| {
            map_sql_error(
                error_profile.target_update_failed_code,
                database_path,
                error,
                error_profile.target_update_failed_message,
            )
        })?;

    if target_updated == 0 {
        return Err(CommandError::user_fixable(
            "workflow_transition_target_missing",
            format!(
                "Cadence cannot apply transition `{}` because target node `{}` does not exist.",
                transition.transition_id, transition.to_node_id
            ),
        ));
    }

    let event_insert_result = transaction.execute(
        r#"
            INSERT INTO workflow_transition_events (
                project_id,
                transition_id,
                causal_transition_id,
                from_node_id,
                to_node_id,
                transition_kind,
                gate_decision,
                gate_decision_context,
                created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        params![
            project_id,
            transition.transition_id.as_str(),
            transition.causal_transition_id.as_deref(),
            transition.from_node_id.as_str(),
            transition.to_node_id.as_str(),
            transition.transition_kind.as_str(),
            workflow_transition_gate_decision_sql_value(&transition.gate_decision),
            transition.gate_decision_context.as_deref(),
            transition.occurred_at.as_str(),
        ],
    );

    match event_insert_result {
        Ok(_) => Ok(WorkflowTransitionMutationApplyOutcome::Applied),
        Err(error) if is_unique_constraint_violation(&error) => {
            let existing = read_transition_event_by_transition_id(
                transaction,
                database_path,
                project_id,
                &transition.transition_id,
            )?
            .ok_or_else(|| {
                map_sql_error(
                    error_profile.event_persist_failed_code,
                    database_path,
                    error,
                    error_profile.event_persist_failed_message,
                )
            })?;

            Ok(WorkflowTransitionMutationApplyOutcome::Replayed(existing))
        }
        Err(error) => Err(map_sql_error(
            error_profile.event_persist_failed_code,
            database_path,
            error,
            error_profile.event_persist_failed_message,
        )),
    }
}

#[allow(clippy::too_many_arguments)]
fn assert_transition_edge_exists(
    transaction: &Transaction<'_>,
    database_path: &Path,
    project_id: &str,
    from_node_id: &str,
    to_node_id: &str,
    transition_kind: &str,
    required_gate_requirement: Option<&str>,
    error_profile: &WorkflowTransitionMutationErrorProfile,
    map_sql_error: WorkflowTransitionSqlErrorMapper,
) -> Result<(), CommandError> {
    let edge_exists: i64 = transaction
        .query_row(
            r#"
            SELECT COUNT(*)
            FROM workflow_graph_edges
            WHERE project_id = ?1
              AND from_node_id = ?2
              AND to_node_id = ?3
              AND transition_kind = ?4
              AND (?5 IS NULL OR gate_requirement = ?5)
            "#,
            params![
                project_id,
                from_node_id,
                to_node_id,
                transition_kind,
                required_gate_requirement,
            ],
            |row| row.get(0),
        )
        .map_err(|error| {
            map_sql_error(
                error_profile.edge_check_failed_code,
                database_path,
                error,
                error_profile.edge_check_failed_message,
            )
        })?;

    if edge_exists == 0 {
        if let Some(gate_requirement) = required_gate_requirement {
            return Err(CommandError::user_fixable(
                "workflow_transition_illegal_edge",
                format!(
                    "Cadence cannot transition from `{from_node_id}` to `{to_node_id}` with kind `{transition_kind}` and gate `{gate_requirement}` because no legal workflow edge exists."
                ),
            ));
        }

        return Err(CommandError::user_fixable(
            "workflow_transition_illegal_edge",
            format!(
                "Cadence cannot transition from `{from_node_id}` to `{to_node_id}` with kind `{transition_kind}` because no legal workflow edge exists."
            ),
        ));
    }

    Ok(())
}
