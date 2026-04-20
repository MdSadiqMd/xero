use tauri::ipc::Channel;

use crate::{
    commands::{
        runtime_support::{
            autonomous_skill_cache_status_dto_from_protocol,
            autonomous_skill_lifecycle_diagnostic_dto_from_protocol,
            autonomous_skill_lifecycle_result_dto_from_protocol,
            autonomous_skill_lifecycle_source_dto_from_protocol,
            autonomous_skill_lifecycle_stage_dto_from_protocol,
            tool_result_summary_dto_from_protocol,
        },
        AutonomousSkillLifecycleDiagnosticDto, AutonomousSkillLifecycleResultDto,
        AutonomousSkillLifecycleSourceDto, AutonomousSkillLifecycleStageDto, CommandError,
        RuntimeStreamItemDto, RuntimeStreamItemKind, RuntimeToolCallState,
    },
    db::project_store::{RuntimeRunSnapshotRecord, RuntimeRunStatus},
    runtime::protocol::{SupervisorLiveEventPayload, SupervisorToolCallState},
};

use super::{
    controller::{RuntimeStreamLease, RuntimeStreamRequest},
    ensure_stream_active, StreamExit, StreamFailure, StreamResult,
};

#[derive(Debug, Clone)]
pub(super) struct PendingActionRequired {
    pub(super) action_id: String,
    pub(super) boundary_id: Option<String>,
    pub(super) action_type: String,
    pub(super) title: String,
    pub(super) detail: String,
    pub(super) created_at: String,
}

pub(super) fn map_supervisor_event_to_stream_item(
    request: &RuntimeStreamRequest,
    sequence: u64,
    created_at: String,
    item: SupervisorLiveEventPayload,
) -> Result<RuntimeStreamItemDto, CommandError> {
    let item = match item {
        SupervisorLiveEventPayload::Transcript { text } => RuntimeStreamItemDto {
            kind: RuntimeStreamItemKind::Transcript,
            run_id: request.run_id.clone(),
            sequence,
            session_id: Some(request.session_id.clone()),
            flow_id: request.flow_id.clone(),
            text: Some(text),
            tool_call_id: None,
            tool_name: None,
            tool_state: None,
            tool_summary: None,
            skill_id: None,
            skill_stage: None,
            skill_result: None,
            skill_source: None,
            skill_cache_status: None,
            skill_diagnostic: None,
            action_id: None,
            boundary_id: None,
            action_type: None,
            title: None,
            detail: None,
            code: None,
            message: None,
            retryable: None,
            created_at,
        },
        SupervisorLiveEventPayload::Tool {
            tool_call_id,
            tool_name,
            tool_state,
            detail,
            tool_summary,
        } => RuntimeStreamItemDto {
            kind: RuntimeStreamItemKind::Tool,
            run_id: request.run_id.clone(),
            sequence,
            session_id: Some(request.session_id.clone()),
            flow_id: request.flow_id.clone(),
            text: None,
            tool_call_id: Some(tool_call_id),
            tool_name: Some(tool_name),
            tool_state: Some(map_supervisor_tool_state(tool_state)),
            tool_summary: tool_summary
                .as_ref()
                .map(tool_result_summary_dto_from_protocol),
            skill_id: None,
            skill_stage: None,
            skill_result: None,
            skill_source: None,
            skill_cache_status: None,
            skill_diagnostic: None,
            action_id: None,
            boundary_id: None,
            action_type: None,
            title: None,
            detail,
            code: None,
            message: None,
            retryable: None,
            created_at,
        },
        SupervisorLiveEventPayload::Skill {
            skill_id,
            stage,
            result,
            detail,
            source,
            cache_status,
            diagnostic,
        } => RuntimeStreamItemDto {
            kind: RuntimeStreamItemKind::Skill,
            run_id: request.run_id.clone(),
            sequence,
            session_id: Some(request.session_id.clone()),
            flow_id: request.flow_id.clone(),
            text: None,
            tool_call_id: None,
            tool_name: None,
            tool_state: None,
            tool_summary: None,
            skill_id: Some(skill_id),
            skill_stage: Some(autonomous_skill_lifecycle_stage_dto_from_protocol(stage)),
            skill_result: Some(autonomous_skill_lifecycle_result_dto_from_protocol(result)),
            skill_source: Some(autonomous_skill_lifecycle_source_dto_from_protocol(&source)),
            skill_cache_status: cache_status.map(autonomous_skill_cache_status_dto_from_protocol),
            skill_diagnostic: diagnostic
                .as_ref()
                .map(autonomous_skill_lifecycle_diagnostic_dto_from_protocol),
            action_id: None,
            boundary_id: None,
            action_type: None,
            title: None,
            detail: Some(detail),
            code: None,
            message: None,
            retryable: None,
            created_at,
        },
        SupervisorLiveEventPayload::Activity {
            code,
            title,
            detail,
        } => RuntimeStreamItemDto {
            kind: RuntimeStreamItemKind::Activity,
            run_id: request.run_id.clone(),
            sequence,
            session_id: Some(request.session_id.clone()),
            flow_id: request.flow_id.clone(),
            text: None,
            tool_call_id: None,
            tool_name: None,
            tool_state: None,
            tool_summary: None,
            skill_id: None,
            skill_stage: None,
            skill_result: None,
            skill_source: None,
            skill_cache_status: None,
            skill_diagnostic: None,
            action_id: None,
            boundary_id: None,
            action_type: None,
            title: Some(title),
            detail,
            code: Some(code),
            message: None,
            retryable: None,
            created_at,
        },
        SupervisorLiveEventPayload::ActionRequired {
            action_id,
            boundary_id,
            action_type,
            title,
            detail,
        } => RuntimeStreamItemDto {
            kind: RuntimeStreamItemKind::ActionRequired,
            run_id: request.run_id.clone(),
            sequence,
            session_id: Some(request.session_id.clone()),
            flow_id: request.flow_id.clone(),
            text: None,
            tool_call_id: None,
            tool_name: None,
            tool_state: None,
            tool_summary: None,
            skill_id: None,
            skill_stage: None,
            skill_result: None,
            skill_source: None,
            skill_cache_status: None,
            skill_diagnostic: None,
            action_id: Some(action_id),
            boundary_id: Some(boundary_id),
            action_type: Some(action_type),
            title: Some(title),
            detail: Some(detail),
            code: None,
            message: None,
            retryable: None,
            created_at,
        },
    };

    validate_stream_item(&item)?;
    Ok(item)
}

