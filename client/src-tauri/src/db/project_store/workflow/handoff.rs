use super::*;

pub fn assemble_workflow_handoff_package(
    repo_root: &Path,
    project_id: &str,
    handoff_transition_id: &str,
) -> Result<WorkflowHandoffPackageUpsertRecord, CommandError> {
    validate_non_empty_text(project_id, "project_id", "workflow_handoff_request_invalid")?;
    validate_non_empty_text(
        handoff_transition_id,
        "handoff_transition_id",
        "workflow_handoff_request_invalid",
    )?;

    let database_path = database_path_for_repo(repo_root);
    let connection = open_project_database(repo_root, &database_path)?;
    read_project_row(&connection, &database_path, repo_root, project_id)?;

    let trigger_transition = read_transition_event_by_transition_id(
        &connection,
        &database_path,
        project_id,
        handoff_transition_id,
    )?
    .ok_or_else(|| {
        CommandError::user_fixable(
            "workflow_handoff_build_transition_missing",
            format!(
                "Cadence could not assemble a workflow handoff package because transition `{handoff_transition_id}` is not present for project `{project_id}`."
            ),
        )
    })?;

    assemble_workflow_handoff_package_upsert_record(
        &connection,
        &database_path,
        project_id,
        &trigger_transition,
    )
}

