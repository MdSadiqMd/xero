use super::*;

#[derive(Debug, Clone)]
struct WorkflowAutomaticDispatchCandidate {
    from_node_id: String,
    to_node_id: String,
    transition_kind: String,
    gate_requirement: Option<String>,
}

#[derive(Debug, Clone)]
struct WorkflowAutomaticDispatchUnresolvedGateCandidate {
    gate_node_id: String,
    gate_key: String,
    gate_state: WorkflowGateState,
    action_type: Option<String>,
    title: Option<String>,
    detail: Option<String>,
}

#[derive(Debug, Clone)]
struct WorkflowAutomaticDispatchUnresolvedContinuationCandidate {
    from_node_id: String,
    to_node_id: String,
    transition_kind: String,
    gate_requirement: Option<String>,
    unresolved_gates: Vec<WorkflowAutomaticDispatchUnresolvedGateCandidate>,
}

#[derive(Debug, Clone)]
enum WorkflowAutomaticDispatchCandidateResolution {
    NoContinuation,
    Candidate(WorkflowAutomaticDispatchCandidate),
    Unresolved {
        completed_node_id: String,
        blocked_candidates: Vec<WorkflowAutomaticDispatchUnresolvedContinuationCandidate>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowAutomaticDispatchPlanModeRequirement {
    Disabled,
    Required,
    Unknown,
}

fn derive_automatic_transition_id(
    causal_transition_id: &str,
    candidate: &WorkflowAutomaticDispatchCandidate,
) -> String {
    let suffix = stable_transition_id_suffix(&[
        "auto",
        causal_transition_id,
        candidate.from_node_id.as_str(),
        candidate.to_node_id.as_str(),
        candidate.transition_kind.as_str(),
        candidate.gate_requirement.as_deref().unwrap_or("no_gate"),
    ]);

    format!("auto:{causal_transition_id}:{suffix}")
}

pub(crate) fn attempt_automatic_dispatch_after_transition(
    connection: &mut Connection,
    database_path: &Path,
    project_id: &str,
    trigger_transition: &WorkflowTransitionEventRecord,
) -> WorkflowAutomaticDispatchOutcome {
    let transaction = match connection.unchecked_transaction() {
        Ok(transaction) => transaction,
        Err(error) => {
            return automatic_dispatch_outcome_from_error(map_workflow_graph_transaction_error(
                "workflow_transition_auto_dispatch_transaction_failed",
                database_path,
                error,
                "Cadence could not start an automatic-dispatch transaction.",
            ));
        }
    };

    let plan_mode_requirement = match resolve_plan_mode_required_for_automatic_dispatch(
        &transaction,
        database_path,
        project_id,
    ) {
        Ok(required) => required,
        Err(error) => return automatic_dispatch_outcome_from_error(error),
    };

    let candidate = match resolve_automatic_dispatch_candidate(
        &transaction,
        database_path,
        project_id,
        &trigger_transition.to_node_id,
        plan_mode_requirement,
    ) {
        Ok(WorkflowAutomaticDispatchCandidateResolution::NoContinuation) => {
            return WorkflowAutomaticDispatchOutcome::NoContinuation;
        }
        Ok(WorkflowAutomaticDispatchCandidateResolution::Candidate(candidate)) => candidate,
        Ok(WorkflowAutomaticDispatchCandidateResolution::Unresolved {
            completed_node_id,
            blocked_candidates,
        }) => {
            let blocked_summary =
                format_unresolved_dispatch_candidate_summary(blocked_candidates.as_slice());

            let persisted = match persist_pending_approval_for_unresolved_auto_dispatch(
                &transaction,
                database_path,
                project_id,
                trigger_transition,
                blocked_candidates.as_slice(),
            ) {
                Ok(persisted) => persisted,
                Err(error) => return automatic_dispatch_outcome_from_error(error),
            };

            if let Err(error) = transaction.commit() {
                return automatic_dispatch_outcome_from_error(map_workflow_graph_commit_error(
                    "workflow_transition_auto_dispatch_commit_failed",
                    database_path,
                    error,
                    "Cadence could not commit gate-unmet automatic-dispatch state.",
                ));
            }

            let enqueue_outcome = enqueue_notification_dispatches_best_effort_with_connection(
                connection,
                database_path,
                &NotificationDispatchEnqueueRecord {
                    project_id: project_id.to_string(),
                    action_id: persisted.action_id.clone(),
                    enqueued_at: persisted.updated_at.clone(),
                },
            );

            return WorkflowAutomaticDispatchOutcome::Skipped {
                code: "workflow_transition_gate_unmet".into(),
                message: format!(
                    "Cadence skipped automatic dispatch from `{completed_node_id}` because continuation edges are still blocked by unresolved gates: {blocked_summary}. Persisted pending operator approval `{}` for deterministic replay. {}",
                    persisted.action_id,
                    format_notification_dispatch_enqueue_outcome(&enqueue_outcome)
                ),
            };
        }
        Err(error) => return automatic_dispatch_outcome_from_error(error),
    };

    let transition_id =
        derive_automatic_transition_id(&trigger_transition.transition_id, &candidate);
    let mutation = WorkflowTransitionMutationRecord {
        transition_id: transition_id.clone(),
        causal_transition_id: Some(trigger_transition.transition_id.clone()),
        from_node_id: candidate.from_node_id,
        to_node_id: candidate.to_node_id,
        transition_kind: candidate.transition_kind,
        gate_decision: WorkflowTransitionGateDecision::NotApplicable,
        gate_decision_context: None,
        gate_updates: Vec::new(),
        required_gate_requirement: candidate.gate_requirement,
        occurred_at: crate::auth::now_timestamp(),
    };

    let mutation_outcome = match apply_workflow_transition_mutation(
        &transaction,
        database_path,
        project_id,
        &mutation,
        &WORKFLOW_AUTOMATIC_DISPATCH_MUTATION_ERROR_PROFILE,
        map_workflow_graph_write_error,
    ) {
        Ok(mutation_outcome) => mutation_outcome,
        Err(error) => return automatic_dispatch_outcome_from_error(error),
    };

    match mutation_outcome {
        WorkflowTransitionMutationApplyOutcome::Replayed(transition_event) => {
            let handoff_package = load_replayed_handoff_package_for_automatic_dispatch(
                &transaction,
                database_path,
                project_id,
                &transition_event,
            );

            WorkflowAutomaticDispatchOutcome::Replayed {
                transition_event,
                handoff_package,
            }
        }
        WorkflowTransitionMutationApplyOutcome::Applied => {
            if let Err(error) = transaction.commit() {
                return automatic_dispatch_outcome_from_error(map_workflow_graph_commit_error(
                    "workflow_transition_auto_dispatch_commit_failed",
                    database_path,
                    error,
                    "Cadence could not commit automatic workflow dispatch.",
                ));
            }

            match read_transition_event_by_transition_id(
                connection,
                database_path,
                project_id,
                &transition_id,
            ) {
                Ok(Some(transition_event)) => {
                    let handoff_package = persist_handoff_package_for_automatic_dispatch(
                        connection,
                        database_path,
                        project_id,
                        &transition_event,
                    );

                    WorkflowAutomaticDispatchOutcome::Applied {
                        transition_event,
                        handoff_package,
                    }
                }
                Ok(None) => WorkflowAutomaticDispatchOutcome::Skipped {
                    code: "workflow_transition_auto_dispatch_event_missing_after_persist".into(),
                    message: format!(
                        "Cadence persisted automatic transition `{transition_id}` in {} but could not read it back.",
                        database_path.display()
                    ),
                },
                Err(error) => automatic_dispatch_outcome_from_error(error),
            }
        }
    }
}

fn persist_handoff_package_for_automatic_dispatch(
    connection: &Connection,
    database_path: &Path,
    project_id: &str,
    transition_event: &WorkflowTransitionEventRecord,
) -> WorkflowAutomaticDispatchPackageOutcome {
    let package_payload = match assemble_workflow_handoff_package_upsert_record(
        connection,
        database_path,
        project_id,
        transition_event,
    ) {
        Ok(payload) => payload,
        Err(error) => return automatic_dispatch_package_outcome_from_error(error),
    };

    let persisted = match persist_workflow_handoff_package_with_connection(
        connection,
        database_path,
        &package_payload,
    ) {
        Ok(persisted) => persisted,
        Err(error) => return automatic_dispatch_package_outcome_from_error(error),
    };

    if let Err(error) =
        validate_workflow_handoff_package_transition_linkage(&persisted.package, transition_event)
    {
        return automatic_dispatch_package_outcome_from_error(error);
    }

    match persisted.disposition {
        WorkflowHandoffPackagePersistDisposition::Persisted => {
            WorkflowAutomaticDispatchPackageOutcome::Persisted {
                package: persisted.package,
            }
        }
        WorkflowHandoffPackagePersistDisposition::Replayed => {
            WorkflowAutomaticDispatchPackageOutcome::Replayed {
                package: persisted.package,
            }
        }
    }
}

fn load_replayed_handoff_package_for_automatic_dispatch(
    connection: &Connection,
    database_path: &Path,
    project_id: &str,
    transition_event: &WorkflowTransitionEventRecord,
) -> WorkflowAutomaticDispatchPackageOutcome {
    let package = match read_workflow_handoff_package_by_transition_id(
        connection,
        database_path,
        project_id,
        &transition_event.transition_id,
    ) {
        Ok(Some(package)) => package,
        Ok(None) => {
            return WorkflowAutomaticDispatchPackageOutcome::Skipped {
                code: "workflow_handoff_replay_not_found".into(),
                message: format!(
                    "Cadence replayed automatic transition `{}` in {} but no workflow handoff package row exists for that transition.",
                    transition_event.transition_id,
                    database_path.display()
                ),
            };
        }
        Err(error) => return automatic_dispatch_package_outcome_from_error(error),
    };

    if let Err(error) =
        validate_workflow_handoff_package_transition_linkage(&package, transition_event)
    {
        return automatic_dispatch_package_outcome_from_error(error);
    }

    WorkflowAutomaticDispatchPackageOutcome::Replayed { package }
}

pub(crate) fn validate_workflow_handoff_package_transition_linkage(
    package: &WorkflowHandoffPackageRecord,
    transition_event: &WorkflowTransitionEventRecord,
) -> Result<(), CommandError> {
    if package.handoff_transition_id != transition_event.transition_id {
        return Err(CommandError::system_fault(
            "workflow_handoff_linkage_mismatch",
            format!(
                "Cadence loaded workflow handoff package `{}` for transition `{}` but transition linkage did not match.",
                package.handoff_transition_id, transition_event.transition_id
            ),
        ));
    }

    if package.from_node_id != transition_event.from_node_id
        || package.to_node_id != transition_event.to_node_id
        || package.transition_kind != transition_event.transition_kind
        || package.causal_transition_id != transition_event.causal_transition_id
    {
        return Err(CommandError::system_fault(
            "workflow_handoff_linkage_mismatch",
            format!(
                "Cadence found inconsistent workflow handoff linkage for transition `{}` (expected {} -> {} [{}], found {} -> {} [{}]).",
                transition_event.transition_id,
                transition_event.from_node_id,
                transition_event.to_node_id,
                transition_event.transition_kind,
                package.from_node_id,
                package.to_node_id,
                package.transition_kind,
            ),
        ));
    }

    Ok(())
}

fn automatic_dispatch_package_outcome_from_error(
    error: CommandError,
) -> WorkflowAutomaticDispatchPackageOutcome {
    WorkflowAutomaticDispatchPackageOutcome::Skipped {
        code: error.code,
        message: error.message,
    }
}

fn resolve_plan_mode_required_for_automatic_dispatch(
    transaction: &Transaction<'_>,
    database_path: &Path,
    project_id: &str,
) -> Result<WorkflowAutomaticDispatchPlanModeRequirement, CommandError> {
    let Some(agent_session) =
        read_selected_agent_session_row(transaction, database_path, project_id)?
    else {
        return Ok(WorkflowAutomaticDispatchPlanModeRequirement::Unknown);
    };

    match read_runtime_run_snapshot(
        transaction,
        database_path,
        project_id,
        &agent_session.agent_session_id,
    ) {
        Ok(Some(snapshot)) => {
            if snapshot.controls.active.plan_mode_required {
                Ok(WorkflowAutomaticDispatchPlanModeRequirement::Required)
            } else {
                Ok(WorkflowAutomaticDispatchPlanModeRequirement::Disabled)
            }
        }
        Ok(None) => Ok(WorkflowAutomaticDispatchPlanModeRequirement::Unknown),
        Err(error) if error.code == "runtime_run_decode_failed" => {
            Ok(WorkflowAutomaticDispatchPlanModeRequirement::Unknown)
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn is_plan_mode_required_gate_key(gate_key: &str) -> bool {
    gate_key.trim() == PLAN_MODE_REQUIRED_GATE_KEY
}

fn format_unresolved_dispatch_candidate_summary(
    blocked_candidates: &[WorkflowAutomaticDispatchUnresolvedContinuationCandidate],
) -> String {
    blocked_candidates
        .iter()
        .map(|candidate| {
            let gate_summary = candidate
                .unresolved_gates
                .iter()
                .map(|gate| {
                    format!(
                        "{}:{}:{}",
                        gate.gate_node_id,
                        gate.gate_key,
                        workflow_gate_state_sql_value(&gate.gate_state)
                    )
                })
                .collect::<Vec<_>>()
                .join("|");

            let gate_requirement_suffix = candidate
                .gate_requirement
                .as_deref()
                .map(|required_gate| format!(" gate={required_gate}"))
                .unwrap_or_default();

            format!(
                "{}->{}:{}{} [{}]",
                candidate.from_node_id,
                candidate.to_node_id,
                candidate.transition_kind,
                gate_requirement_suffix,
                gate_summary,
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn derive_auto_dispatch_operator_scope(
    transaction: &Transaction<'_>,
    database_path: &Path,
    project_id: &str,
    trigger_transition: &WorkflowTransitionEventRecord,
) -> Result<(String, Option<String>), CommandError> {
    let runtime_session = read_runtime_session_row(transaction, database_path, project_id)?;

    let runtime_flow_id = runtime_session
        .as_ref()
        .and_then(|session| session.flow_id.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let runtime_session_id = runtime_session
        .as_ref()
        .and_then(|session| session.session_id.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let (session_id, flow_id) = if runtime_flow_id.is_some() || runtime_session_id.is_some() {
        (
            runtime_session_id.unwrap_or_else(|| format!("workflow-auto-dispatch:{project_id}")),
            runtime_flow_id,
        )
    } else {
        (
            format!("workflow-auto-dispatch:{project_id}"),
            Some(format!(
                "workflow-auto-dispatch:{project_id}:{}",
                trigger_transition.transition_id
            )),
        )
    };

    validate_non_empty_text(&session_id, "session_id", "runtime_action_request_invalid")?;
    if let Some(flow_id) = flow_id.as_deref() {
        validate_non_empty_text(flow_id, "flow_id", "runtime_action_request_invalid")?;
    }

    Ok((session_id, flow_id))
}

#[allow(clippy::too_many_arguments)]
fn upsert_pending_operator_approval_row(
    transaction: &Transaction<'_>,
    database_path: &Path,
    project_id: &str,
    session_id: &str,
    flow_id: Option<&str>,
    action_type: &str,
    title: &str,
    detail: &str,
    created_at: &str,
    gate_link: Option<&OperatorApprovalGateLink>,
) -> Result<String, CommandError> {
    let normalized_session_id = session_id.trim();
    let normalized_flow_id = flow_id.map(str::trim).filter(|value| !value.is_empty());

    if normalized_session_id.is_empty() && normalized_flow_id.is_none() {
        return Err(CommandError::system_fault(
            "runtime_action_request_invalid",
            "Cadence could not persist gate-unmet auto-dispatch approval because both session and flow scopes were empty.",
        ));
    }

    validate_non_empty_text(action_type, "action_type", "runtime_action_request_invalid")?;
    validate_non_empty_text(title, "title", "runtime_action_request_invalid")?;
    validate_non_empty_text(detail, "detail", "runtime_action_request_invalid")?;
    validate_non_empty_text(created_at, "created_at", "runtime_action_request_invalid")?;

    let action_id = derive_operator_action_id(
        normalized_session_id,
        normalized_flow_id,
        action_type,
        gate_link,
    )?;

    let existing =
        read_operator_approval_by_action_id(transaction, database_path, project_id, &action_id)?;
    match existing {
        None => {
            transaction
                .execute(
                    r#"
                    INSERT INTO operator_approvals (
                        project_id,
                        action_id,
                        session_id,
                        flow_id,
                        action_type,
                        title,
                        detail,
                        gate_node_id,
                        gate_key,
                        transition_from_node_id,
                        transition_to_node_id,
                        transition_kind,
                        user_answer,
                        status,
                        decision_note,
                        created_at,
                        updated_at,
                        resolved_at
                    )
                    VALUES (
                        ?1,
                        ?2,
                        ?3,
                        ?4,
                        ?5,
                        ?6,
                        ?7,
                        ?8,
                        ?9,
                        ?10,
                        ?11,
                        ?12,
                        NULL,
                        'pending',
                        NULL,
                        ?13,
                        ?13,
                        NULL
                    )
                    "#,
                    params![
                        project_id,
                        action_id,
                        if normalized_session_id.is_empty() {
                            None
                        } else {
                            Some(normalized_session_id)
                        },
                        normalized_flow_id,
                        action_type,
                        title,
                        detail,
                        gate_link.as_ref().map(|link| link.gate_node_id.as_str()),
                        gate_link.as_ref().map(|link| link.gate_key.as_str()),
                        gate_link
                            .as_ref()
                            .map(|link| link.transition_from_node_id.as_str()),
                        gate_link
                            .as_ref()
                            .map(|link| link.transition_to_node_id.as_str()),
                        gate_link.as_ref().map(|link| link.transition_kind.as_str()),
                        created_at,
                    ],
                )
                .map_err(|error| {
                    map_operator_loop_write_error(
                        "operator_approval_upsert_failed",
                        database_path,
                        error,
                        "Cadence could not persist the pending operator approval.",
                    )
                })?;
        }
        Some(approval) => match approval.status {
            OperatorApprovalStatus::Pending => {
                transaction
                    .execute(
                        r#"
                        UPDATE operator_approvals
                        SET session_id = ?3,
                            flow_id = ?4,
                            title = ?5,
                            detail = ?6,
                            gate_node_id = ?7,
                            gate_key = ?8,
                            transition_from_node_id = ?9,
                            transition_to_node_id = ?10,
                            transition_kind = ?11,
                            updated_at = ?12
                        WHERE project_id = ?1
                          AND action_id = ?2
                          AND status = 'pending'
                        "#,
                        params![
                            project_id,
                            action_id,
                            if normalized_session_id.is_empty() {
                                None
                            } else {
                                Some(normalized_session_id)
                            },
                            normalized_flow_id,
                            title,
                            detail,
                            gate_link.as_ref().map(|link| link.gate_node_id.as_str()),
                            gate_link.as_ref().map(|link| link.gate_key.as_str()),
                            gate_link
                                .as_ref()
                                .map(|link| link.transition_from_node_id.as_str()),
                            gate_link
                                .as_ref()
                                .map(|link| link.transition_to_node_id.as_str()),
                            gate_link.as_ref().map(|link| link.transition_kind.as_str()),
                            created_at,
                        ],
                    )
                    .map_err(|error| {
                        map_operator_loop_write_error(
                            "operator_approval_upsert_failed",
                            database_path,
                            error,
                            "Cadence could not refresh the pending operator approval.",
                        )
                    })?;
            }
            OperatorApprovalStatus::Approved | OperatorApprovalStatus::Rejected => {
                return Err(CommandError::retryable(
                    "runtime_action_sync_conflict",
                    format!(
                        "Cadence received a retained runtime action for already-resolved operator request `{action_id}`. Reopen or refresh the selected project before retrying."
                    ),
                ));
            }
        },
    }

    Ok(action_id)
}

fn persist_pending_approval_for_unresolved_auto_dispatch(
    transaction: &Transaction<'_>,
    database_path: &Path,
    project_id: &str,
    trigger_transition: &WorkflowTransitionEventRecord,
    blocked_candidates: &[WorkflowAutomaticDispatchUnresolvedContinuationCandidate],
) -> Result<OperatorApprovalDto, CommandError> {
    let candidate = match blocked_candidates {
        [candidate] => candidate,
        [] => {
            return Err(CommandError::user_fixable(
                "workflow_transition_gate_unmet",
                "Cadence skipped automatic dispatch because no unresolved continuation candidates were available for persistence.",
            ))
        }
        candidates => {
            let blocked_summary = format_unresolved_dispatch_candidate_summary(candidates);
            return Err(CommandError::user_fixable(
                "workflow_transition_gate_unmet",
                format!(
                    "Cadence skipped automatic dispatch because unresolved continuation metadata was ambiguous ({blocked_summary})."
                ),
            ));
        }
    };

    let filtered_gates: Vec<&WorkflowAutomaticDispatchUnresolvedGateCandidate> =
        match candidate.gate_requirement.as_deref() {
            Some(required_gate) => candidate
                .unresolved_gates
                .iter()
                .filter(|gate| gate.gate_key == required_gate)
                .collect(),
            None => candidate.unresolved_gates.iter().collect(),
        };

    let gate = match filtered_gates.as_slice() {
        [gate] => *gate,
        [] => {
            return Err(CommandError::user_fixable(
                "workflow_transition_gate_unmet",
                format!(
                    "Cadence skipped automatic dispatch for `{}` -> `{}` ({}) because required gate linkage could not be resolved from unresolved metadata.",
                    candidate.from_node_id, candidate.to_node_id, candidate.transition_kind
                ),
            ));
        }
        _ => {
            return Err(CommandError::user_fixable(
                "workflow_transition_gate_unmet",
                format!(
                    "Cadence skipped automatic dispatch for `{}` -> `{}` ({}) because unresolved gate metadata was ambiguous for deterministic approval persistence.",
                    candidate.from_node_id, candidate.to_node_id, candidate.transition_kind
                ),
            ));
        }
    };

    if gate.gate_node_id != candidate.to_node_id {
        return Err(CommandError::system_fault(
            "runtime_action_request_invalid",
            format!(
                "Cadence could not persist gate-unmet auto-dispatch approval because gate node `{}` did not match continuation target `{}`.",
                gate.gate_node_id, candidate.to_node_id
            ),
        ));
    }

    if let Some(required_gate) = candidate.gate_requirement.as_deref() {
        if gate.gate_key != required_gate {
            return Err(CommandError::system_fault(
                "runtime_action_request_invalid",
                format!(
                    "Cadence could not persist gate-unmet auto-dispatch approval because gate `{}` did not match required transition gate `{required_gate}`.",
                    gate.gate_key
                ),
            ));
        }
    }

    let action_type = gate
        .action_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CommandError::user_fixable(
                "workflow_transition_gate_unmet",
                format!(
                    "Cadence skipped automatic dispatch for `{}` -> `{}` ({}) because unresolved gate `{}` is non-actionable (missing `action_type`).",
                    candidate.from_node_id,
                    candidate.to_node_id,
                    candidate.transition_kind,
                    gate.gate_key,
                ),
            )
        })?;
    let title = gate
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CommandError::user_fixable(
                "workflow_transition_gate_unmet",
                format!(
                    "Cadence skipped automatic dispatch for `{}` -> `{}` ({}) because unresolved gate `{}` is non-actionable (missing `title`).",
                    candidate.from_node_id,
                    candidate.to_node_id,
                    candidate.transition_kind,
                    gate.gate_key,
                ),
            )
        })?;
    let detail = gate
        .detail
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CommandError::user_fixable(
                "workflow_transition_gate_unmet",
                format!(
                    "Cadence skipped automatic dispatch for `{}` -> `{}` ({}) because unresolved gate `{}` is non-actionable (missing `detail`).",
                    candidate.from_node_id,
                    candidate.to_node_id,
                    candidate.transition_kind,
                    gate.gate_key,
                ),
            )
        })?;

    let gate_link = OperatorApprovalGateLink {
        gate_node_id: gate.gate_node_id.clone(),
        gate_key: gate.gate_key.clone(),
        transition_from_node_id: candidate.from_node_id.clone(),
        transition_to_node_id: candidate.to_node_id.clone(),
        transition_kind: candidate.transition_kind.clone(),
    };

    let (session_id, flow_id) = derive_auto_dispatch_operator_scope(
        transaction,
        database_path,
        project_id,
        trigger_transition,
    )?;

    let created_at = crate::auth::now_timestamp();
    let action_id = upsert_pending_operator_approval_row(
        transaction,
        database_path,
        project_id,
        &session_id,
        flow_id.as_deref(),
        action_type,
        title,
        detail,
        &created_at,
        Some(&gate_link),
    )?;

    read_operator_approval_by_action_id(transaction, database_path, project_id, &action_id)?
        .ok_or_else(|| {
            CommandError::system_fault(
                "operator_approval_missing_after_persist",
                format!(
                    "Cadence persisted gate-unmet auto-dispatch approval `{action_id}` in {} but could not read it back.",
                    database_path.display()
                ),
            )
        })
}

fn resolve_automatic_dispatch_candidate(
    transaction: &Transaction<'_>,
    database_path: &Path,
    project_id: &str,
    completed_node_id: &str,
    plan_mode_requirement: WorkflowAutomaticDispatchPlanModeRequirement,
) -> Result<WorkflowAutomaticDispatchCandidateResolution, CommandError> {
    let nodes = read_workflow_graph_nodes(transaction, database_path, project_id)?;
    if !nodes.iter().any(|node| node.node_id == completed_node_id) {
        return Err(CommandError::user_fixable(
            "workflow_transition_auto_dispatch_source_missing",
            format!(
                "Cadence cannot resolve automatic continuation from `{completed_node_id}` because the workflow node is missing."
            ),
        ));
    }

    let mut outgoing_edges: Vec<WorkflowGraphEdgeRecord> =
        read_workflow_graph_edges(transaction, database_path, project_id)?
            .into_iter()
            .filter(|edge| edge.from_node_id == completed_node_id)
            .collect();

    outgoing_edges.sort_by(|left, right| {
        left.to_node_id
            .cmp(&right.to_node_id)
            .then_with(|| left.transition_kind.cmp(&right.transition_kind))
            .then_with(|| left.gate_requirement.cmp(&right.gate_requirement))
    });

    if outgoing_edges.is_empty() {
        return Ok(WorkflowAutomaticDispatchCandidateResolution::NoContinuation);
    }

    let gates = read_workflow_gate_metadata(transaction, database_path, project_id)?;
    let mut gates_by_node: HashMap<String, Vec<WorkflowGateMetadataRecord>> = HashMap::new();
    for gate in gates {
        gates_by_node
            .entry(gate.node_id.clone())
            .or_default()
            .push(gate);
    }

    let node_ids = nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect::<std::collections::HashSet<_>>();

    let mut legal_candidates = Vec::new();
    let mut blocked_candidates = Vec::new();

    for edge in outgoing_edges {
        if !node_ids.contains(edge.to_node_id.as_str()) {
            return Err(CommandError::user_fixable(
                "workflow_transition_illegal_edge",
                format!(
                    "Cadence cannot automatically dispatch `{}` -> `{}` ({}) because target node `{}` does not exist.",
                    edge.from_node_id,
                    edge.to_node_id,
                    edge.transition_kind,
                    edge.to_node_id
                ),
            ));
        }

        let target_gates = gates_by_node
            .get(edge.to_node_id.as_str())
            .cloned()
            .unwrap_or_default();

        let is_plan_mode_implementation_continuation =
            is_plan_mode_required_implementation_continuation(&nodes, &edge);

        if matches!(
            plan_mode_requirement,
            WorkflowAutomaticDispatchPlanModeRequirement::Unknown
        ) && is_plan_mode_implementation_continuation
        {
            let to_node_id = edge.to_node_id.clone();
            blocked_candidates.push(WorkflowAutomaticDispatchUnresolvedContinuationCandidate {
                from_node_id: edge.from_node_id,
                to_node_id: to_node_id.clone(),
                transition_kind: edge.transition_kind,
                gate_requirement: Some(PLAN_MODE_REQUIRED_GATE_KEY.to_string()),
                unresolved_gates: vec![plan_mode_required_unresolved_gate_candidate(&to_node_id)],
            });
            continue;
        }

        let requires_plan_mode_gate = matches!(
            plan_mode_requirement,
            WorkflowAutomaticDispatchPlanModeRequirement::Required
        ) && is_plan_mode_implementation_continuation;

        if let Some(required_gate) = edge.gate_requirement.as_deref() {
            let required_gate_present = target_gates
                .iter()
                .any(|gate| gate.gate_key == required_gate);

            if !required_gate_present {
                if requires_plan_mode_gate {
                    blocked_candidates.push(
                        WorkflowAutomaticDispatchUnresolvedContinuationCandidate {
                            from_node_id: edge.from_node_id,
                            to_node_id: edge.to_node_id,
                            transition_kind: edge.transition_kind,
                            gate_requirement: Some(required_gate.to_string()),
                            unresolved_gates: Vec::new(),
                        },
                    );
                    continue;
                }

                return Err(CommandError::system_fault(
                    "workflow_transition_auto_dispatch_gate_mapping_invalid",
                    format!(
                        "Cadence found invalid automatic-dispatch gate mapping for `{}` -> `{}` ({}) because required gate `{required_gate}` is missing on target node `{}`.",
                        edge.from_node_id,
                        edge.to_node_id,
                        edge.transition_kind,
                        edge.to_node_id,
                    ),
                ));
            }
        } else if requires_plan_mode_gate {
            let to_node_id = edge.to_node_id.clone();
            blocked_candidates.push(WorkflowAutomaticDispatchUnresolvedContinuationCandidate {
                from_node_id: edge.from_node_id,
                to_node_id: to_node_id.clone(),
                transition_kind: edge.transition_kind,
                gate_requirement: Some(PLAN_MODE_REQUIRED_GATE_KEY.to_string()),
                unresolved_gates: vec![plan_mode_required_unresolved_gate_candidate(&to_node_id)],
            });
            continue;
        }

        let unresolved_gates: Vec<WorkflowAutomaticDispatchUnresolvedGateCandidate> = target_gates
            .iter()
            .filter(|gate| {
                matches!(
                    gate.gate_state,
                    WorkflowGateState::Pending | WorkflowGateState::Blocked
                )
            })
            .map(|gate| WorkflowAutomaticDispatchUnresolvedGateCandidate {
                gate_node_id: gate.node_id.clone(),
                gate_key: gate.gate_key.clone(),
                gate_state: gate.gate_state.clone(),
                action_type: gate.action_type.clone(),
                title: gate.title.clone(),
                detail: gate.detail.clone(),
            })
            .collect();

        if unresolved_gates.is_empty() {
            legal_candidates.push(WorkflowAutomaticDispatchCandidate {
                from_node_id: edge.from_node_id,
                to_node_id: edge.to_node_id,
                transition_kind: edge.transition_kind,
                gate_requirement: edge.gate_requirement,
            });
        } else {
            blocked_candidates.push(WorkflowAutomaticDispatchUnresolvedContinuationCandidate {
                from_node_id: edge.from_node_id,
                to_node_id: edge.to_node_id,
                transition_kind: edge.transition_kind,
                gate_requirement: edge.gate_requirement,
                unresolved_gates,
            });
        }
    }

    match legal_candidates.as_slice() {
        [] if blocked_candidates.is_empty() => {
            Ok(WorkflowAutomaticDispatchCandidateResolution::NoContinuation)
        }
        [] => Ok(WorkflowAutomaticDispatchCandidateResolution::Unresolved {
            completed_node_id: completed_node_id.to_string(),
            blocked_candidates,
        }),
        [single] => Ok(WorkflowAutomaticDispatchCandidateResolution::Candidate(
            single.clone(),
        )),
        candidates => {
            let options = candidates
                .iter()
                .map(|candidate| {
                    format!(
                        "{}->{}:{}",
                        candidate.from_node_id, candidate.to_node_id, candidate.transition_kind
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            Err(CommandError::user_fixable(
                "workflow_transition_ambiguous_next_step",
                format!(
                    "Cadence cannot auto-dispatch from `{completed_node_id}` because multiple legal continuation edges exist ({options})."
                ),
            ))
        }
    }
}

fn plan_mode_required_unresolved_gate_candidate(
    gate_node_id: &str,
) -> WorkflowAutomaticDispatchUnresolvedGateCandidate {
    WorkflowAutomaticDispatchUnresolvedGateCandidate {
        gate_node_id: gate_node_id.to_string(),
        gate_key: PLAN_MODE_REQUIRED_GATE_KEY.to_string(),
        gate_state: WorkflowGateState::Pending,
        action_type: Some(PLAN_MODE_REQUIRED_ACTION_TYPE.to_string()),
        title: Some(PLAN_MODE_REQUIRED_TITLE.to_string()),
        detail: Some(PLAN_MODE_REQUIRED_DETAIL.to_string()),
    }
}

fn is_plan_mode_required_implementation_continuation(
    nodes: &[WorkflowGraphNodeRecord],
    edge: &WorkflowGraphEdgeRecord,
) -> bool {
    let Some(from_node) = nodes.iter().find(|node| node.node_id == edge.from_node_id) else {
        return false;
    };
    let Some(to_node) = nodes.iter().find(|node| node.node_id == edge.to_node_id) else {
        return false;
    };

    is_planning_lifecycle_node_id(&from_node.node_id)
        && !is_planning_lifecycle_node_id(&to_node.node_id)
        && is_implementation_node(to_node)
}

fn is_planning_lifecycle_node_id(node_id: &str) -> bool {
    let normalized = node_id.trim().to_ascii_lowercase().replace('_', "-");
    matches!(
        normalized.as_str(),
        "discussion"
            | "discuss"
            | "plan-discussion"
            | "planning-discussion"
            | "workflow-discussion"
            | "lifecycle-discussion"
            | "research"
            | "plan-research"
            | "planning-research"
            | "workflow-research"
            | "lifecycle-research"
            | "requirements"
            | "requirement"
            | "plan-requirements"
            | "planning-requirements"
            | "workflow-requirements"
            | "lifecycle-requirements"
            | "roadmap"
            | "plan-roadmap"
            | "planning-roadmap"
            | "workflow-roadmap"
            | "lifecycle-roadmap"
    )
}

fn is_implementation_node(node: &WorkflowGraphNodeRecord) -> bool {
    if matches!(node.current_step, Some(PhaseStep::Execute)) {
        return true;
    }

    let normalized = node.node_id.trim().to_ascii_lowercase().replace('_', "-");
    normalized.contains("implement") || normalized.contains("execute")
}

fn automatic_dispatch_outcome_from_error(error: CommandError) -> WorkflowAutomaticDispatchOutcome {
    WorkflowAutomaticDispatchOutcome::Skipped {
        code: error.code,
        message: error.message,
    }
}