pub(super) fn emit_terminal_item(
    channel: &Channel<RuntimeStreamItemDto>,
    request: &RuntimeStreamRequest,
    lease: &RuntimeStreamLease,
    snapshot: &RuntimeRunSnapshotRecord,
    last_sequence: u64,
) -> StreamResult<u64> {
    let next_sequence = last_sequence.saturating_add(1);

    match snapshot.run.status {
        RuntimeRunStatus::Stopped => {
            emit_item_if_requested(
                channel,
                request,
                lease,
                complete_item(
                    request,
                    next_sequence,
                    format!(
                        "Detached runtime run `{}` finished and closed the live stream.",
                        snapshot.run.run_id
                    ),
                ),
            )?;
            Ok(next_sequence)
        }
        RuntimeRunStatus::Failed => {
            let error = snapshot.run.last_error.as_ref().map_or_else(
                || {
                    CommandError::user_fixable(
                        "runtime_stream_run_failed",
                        format!(
                            "Cadence marked detached runtime run `{}` as failed after the live stream closed.",
                            snapshot.run.run_id
                        ),
                    )
                },
                |diagnostic| CommandError::user_fixable(&diagnostic.code, &diagnostic.message),
            );
            emit_failure_item(channel, request, next_sequence, error).map_err(|error| {
                StreamExit::Failed(StreamFailure {
                    error,
                    last_sequence,
                })
            })?;
            Ok(next_sequence)
        }
        RuntimeRunStatus::Stale | RuntimeRunStatus::Starting | RuntimeRunStatus::Running => {
            let error = snapshot.run.last_error.as_ref().map_or_else(
                || {
                    CommandError::retryable(
                        "runtime_stream_run_stale",
                        format!(
                            "Cadence lost the detached supervisor attach stream for run `{}` before it reached a terminal state.",
                            snapshot.run.run_id
                        ),
                    )
                },
                |diagnostic| CommandError::retryable(&diagnostic.code, &diagnostic.message),
            );
            emit_failure_item(channel, request, next_sequence, error).map_err(|error| {
                StreamExit::Failed(StreamFailure {
                    error,
                    last_sequence,
                })
            })?;
            Ok(next_sequence)
        }
    }
}

