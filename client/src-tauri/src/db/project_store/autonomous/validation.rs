use super::*;

pub(crate) fn validate_autonomous_run_payload(
    payload: &AutonomousRunRecord,
) -> Result<(), CommandError> {
    validate_non_empty_text(
        &payload.project_id,
        "project_id",
        "autonomous_run_request_invalid",
    )?;
    validate_non_empty_text(
        &payload.agent_session_id,
        "agent_session_id",
        "autonomous_run_request_invalid",
    )?;
    validate_non_empty_text(&payload.run_id, "run_id", "autonomous_run_request_invalid")?;
    validate_non_empty_text(
        &payload.runtime_kind,
        "runtime_kind",
        "autonomous_run_request_invalid",
    )?;
    validate_non_empty_text(
        &payload.provider_id,
        "provider_id",
        "autonomous_run_request_invalid",
    )?;
    crate::runtime::resolve_runtime_provider_identity(
        Some(payload.provider_id.as_str()),
        Some(payload.runtime_kind.as_str()),
    )
    .map_err(|diagnostic| {
        CommandError::user_fixable(
            "autonomous_run_request_invalid",
            format!(
                "Cadence rejected the durable autonomous-run identity because {}",
                diagnostic.message
            ),
        )
    })?;
    validate_non_empty_text(
        &payload.supervisor_kind,
        "supervisor_kind",
        "autonomous_run_request_invalid",
    )?;
    validate_non_empty_text(
        &payload.started_at,
        "started_at",
        "autonomous_run_request_invalid",
    )?;
    validate_non_empty_text(
        &payload.updated_at,
        "updated_at",
        "autonomous_run_request_invalid",
    )?;

    if let Some(active_unit_sequence) = payload.active_unit_sequence {
        if active_unit_sequence == 0 {
            return Err(CommandError::system_fault(
                "autonomous_run_request_invalid",
                "Cadence requires autonomous active-unit sequences to start at 1.",
            ));
        }
    }

    for (value, field) in [
        (payload.last_heartbeat_at.as_deref(), "last_heartbeat_at"),
        (payload.last_checkpoint_at.as_deref(), "last_checkpoint_at"),
        (payload.paused_at.as_deref(), "paused_at"),
        (payload.cancelled_at.as_deref(), "cancelled_at"),
        (payload.completed_at.as_deref(), "completed_at"),
        (payload.crashed_at.as_deref(), "crashed_at"),
        (payload.stopped_at.as_deref(), "stopped_at"),
        (
            payload.duplicate_start_run_id.as_deref(),
            "duplicate_start_run_id",
        ),
        (
            payload.duplicate_start_reason.as_deref(),
            "duplicate_start_reason",
        ),
    ] {
        if let Some(value) = value {
            validate_non_empty_text(value, field, "autonomous_run_request_invalid")?;
        }
    }

    for (reason, label) in [
        (payload.pause_reason.as_ref(), "pause_reason"),
        (payload.cancel_reason.as_ref(), "cancel_reason"),
        (payload.crash_reason.as_ref(), "crash_reason"),
        (payload.last_error.as_ref(), "last_error"),
    ] {
        if let Some(reason) = reason {
            validate_non_empty_text(
                &reason.code,
                &format!("{label}_code"),
                "autonomous_run_request_invalid",
            )?;
            validate_non_empty_text(
                &reason.message,
                &format!("{label}_message"),
                "autonomous_run_request_invalid",
            )?;
            if let Some(secret_hint) = find_prohibited_runtime_persistence_content(&reason.message)
            {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    format!(
                        "Autonomous run {label} must not include {secret_hint}. Remove secret-bearing content before retrying."
                    ),
                ));
            }
        }
    }

    Ok(())
}