pub fn assemble_and_persist_workflow_handoff_package(
    repo_root: &Path,
    project_id: &str,
    handoff_transition_id: &str,
) -> Result<WorkflowHandoffPackageRecord, CommandError> {
    let payload = assemble_workflow_handoff_package(repo_root, project_id, handoff_transition_id)?;
    upsert_workflow_handoff_package(repo_root, &payload)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkflowHandoffPackagePersistDisposition {
    Persisted,
    Replayed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkflowHandoffPackagePersistResult {
    pub(crate) package: WorkflowHandoffPackageRecord,
    pub(crate) disposition: WorkflowHandoffPackagePersistDisposition,
}

pub fn upsert_workflow_handoff_package(
    repo_root: &Path,
    payload: &WorkflowHandoffPackageUpsertRecord,
) -> Result<WorkflowHandoffPackageRecord, CommandError> {
    let database_path = database_path_for_repo(repo_root);
    let connection = open_project_database(repo_root, &database_path)?;
    read_project_row(&connection, &database_path, repo_root, &payload.project_id)?;

    let persisted =
        persist_workflow_handoff_package_with_connection(&connection, &database_path, payload)?;

    Ok(persisted.package)
}

pub(crate) fn persist_workflow_handoff_package_with_connection(
    connection: &Connection,
    database_path: &Path,
    payload: &WorkflowHandoffPackageUpsertRecord,
) -> Result<WorkflowHandoffPackagePersistResult, CommandError> {
    validate_workflow_handoff_package_payload(payload)?;

    let canonical_payload = canonicalize_workflow_handoff_package_payload(
        &payload.package_payload,
        Some(database_path),
        "workflow_handoff_request_invalid",
    )?;
    let package_hash = compute_workflow_handoff_package_hash(&canonical_payload);

    let transaction = connection.unchecked_transaction().map_err(|error| {
        map_workflow_handoff_transaction_error(
            "workflow_handoff_transaction_failed",
            database_path,
            error,
            "Cadence could not start the workflow handoff-package transaction.",
        )
    })?;

    let transition_event = read_transition_event_by_transition_id(
        &transaction,
        database_path,
        &payload.project_id,
        &payload.handoff_transition_id,
    )?
    .ok_or_else(|| {
        CommandError::user_fixable(
            "workflow_handoff_transition_missing",
            format!(
                "Cadence cannot persist a workflow handoff package for transition `{}` because no matching workflow transition event exists.",
                payload.handoff_transition_id
            ),
        )
    })?;

    validate_workflow_handoff_transition_metadata(payload, &transition_event)?;

    let inserted_rows = transaction
        .execute(
            r#"
            INSERT INTO workflow_handoff_packages (
                project_id,
                handoff_transition_id,
                causal_transition_id,
                from_node_id,
                to_node_id,
                transition_kind,
                package_payload,
                package_hash,
                created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(project_id, handoff_transition_id) DO NOTHING
            "#,
            params![
                payload.project_id.as_str(),
                payload.handoff_transition_id.as_str(),
                transition_event.causal_transition_id.as_deref(),
                payload.from_node_id.as_str(),
                payload.to_node_id.as_str(),
                payload.transition_kind.as_str(),
                canonical_payload.as_str(),
                package_hash.as_str(),
                payload.created_at.as_str(),
            ],
        )
        .map_err(|error| {
            map_workflow_handoff_insert_error(
                database_path,
                error,
                &payload.project_id,
                &payload.handoff_transition_id,
            )
        })?;

    if inserted_rows == 0 {
        let existing = read_workflow_handoff_package_by_transition_id(
            &transaction,
            database_path,
            &payload.project_id,
            &payload.handoff_transition_id,
        )?
        .ok_or_else(|| {
            CommandError::system_fault(
                "workflow_handoff_missing_after_replay",
                format!(
                    "Cadence replayed workflow handoff package write for transition `{}` in {} but could not read the stored package row.",
                    payload.handoff_transition_id,
                    database_path.display()
                ),
            )
        })?;

        if existing.package_hash != package_hash {
            return Err(CommandError::system_fault(
                "workflow_handoff_hash_conflict",
                format!(
                    "Cadence refused to overwrite replayed workflow handoff package for transition `{}` because stored hash `{}` did not match derived hash `{}` in {}.",
                    payload.handoff_transition_id,
                    existing.package_hash,
                    package_hash,
                    database_path.display()
                ),
            ));
        }

        transaction.rollback().map_err(|error| {
            map_workflow_handoff_commit_error(
                "workflow_handoff_commit_failed",
                database_path,
                error,
                "Cadence could not close the workflow handoff-package replay transaction.",
            )
        })?;

        return Ok(WorkflowHandoffPackagePersistResult {
            package: existing,
            disposition: WorkflowHandoffPackagePersistDisposition::Replayed,
        });
    }

    transaction.commit().map_err(|error| {
        map_workflow_handoff_commit_error(
            "workflow_handoff_commit_failed",
            database_path,
            error,
            "Cadence could not commit the workflow handoff-package transaction.",
        )
    })?;

    let package = read_workflow_handoff_package_by_transition_id(
        connection,
        database_path,
        &payload.project_id,
        &payload.handoff_transition_id,
    )?
    .ok_or_else(|| {
        CommandError::system_fault(
            "workflow_handoff_missing_after_persist",
            format!(
                "Cadence persisted workflow handoff package transition `{}` in {} but could not read it back.",
                payload.handoff_transition_id,
                database_path.display()
            ),
        )
    })?;

    Ok(WorkflowHandoffPackagePersistResult {
        package,
        disposition: WorkflowHandoffPackagePersistDisposition::Persisted,
    })
}

pub fn load_workflow_handoff_package(
    repo_root: &Path,
    expected_project_id: &str,
    handoff_transition_id: &str,
) -> Result<Option<WorkflowHandoffPackageRecord>, CommandError> {
    validate_non_empty_text(
        handoff_transition_id,
        "handoff_transition_id",
        "workflow_handoff_request_invalid",
    )?;

    let database_path = database_path_for_repo(repo_root);
    let connection = open_project_database(repo_root, &database_path)?;
    read_project_row(&connection, &database_path, repo_root, expected_project_id)?;

    read_workflow_handoff_package_by_transition_id(
        &connection,
        &database_path,
        expected_project_id,
        handoff_transition_id,
    )
}

pub fn load_recent_workflow_handoff_packages(
    repo_root: &Path,
    expected_project_id: &str,
    limit: Option<u32>,
) -> Result<Vec<WorkflowHandoffPackageRecord>, CommandError> {
    let database_path = database_path_for_repo(repo_root);
    let connection = open_project_database(repo_root, &database_path)?;
    read_project_row(&connection, &database_path, repo_root, expected_project_id)?;

    read_workflow_handoff_packages(
        &connection,
        &database_path,
        expected_project_id,
        limit.map(i64::from),
    )
}

pub(crate) fn assemble_workflow_handoff_package_upsert_record(
    connection: &Connection,
    database_path: &Path,
    project_id: &str,
    trigger_transition: &WorkflowTransitionEventRecord,
) -> Result<WorkflowHandoffPackageUpsertRecord, CommandError> {
    if trigger_transition.transition_id.starts_with("auto:")
        && trigger_transition.causal_transition_id.is_none()
    {
        return Err(CommandError::system_fault(
            "workflow_handoff_build_causal_missing",
            format!(
                "Cadence cannot assemble workflow handoff package `{}` because automatic transitions must retain causal transition linkage.",
                trigger_transition.transition_id
            ),
        ));
    }

    ensure_workflow_handoff_safe_text(
        &trigger_transition.transition_id,
        "triggerTransition.transitionId",
    )?;
    ensure_workflow_handoff_optional_text(
        trigger_transition.causal_transition_id.as_deref(),
        "triggerTransition.causalTransitionId",
    )?;
    ensure_workflow_handoff_safe_text(
        &trigger_transition.from_node_id,
        "triggerTransition.fromNodeId",
    )?;
    ensure_workflow_handoff_safe_text(
        &trigger_transition.to_node_id,
        "triggerTransition.toNodeId",
    )?;
    ensure_workflow_handoff_safe_text(
        &trigger_transition.transition_kind,
        "triggerTransition.transitionKind",
    )?;

    let nodes =
        read_workflow_graph_nodes(connection, database_path, project_id).map_err(|error| {
            map_workflow_handoff_build_dependency_error(
                "workflow_handoff_build_node_state_invalid",
                "workflow node state",
                error,
            )
        })?;

    let destination_node = nodes
        .into_iter()
        .find(|node| node.node_id == trigger_transition.to_node_id)
        .ok_or_else(|| {
            CommandError::user_fixable(
                "workflow_handoff_build_target_missing",
                format!(
                    "Cadence cannot assemble workflow handoff package `{}` because destination node `{}` metadata is missing.",
                    trigger_transition.transition_id, trigger_transition.to_node_id
                ),
            )
        })?;

    ensure_workflow_handoff_safe_text(&destination_node.node_id, "destinationState.nodeId")?;
    ensure_workflow_handoff_safe_text(&destination_node.name, "destinationState.name")?;
    ensure_workflow_handoff_safe_text(
        &destination_node.description,
        "destinationState.description",
    )?;

    let mut destination_gates = read_workflow_gate_metadata(connection, database_path, project_id)
        .map_err(|error| {
            map_workflow_handoff_build_dependency_error(
                "workflow_handoff_build_gate_state_invalid",
                "destination gate state",
                error,
            )
        })?
        .into_iter()
        .filter(|gate| gate.node_id == destination_node.node_id)
        .map(|gate| {
            ensure_workflow_handoff_safe_text(&gate.gate_key, "destinationState.gates[].gateKey")?;
            ensure_workflow_handoff_optional_text(
                gate.action_type.as_deref(),
                "destinationState.gates[].actionType",
            )?;

            Ok(WorkflowHandoffDestinationGatePayload {
                gate_key: gate.gate_key,
                gate_state: workflow_gate_state_sql_value(&gate.gate_state).to_string(),
                action_type: gate.action_type,
                detail_present: gate.detail.is_some(),
                decision_context_present: gate.decision_context.is_some(),
            })
        })
        .collect::<Result<Vec<_>, CommandError>>()?;

    destination_gates.sort_by(|left, right| {
        left.gate_key
            .cmp(&right.gate_key)
            .then_with(|| left.gate_state.cmp(&right.gate_state))
            .then_with(|| left.action_type.cmp(&right.action_type))
    });

    let pending_gate_count = destination_gates
        .iter()
        .filter(|gate| matches!(gate.gate_state.as_str(), "pending" | "blocked"))
        .count() as u32;

    let lifecycle_projection =
        read_planning_lifecycle_projection(connection, database_path, project_id).map_err(
            |error| {
                map_workflow_handoff_build_dependency_error(
                    "workflow_handoff_build_lifecycle_invalid",
                    "lifecycle projection",
                    error,
                )
            },
        )?;

    validate_workflow_handoff_lifecycle_projection(
        &lifecycle_projection,
        &trigger_transition.transition_id,
    )?;

    let lifecycle_stages = lifecycle_projection
        .stages
        .into_iter()
        .map(|stage| {
            ensure_workflow_handoff_safe_text(
                &stage.node_id,
                "lifecycleProjection.stages[].nodeId",
            )?;
            Ok(stage)
        })
        .collect::<Result<Vec<_>, CommandError>>()?;

    let operator_approvals = read_operator_approvals(connection, database_path, project_id)
        .map_err(|error| {
            map_workflow_handoff_build_dependency_error(
                "workflow_handoff_build_operator_state_invalid",
                "operator approvals",
                error,
            )
        })?;

    let mut pending_gate_actions = operator_approvals
        .into_iter()
        .filter(|approval| approval.status == OperatorApprovalStatus::Pending)
        .filter_map(|approval| {
            let OperatorApprovalDto {
                action_id,
                action_type,
                gate_node_id,
                gate_key,
                transition_from_node_id,
                transition_to_node_id,
                transition_kind,
                created_at,
                updated_at,
                ..
            } = approval;

            let (
                Some(gate_node_id),
                Some(gate_key),
                Some(transition_from_node_id),
                Some(transition_to_node_id),
                Some(transition_kind),
            ) = (
                gate_node_id,
                gate_key,
                transition_from_node_id,
                transition_to_node_id,
                transition_kind,
            )
            else {
                return None;
            };

            if transition_to_node_id != trigger_transition.to_node_id {
                return None;
            }

            Some((
                action_id,
                action_type,
                gate_node_id,
                gate_key,
                transition_from_node_id,
                transition_to_node_id,
                transition_kind,
                created_at,
                updated_at,
            ))
        })
        .map(
            |(
                action_id,
                action_type,
                gate_node_id,
                gate_key,
                transition_from_node_id,
                transition_to_node_id,
                transition_kind,
                created_at,
                updated_at,
            )| {
                ensure_workflow_handoff_safe_text(
                    &action_id,
                    "operatorContinuity.pendingGateActions[].actionId",
                )?;
                ensure_workflow_handoff_safe_text(
                    &action_type,
                    "operatorContinuity.pendingGateActions[].actionType",
                )?;
                ensure_workflow_handoff_safe_text(
                    &gate_node_id,
                    "operatorContinuity.pendingGateActions[].gateNodeId",
                )?;
                ensure_workflow_handoff_safe_text(
                    &gate_key,
                    "operatorContinuity.pendingGateActions[].gateKey",
                )?;
                ensure_workflow_handoff_safe_text(
                    &transition_from_node_id,
                    "operatorContinuity.pendingGateActions[].transitionFromNodeId",
                )?;
                ensure_workflow_handoff_safe_text(
                    &transition_to_node_id,
                    "operatorContinuity.pendingGateActions[].transitionToNodeId",
                )?;
                ensure_workflow_handoff_safe_text(
                    &transition_kind,
                    "operatorContinuity.pendingGateActions[].transitionKind",
                )?;

                Ok(WorkflowHandoffPendingGateActionPayload {
                    action_id,
                    action_type,
                    gate_node_id,
                    gate_key,
                    transition_from_node_id,
                    transition_to_node_id,
                    transition_kind,
                    created_at,
                    updated_at,
                })
            },
        )
        .collect::<Result<Vec<_>, CommandError>>()?;

    pending_gate_actions.sort_by(|left, right| {
        left.action_id
            .cmp(&right.action_id)
            .then_with(|| {
                left.transition_from_node_id
                    .cmp(&right.transition_from_node_id)
            })
            .then_with(|| left.transition_to_node_id.cmp(&right.transition_to_node_id))
            .then_with(|| left.transition_kind.cmp(&right.transition_kind))
    });

    let pending_action_ids = pending_gate_actions
        .iter()
        .map(|action| action.action_id.as_str())
        .collect::<std::collections::HashSet<_>>();

    let resume_history =
        read_resume_history(connection, database_path, project_id).map_err(|error| {
            map_workflow_handoff_build_dependency_error(
                "workflow_handoff_build_operator_state_invalid",
                "operator resume history",
                error,
            )
        })?;

    let latest_resume_row = if pending_action_ids.is_empty() {
        resume_history.into_iter().next()
    } else {
        resume_history.into_iter().find(|entry| {
            entry
                .source_action_id
                .as_deref()
                .is_some_and(|source_action_id| pending_action_ids.contains(source_action_id))
        })
    };

    let latest_resume = latest_resume_row
        .map(|entry| {
            ensure_workflow_handoff_optional_text(
                entry.source_action_id.as_deref(),
                "operatorContinuity.latestResume.sourceActionId",
            )?;

            Ok(WorkflowHandoffLatestResumePayload {
                source_action_id: entry.source_action_id,
                status: entry.status,
                created_at: entry.created_at,
            })
        })
        .transpose()?;

    let payload = WorkflowHandoffPackagePayload {
        schema_version: WORKFLOW_HANDOFF_PACKAGE_SCHEMA_VERSION,
        trigger_transition: WorkflowHandoffTriggerTransitionPayload {
            transition_id: trigger_transition.transition_id.clone(),
            causal_transition_id: trigger_transition.causal_transition_id.clone(),
            from_node_id: trigger_transition.from_node_id.clone(),
            to_node_id: trigger_transition.to_node_id.clone(),
            transition_kind: trigger_transition.transition_kind.clone(),
            gate_decision: workflow_transition_gate_decision_sql_value(
                &trigger_transition.gate_decision,
            )
            .to_string(),
            gate_decision_context_present: trigger_transition.gate_decision_context.is_some(),
            occurred_at: trigger_transition.created_at.clone(),
        },
        destination_state: WorkflowHandoffDestinationStatePayload {
            node_id: destination_node.node_id,
            phase_id: destination_node.phase_id,
            sort_order: destination_node.sort_order,
            name: destination_node.name,
            description: destination_node.description,
            status: destination_node.status,
            current_step: destination_node.current_step,
            task_count: destination_node.task_count,
            completed_tasks: destination_node.completed_tasks,
            pending_gate_count,
            gates: destination_gates,
        },
        lifecycle_projection: WorkflowHandoffLifecycleProjectionPayload {
            stages: lifecycle_stages,
        },
        operator_continuity: WorkflowHandoffOperatorContinuityPayload {
            pending_gate_action_count: pending_gate_actions.len() as u32,
            pending_gate_actions,
            latest_resume,
        },
    };

    let package_payload = serialize_workflow_handoff_package_payload(&payload, database_path)?;

    Ok(WorkflowHandoffPackageUpsertRecord {
        project_id: project_id.to_string(),
        handoff_transition_id: trigger_transition.transition_id.clone(),
        causal_transition_id: trigger_transition.causal_transition_id.clone(),
        from_node_id: trigger_transition.from_node_id.clone(),
        to_node_id: trigger_transition.to_node_id.clone(),
        transition_kind: trigger_transition.transition_kind.clone(),
        package_payload,
        created_at: trigger_transition.created_at.clone(),
    })
}

fn validate_workflow_handoff_lifecycle_projection(
    lifecycle_projection: &PlanningLifecycleProjectionDto,
    transition_id: &str,
) -> Result<(), CommandError> {
    let mut previous_index: Option<usize> = None;
    let mut seen_stage_indexes = [false; 4];

    for stage in &lifecycle_projection.stages {
        let stage_index = workflow_handoff_lifecycle_stage_index(stage.stage);

        if seen_stage_indexes[stage_index] {
            return Err(CommandError::user_fixable(
                "workflow_handoff_build_lifecycle_invalid",
                format!(
                    "Cadence cannot assemble workflow handoff package `{transition_id}` because lifecycle stage `{}` appears more than once.",
                    planning_lifecycle_stage_label(&stage.stage)
                ),
            ));
        }

        if let Some(previous_index) = previous_index {
            if stage_index < previous_index {
                return Err(CommandError::user_fixable(
                    "workflow_handoff_build_lifecycle_invalid",
                    format!(
                        "Cadence cannot assemble workflow handoff package `{transition_id}` because lifecycle stages are not in canonical order."
                    ),
                ));
            }
        }

        seen_stage_indexes[stage_index] = true;
        previous_index = Some(stage_index);
    }

    Ok(())
}

fn workflow_handoff_lifecycle_stage_index(stage: PlanningLifecycleStageKindDto) -> usize {
    match stage {
        PlanningLifecycleStageKindDto::Discussion => 0,
        PlanningLifecycleStageKindDto::Research => 1,
        PlanningLifecycleStageKindDto::Requirements => 2,
        PlanningLifecycleStageKindDto::Roadmap => 3,
    }
}

fn ensure_workflow_handoff_optional_text(
    value: Option<&str>,
    field: &'static str,
) -> Result<(), CommandError> {
    if let Some(value) = value {
        ensure_workflow_handoff_safe_text(value, field)?;
    }

    Ok(())
}

fn ensure_workflow_handoff_safe_text(value: &str, field: &'static str) -> Result<(), CommandError> {
    if let Some(secret_hint) = find_prohibited_workflow_handoff_content(value) {
        return Err(CommandError::user_fixable(
            "workflow_handoff_redaction_failed",
            format!(
                "Cadence refused to assemble workflow handoff package because `{field}` contained {secret_hint}. Remove secret-bearing transcript/tool/auth content before retrying."
            ),
        ));
    }

    Ok(())
}

pub(crate) fn find_prohibited_workflow_handoff_content(value: &str) -> Option<&'static str> {
    find_prohibited_runtime_persistence_content(value)
}

fn serialize_workflow_handoff_package_payload(
    payload: &WorkflowHandoffPackagePayload,
    database_path: &Path,
) -> Result<String, CommandError> {
    let raw_payload = serde_json::to_value(payload).map_err(|error| {
        CommandError::system_fault(
            "workflow_handoff_serialize_failed",
            format!(
                "Cadence could not serialize workflow handoff package payload in {}: {error}",
                database_path.display()
            ),
        )
    })?;

    let canonical_payload = canonicalize_workflow_handoff_json_value(raw_payload);
    let serialized_payload = serde_json::to_string(&canonical_payload).map_err(|error| {
        CommandError::system_fault(
            "workflow_handoff_serialize_failed",
            format!(
                "Cadence could not canonicalize workflow handoff package payload in {}: {error}",
                database_path.display()
            ),
        )
    })?;

    if let Some(secret_hint) = find_prohibited_workflow_handoff_content(&serialized_payload) {
        return Err(CommandError::user_fixable(
            "workflow_handoff_redaction_failed",
            format!(
                "Cadence refused to assemble workflow handoff package because serialized payload contained {secret_hint}. Remove secret-bearing transcript/tool/auth content before retrying."
            ),
        ));
    }

    Ok(serialized_payload)
}

fn map_workflow_handoff_build_dependency_error(
    code: &str,
    dependency: &str,
    error: CommandError,
) -> CommandError {
    let message = format!(
        "Cadence could not assemble workflow handoff package because {dependency} could not be loaded: {}",
        error.message
    );

    match error.class {
        CommandErrorClass::UserFixable | CommandErrorClass::PolicyDenied => {
            CommandError::user_fixable(code, message)
        }
        CommandErrorClass::Retryable => CommandError::retryable(code, message),
        CommandErrorClass::SystemFault => CommandError::system_fault(code, message),
    }
}