pub(super) fn emit_item_if_requested(
    channel: &Channel<RuntimeStreamItemDto>,
    request: &RuntimeStreamRequest,
    lease: &RuntimeStreamLease,
    item: RuntimeStreamItemDto,
) -> StreamResult<()> {
    ensure_stream_active(lease)?;

    if !should_emit(&request.requested_item_kinds, &item.kind) {
        return Ok(());
    }

    validate_stream_item(&item).map_err(|error| {
        StreamExit::Failed(StreamFailure {
            last_sequence: item.sequence.saturating_sub(1),
            error,
        })
    })?;

    let sequence = item.sequence;
    channel.send(item).map_err(|error| {
        StreamExit::Failed(StreamFailure {
            last_sequence: sequence,
            error: CommandError::retryable(
                "runtime_stream_channel_closed",
                format!(
                    "Cadence could not deliver the runtime stream item because the desktop channel closed: {error}"
                ),
            ),
        })
    })
}

pub(super) fn emit_failure_item(
    channel: &Channel<RuntimeStreamItemDto>,
    request: &RuntimeStreamRequest,
    sequence: u64,
    error: CommandError,
) -> Result<(), CommandError> {
    let item = failure_item(request, sequence, error);
    validate_stream_item(&item)?;
    channel.send(item).map_err(|send_error| {
        CommandError::retryable(
            "runtime_stream_channel_closed",
            format!(
                "Cadence could not deliver the runtime failure item because the desktop channel closed: {send_error}"
            ),
        )
    })
}

pub(super) fn action_required_item(
    request: &RuntimeStreamRequest,
    sequence: u64,
    action_required: PendingActionRequired,
) -> RuntimeStreamItemDto {
    RuntimeStreamItemDto {
        kind: RuntimeStreamItemKind::ActionRequired,
        run_id: request.run_id.clone(),
        sequence,
        session_id: Some(request.session_id.clone()),
        flow_id: request.flow_id.clone(),
        text: None,
        tool_call_id: None,
        tool_name: None,
        tool_state: None,
        tool_summary: None,
        skill_id: None,
        skill_stage: None,
        skill_result: None,
        skill_source: None,
        skill_cache_status: None,
        skill_diagnostic: None,
        action_id: Some(action_required.action_id),
        boundary_id: action_required.boundary_id,
        action_type: Some(action_required.action_type),
        title: Some(action_required.title),
        detail: Some(action_required.detail),
        code: None,
        message: None,
        retryable: None,
        created_at: action_required.created_at,
    }
}

pub(super) fn require_non_empty(
    value: Option<&str>,
    field: &str,
    kind: &str,
) -> Result<(), CommandError> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        Some(_) => Ok(()),
        None => Err(CommandError::system_fault(
            "runtime_stream_item_invalid",
            format!("Cadence produced a {kind} without a non-empty `{field}` field."),
        )),
    }
}

pub(super) fn parse_runtime_boundary_id_for_run(
    action_id: &str,
    run_id: &str,
) -> Result<Option<String>, CommandError> {
    if !action_id.contains(":run:") || !action_id.contains(":boundary:") {
        return Ok(None);
    }

    let run_marker = format!(":run:{}:boundary:", run_id.trim());
    if !action_id.contains(&run_marker) {
        return Ok(None);
    }

    let Some(boundary_start) = action_id.find(&run_marker) else {
        return Err(CommandError::system_fault(
            "runtime_stream_item_invalid",
            format!(
                "Cadence could not parse runtime action-required id `{action_id}` for run `{run_id}`."
            ),
        ));
    };

    let boundary_and_action = &action_id[boundary_start + run_marker.len()..];
    let Some((boundary_id, _action_type)) = boundary_and_action.split_once(':') else {
        return Err(CommandError::system_fault(
            "runtime_stream_item_invalid",
            format!(
                "Cadence could not parse runtime boundary id from action-required id `{action_id}`."
            ),
        ));
    };

    let boundary_id = boundary_id.trim();
    if boundary_id.is_empty() {
        return Err(CommandError::system_fault(
            "runtime_stream_item_invalid",
            format!(
                "Cadence could not parse a non-empty runtime boundary id from action-required id `{action_id}`."
            ),
        ));
    }

    Ok(Some(boundary_id.to_string()))
}