pub(crate) fn normalize_autonomous_run_upsert_payload(
    payload: &AutonomousRunUpsertRecord,
) -> Result<AutonomousRunUpsertRecord, CommandError> {
    validate_autonomous_run_payload(&payload.run)?;

    let Some(unit) = payload.unit.as_ref() else {
        if payload.attempt.is_some() || !payload.artifacts.is_empty() {
            return Err(CommandError::system_fault(
                "autonomous_run_request_invalid",
                "Cadence requires a durable autonomous unit row before attempts or artifacts can be persisted.",
            ));
        }
        return Ok(payload.clone());
    };

    validate_non_empty_text(&unit.unit_id, "unit_id", "autonomous_run_request_invalid")?;
    validate_non_empty_text(&unit.summary, "summary", "autonomous_run_request_invalid")?;
    validate_non_empty_text(
        &unit.started_at,
        "started_at",
        "autonomous_run_request_invalid",
    )?;
    validate_non_empty_text(
        &unit.updated_at,
        "updated_at",
        "autonomous_run_request_invalid",
    )?;
    if unit.sequence == 0 {
        return Err(CommandError::system_fault(
            "autonomous_run_request_invalid",
            "Cadence requires autonomous unit sequences to start at 1.",
        ));
    }
    if unit.project_id != payload.run.project_id || unit.run_id != payload.run.run_id {
        return Err(CommandError::system_fault(
            "autonomous_run_request_invalid",
            "Cadence requires autonomous unit rows to share the parent run project_id and run_id.",
        ));
    }
    if let Some(boundary_id) = unit.boundary_id.as_deref() {
        validate_non_empty_text(boundary_id, "boundary_id", "autonomous_run_request_invalid")?;
    }
    if let Some(secret_hint) = find_prohibited_runtime_persistence_content(&unit.summary) {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            format!(
                "Autonomous unit summaries must not include {secret_hint}. Remove secret-bearing content before retrying."
            ),
        ));
    }

    let normalized_unit_workflow_linkage = normalize_autonomous_workflow_linkage_payload(
        unit.workflow_linkage.as_ref(),
        "unit_workflow_linkage",
    )?;

    let normalized_attempt = if let Some(attempt) = payload.attempt.as_ref() {
        validate_non_empty_text(
            &attempt.attempt_id,
            "attempt_id",
            "autonomous_run_request_invalid",
        )?;
        validate_non_empty_text(
            &attempt.child_session_id,
            "child_session_id",
            "autonomous_run_request_invalid",
        )?;
        validate_non_empty_text(
            &attempt.started_at,
            "attempt_started_at",
            "autonomous_run_request_invalid",
        )?;
        validate_non_empty_text(
            &attempt.updated_at,
            "attempt_updated_at",
            "autonomous_run_request_invalid",
        )?;
        if attempt.attempt_number == 0 {
            return Err(CommandError::system_fault(
                "autonomous_run_request_invalid",
                "Cadence requires autonomous attempt numbers to start at 1.",
            ));
        }
        if attempt.project_id != payload.run.project_id
            || attempt.run_id != payload.run.run_id
            || attempt.unit_id != unit.unit_id
        {
            return Err(CommandError::system_fault(
                "autonomous_run_request_invalid",
                "Cadence requires autonomous attempts to share the parent run and unit linkage.",
            ));
        }
        if let Some(boundary_id) = attempt.boundary_id.as_deref() {
            validate_non_empty_text(
                boundary_id,
                "attempt_boundary_id",
                "autonomous_run_request_invalid",
            )?;
        }
        if let Some(reason) = attempt.last_error.as_ref() {
            validate_non_empty_text(
                &reason.code,
                "attempt_last_error_code",
                "autonomous_run_request_invalid",
            )?;
            validate_non_empty_text(
                &reason.message,
                "attempt_last_error_message",
                "autonomous_run_request_invalid",
            )?;
        }

        let normalized_attempt_workflow_linkage = normalize_autonomous_workflow_linkage_payload(
            attempt.workflow_linkage.as_ref(),
            "attempt_workflow_linkage",
        )?;
        validate_matching_autonomous_workflow_linkage_payloads(
            normalized_unit_workflow_linkage.as_ref(),
            normalized_attempt_workflow_linkage.as_ref(),
        )?;

        Some(AutonomousUnitAttemptRecord {
            workflow_linkage: normalized_attempt_workflow_linkage,
            ..attempt.clone()
        })
    } else {
        None
    };

    let normalized_artifacts = payload
        .artifacts
        .iter()
        .map(|artifact| {
            normalize_autonomous_unit_artifact_record(
                artifact,
                &payload.run,
                unit,
                payload.attempt.as_ref(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(AutonomousRunUpsertRecord {
        run: payload.run.clone(),
        unit: Some(AutonomousUnitRecord {
            workflow_linkage: normalized_unit_workflow_linkage,
            ..unit.clone()
        }),
        attempt: normalized_attempt,
        artifacts: normalized_artifacts,
    })
}

pub(crate) fn normalize_autonomous_workflow_linkage_payload(
    linkage: Option<&AutonomousWorkflowLinkageRecord>,
    field_prefix: &str,
) -> Result<Option<AutonomousWorkflowLinkageRecord>, CommandError> {
    let Some(linkage) = linkage else {
        return Ok(None);
    };

    validate_non_empty_text(
        &linkage.workflow_node_id,
        &format!("{field_prefix}_workflow_node_id"),
        "autonomous_run_request_invalid",
    )?;
    validate_non_empty_text(
        &linkage.transition_id,
        &format!("{field_prefix}_transition_id"),
        "autonomous_run_request_invalid",
    )?;
    if let Some(causal_transition_id) = linkage.causal_transition_id.as_deref() {
        validate_non_empty_text(
            causal_transition_id,
            &format!("{field_prefix}_causal_transition_id"),
            "autonomous_run_request_invalid",
        )?;
    }
    validate_non_empty_text(
        &linkage.handoff_transition_id,
        &format!("{field_prefix}_handoff_transition_id"),
        "autonomous_run_request_invalid",
    )?;
    validate_non_empty_text(
        &linkage.handoff_package_hash,
        &format!("{field_prefix}_handoff_package_hash"),
        "autonomous_run_request_invalid",
    )?;

    if linkage.handoff_package_hash.len() != 64
        || linkage
            .handoff_package_hash
            .chars()
            .any(|ch| !ch.is_ascii_hexdigit() || ch.is_ascii_uppercase())
    {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            format!(
                "Cadence requires {field_prefix} handoff package hashes to be lowercase 64-character hex digests."
            ),
        ));
    }

    Ok(Some(linkage.clone()))
}

pub(crate) fn validate_matching_autonomous_workflow_linkage_payloads(
    unit_linkage: Option<&AutonomousWorkflowLinkageRecord>,
    attempt_linkage: Option<&AutonomousWorkflowLinkageRecord>,
) -> Result<(), CommandError> {
    match (unit_linkage, attempt_linkage) {
        (None, None) | (Some(_), None) => Ok(()),
        (None, Some(_)) => Err(CommandError::system_fault(
            "autonomous_run_request_invalid",
            "Cadence requires autonomous attempts to omit workflow linkage until the parent unit carries durable workflow linkage.",
        )),
        (Some(unit_linkage), Some(attempt_linkage)) if unit_linkage == attempt_linkage => Ok(()),
        (Some(_), Some(_)) => Err(CommandError::system_fault(
            "autonomous_run_request_invalid",
            "Cadence requires autonomous attempt workflow linkage to match the owning unit linkage exactly.",
        )),
    }
}

pub(crate) fn validate_autonomous_workflow_linkage_record(
    connection: &Connection,
    database_path: &Path,
    project_id: &str,
    linkage: &AutonomousWorkflowLinkageRecord,
    owner_kind: &str,
    owner_id: &str,
    error_code: &'static str,
) -> Result<(), CommandError> {
    let transition_event = read_transition_event_by_transition_id(
        connection,
        database_path,
        project_id,
        &linkage.transition_id,
    )?
    .ok_or_else(|| {
        autonomous_workflow_linkage_error(
            error_code,
            database_path,
            format!(
                "Autonomous {owner_kind} `{owner_id}` references workflow transition `{}` that is missing for project `{project_id}`.",
                linkage.transition_id
            ),
        )
    })?;

    if transition_event.to_node_id != linkage.workflow_node_id {
        return Err(autonomous_workflow_linkage_error(
            error_code,
            database_path,
            format!(
                "Autonomous {owner_kind} `{owner_id}` workflow node `{}` does not match transition `{}` destination node `{}`.",
                linkage.workflow_node_id, linkage.transition_id, transition_event.to_node_id
            ),
        ));
    }

    if transition_event.causal_transition_id != linkage.causal_transition_id {
        return Err(autonomous_workflow_linkage_error(
            error_code,
            database_path,
            format!(
                "Autonomous {owner_kind} `{owner_id}` causal transition linkage {:?} does not match durable transition `{}` causal linkage {:?}.",
                linkage.causal_transition_id,
                linkage.transition_id,
                transition_event.causal_transition_id
            ),
        ));
    }

    let handoff_package = read_workflow_handoff_package_by_transition_id(
        connection,
        database_path,
        project_id,
        &linkage.handoff_transition_id,
    )?
    .ok_or_else(|| {
        autonomous_workflow_linkage_error(
            error_code,
            database_path,
            format!(
                "Autonomous {owner_kind} `{owner_id}` references workflow handoff `{}` that is missing for project `{project_id}`.",
                linkage.handoff_transition_id
            ),
        )
    })?;

    validate_workflow_handoff_package_transition_linkage(&handoff_package, &transition_event)
        .map_err(|error| {
            autonomous_workflow_linkage_error(error_code, database_path, error.message)
        })?;

    if handoff_package.package_hash != linkage.handoff_package_hash {
        return Err(autonomous_workflow_linkage_error(
            error_code,
            database_path,
            format!(
                "Autonomous {owner_kind} `{owner_id}` handoff package hash `{}` does not match durable package hash `{}` for transition `{}`.",
                linkage.handoff_package_hash,
                handoff_package.package_hash,
                linkage.handoff_transition_id
            ),
        ));
    }

    Ok(())
}

pub(crate) fn autonomous_workflow_linkage_error(
    error_code: &'static str,
    database_path: &Path,
    message: String,
) -> CommandError {
    if error_code == "runtime_run_decode_failed" {
        return map_runtime_run_decode_error(database_path, message);
    }

    CommandError::system_fault(error_code, message)
}

pub(crate) fn normalize_autonomous_unit_artifact_record(
    artifact: &AutonomousUnitArtifactRecord,
    run: &AutonomousRunRecord,
    unit: &AutonomousUnitRecord,
    attempt: Option<&AutonomousUnitAttemptRecord>,
) -> Result<AutonomousUnitArtifactRecord, CommandError> {
    validate_non_empty_text(
        &artifact.artifact_id,
        "artifact_id",
        "autonomous_run_request_invalid",
    )?;
    validate_non_empty_text(
        &artifact.artifact_kind,
        "artifact_kind",
        "autonomous_run_request_invalid",
    )?;
    validate_non_empty_text(
        &artifact.summary,
        "artifact_summary",
        "autonomous_run_request_invalid",
    )?;
    validate_non_empty_text(
        &artifact.created_at,
        "artifact_created_at",
        "autonomous_run_request_invalid",
    )?;
    validate_non_empty_text(
        &artifact.updated_at,
        "artifact_updated_at",
        "autonomous_run_request_invalid",
    )?;

    if artifact.project_id != run.project_id
        || artifact.run_id != run.run_id
        || artifact.unit_id != unit.unit_id
    {
        return Err(CommandError::system_fault(
            "autonomous_run_request_invalid",
            "Cadence requires autonomous artifacts to share the parent run and unit linkage.",
        ));
    }
    if attempt.is_some_and(|attempt| artifact.attempt_id != attempt.attempt_id) {
        return Err(CommandError::system_fault(
            "autonomous_run_request_invalid",
            "Cadence requires autonomous artifacts to link to the persisted attempt id.",
        ));
    }
    if let Some(secret_hint) = find_prohibited_runtime_persistence_content(&artifact.summary) {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            format!(
                "Autonomous artifact summaries must not include {secret_hint}. Remove secret-bearing content before retrying."
            ),
        ));
    }

    let canonical_payload = artifact
        .payload
        .as_ref()
        .map(|payload| {
            validate_autonomous_artifact_payload(
                payload,
                &artifact.project_id,
                &artifact.run_id,
                &artifact.unit_id,
                &artifact.attempt_id,
                &artifact.artifact_id,
                &artifact.artifact_kind,
            )?;
            canonicalize_autonomous_artifact_payload_json(payload)
        })
        .transpose()?;

    if artifact.payload.is_none()
        && autonomous_artifact_kind_requires_payload(&artifact.artifact_kind)
    {
        let message = format!(
            "Cadence requires `{}` autonomous artifacts to persist a structured payload.",
            artifact.artifact_kind
        );
        return if artifact.artifact_kind == AUTONOMOUS_ARTIFACT_KIND_POLICY_DENIED {
            Err(CommandError::policy_denied(message))
        } else {
            Err(CommandError::user_fixable(
                "autonomous_run_request_invalid",
                message,
            ))
        };
    }

    let normalized_hash = match canonical_payload.as_deref() {
        Some(payload_json) => {
            let expected_hash = compute_workflow_handoff_package_hash(payload_json);
            if let Some(content_hash) = artifact.content_hash.as_deref() {
                validate_non_empty_text(
                    content_hash,
                    "artifact_content_hash",
                    "autonomous_run_request_invalid",
                )?;
                if content_hash.len() != 64
                    || content_hash
                        .chars()
                        .any(|ch| !ch.is_ascii_hexdigit() || ch.is_ascii_uppercase())
                {
                    return Err(CommandError::user_fixable(
                        "autonomous_run_request_invalid",
                        "Cadence requires autonomous artifact content hashes to be lowercase 64-character hex digests.",
                    ));
                }
                if content_hash != expected_hash {
                    return Err(CommandError::user_fixable(
                        "autonomous_run_request_invalid",
                        "Cadence requires autonomous artifact content_hash values to match the canonical structured payload.",
                    ));
                }
            }
            Some(expected_hash)
        }
        None => {
            if let Some(content_hash) = artifact.content_hash.as_deref() {
                validate_non_empty_text(
                    content_hash,
                    "artifact_content_hash",
                    "autonomous_run_request_invalid",
                )?;
                if content_hash.len() != 64
                    || content_hash
                        .chars()
                        .any(|ch| !ch.is_ascii_hexdigit() || ch.is_ascii_uppercase())
                {
                    return Err(CommandError::user_fixable(
                        "autonomous_run_request_invalid",
                        "Cadence requires autonomous artifact content hashes to be lowercase 64-character hex digests.",
                    ));
                }
            }
            artifact.content_hash.clone()
        }
    };

    Ok(AutonomousUnitArtifactRecord {
        content_hash: normalized_hash,
        ..artifact.clone()
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn decode_autonomous_artifact_payload_json(
    payload_json: &str,
    project_id: &str,
    run_id: &str,
    unit_id: &str,
    attempt_id: &str,
    artifact_id: &str,
    artifact_kind: &str,
    database_path: &Path,
) -> Result<AutonomousArtifactPayloadRecord, CommandError> {
    let parsed =
        serde_json::from_str::<AutonomousArtifactPayloadRecord>(payload_json).map_err(|error| {
            map_runtime_run_decode_error(
                database_path,
                format!(
                    "Autonomous artifact `{artifact_id}` stored malformed payload_json: {error}"
                ),
            )
        })?;

    validate_autonomous_artifact_payload(
        &parsed,
        project_id,
        run_id,
        unit_id,
        attempt_id,
        artifact_id,
        artifact_kind,
    )
    .map_err(|error| map_runtime_run_decode_error(database_path, error.message))?;

    Ok(parsed)
}

pub(crate) fn canonicalize_autonomous_artifact_payload_json(
    payload: &AutonomousArtifactPayloadRecord,
) -> Result<String, CommandError> {
    let value = serde_json::to_value(payload).map_err(|error| {
        CommandError::system_fault(
            "autonomous_run_request_invalid",
            format!(
                "Cadence could not serialize the autonomous artifact payload to canonical JSON: {error}"
            ),
        )
    })?;

    let canonical = canonicalize_json_value(value);
    serde_json::to_string(&canonical).map_err(|error| {
        CommandError::system_fault(
            "autonomous_run_request_invalid",
            format!("Cadence could not canonicalize the autonomous artifact payload JSON: {error}"),
        )
    })
}

pub(crate) fn validate_autonomous_artifact_payload(
    payload: &AutonomousArtifactPayloadRecord,
    project_id: &str,
    run_id: &str,
    unit_id: &str,
    attempt_id: &str,
    artifact_id: &str,
    artifact_kind: &str,
) -> Result<(), CommandError> {
    let expected_kind = autonomous_artifact_payload_kind(payload);
    if artifact_kind != expected_kind {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            format!(
                "Cadence requires autonomous artifact kind `{artifact_kind}` to match payload kind `{expected_kind}`."
            ),
        ));
    }

    match payload {
        AutonomousArtifactPayloadRecord::ToolResult(tool) => {
            validate_autonomous_artifact_payload_linkage(
                &tool.project_id,
                &tool.run_id,
                &tool.unit_id,
                &tool.attempt_id,
                &tool.artifact_id,
                project_id,
                run_id,
                unit_id,
                attempt_id,
                artifact_id,
            )?;
            validate_non_empty_text(
                &tool.tool_call_id,
                "tool_call_id",
                "autonomous_run_request_invalid",
            )?;
            validate_non_empty_text(
                &tool.tool_name,
                "tool_name",
                "autonomous_run_request_invalid",
            )?;
            validate_autonomous_artifact_text(&tool.tool_name, "tool_name")?;
            validate_autonomous_artifact_action_boundary_linkage(
                tool.action_id.as_deref(),
                tool.boundary_id.as_deref(),
            )?;
            if let Some(command_result) = tool.command_result.as_ref() {
                validate_autonomous_artifact_command_result(command_result)?;
            }
            validate_autonomous_tool_result_summary(
                &tool.tool_state,
                tool.command_result.as_ref(),
                tool.tool_summary.as_ref(),
            )?;
        }
        AutonomousArtifactPayloadRecord::VerificationEvidence(evidence) => {
            validate_autonomous_artifact_payload_linkage(
                &evidence.project_id,
                &evidence.run_id,
                &evidence.unit_id,
                &evidence.attempt_id,
                &evidence.artifact_id,
                project_id,
                run_id,
                unit_id,
                attempt_id,
                artifact_id,
            )?;
            validate_non_empty_text(
                &evidence.evidence_kind,
                "evidence_kind",
                "autonomous_run_request_invalid",
            )?;
            validate_non_empty_text(
                &evidence.label,
                "evidence_label",
                "autonomous_run_request_invalid",
            )?;
            validate_autonomous_artifact_text(&evidence.evidence_kind, "evidence_kind")?;
            validate_autonomous_artifact_text(&evidence.label, "evidence_label")?;
            validate_autonomous_artifact_action_boundary_linkage(
                evidence.action_id.as_deref(),
                evidence.boundary_id.as_deref(),
            )?;
            if let Some(command_result) = evidence.command_result.as_ref() {
                validate_autonomous_artifact_command_result(command_result)?;
            }
        }
        AutonomousArtifactPayloadRecord::PolicyDenied(policy) => {
            validate_autonomous_artifact_payload_linkage(
                &policy.project_id,
                &policy.run_id,
                &policy.unit_id,
                &policy.attempt_id,
                &policy.artifact_id,
                project_id,
                run_id,
                unit_id,
                attempt_id,
                artifact_id,
            )?;
            if policy.diagnostic_code.trim().is_empty() {
                return Err(CommandError::policy_denied(
                    "Cadence requires policy_denied artifacts to include a stable diagnostic_code.",
                ));
            }
            validate_non_empty_text(
                &policy.diagnostic_code,
                "policy_denied_code",
                "autonomous_run_request_invalid",
            )?;
            validate_non_empty_text(
                &policy.message,
                "policy_denied_message",
                "autonomous_run_request_invalid",
            )?;
            validate_autonomous_artifact_text(&policy.message, "policy_denied_message")?;
            if let Some(tool_name) = policy.tool_name.as_deref() {
                validate_non_empty_text(
                    tool_name,
                    "policy_denied_tool_name",
                    "autonomous_run_request_invalid",
                )?;
                validate_autonomous_artifact_text(tool_name, "policy_denied_tool_name")?;
            }
            validate_autonomous_artifact_action_boundary_linkage(
                policy.action_id.as_deref(),
                policy.boundary_id.as_deref(),
            )?;
        }
        AutonomousArtifactPayloadRecord::SkillLifecycle(skill) => {
            validate_autonomous_artifact_payload_linkage(
                &skill.project_id,
                &skill.run_id,
                &skill.unit_id,
                &skill.attempt_id,
                &skill.artifact_id,
                project_id,
                run_id,
                unit_id,
                attempt_id,
                artifact_id,
            )?;
            validate_autonomous_skill_lifecycle_payload(skill)?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_autonomous_artifact_payload_linkage(
    payload_project_id: &str,
    payload_run_id: &str,
    payload_unit_id: &str,
    payload_attempt_id: &str,
    payload_artifact_id: &str,
    project_id: &str,
    run_id: &str,
    unit_id: &str,
    attempt_id: &str,
    artifact_id: &str,
) -> Result<(), CommandError> {
    for (value, field) in [
        (payload_project_id, "payload_project_id"),
        (payload_run_id, "payload_run_id"),
        (payload_unit_id, "payload_unit_id"),
        (payload_attempt_id, "payload_attempt_id"),
        (payload_artifact_id, "payload_artifact_id"),
    ] {
        validate_non_empty_text(value, field, "autonomous_run_request_invalid")?;
    }

    if payload_project_id != project_id
        || payload_run_id != run_id
        || payload_unit_id != unit_id
        || payload_attempt_id != attempt_id
        || payload_artifact_id != artifact_id
    {
        return Err(CommandError::system_fault(
            "autonomous_run_request_invalid",
            "Cadence requires autonomous artifact payload linkage to match the owning project/run/unit/attempt/artifact row.",
        ));
    }

    Ok(())
}

pub(crate) fn validate_autonomous_artifact_action_boundary_linkage(
    action_id: Option<&str>,
    boundary_id: Option<&str>,
) -> Result<(), CommandError> {
    match (action_id, boundary_id) {
        (Some(action_id), Some(boundary_id)) => {
            validate_non_empty_text(
                action_id,
                "artifact_action_id",
                "autonomous_run_request_invalid",
            )?;
            validate_non_empty_text(
                boundary_id,
                "artifact_boundary_id",
                "autonomous_run_request_invalid",
            )?;
            validate_runtime_action_boundary_identity(action_id.trim(), boundary_id.trim())
        }
        (None, None) => Ok(()),
        _ => Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            "Cadence requires autonomous artifact action_id and boundary_id to be provided together.",
        )),
    }
}

pub(crate) fn validate_runtime_action_boundary_identity(
    action_id: &str,
    boundary_id: &str,
) -> Result<(), CommandError> {
    if action_id.chars().any(char::is_whitespace) {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            "Cadence requires boundary-linked autonomous artifacts to persist canonical action_id values without whitespace.",
        ));
    }

    if boundary_id.contains(':') || boundary_id.chars().any(char::is_whitespace) {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            "Cadence requires boundary-linked autonomous artifacts to persist canonical boundary_id values.",
        ));
    }

    let run_marker = ":run:";
    let Some(run_start) = action_id.find(run_marker) else {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            "Cadence requires boundary-linked autonomous artifacts to use runtime-scoped canonical action_id values.",
        ));
    };
    if run_start == 0 {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            "Cadence requires boundary-linked autonomous artifacts to include a stable action scope prefix.",
        ));
    }

    let boundary_marker = format!(":boundary:{boundary_id}:");
    let Some(boundary_start) = action_id.find(boundary_marker.as_str()) else {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            "Cadence requires boundary-linked autonomous artifacts to keep action_id and boundary_id in canonical agreement.",
        ));
    };

    let run_id = &action_id[(run_start + run_marker.len())..boundary_start];
    if run_id.is_empty() || run_id.contains(':') || run_id.chars().any(char::is_whitespace) {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            "Cadence requires boundary-linked autonomous artifacts to persist a canonical run-scoped action_id.",
        ));
    }

    let action_type = &action_id[(boundary_start + boundary_marker.len())..];
    if action_type.is_empty()
        || action_type.contains(':')
        || action_type.chars().any(char::is_whitespace)
    {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            "Cadence requires boundary-linked autonomous artifacts to persist an action_id with a canonical action type suffix.",
        ));
    }

    Ok(())
}

pub(crate) fn validate_autonomous_artifact_command_result(
    command_result: &AutonomousArtifactCommandResultRecord,
) -> Result<(), CommandError> {
    validate_non_empty_text(
        &command_result.summary,
        "artifact_command_summary",
        "autonomous_run_request_invalid",
    )?;
    validate_autonomous_artifact_text(&command_result.summary, "artifact_command_summary")
}

pub(crate) fn validate_autonomous_tool_result_summary(
    tool_state: &AutonomousToolCallStateRecord,
    command_result: Option<&AutonomousArtifactCommandResultRecord>,
    tool_summary: Option<&ToolResultSummary>,
) -> Result<(), CommandError> {
    if let Some(command_result) = command_result {
        if matches!(
            tool_state,
            AutonomousToolCallStateRecord::Pending | AutonomousToolCallStateRecord::Running
        ) {
            return Err(CommandError::user_fixable(
                "autonomous_run_request_invalid",
                "Cadence only persists command_result metadata after a tool reaches a terminal state.",
            ));
        }
        if matches!(tool_state, AutonomousToolCallStateRecord::Failed)
            && command_result.exit_code == Some(0)
            && !command_result.timed_out
        {
            return Err(CommandError::user_fixable(
                "autonomous_run_request_invalid",
                "Cadence rejected a failed tool_result payload whose command_result reported a successful exit code.",
            ));
        }
    }

    let Some(tool_summary) = tool_summary else {
        return Ok(());
    };

    match tool_summary {
        ToolResultSummary::Command(summary) => {
            if matches!(
                tool_state,
                AutonomousToolCallStateRecord::Pending | AutonomousToolCallStateRecord::Running
            ) {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence only persists command tool_summary metadata after a tool reaches a terminal state.",
                ));
            }
            let Some(command_result) = command_result else {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence requires command tool_summary metadata to include the paired command_result payload.",
                ));
            };
            if summary.exit_code != command_result.exit_code
                || summary.timed_out != command_result.timed_out
            {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence requires command tool_summary exit metadata to match the paired command_result payload.",
                ));
            }
            if matches!(tool_state, AutonomousToolCallStateRecord::Failed)
                && summary.exit_code == Some(0)
                && !summary.timed_out
            {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence rejected a failed tool_result payload whose command tool_summary reported a successful exit code.",
                ));
            }
        }
        ToolResultSummary::File(summary) => {
            if matches!(tool_state, AutonomousToolCallStateRecord::Failed) {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence does not persist file tool_summary metadata for failed tool results.",
                ));
            }
            if command_result.is_some() {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence requires file tool_summary metadata to omit command_result payloads.",
                ));
            }
            if summary.path.is_none() && summary.scope.is_none() {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence requires file tool_summary metadata to include a bounded path or scope.",
                ));
            }
            if let Some(path) = summary.path.as_deref() {
                validate_non_empty_text(
                    path,
                    "tool_summary_file_path",
                    "autonomous_run_request_invalid",
                )?;
                validate_autonomous_artifact_text(path, "tool_summary_file_path")?;
            }
            if let Some(scope) = summary.scope.as_deref() {
                validate_non_empty_text(
                    scope,
                    "tool_summary_file_scope",
                    "autonomous_run_request_invalid",
                )?;
                validate_autonomous_artifact_text(scope, "tool_summary_file_scope")?;
            }
        }
        ToolResultSummary::Git(summary) => {
            if matches!(tool_state, AutonomousToolCallStateRecord::Failed) {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence does not persist git tool_summary metadata for failed tool results.",
                ));
            }
            if command_result.is_some() {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence requires git tool_summary metadata to omit command_result payloads.",
                ));
            }
            if let Some(base_revision) = summary.base_revision.as_deref() {
                validate_non_empty_text(
                    base_revision,
                    "tool_summary_git_base_revision",
                    "autonomous_run_request_invalid",
                )?;
                validate_autonomous_artifact_text(base_revision, "tool_summary_git_base_revision")?;
            }
            if let Some(scope) = summary.scope.as_ref() {
                match scope {
                    GitToolResultScope::Staged
                    | GitToolResultScope::Unstaged
                    | GitToolResultScope::Worktree => {}
                }
            }
        }
        ToolResultSummary::Web(summary) => {
            if matches!(tool_state, AutonomousToolCallStateRecord::Failed) {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence does not persist web tool_summary metadata for failed tool results.",
                ));
            }
            if command_result.is_some() {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence requires web tool_summary metadata to omit command_result payloads.",
                ));
            }
            validate_non_empty_text(
                &summary.target,
                "tool_summary_web_target",
                "autonomous_run_request_invalid",
            )?;
            validate_autonomous_artifact_text(&summary.target, "tool_summary_web_target")?;
            if let Some(final_url) = summary.final_url.as_deref() {
                validate_non_empty_text(
                    final_url,
                    "tool_summary_web_final_url",
                    "autonomous_run_request_invalid",
                )?;
                validate_autonomous_artifact_text(final_url, "tool_summary_web_final_url")?;
            }
            if let Some(content_type) = summary.content_type.as_deref() {
                validate_non_empty_text(
                    content_type,
                    "tool_summary_web_content_type",
                    "autonomous_run_request_invalid",
                )?;
                validate_autonomous_artifact_text(content_type, "tool_summary_web_content_type")?;
            }
        }
        ToolResultSummary::BrowserComputerUse(summary) => {
            if command_result.is_some() {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence requires browser/computer-use tool_summary metadata to omit command_result payloads.",
                ));
            }

            validate_non_empty_text(
                &summary.action,
                "tool_summary_browser_computer_use_action",
                "autonomous_run_request_invalid",
            )?;
            validate_bounded_autonomous_artifact_text(
                &summary.action,
                "tool_summary_browser_computer_use_action",
                MAX_BROWSER_COMPUTER_USE_SUMMARY_TEXT_CHARS,
            )?;

            if let Some(target) = summary.target.as_deref() {
                validate_non_empty_text(
                    target,
                    "tool_summary_browser_computer_use_target",
                    "autonomous_run_request_invalid",
                )?;
                validate_bounded_autonomous_artifact_text(
                    target,
                    "tool_summary_browser_computer_use_target",
                    MAX_BROWSER_COMPUTER_USE_SUMMARY_TEXT_CHARS,
                )?;
            }

            if let Some(outcome) = summary.outcome.as_deref() {
                validate_non_empty_text(
                    outcome,
                    "tool_summary_browser_computer_use_outcome",
                    "autonomous_run_request_invalid",
                )?;
                validate_bounded_autonomous_artifact_text(
                    outcome,
                    "tool_summary_browser_computer_use_outcome",
                    MAX_BROWSER_COMPUTER_USE_SUMMARY_TEXT_CHARS,
                )?;
            }

            match summary.surface {
                BrowserComputerUseSurface::Browser | BrowserComputerUseSurface::ComputerUse => {}
            }

            match summary.status {
                BrowserComputerUseActionStatus::Pending
                | BrowserComputerUseActionStatus::Running
                | BrowserComputerUseActionStatus::Succeeded
                | BrowserComputerUseActionStatus::Failed
                | BrowserComputerUseActionStatus::Blocked => {}
            }

            validate_browser_computer_use_status_for_tool_state(tool_state, &summary.status)?;
        }
        ToolResultSummary::McpCapability(summary) => {
            if command_result.is_some() {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence requires MCP capability tool_summary metadata to omit command_result payloads.",
                ));
            }
            validate_non_empty_text(
                &summary.server_id,
                "tool_summary_mcp_server_id",
                "autonomous_run_request_invalid",
            )?;
            validate_non_empty_text(
                &summary.capability_id,
                "tool_summary_mcp_capability_id",
                "autonomous_run_request_invalid",
            )?;
            validate_autonomous_artifact_text(&summary.server_id, "tool_summary_mcp_server_id")?;
            validate_autonomous_artifact_text(
                &summary.capability_id,
                "tool_summary_mcp_capability_id",
            )?;
            if let Some(capability_name) = summary.capability_name.as_deref() {
                validate_non_empty_text(
                    capability_name,
                    "tool_summary_mcp_capability_name",
                    "autonomous_run_request_invalid",
                )?;
                validate_autonomous_artifact_text(
                    capability_name,
                    "tool_summary_mcp_capability_name",
                )?;
            }
            match summary.capability_kind {
                McpCapabilityKind::Tool
                | McpCapabilityKind::Resource
                | McpCapabilityKind::Prompt
                | McpCapabilityKind::Command => {}
            }
        }
    }

    Ok(())
}