fn map_supervisor_tool_state(state: SupervisorToolCallState) -> RuntimeToolCallState {
    match state {
        SupervisorToolCallState::Pending => RuntimeToolCallState::Pending,
        SupervisorToolCallState::Running => RuntimeToolCallState::Running,
        SupervisorToolCallState::Succeeded => RuntimeToolCallState::Succeeded,
        SupervisorToolCallState::Failed => RuntimeToolCallState::Failed,
    }
}

fn should_emit(
    requested_item_kinds: &[RuntimeStreamItemKind],
    kind: &RuntimeStreamItemKind,
) -> bool {
    if *kind == RuntimeStreamItemKind::Failure {
        return true;
    }

    requested_item_kinds
        .iter()
        .any(|requested| requested == kind)
}

fn validate_stream_item(item: &RuntimeStreamItemDto) -> Result<(), CommandError> {
    require_non_empty(Some(item.run_id.as_str()), "runId", "runtime stream item")?;

    if item.sequence == 0 {
        return Err(CommandError::system_fault(
            "runtime_stream_item_invalid",
            "Cadence produced a runtime stream item without a positive `sequence` value.",
        ));
    }

    match item.kind {
        RuntimeStreamItemKind::Transcript => {
            require_non_empty(item.text.as_deref(), "text", "runtime transcript item")?
        }
        RuntimeStreamItemKind::Tool => {
            require_non_empty(
                item.tool_call_id.as_deref(),
                "toolCallId",
                "runtime tool item",
            )?;
            require_non_empty(item.tool_name.as_deref(), "toolName", "runtime tool item")?;
            if item.tool_state.is_none() {
                return Err(CommandError::system_fault(
                    "runtime_stream_item_invalid",
                    "Cadence produced a runtime tool item without a tool state.",
                ));
            }
        }
        RuntimeStreamItemKind::Skill => {
            require_non_empty(item.skill_id.as_deref(), "skillId", "runtime skill item")?;
            require_non_empty(item.detail.as_deref(), "detail", "runtime skill item")?;

            let Some(stage) = item.skill_stage.as_ref() else {
                return Err(CommandError::system_fault(
                    "runtime_stream_item_invalid",
                    "Cadence produced a runtime skill item without a lifecycle stage.",
                ));
            };
            let Some(result) = item.skill_result.as_ref() else {
                return Err(CommandError::system_fault(
                    "runtime_stream_item_invalid",
                    "Cadence produced a runtime skill item without a lifecycle result.",
                ));
            };
            let Some(source) = item.skill_source.as_ref() else {
                return Err(CommandError::system_fault(
                    "runtime_stream_item_invalid",
                    "Cadence produced a runtime skill item without source metadata.",
                ));
            };
            validate_runtime_skill_source(source)?;

            match (result, item.skill_diagnostic.as_ref()) {
                (AutonomousSkillLifecycleResultDto::Succeeded, Some(_)) => {
                    return Err(CommandError::system_fault(
                        "runtime_stream_item_invalid",
                        "Cadence produced a successful runtime skill item that also included diagnostics.",
                    ));
                }
                (AutonomousSkillLifecycleResultDto::Failed, None) => {
                    return Err(CommandError::system_fault(
                        "runtime_stream_item_invalid",
                        "Cadence produced a failed runtime skill item without diagnostics.",
                    ));
                }
                (AutonomousSkillLifecycleResultDto::Failed, Some(diagnostic)) => {
                    validate_runtime_skill_diagnostic(diagnostic)?;
                }
                (AutonomousSkillLifecycleResultDto::Succeeded, None) => {}
            }

            if matches!(stage, AutonomousSkillLifecycleStageDto::Discovery)
                && item.skill_cache_status.is_some()
            {
                return Err(CommandError::system_fault(
                    "runtime_stream_item_invalid",
                    "Cadence produced a discovery runtime skill item with cache status.",
                ));
            }
            if matches!(
                stage,
                AutonomousSkillLifecycleStageDto::Install
                    | AutonomousSkillLifecycleStageDto::Invoke
            ) && matches!(result, AutonomousSkillLifecycleResultDto::Succeeded)
                && item.skill_cache_status.is_none()
            {
                return Err(CommandError::system_fault(
                    "runtime_stream_item_invalid",
                    "Cadence produced a successful install/invoke runtime skill item without cache status.",
                ));
            }
        }
        RuntimeStreamItemKind::Activity => {
            require_non_empty(item.code.as_deref(), "code", "runtime activity item")?;
            require_non_empty(item.title.as_deref(), "title", "runtime activity item")?;
        }
        RuntimeStreamItemKind::ActionRequired => {
            require_non_empty(
                item.action_id.as_deref(),
                "actionId",
                "runtime action-required item",
            )?;
            require_non_empty(
                item.action_type.as_deref(),
                "actionType",
                "runtime action-required item",
            )?;
            require_non_empty(
                item.title.as_deref(),
                "title",
                "runtime action-required item",
            )?;
            require_non_empty(
                item.detail.as_deref(),
                "detail",
                "runtime action-required item",
            )?;
        }
        RuntimeStreamItemKind::Complete => {
            require_non_empty(item.detail.as_deref(), "detail", "runtime completion item")?;
        }
        RuntimeStreamItemKind::Failure => {
            require_non_empty(item.code.as_deref(), "code", "runtime failure item")?;
            require_non_empty(item.message.as_deref(), "message", "runtime failure item")?;
            if item.retryable.is_none() {
                return Err(CommandError::system_fault(
                    "runtime_stream_item_invalid",
                    "Cadence produced a runtime failure item without a retryable flag.",
                ));
            }
        }
    }

    Ok(())
}