pub(crate) fn validate_autonomous_skill_lifecycle_payload(
    skill: &AutonomousSkillLifecyclePayloadRecord,
) -> Result<(), CommandError> {
    validate_autonomous_skill_lifecycle_skill_id(&skill.skill_id)?;
    validate_autonomous_skill_lifecycle_source(&skill.source)?;

    validate_non_empty_text(
        &skill.cache.key,
        "skill_lifecycle_cache_key",
        "autonomous_run_request_invalid",
    )?;
    validate_autonomous_artifact_text(&skill.cache.key, "skill_lifecycle_cache_key")?;

    match skill.stage {
        AutonomousSkillLifecycleStageRecord::Discovery => {
            if skill.cache.status.is_some() {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence discovery skill_lifecycle payloads must omit cache status because no install or invoke step has completed yet.",
                ));
            }
        }
        AutonomousSkillLifecycleStageRecord::Install
        | AutonomousSkillLifecycleStageRecord::Invoke => {
            if matches!(
                skill.result,
                AutonomousSkillLifecycleResultRecord::Succeeded
            ) && skill.cache.status.is_none()
            {
                return Err(CommandError::user_fixable(
                    "autonomous_run_request_invalid",
                    "Cadence successful install/invoke skill_lifecycle payloads must include cache status.",
                ));
            }
        }
    }

    match (&skill.result, skill.diagnostic.as_ref()) {
        (AutonomousSkillLifecycleResultRecord::Succeeded, Some(_)) => {
            return Err(CommandError::user_fixable(
                "autonomous_run_request_invalid",
                "Cadence rejected a successful skill_lifecycle payload that also reported failure diagnostics.",
            ));
        }
        (AutonomousSkillLifecycleResultRecord::Failed, None) => {
            return Err(CommandError::user_fixable(
                "autonomous_run_request_invalid",
                "Cadence failed skill_lifecycle payloads require typed diagnostics.",
            ));
        }
        (AutonomousSkillLifecycleResultRecord::Failed, Some(diagnostic)) => {
            validate_non_empty_text(
                &diagnostic.code,
                "skill_lifecycle_diagnostic_code",
                "autonomous_run_request_invalid",
            )?;
            validate_non_empty_text(
                &diagnostic.message,
                "skill_lifecycle_diagnostic_message",
                "autonomous_run_request_invalid",
            )?;
            validate_autonomous_artifact_text(
                &diagnostic.message,
                "skill_lifecycle_diagnostic_message",
            )?;
        }
        (AutonomousSkillLifecycleResultRecord::Succeeded, None) => {}
    }

    Ok(())
}