fn validate_runtime_skill_source(
    source: &AutonomousSkillLifecycleSourceDto,
) -> Result<(), CommandError> {
    require_non_empty(Some(source.repo.as_str()), "repo", "runtime skill source")?;
    require_non_empty(Some(source.path.as_str()), "path", "runtime skill source")?;
    require_non_empty(
        Some(source.reference.as_str()),
        "reference",
        "runtime skill source",
    )?;
    require_non_empty(
        Some(source.tree_hash.as_str()),
        "treeHash",
        "runtime skill source",
    )?;

    if source.tree_hash.len() != 40
        || source
            .tree_hash
            .chars()
            .any(|character| !character.is_ascii_hexdigit() || character.is_ascii_uppercase())
    {
        return Err(CommandError::system_fault(
            "runtime_stream_item_invalid",
            "Cadence produced a runtime skill item with an invalid source tree hash.",
        ));
    }

    Ok(())
}

fn validate_runtime_skill_diagnostic(
    diagnostic: &AutonomousSkillLifecycleDiagnosticDto,
) -> Result<(), CommandError> {
    require_non_empty(
        Some(diagnostic.code.as_str()),
        "code",
        "runtime skill diagnostic",
    )?;
    require_non_empty(
        Some(diagnostic.message.as_str()),
        "message",
        "runtime skill diagnostic",
    )?;
    Ok(())
}

fn complete_item(
    request: &RuntimeStreamRequest,
    sequence: u64,
    detail: String,
) -> RuntimeStreamItemDto {
    RuntimeStreamItemDto {
        kind: RuntimeStreamItemKind::Complete,
        run_id: request.run_id.clone(),
        sequence,
        session_id: Some(request.session_id.clone()),
        flow_id: request.flow_id.clone(),
        text: None,
        tool_call_id: None,
        tool_name: None,
        tool_state: None,
        tool_summary: None,
        skill_id: None,
        skill_stage: None,
        skill_result: None,
        skill_source: None,
        skill_cache_status: None,
        skill_diagnostic: None,
        action_id: None,
        boundary_id: None,
        action_type: None,
        title: None,
        detail: Some(detail),
        code: None,
        message: None,
        retryable: None,
        created_at: crate::auth::now_timestamp(),
    }
}

fn failure_item(
    request: &RuntimeStreamRequest,
    sequence: u64,
    error: CommandError,
) -> RuntimeStreamItemDto {
    RuntimeStreamItemDto {
        kind: RuntimeStreamItemKind::Failure,
        run_id: request.run_id.clone(),
        sequence,
        session_id: Some(request.session_id.clone()),
        flow_id: request.flow_id.clone(),
        text: None,
        tool_call_id: None,
        tool_name: None,
        tool_state: None,
        tool_summary: None,
        skill_id: None,
        skill_stage: None,
        skill_result: None,
        skill_source: None,
        skill_cache_status: None,
        skill_diagnostic: None,
        action_id: None,
        boundary_id: None,
        action_type: None,
        title: Some("Runtime stream failed".into()),
        detail: None,
        code: Some(error.code),
        message: Some(error.message),
        retryable: Some(error.retryable),
        created_at: crate::auth::now_timestamp(),
    }
}