pub(crate) fn validate_autonomous_skill_lifecycle_skill_id(
    skill_id: &str,
) -> Result<(), CommandError> {
    validate_non_empty_text(
        skill_id,
        "skill_lifecycle_skill_id",
        "autonomous_run_request_invalid",
    )?;
    validate_autonomous_artifact_text(skill_id, "skill_lifecycle_skill_id")?;
    if !skill_id.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    }) {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            "Cadence requires skill_lifecycle skill ids to stay lowercase kebab-case values.",
        ));
    }

    Ok(())
}

pub(crate) fn validate_autonomous_skill_lifecycle_source(
    source: &AutonomousSkillLifecycleSourceRecord,
) -> Result<(), CommandError> {
    for (value, field) in [
        (&source.repo, "skill_lifecycle_source_repo"),
        (&source.path, "skill_lifecycle_source_path"),
        (&source.reference, "skill_lifecycle_source_reference"),
        (&source.tree_hash, "skill_lifecycle_source_tree_hash"),
    ] {
        validate_non_empty_text(value, field, "autonomous_run_request_invalid")?;
        validate_autonomous_artifact_text(value, field)?;
    }

    if source.tree_hash.len() != 40
        || source
            .tree_hash
            .chars()
            .any(|ch| !ch.is_ascii_hexdigit() || ch.is_ascii_uppercase())
    {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            "Cadence requires skill_lifecycle source tree_hash values to be lowercase 40-character hexadecimal Git tree hashes.",
        ));
    }

    Ok(())
}

pub(crate) fn validate_autonomous_artifact_text(
    value: &str,
    field: &str,
) -> Result<(), CommandError> {
    if let Some(secret_hint) = find_prohibited_runtime_persistence_content(value) {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            format!(
                "Autonomous artifact field `{field}` must not include {secret_hint}. Remove secret-bearing content before retrying."
            ),
        ));
    }

    Ok(())
}

pub(crate) fn validate_bounded_autonomous_artifact_text(
    value: &str,
    field: &str,
    max_chars: usize,
) -> Result<(), CommandError> {
    validate_autonomous_artifact_text(value, field)?;
    if value.chars().count() > max_chars {
        return Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            format!(
                "Autonomous artifact field `{field}` must be <= {max_chars} characters after sanitization."
            ),
        ));
    }
    Ok(())
}

pub(crate) fn validate_browser_computer_use_status_for_tool_state(
    tool_state: &AutonomousToolCallStateRecord,
    status: &BrowserComputerUseActionStatus,
) -> Result<(), CommandError> {
    let allowed = match tool_state {
        AutonomousToolCallStateRecord::Pending => {
            matches!(status, BrowserComputerUseActionStatus::Pending)
        }
        AutonomousToolCallStateRecord::Running => matches!(
            status,
            BrowserComputerUseActionStatus::Pending | BrowserComputerUseActionStatus::Running
        ),
        AutonomousToolCallStateRecord::Succeeded => {
            matches!(status, BrowserComputerUseActionStatus::Succeeded)
        }
        AutonomousToolCallStateRecord::Failed => matches!(
            status,
            BrowserComputerUseActionStatus::Failed | BrowserComputerUseActionStatus::Blocked
        ),
    };

    if allowed {
        Ok(())
    } else {
        Err(CommandError::user_fixable(
            "autonomous_run_request_invalid",
            "Cadence rejected browser/computer-use tool_summary metadata whose status does not match the tool_state lifecycle.",
        ))
    }
}

pub(crate) fn autonomous_artifact_payload_kind(
    payload: &AutonomousArtifactPayloadRecord,
) -> &'static str {
    match payload {
        AutonomousArtifactPayloadRecord::ToolResult(_) => AUTONOMOUS_ARTIFACT_KIND_TOOL_RESULT,
        AutonomousArtifactPayloadRecord::VerificationEvidence(_) => {
            AUTONOMOUS_ARTIFACT_KIND_VERIFICATION_EVIDENCE
        }
        AutonomousArtifactPayloadRecord::PolicyDenied(_) => AUTONOMOUS_ARTIFACT_KIND_POLICY_DENIED,
        AutonomousArtifactPayloadRecord::SkillLifecycle(_) => {
            AUTONOMOUS_ARTIFACT_KIND_SKILL_LIFECYCLE
        }
    }
}

pub(crate) fn autonomous_artifact_kind_requires_payload(kind: &str) -> bool {
    matches!(
        kind,
        AUTONOMOUS_ARTIFACT_KIND_TOOL_RESULT
            | AUTONOMOUS_ARTIFACT_KIND_VERIFICATION_EVIDENCE
            | AUTONOMOUS_ARTIFACT_KIND_POLICY_DENIED
            | AUTONOMOUS_ARTIFACT_KIND_SKILL_LIFECYCLE
    )
}

pub(crate) fn canonicalize_json_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut sorted = std::collections::BTreeMap::new();
            for (key, nested) in map {
                sorted.insert(key, canonicalize_json_value(nested));
            }

            serde_json::Value::Object(sorted.into_iter().collect())
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(canonicalize_json_value).collect())
        }
        other => other,
    }
}
