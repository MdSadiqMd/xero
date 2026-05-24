use std::collections::BTreeMap;

use serde_json::{json, Value as JsonValue};
use tauri::{AppHandle, Runtime};
use time::{format_description::well_known::Rfc3339, Duration, OffsetDateTime};

use crate::{
    commands::{
        agent_task::start_agent_task_blocking,
        contracts::{
            runtime::{RuntimeRunApprovalModeDto, RuntimeRunControlInputDto},
            workflow_agents::AgentRefDto,
            workflows::{
                WorkflowDefinitionDto, WorkflowEdgeDto, WorkflowEdgeTypeDto,
                WorkflowHumanCheckpointTypeDto, WorkflowInputBindingDto,
                WorkflowMergeWaitPolicyDto, WorkflowNodeDto, WorkflowNodeRunStatusDto,
                WorkflowOutputContractDto, WorkflowResourceConflictModeDto, WorkflowRunDto,
                WorkflowRunNodeDto, WorkflowRunOverrideDto, WorkflowRunStatusDto,
                WorkflowStallDetectorDto, WorkflowTerminalStatusDto,
            },
        },
        default_runtime_agent_approval_mode, CommandError, CommandResult, StartAgentTaskRequestDto,
    },
    db::project_store::{
        self, AgentRunRecord, AgentRunSnapshotRecord, AgentRunStatus, AgentSessionCreateRecord,
    },
    runtime::DesktopAgentCoreRuntime,
    state::DesktopState,
};

use super::{
    artifacts::{build_agent_node_prompt, extract_workflow_artifact_payload, final_assistant_text},
    condition_eval::{evaluate_workflow_condition, WorkflowConditionContext},
};

const MAX_RECONCILE_STEPS: usize = 32;
const RUNTIME_ACTIVITY_TIMEOUT_FAILURE_CLASS: &str = "runtime_activity_timeout";
const USER_SKIPPED_FAILURE_CLASS: &str = "skipped_by_user";

pub fn reconcile_workflow_run<R: Runtime + 'static>(
    app: &AppHandle<R>,
    state: &DesktopState,
    project_id: &str,
    run_id: &str,
) -> CommandResult<WorkflowRunDto> {
    let repo_root = crate::commands::runtime_support::resolve_project_root(app, state, project_id)?;
    for _ in 0..MAX_RECONCILE_STEPS {
        let run =
            project_store::get_workflow_run(&repo_root, project_id, run_id)?.ok_or_else(|| {
                CommandError::user_fixable(
                    "workflow_run_not_found",
                    format!("Xero could not find Workflow run `{run_id}`."),
                )
            })?;
        if is_terminal_run(run.status) || run.status == WorkflowRunStatusDto::Paused {
            return Ok(run);
        }

        if run.status == WorkflowRunStatusDto::Queued {
            project_store::update_workflow_run_status(
                &repo_root,
                project_id,
                run_id,
                WorkflowRunStatusDto::Running,
                None,
                None,
            )?;
            ensure_node_run(&repo_root, &run, &run.definition_snapshot.start_node_id, 0)?;
            continue;
        }

        if reconcile_running_agent_nodes(&repo_root, project_id, &run)? {
            continue;
        }

        if route_completed_nodes(&repo_root, project_id, &run)? {
            continue;
        }

        if start_eligible_nodes(app, state, &repo_root, project_id, &run)? {
            continue;
        }

        return project_store::get_workflow_run(&repo_root, project_id, run_id)?.ok_or_else(|| {
            CommandError::system_fault(
                "workflow_run_missing_after_reconcile",
                format!("Workflow run `{run_id}` disappeared during reconcile."),
            )
        });
    }

    project_store::get_workflow_run(&repo_root, project_id, run_id)?.ok_or_else(|| {
        CommandError::system_fault(
            "workflow_run_missing_after_reconcile",
            format!("Workflow run `{run_id}` disappeared during reconcile."),
        )
    })
}

pub fn resume_workflow_checkpoint<R: Runtime + 'static>(
    app: &AppHandle<R>,
    state: &DesktopState,
    project_id: &str,
    run_id: &str,
    node_run_id: &str,
    decision: &str,
    payload: Option<JsonValue>,
) -> CommandResult<WorkflowRunDto> {
    let repo_root = crate::commands::runtime_support::resolve_project_root(app, state, project_id)?;
    let run =
        project_store::get_workflow_run(&repo_root, project_id, run_id)?.ok_or_else(|| {
            CommandError::user_fixable(
                "workflow_run_not_found",
                format!("Xero could not find Workflow run `{run_id}`."),
            )
        })?;
    let node_run = run
        .nodes
        .iter()
        .find(|node| node.id == node_run_id)
        .ok_or_else(|| {
            CommandError::user_fixable(
                "workflow_checkpoint_not_found",
                format!("Xero could not find Workflow checkpoint node run `{node_run_id}`."),
            )
        })?;
    let checkpoint_type = checkpoint_type_for_node(&run.definition_snapshot, &node_run.node_id)
        .unwrap_or(WorkflowHumanCheckpointTypeDto::Decision);
    project_store::insert_workflow_gate_decision(
        &repo_root,
        project_id,
        run_id,
        node_run_id,
        checkpoint_type,
        decision,
        payload.as_ref(),
    )?;
    project_store::insert_workflow_artifact(
        &repo_root,
        project_id,
        run_id,
        node_run_id,
        "human_decision",
        1,
        &json!({ "decision": decision, "payload": payload }),
        Some(decision),
    )?;
    project_store::update_workflow_run_node(
        &repo_root,
        project_id,
        node_run_id,
        WorkflowNodeRunStatusDto::Succeeded,
        None,
        None,
        None,
    )?;
    project_store::update_workflow_run_status(
        &repo_root,
        project_id,
        run_id,
        WorkflowRunStatusDto::Running,
        None,
        None,
    )?;
    reconcile_workflow_run(app, state, project_id, run_id)
}

pub fn retry_workflow_node_run<R: Runtime + 'static>(
    app: &AppHandle<R>,
    state: &DesktopState,
    project_id: &str,
    run_id: &str,
    node_run_id: &str,
) -> CommandResult<WorkflowRunDto> {
    let repo_root = crate::commands::runtime_support::resolve_project_root(app, state, project_id)?;
    let run =
        project_store::get_workflow_run(&repo_root, project_id, run_id)?.ok_or_else(|| {
            CommandError::user_fixable(
                "workflow_run_not_found",
                format!("Xero could not find Workflow run `{run_id}`."),
            )
        })?;
    if matches!(
        run.status,
        WorkflowRunStatusDto::Completed | WorkflowRunStatusDto::Cancelled
    ) {
        return Err(CommandError::user_fixable(
            "workflow_run_not_retryable",
            "Completed or cancelled Workflow runs cannot be retried from a node.",
        ));
    }
    let node_run = run
        .nodes
        .iter()
        .find(|node| node.id == node_run_id)
        .cloned()
        .ok_or_else(|| {
            CommandError::user_fixable(
                "workflow_node_run_not_found",
                format!("Xero could not find Workflow node run `{node_run_id}`."),
            )
        })?;
    if !is_retryable_node_status(node_run.status) {
        return Err(CommandError::user_fixable(
            "workflow_node_run_not_retryable",
            format!(
                "Workflow node run `{node_run_id}` cannot be retried while it is `{}`.",
                node_run.status.as_str()
            ),
        ));
    }
    if find_node(&run.definition_snapshot, &node_run.node_id).is_none() {
        return Err(CommandError::system_fault(
            "workflow_retry_node_missing",
            format!(
                "Workflow node `{}` was missing from run `{run_id}`.",
                node_run.node_id
            ),
        ));
    }
    if has_control_event_after_completion(&run, &node_run, "workflow_node_retry_requested") {
        return reconcile_workflow_run(app, state, project_id, run_id);
    }

    let attempt = next_attempt_for_node(&run, &node_run.node_id);
    let retry_node = ensure_node_run(&repo_root, &run, &node_run.node_id, attempt)?;
    project_store::insert_workflow_event(
        &repo_root,
        project_id,
        run_id,
        Some(&node_run.id),
        "workflow_node_retry_requested",
        &json!({
            "nodeId": node_run.node_id,
            "previousStatus": node_run.status.as_str(),
            "retryNodeRunId": retry_node.id,
            "attemptNumber": retry_node.attempt_number,
        }),
    )?;
    project_store::update_workflow_run_status(
        &repo_root,
        project_id,
        run_id,
        WorkflowRunStatusDto::Running,
        None,
        None,
    )?;
    reconcile_workflow_run(app, state, project_id, run_id)
}

pub fn skip_workflow_branch<R: Runtime + 'static>(
    app: &AppHandle<R>,
    state: &DesktopState,
    project_id: &str,
    run_id: &str,
    node_run_id: &str,
    reason: Option<&str>,
) -> CommandResult<WorkflowRunDto> {
    let repo_root = crate::commands::runtime_support::resolve_project_root(app, state, project_id)?;
    let run =
        project_store::get_workflow_run(&repo_root, project_id, run_id)?.ok_or_else(|| {
            CommandError::user_fixable(
                "workflow_run_not_found",
                format!("Xero could not find Workflow run `{run_id}`."),
            )
        })?;
    if is_terminal_run(run.status) {
        return Err(CommandError::user_fixable(
            "workflow_run_not_skippable",
            "Completed, failed, or cancelled Workflow runs cannot skip branches.",
        ));
    }
    let node_run = run
        .nodes
        .iter()
        .find(|node| node.id == node_run_id)
        .cloned()
        .ok_or_else(|| {
            CommandError::user_fixable(
                "workflow_node_run_not_found",
                format!("Xero could not find Workflow node run `{node_run_id}`."),
            )
        })?;
    if !is_skippable_node_status(node_run.status) {
        return Err(CommandError::user_fixable(
            "workflow_node_run_not_skippable",
            format!(
                "Workflow node run `{node_run_id}` cannot be skipped while it is `{}`.",
                node_run.status.as_str()
            ),
        ));
    }

    if let Some(runtime_run_id) = node_run.runtime_run_id.as_ref() {
        let runtime = DesktopAgentCoreRuntime::new(state.agent_run_supervisor().clone());
        let _ = runtime.cancel_run(
            repo_root.clone(),
            project_id.to_owned(),
            runtime_run_id.to_owned(),
        );
    }
    project_store::update_workflow_run_node(
        &repo_root,
        project_id,
        &node_run.id,
        WorkflowNodeRunStatusDto::Skipped,
        None,
        None,
        Some(USER_SKIPPED_FAILURE_CLASS),
    )?;
    let merge_target_node_ids = ensure_direct_merge_targets_for_skipped_branch(
        &repo_root,
        project_id,
        &run,
        &node_run.node_id,
    )?;
    project_store::insert_workflow_event(
        &repo_root,
        project_id,
        run_id,
        Some(&node_run.id),
        "workflow_branch_skipped",
        &json!({
            "nodeId": node_run.node_id,
            "previousStatus": node_run.status.as_str(),
            "reason": reason,
            "mergeTargetNodeIds": merge_target_node_ids,
        }),
    )?;
    project_store::update_workflow_run_status(
        &repo_root,
        project_id,
        run_id,
        WorkflowRunStatusDto::Running,
        None,
        None,
    )?;
    reconcile_workflow_run(app, state, project_id, run_id)
}

fn reconcile_running_agent_nodes(
    repo_root: &std::path::Path,
    project_id: &str,
    run: &WorkflowRunDto,
) -> CommandResult<bool> {
    let mut changed = false;
    let now = OffsetDateTime::now_utc();
    for node_run in run
        .nodes
        .iter()
        .filter(|node| node.status == WorkflowNodeRunStatusDto::Running)
    {
        let Some(runtime_run_id) = node_run.runtime_run_id.as_deref() else {
            continue;
        };
        let snapshot = project_store::load_agent_run(repo_root, project_id, runtime_run_id)?;
        match snapshot.run.status {
            AgentRunStatus::Starting | AgentRunStatus::Running => {
                if let Some(timeout_seconds) =
                    activity_timeout_seconds_for_node(&run.definition_snapshot, &node_run.node_id)
                {
                    if let Some(last_activity_at) =
                        stale_agent_activity_at(&snapshot.run, timeout_seconds, now)
                    {
                        project_store::update_workflow_run_node(
                            repo_root,
                            project_id,
                            &node_run.id,
                            WorkflowNodeRunStatusDto::Stalled,
                            None,
                            None,
                            Some(RUNTIME_ACTIVITY_TIMEOUT_FAILURE_CLASS),
                        )?;
                        project_store::insert_workflow_event(
                            repo_root,
                            project_id,
                            &run.id,
                            Some(&node_run.id),
                            "workflow_node_stalled",
                            &json!({
                                "nodeId": node_run.node_id,
                                "runtimeRunId": runtime_run_id,
                                "failureClass": RUNTIME_ACTIVITY_TIMEOUT_FAILURE_CLASS,
                                "timeoutSeconds": timeout_seconds,
                                "lastActivityAt": last_activity_at,
                            }),
                        )?;
                        changed = true;
                    }
                }
            }
            AgentRunStatus::Completed => {
                if let Some(contract) =
                    output_contract_for_node(&run.definition_snapshot, &node_run.node_id)
                {
                    let final_text = final_assistant_text(&snapshot).unwrap_or_default();
                    let (payload, render_text) =
                        match extract_workflow_artifact_payload(contract, &final_text) {
                            Ok(artifact) => artifact,
                            Err(error) if error.code == "workflow_artifact_extraction_failed" => {
                                fail_node_with_recoverable_error(
                                    repo_root,
                                    project_id,
                                    run,
                                    node_run,
                                    "workflow_artifact_extraction_failed",
                                    &error.code,
                                    &error.message,
                                )?;
                                changed = true;
                                continue;
                            }
                            Err(error) => return Err(error),
                        };
                    project_store::insert_workflow_artifact(
                        repo_root,
                        project_id,
                        &run.id,
                        &node_run.id,
                        &contract.artifact_type,
                        contract.schema_version,
                        &payload,
                        render_text.as_deref(),
                    )?;
                }
                project_store::update_workflow_run_node(
                    repo_root,
                    project_id,
                    &node_run.id,
                    WorkflowNodeRunStatusDto::Succeeded,
                    None,
                    None,
                    None,
                )?;
                changed = true;
            }
            AgentRunStatus::Failed => {
                let failure_class = snapshot
                    .run
                    .last_error
                    .as_ref()
                    .map(|error| error.code.as_str())
                    .unwrap_or("agent_failed");
                project_store::update_workflow_run_node(
                    repo_root,
                    project_id,
                    &node_run.id,
                    WorkflowNodeRunStatusDto::Failed,
                    None,
                    None,
                    Some(failure_class),
                )?;
                changed = true;
            }
            AgentRunStatus::Cancelled => {
                project_store::update_workflow_run_node(
                    repo_root,
                    project_id,
                    &node_run.id,
                    WorkflowNodeRunStatusDto::Cancelled,
                    None,
                    None,
                    Some("cancelled"),
                )?;
                changed = true;
            }
            _ => {}
        }
    }
    Ok(changed)
}

fn activity_timeout_seconds_for_node(
    definition: &WorkflowDefinitionDto,
    node_id: &str,
) -> Option<u32> {
    match find_node(definition, node_id) {
        Some(WorkflowNodeDto::Agent { failure_policy, .. }) => failure_policy
            .runtime_activity_timeout_seconds
            .or(definition.run_policy.node_timeout_seconds),
        _ => None,
    }
}

fn stale_agent_activity_at(
    agent_run: &AgentRunRecord,
    timeout_seconds: u32,
    now: OffsetDateTime,
) -> Option<&str> {
    let timeout = Duration::seconds(timeout_seconds.into());
    let latest_activity = [
        agent_run.last_heartbeat_at.as_deref(),
        Some(agent_run.updated_at.as_str()),
        Some(agent_run.started_at.as_str()),
    ]
    .into_iter()
    .flatten()
    .filter_map(|timestamp| {
        OffsetDateTime::parse(timestamp, &Rfc3339)
            .ok()
            .map(|parsed| (timestamp, parsed))
    })
    .max_by_key(|(_, timestamp)| timestamp.unix_timestamp_nanos())?;

    (now - latest_activity.1 >= timeout).then_some(latest_activity.0)
}

fn route_completed_nodes(
    repo_root: &std::path::Path,
    project_id: &str,
    run: &WorkflowRunDto,
) -> CommandResult<bool> {
    for node_run in run.nodes.iter().filter(|node| {
        matches!(
            node.status,
            WorkflowNodeRunStatusDto::Succeeded
                | WorkflowNodeRunStatusDto::Failed
                | WorkflowNodeRunStatusDto::Stalled
                | WorkflowNodeRunStatusDto::Cancelled
        ) && !has_routed_node_run(run, node)
    }) {
        let Some(node) = find_node(&run.definition_snapshot, &node_run.node_id) else {
            continue;
        };
        if let WorkflowNodeDto::Terminal {
            terminal_status, ..
        } = node
        {
            complete_for_terminal(repo_root, project_id, run, *terminal_status)?;
            return Ok(true);
        }

        let context = condition_context(run);
        let mut outgoing = run
            .definition_snapshot
            .edges
            .iter()
            .filter(|edge| {
                edge.from_node_id == node_run.node_id
                    && edge_applies_to_node_status(edge.r#type, node_run.status)
            })
            .collect::<Vec<_>>();
        outgoing.sort_by_key(|edge| edge.priority);

        let mut matched_edges = Vec::new();
        let mut default_edge = None;
        for edge in outgoing {
            let evaluation = evaluate_workflow_condition(&edge.condition, &context);
            let condition_json = encode_workflow_condition(&edge.condition)?;
            project_store::insert_workflow_event(
                repo_root,
                project_id,
                &run.id,
                Some(&node_run.id),
                "workflow_edge_evaluated",
                &json!({
                    "edgeId": edge.id,
                    "fromNodeId": edge.from_node_id,
                    "toNodeId": edge.to_node_id,
                    "matched": evaluation.matched,
                    "condition": condition_json,
                    "evidence": evaluation.evidence.clone(),
                }),
            )?;
            if !evaluation.matched {
                continue;
            }
            if matches!(
                edge.condition,
                crate::commands::contracts::workflows::WorkflowConditionDto::Always
            ) {
                default_edge = Some((edge, condition_json, evaluation.evidence));
                continue;
            }
            matched_edges.push((edge, condition_json, evaluation.evidence));
            if routes_single_match(node) {
                break;
            }
        }
        if matched_edges.is_empty() {
            if let Some((edge, condition_json, evidence)) = default_edge {
                matched_edges.push((edge, condition_json, evidence));
            }
        }

        if !matched_edges.is_empty() {
            let mut created = false;
            if node_run.status == WorkflowNodeRunStatusDto::Succeeded
                && had_prior_unsuccessful_attempt(run, node_run)
                && !has_metric_event_for_node(run, node_run, "recovery_success")
            {
                insert_workflow_metric_event(
                    repo_root,
                    project_id,
                    &run.id,
                    Some(&node_run.id),
                    "recovery_success",
                    &json!({
                        "nodeId": node_run.node_id,
                        "attemptNumber": node_run.attempt_number,
                    }),
                )?;
            }
            for (edge, condition_json, evidence) in matched_edges {
                let target_node_id =
                    loop_target_for_edge(repo_root, project_id, run, node_run, edge)?;
                project_store::insert_workflow_edge_decision(
                    repo_root,
                    project_id,
                    &run.id,
                    &edge.from_node_id,
                    &target_node_id,
                    &edge.id,
                    &condition_json,
                    &evidence,
                )?;
                let attempt = next_attempt_for_node(run, &target_node_id);
                ensure_node_run(repo_root, run, &target_node_id, attempt)?;
                created = true;
            }
            return Ok(created);
        }

        project_store::insert_workflow_event(
            repo_root,
            project_id,
            &run.id,
            Some(&node_run.id),
            "workflow_route_missing",
            &json!({ "nodeId": node_run.node_id }),
        )?;
        project_store::update_workflow_run_status(
            repo_root,
            project_id,
            &run.id,
            WorkflowRunStatusDto::Paused,
            Some(WorkflowTerminalStatusDto::NeedsHuman),
            None,
        )?;
        return Ok(true);
    }
    Ok(false)
}

fn start_eligible_nodes<R: Runtime + 'static>(
    app: &AppHandle<R>,
    state: &DesktopState,
    repo_root: &std::path::Path,
    project_id: &str,
    run: &WorkflowRunDto,
) -> CommandResult<bool> {
    let concurrency_limit = run.definition_snapshot.run_policy.concurrency_limit.max(1) as usize;
    let running_agent_count = run
        .nodes
        .iter()
        .filter(|node| node.status == WorkflowNodeRunStatusDto::Running)
        .count();
    for node_run in run
        .nodes
        .iter()
        .filter(|node| node.status == WorkflowNodeRunStatusDto::Eligible)
    {
        let Some(node) = find_node(&run.definition_snapshot, &node_run.node_id) else {
            continue;
        };
        match node {
            WorkflowNodeDto::Agent {
                title,
                agent_ref,
                input_bindings,
                run_overrides,
                ..
            } => {
                if running_agent_count >= concurrency_limit {
                    continue;
                }
                if let Some(conflict) = resource_conflict_for_node(run, node_run, node) {
                    if !has_node_event(run, node_run, "workflow_resource_conflict_wait") {
                        project_store::insert_workflow_event(
                            repo_root,
                            project_id,
                            &run.id,
                            Some(&node_run.id),
                            "workflow_resource_conflict_wait",
                            &json!({
                                "nodeId": node_run.node_id,
                                "blockedByNodeRunId": conflict.node_run_id,
                                "blockedByNodeId": conflict.node_id,
                                "scopes": conflict.scopes,
                            }),
                        )?;
                        return Ok(true);
                    }
                    continue;
                }
                start_agent_node(
                    app,
                    state,
                    repo_root,
                    project_id,
                    run,
                    node_run,
                    title,
                    agent_ref,
                    input_bindings,
                    run_overrides.as_ref(),
                )?;
                return Ok(true);
            }
            WorkflowNodeDto::Router { .. } => {
                project_store::update_workflow_run_node(
                    repo_root,
                    project_id,
                    &node_run.id,
                    WorkflowNodeRunStatusDto::Succeeded,
                    None,
                    None,
                    None,
                )?;
                return Ok(true);
            }
            WorkflowNodeDto::Merge {
                wait_policy,
                quorum,
                fail_fast,
                ..
            } => match evaluate_merge_node(run, node_run, *wait_policy, *quorum, *fail_fast) {
                MergeEvaluation::Waiting => {}
                MergeEvaluation::Succeeded => {
                    project_store::update_workflow_run_node(
                        repo_root,
                        project_id,
                        &node_run.id,
                        WorkflowNodeRunStatusDto::Succeeded,
                        None,
                        None,
                        None,
                    )?;
                    return Ok(true);
                }
                MergeEvaluation::Failed(failure_class) => {
                    project_store::update_workflow_run_node(
                        repo_root,
                        project_id,
                        &node_run.id,
                        WorkflowNodeRunStatusDto::Failed,
                        None,
                        None,
                        Some(failure_class),
                    )?;
                    return Ok(true);
                }
            },
            WorkflowNodeDto::Gate {
                required_checks,
                on_blocked,
                ..
            } => {
                let context = condition_context(run);
                let passed = required_checks
                    .iter()
                    .all(|condition| evaluate_workflow_condition(condition, &context).matched);
                if passed {
                    project_store::update_workflow_run_node(
                        repo_root,
                        project_id,
                        &node_run.id,
                        WorkflowNodeRunStatusDto::Succeeded,
                        None,
                        None,
                        None,
                    )?;
                } else if on_blocked == "fail" {
                    project_store::update_workflow_run_node(
                        repo_root,
                        project_id,
                        &node_run.id,
                        WorkflowNodeRunStatusDto::Failed,
                        None,
                        None,
                        Some("gate_failed"),
                    )?;
                } else {
                    pause_at_checkpoint(repo_root, project_id, run, node_run, "gate_waiting")?;
                }
                return Ok(true);
            }
            WorkflowNodeDto::HumanCheckpoint { .. } => {
                pause_at_checkpoint(repo_root, project_id, run, node_run, "human_checkpoint")?;
                return Ok(true);
            }
            WorkflowNodeDto::Terminal {
                terminal_status, ..
            } => {
                project_store::update_workflow_run_node(
                    repo_root,
                    project_id,
                    &node_run.id,
                    WorkflowNodeRunStatusDto::Succeeded,
                    None,
                    None,
                    None,
                )?;
                complete_for_terminal(repo_root, project_id, run, *terminal_status)?;
                return Ok(true);
            }
        }
    }
    Ok(false)
}

struct ResourceConflict {
    node_run_id: String,
    node_id: String,
    scopes: Vec<String>,
}

fn resource_conflict_for_node(
    run: &WorkflowRunDto,
    node_run: &WorkflowRunNodeDto,
    node: &WorkflowNodeDto,
) -> Option<ResourceConflict> {
    if run
        .definition_snapshot
        .run_policy
        .resource_conflict_policy
        .mode
        == WorkflowResourceConflictModeDto::AllowConflicts
    {
        return None;
    }
    let candidate_scopes = resource_scopes_for_node(&run.definition_snapshot, node);
    if candidate_scopes.is_empty() {
        return None;
    }

    for running in run.nodes.iter().filter(|running| {
        running.id != node_run.id
            && matches!(
                running.status,
                WorkflowNodeRunStatusDto::Starting | WorkflowNodeRunStatusDto::Running
            )
    }) {
        let Some(running_node) = find_node(&run.definition_snapshot, &running.node_id) else {
            continue;
        };
        let running_scopes = resource_scopes_for_node(&run.definition_snapshot, running_node);
        let overlap = overlapping_resource_scopes(&candidate_scopes, &running_scopes);
        if !overlap.is_empty() {
            return Some(ResourceConflict {
                node_run_id: running.id.clone(),
                node_id: running.node_id.clone(),
                scopes: overlap,
            });
        }
    }
    None
}

fn resource_scopes_for_node(
    definition: &WorkflowDefinitionDto,
    node: &WorkflowNodeDto,
) -> Vec<String> {
    match node {
        WorkflowNodeDto::Agent {
            resource_scopes, ..
        } if !resource_scopes.is_empty() => normalize_resource_scopes(resource_scopes),
        WorkflowNodeDto::Agent { .. } => normalize_resource_scopes(
            &definition
                .run_policy
                .resource_conflict_policy
                .default_scopes,
        ),
        _ => Vec::new(),
    }
}

fn normalize_resource_scopes(scopes: &[String]) -> Vec<String> {
    let mut normalized = scopes
        .iter()
        .map(|scope| scope.trim())
        .filter(|scope| !scope.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn overlapping_resource_scopes(left: &[String], right: &[String]) -> Vec<String> {
    left.iter()
        .filter(|scope| right.iter().any(|candidate| candidate == *scope))
        .cloned()
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn start_agent_node<R: Runtime + 'static>(
    app: &AppHandle<R>,
    state: &DesktopState,
    repo_root: &std::path::Path,
    project_id: &str,
    run: &WorkflowRunDto,
    node_run: &WorkflowRunNodeDto,
    title: &str,
    agent_ref: &AgentRefDto,
    input_bindings: &[WorkflowInputBindingDto],
    run_overrides: Option<&WorkflowRunOverrideDto>,
) -> CommandResult<()> {
    if let Some(snapshot) = load_existing_agent_run_for_node(repo_root, project_id, node_run)? {
        project_store::update_workflow_run_node(
            repo_root,
            project_id,
            &node_run.id,
            WorkflowNodeRunStatusDto::Running,
            Some(&snapshot.run.run_id),
            Some(&snapshot.run.agent_session_id),
            None,
        )?;
        project_store::insert_workflow_event(
            repo_root,
            project_id,
            &run.id,
            Some(&node_run.id),
            "workflow_agent_reconnected",
            &json!({
                "nodeId": node_run.node_id,
                "runtimeRunId": snapshot.run.run_id,
                "agentSessionId": snapshot.run.agent_session_id
            }),
        )?;
        return Ok(());
    }

    let prompt = match build_agent_node_prompt(
        &run.definition_snapshot.name,
        title,
        run_overrides.map(|overrides| overrides.prompt_preface.as_str()),
        run.initial_input.as_ref(),
        input_bindings,
        &run.artifacts,
    ) {
        Ok(prompt) => prompt,
        Err(error) if error.code == "workflow_required_input_missing" => {
            fail_node_with_recoverable_error(
                repo_root,
                project_id,
                run,
                node_run,
                "workflow_required_input_missing",
                &error.code,
                &error.message,
            )?;
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    let controls = controls_for_agent_ref(
        repo_root,
        &run.definition_snapshot,
        agent_ref,
        run_overrides,
    )?;
    let session = project_store::create_agent_session(
        repo_root,
        &AgentSessionCreateRecord {
            project_id: project_id.into(),
            title: format!("Workflow: {title}"),
            summary: format!(
                "Node `{}` in Workflow `{}`.",
                node_run.node_id, run.definition_snapshot.name
            ),
            selected: false,
        },
    )?;
    let resource_scopes = find_node(&run.definition_snapshot, &node_run.node_id)
        .map(|node| resource_scopes_for_node(&run.definition_snapshot, node))
        .unwrap_or_default();
    project_store::insert_workflow_event(
        repo_root,
        project_id,
        &run.id,
        Some(&node_run.id),
        "workflow_agent_start_requested",
        &json!({
            "nodeId": node_run.node_id,
            "agentRef": agent_ref,
            "agentSessionId": session.agent_session_id.clone(),
            "resourceScopes": resource_scopes,
        }),
    )?;
    let agent_run = start_agent_task_blocking(
        app,
        state,
        StartAgentTaskRequestDto {
            project_id: project_id.into(),
            agent_session_id: session.agent_session_id.clone(),
            run_id: Some(node_run.idempotency_key.clone()),
            prompt,
            controls: Some(controls),
            attachments: Vec::new(),
        },
    )?;
    project_store::update_workflow_run_node(
        repo_root,
        project_id,
        &node_run.id,
        WorkflowNodeRunStatusDto::Running,
        Some(&agent_run.run_id),
        Some(&session.agent_session_id),
        None,
    )?;
    project_store::insert_workflow_event(
        repo_root,
        project_id,
        &run.id,
        Some(&node_run.id),
        "workflow_agent_started",
        &json!({
            "runtimeRunId": agent_run.run_id,
            "agentSessionId": session.agent_session_id
        }),
    )?;
    Ok(())
}

fn load_existing_agent_run_for_node(
    repo_root: &std::path::Path,
    project_id: &str,
    node_run: &WorkflowRunNodeDto,
) -> CommandResult<Option<AgentRunSnapshotRecord>> {
    match project_store::load_agent_run(repo_root, project_id, &node_run.idempotency_key) {
        Ok(snapshot) => Ok(Some(snapshot)),
        Err(error) if error.code == "agent_run_not_found" => Ok(None),
        Err(error) => Err(error),
    }
}

fn fail_node_with_recoverable_error(
    repo_root: &std::path::Path,
    project_id: &str,
    run: &WorkflowRunDto,
    node_run: &WorkflowRunNodeDto,
    event_type: &str,
    failure_class: &str,
    message: &str,
) -> CommandResult<()> {
    project_store::update_workflow_run_node(
        repo_root,
        project_id,
        &node_run.id,
        WorkflowNodeRunStatusDto::Failed,
        None,
        None,
        Some(failure_class),
    )?;
    project_store::insert_workflow_event(
        repo_root,
        project_id,
        &run.id,
        Some(&node_run.id),
        event_type,
        &json!({
            "nodeId": node_run.node_id,
            "failureClass": failure_class,
            "message": message,
        }),
    )
}

fn controls_for_agent_ref(
    repo_root: &std::path::Path,
    definition: &WorkflowDefinitionDto,
    agent_ref: &AgentRefDto,
    run_overrides: Option<&WorkflowRunOverrideDto>,
) -> CommandResult<RuntimeRunControlInputDto> {
    let (runtime_agent_id, agent_definition_id) = match agent_ref {
        AgentRefDto::BuiltIn {
            runtime_agent_id, ..
        } => (*runtime_agent_id, None),
        AgentRefDto::Custom { definition_id, .. } => {
            let selection = project_store::resolve_agent_definition_for_run(
                repo_root,
                Some(definition_id),
                crate::commands::default_runtime_agent_id(),
            )?;
            (selection.runtime_agent_id, Some(selection.definition_id))
        }
    };
    let approval_mode: RuntimeRunApprovalModeDto = run_overrides
        .and_then(|overrides| overrides.approval_mode.clone())
        .or_else(|| definition.run_policy.approval_mode.clone())
        .unwrap_or_else(|| default_runtime_agent_approval_mode(&runtime_agent_id));
    Ok(RuntimeRunControlInputDto {
        runtime_agent_id,
        agent_definition_id,
        provider_profile_id: run_overrides
            .and_then(|overrides| overrides.provider_profile_id.clone())
            .or_else(|| definition.run_policy.default_provider_profile_id.clone()),
        model_id: run_overrides
            .and_then(|overrides| overrides.model_id.clone())
            .or_else(|| definition.run_policy.default_model_id.clone())
            .unwrap_or_default(),
        thinking_effort: None,
        approval_mode,
        plan_mode_required: run_overrides
            .map(|overrides| overrides.plan_mode_required)
            .unwrap_or(false),
        auto_compact_enabled: run_overrides
            .map(|overrides| overrides.auto_compact_enabled)
            .unwrap_or(true),
    })
}

fn ensure_node_run(
    repo_root: &std::path::Path,
    run: &WorkflowRunDto,
    node_id: &str,
    attempt: u32,
) -> CommandResult<WorkflowRunNodeDto> {
    let node = find_node(&run.definition_snapshot, node_id).ok_or_else(|| {
        CommandError::system_fault(
            "workflow_target_node_missing",
            format!("Workflow target node `{node_id}` was missing from its snapshot."),
        )
    })?;
    let idempotency_key = format!("{}:{}:{attempt}", run.id, node_id);
    project_store::insert_workflow_run_node(
        repo_root,
        &run.project_id,
        &run.id,
        node_id,
        node.node_type().as_str(),
        attempt,
        WorkflowNodeRunStatusDto::Eligible,
        &idempotency_key,
    )
}

fn ensure_direct_merge_targets_for_skipped_branch(
    repo_root: &std::path::Path,
    project_id: &str,
    run: &WorkflowRunDto,
    skipped_node_id: &str,
) -> CommandResult<Vec<String>> {
    let mut targets = Vec::new();
    for edge in run
        .definition_snapshot
        .edges
        .iter()
        .filter(|edge| edge.from_node_id == skipped_node_id)
    {
        let Some(WorkflowNodeDto::Merge { .. }) =
            find_node(&run.definition_snapshot, &edge.to_node_id)
        else {
            continue;
        };
        let attempt = next_attempt_for_node(run, &edge.to_node_id);
        ensure_node_run(repo_root, run, &edge.to_node_id, attempt)?;
        targets.push(edge.to_node_id.clone());
    }
    if targets.is_empty() {
        project_store::insert_workflow_event(
            repo_root,
            project_id,
            &run.id,
            None,
            "workflow_branch_skip_no_merge_target",
            &json!({ "nodeId": skipped_node_id }),
        )?;
    }
    Ok(targets)
}

fn loop_target_for_edge(
    repo_root: &std::path::Path,
    project_id: &str,
    run: &WorkflowRunDto,
    node_run: &WorkflowRunNodeDto,
    edge: &WorkflowEdgeDto,
) -> CommandResult<String> {
    let Some(policy) = edge.loop_policy.as_ref() else {
        return Ok(edge.to_node_id.clone());
    };
    if let Some(detector) = policy.stall_detector {
        if let Some(failure_class) = stall_failure_class_for_detector(run, node_run, detector) {
            project_store::update_workflow_run_node(
                repo_root,
                project_id,
                &node_run.id,
                WorkflowNodeRunStatusDto::Stalled,
                None,
                None,
                Some(failure_class),
            )?;
            project_store::increment_workflow_loop_attempt(
                repo_root,
                project_id,
                &run.id,
                &policy.loop_key,
                &node_run.id,
                true,
            )?;
            project_store::insert_workflow_event(
                repo_root,
                project_id,
                &run.id,
                Some(&node_run.id),
                "workflow_node_stalled",
                &json!({
                    "nodeId": node_run.node_id,
                    "failureClass": failure_class,
                    "stallDetector": detector.as_str(),
                    "loopKey": policy.loop_key.as_str(),
                }),
            )?;
            insert_workflow_metric_event(
                repo_root,
                project_id,
                &run.id,
                Some(&node_run.id),
                "loop_exhaustion",
                &json!({
                    "loopKey": policy.loop_key.as_str(),
                    "stallDetector": detector.as_str(),
                    "failureClass": failure_class,
                    "onExhausted": policy.on_exhausted.as_str(),
                }),
            )?;
            return Ok(policy.on_exhausted.clone());
        }
    }
    let current_attempts = run
        .loop_attempts
        .iter()
        .find(|attempt| attempt.loop_key == policy.loop_key)
        .map(|attempt| attempt.attempt_count)
        .unwrap_or(0);
    if current_attempts >= policy.max_attempts {
        project_store::increment_workflow_loop_attempt(
            repo_root,
            project_id,
            &run.id,
            &policy.loop_key,
            &node_run.id,
            true,
        )?;
        insert_workflow_metric_event(
            repo_root,
            project_id,
            &run.id,
            Some(&node_run.id),
            "loop_exhaustion",
            &json!({
                "loopKey": policy.loop_key.as_str(),
                "attemptCount": current_attempts.saturating_add(1),
                "maxAttempts": policy.max_attempts,
                "onExhausted": policy.on_exhausted.as_str(),
            }),
        )?;
        return Ok(policy.on_exhausted.clone());
    }
    project_store::increment_workflow_loop_attempt(
        repo_root,
        project_id,
        &run.id,
        &policy.loop_key,
        &node_run.id,
        false,
    )?;
    Ok(edge.to_node_id.clone())
}

fn stall_failure_class_for_detector(
    run: &WorkflowRunDto,
    node_run: &WorkflowRunNodeDto,
    detector: WorkflowStallDetectorDto,
) -> Option<&'static str> {
    match detector {
        WorkflowStallDetectorDto::FindingCountNotDecreasing => {
            finding_count_not_decreasing(run, node_run).then_some("finding_count_not_decreasing")
        }
        WorkflowStallDetectorDto::SameFailureClassRepeated => {
            same_failure_class_repeated(run, node_run).then_some("same_failure_class_repeated")
        }
        WorkflowStallDetectorDto::NoArtifactProgress => {
            no_artifact_progress(run, node_run).then_some("no_artifact_progress")
        }
        WorkflowStallDetectorDto::RuntimeActivityTimeout => (node_run.failure_class.as_deref()
            == Some(RUNTIME_ACTIVITY_TIMEOUT_FAILURE_CLASS))
        .then_some(RUNTIME_ACTIVITY_TIMEOUT_FAILURE_CLASS),
        WorkflowStallDetectorDto::RetryLimitExceeded => (node_run.failure_class.as_deref()
            == Some("retry_limit_exceeded"))
        .then_some("retry_limit_exceeded"),
    }
}

fn same_failure_class_repeated(run: &WorkflowRunDto, node_run: &WorkflowRunNodeDto) -> bool {
    let Some(current_failure) = node_run.failure_class.as_deref() else {
        return false;
    };
    run.nodes
        .iter()
        .filter(|candidate| {
            candidate.node_id == node_run.node_id
                && candidate.attempt_number < node_run.attempt_number
        })
        .max_by_key(|candidate| candidate.attempt_number)
        .and_then(|candidate| candidate.failure_class.as_deref())
        == Some(current_failure)
}

fn no_artifact_progress(run: &WorkflowRunDto, node_run: &WorkflowRunNodeDto) -> bool {
    let Some(contract) = output_contract_for_node(&run.definition_snapshot, &node_run.node_id)
    else {
        return false;
    };
    contract.required && artifacts_for_node_run(run, &node_run.id).is_empty()
}

fn finding_count_not_decreasing(run: &WorkflowRunDto, node_run: &WorkflowRunNodeDto) -> bool {
    let Some(current_count) = latest_finding_count_for_node_run(run, &node_run.id) else {
        return false;
    };
    let Some(previous_count) = run
        .nodes
        .iter()
        .filter(|candidate| {
            candidate.node_id == node_run.node_id
                && candidate.attempt_number < node_run.attempt_number
        })
        .max_by_key(|candidate| candidate.attempt_number)
        .and_then(|candidate| latest_finding_count_for_node_run(run, &candidate.id))
    else {
        return false;
    };
    current_count >= previous_count
}

fn latest_finding_count_for_node_run(run: &WorkflowRunDto, node_run_id: &str) -> Option<f64> {
    artifacts_for_node_run(run, node_run_id)
        .into_iter()
        .rev()
        .find_map(|artifact| finding_count_in_value(&artifact.payload))
}

fn artifacts_for_node_run<'a>(
    run: &'a WorkflowRunDto,
    node_run_id: &str,
) -> Vec<&'a crate::commands::contracts::workflows::WorkflowArtifactRecordDto> {
    run.artifacts
        .iter()
        .filter(|artifact| artifact.producer_node_run_id == node_run_id)
        .collect()
}

fn finding_count_in_value(value: &JsonValue) -> Option<f64> {
    match value {
        JsonValue::Object(map) => {
            for key in [
                "high_count",
                "highCount",
                "finding_count",
                "findingCount",
                "findings_count",
                "findingsCount",
                "gap_count",
                "gapCount",
                "gaps_count",
                "gapsCount",
            ] {
                if let Some(count) = map.get(key).and_then(JsonValue::as_f64) {
                    return Some(count);
                }
            }
            map.values().find_map(finding_count_in_value)
        }
        JsonValue::Array(items) => items.iter().find_map(finding_count_in_value),
        _ => None,
    }
}

fn condition_context(run: &WorkflowRunDto) -> WorkflowConditionContext {
    let mut context = WorkflowConditionContext::default();
    let mut node_id_by_run_id = BTreeMap::new();
    for node in &run.nodes {
        context
            .node_statuses
            .insert(node.node_id.clone(), node.status);
        if let Some(failure_class) = node.failure_class.as_ref() {
            context
                .failure_classes
                .insert(node.node_id.clone(), failure_class.clone());
            context.latest_failure_class = Some(failure_class.clone());
        }
        node_id_by_run_id.insert(node.id.clone(), node.node_id.clone());
    }
    for artifact in &run.artifacts {
        if let Some(node_id) = node_id_by_run_id.get(&artifact.producer_node_run_id) {
            context.artifacts.insert(
                format!("{node_id}.{}", artifact.artifact_type),
                artifact.payload.clone(),
            );
        }
    }
    for attempt in &run.loop_attempts {
        context
            .loop_attempts
            .insert(attempt.loop_key.clone(), attempt.attempt_count);
    }
    for decision in &run.gate_decisions {
        if let Some(node_id) = node_id_by_run_id.get(&decision.node_run_id) {
            context
                .human_decisions
                .insert(node_id.clone(), decision.decision.clone());
        }
    }
    context
}

fn encode_workflow_condition(
    condition: &crate::commands::contracts::workflows::WorkflowConditionDto,
) -> CommandResult<JsonValue> {
    serde_json::to_value(condition).map_err(|error| {
        CommandError::system_fault(
            "workflow_condition_encode_failed",
            format!("Xero could not encode Workflow condition: {error}"),
        )
    })
}

fn had_prior_unsuccessful_attempt(run: &WorkflowRunDto, node_run: &WorkflowRunNodeDto) -> bool {
    run.nodes.iter().any(|candidate| {
        candidate.node_id == node_run.node_id
            && candidate.attempt_number < node_run.attempt_number
            && (is_failed_status(candidate.status)
                || candidate.status == WorkflowNodeRunStatusDto::Skipped)
    })
}

fn insert_workflow_metric_event(
    repo_root: &std::path::Path,
    project_id: &str,
    run_id: &str,
    node_run_id: Option<&str>,
    metric: &str,
    fields: &JsonValue,
) -> CommandResult<()> {
    project_store::insert_workflow_event(
        repo_root,
        project_id,
        run_id,
        node_run_id,
        "workflow_metric_recorded",
        &json!({
            "metric": metric,
            "fields": fields,
        }),
    )
}

fn output_contract_for_node<'a>(
    definition: &'a WorkflowDefinitionDto,
    node_id: &str,
) -> Option<&'a WorkflowOutputContractDto> {
    find_node(definition, node_id).and_then(WorkflowNodeDto::output_contract)
}

fn find_node<'a>(
    definition: &'a WorkflowDefinitionDto,
    node_id: &str,
) -> Option<&'a WorkflowNodeDto> {
    definition.nodes.iter().find(|node| node.id() == node_id)
}

fn next_attempt_for_node(run: &WorkflowRunDto, node_id: &str) -> u32 {
    run.nodes
        .iter()
        .filter(|node| node.node_id == node_id)
        .map(|node| node.attempt_number)
        .max()
        .map(|attempt| attempt.saturating_add(1))
        .unwrap_or(0)
}

fn checkpoint_type_for_node(
    definition: &WorkflowDefinitionDto,
    node_id: &str,
) -> Option<WorkflowHumanCheckpointTypeDto> {
    match find_node(definition, node_id)? {
        WorkflowNodeDto::HumanCheckpoint {
            checkpoint_type, ..
        } => Some(*checkpoint_type),
        _ => None,
    }
}

fn pause_at_checkpoint(
    repo_root: &std::path::Path,
    project_id: &str,
    run: &WorkflowRunDto,
    node_run: &WorkflowRunNodeDto,
    reason: &str,
) -> CommandResult<()> {
    project_store::update_workflow_run_node(
        repo_root,
        project_id,
        &node_run.id,
        WorkflowNodeRunStatusDto::WaitingOnGate,
        None,
        None,
        None,
    )?;
    project_store::update_workflow_run_status(
        repo_root,
        project_id,
        &run.id,
        WorkflowRunStatusDto::Paused,
        Some(WorkflowTerminalStatusDto::NeedsHuman),
        None,
    )?;
    project_store::insert_workflow_event(
        repo_root,
        project_id,
        &run.id,
        Some(&node_run.id),
        "workflow_paused",
        &json!({ "reason": reason, "nodeId": node_run.node_id }),
    )?;
    insert_workflow_metric_event(
        repo_root,
        project_id,
        &run.id,
        Some(&node_run.id),
        "checkpoint_pause",
        &json!({
            "reason": reason,
            "nodeId": node_run.node_id,
        }),
    )
}

fn complete_for_terminal(
    repo_root: &std::path::Path,
    project_id: &str,
    run: &WorkflowRunDto,
    terminal_status: WorkflowTerminalStatusDto,
) -> CommandResult<()> {
    let run_status = match terminal_status {
        WorkflowTerminalStatusDto::Success => WorkflowRunStatusDto::Completed,
        WorkflowTerminalStatusDto::Failure => WorkflowRunStatusDto::Failed,
        WorkflowTerminalStatusDto::Cancelled => WorkflowRunStatusDto::Cancelled,
        WorkflowTerminalStatusDto::NeedsHuman => WorkflowRunStatusDto::Paused,
    };
    project_store::update_workflow_run_status(
        repo_root,
        project_id,
        &run.id,
        run_status,
        Some(terminal_status),
        None,
    )?;
    project_store::insert_workflow_event(
        repo_root,
        project_id,
        &run.id,
        None,
        "workflow_completed",
        &json!({ "terminalStatus": terminal_status.as_str() }),
    )
}

fn is_terminal_run(status: WorkflowRunStatusDto) -> bool {
    matches!(
        status,
        WorkflowRunStatusDto::Completed
            | WorkflowRunStatusDto::Failed
            | WorkflowRunStatusDto::Cancelled
    )
}

fn edge_applies_to_node_status(
    edge_type: WorkflowEdgeTypeDto,
    node_status: WorkflowNodeRunStatusDto,
) -> bool {
    match edge_type {
        WorkflowEdgeTypeDto::Success => node_status == WorkflowNodeRunStatusDto::Succeeded,
        WorkflowEdgeTypeDto::Failure => matches!(
            node_status,
            WorkflowNodeRunStatusDto::Failed
                | WorkflowNodeRunStatusDto::Stalled
                | WorkflowNodeRunStatusDto::Cancelled
        ),
        WorkflowEdgeTypeDto::Recovery => matches!(
            node_status,
            WorkflowNodeRunStatusDto::Failed | WorkflowNodeRunStatusDto::Stalled
        ),
        WorkflowEdgeTypeDto::Conditional
        | WorkflowEdgeTypeDto::Loop
        | WorkflowEdgeTypeDto::ManualOverride => true,
    }
}

fn routes_single_match(node: &WorkflowNodeDto) -> bool {
    matches!(
        node,
        WorkflowNodeDto::Router { .. }
            | WorkflowNodeDto::Gate { .. }
            | WorkflowNodeDto::HumanCheckpoint { .. }
    )
}

fn has_routed_node_run(run: &WorkflowRunDto, node_run: &WorkflowRunNodeDto) -> bool {
    if has_control_event_after_completion(run, node_run, "workflow_node_retry_requested") {
        return true;
    }
    run.edge_decisions.iter().any(|decision| {
        if decision.from_node_id != node_run.node_id {
            return false;
        }
        node_run
            .completed_at
            .as_ref()
            .map(|completed_at| decision.created_at >= *completed_at)
            .unwrap_or(true)
    })
}

fn has_node_event(run: &WorkflowRunDto, node_run: &WorkflowRunNodeDto, event_type: &str) -> bool {
    run.events.iter().any(|event| {
        event.node_run_id.as_deref() == Some(node_run.id.as_str()) && event.event_type == event_type
    })
}

fn has_metric_event_for_node(
    run: &WorkflowRunDto,
    node_run: &WorkflowRunNodeDto,
    metric: &str,
) -> bool {
    run.events.iter().any(|event| {
        event.node_run_id.as_deref() == Some(node_run.id.as_str())
            && event.event_type == "workflow_metric_recorded"
            && event.event.get("metric").and_then(JsonValue::as_str) == Some(metric)
    })
}

fn has_control_event_after_completion(
    run: &WorkflowRunDto,
    node_run: &WorkflowRunNodeDto,
    event_type: &str,
) -> bool {
    run.events.iter().any(|event| {
        event.node_run_id.as_deref() == Some(node_run.id.as_str())
            && event.event_type == event_type
            && node_run
                .completed_at
                .as_ref()
                .map(|completed_at| event.created_at >= *completed_at)
                .unwrap_or(true)
    })
}

fn is_retryable_node_status(status: WorkflowNodeRunStatusDto) -> bool {
    matches!(
        status,
        WorkflowNodeRunStatusDto::Failed
            | WorkflowNodeRunStatusDto::Stalled
            | WorkflowNodeRunStatusDto::Skipped
            | WorkflowNodeRunStatusDto::Cancelled
    )
}

fn is_skippable_node_status(status: WorkflowNodeRunStatusDto) -> bool {
    matches!(
        status,
        WorkflowNodeRunStatusDto::Pending
            | WorkflowNodeRunStatusDto::Eligible
            | WorkflowNodeRunStatusDto::Starting
            | WorkflowNodeRunStatusDto::Running
            | WorkflowNodeRunStatusDto::WaitingOnGate
    )
}

#[cfg(test)]
#[derive(Debug, Default, PartialEq, Eq)]
struct WorkflowEventReplaySummary {
    edge_evaluations: usize,
    node_start_requests: usize,
    resource_conflict_waits: usize,
    loop_exhaustions: usize,
    checkpoint_pauses: usize,
    recovery_successes: usize,
}

#[cfg(test)]
fn replay_workflow_events(run: &WorkflowRunDto) -> WorkflowEventReplaySummary {
    let mut summary = WorkflowEventReplaySummary::default();
    for event in &run.events {
        match event.event_type.as_str() {
            "workflow_edge_evaluated" => summary.edge_evaluations += 1,
            "workflow_agent_start_requested" => summary.node_start_requests += 1,
            "workflow_resource_conflict_wait" => summary.resource_conflict_waits += 1,
            "workflow_metric_recorded" => {
                match event.event.get("metric").and_then(JsonValue::as_str) {
                    Some("loop_exhaustion") => summary.loop_exhaustions += 1,
                    Some("checkpoint_pause") => summary.checkpoint_pauses += 1,
                    Some("recovery_success") => summary.recovery_successes += 1,
                    _ => {}
                }
            }
            _ => {}
        }
    }
    summary
}

#[derive(Debug, PartialEq, Eq)]
enum MergeEvaluation {
    Waiting,
    Succeeded,
    Failed(&'static str),
}

fn evaluate_merge_node(
    run: &WorkflowRunDto,
    node_run: &WorkflowRunNodeDto,
    wait_policy: WorkflowMergeWaitPolicyDto,
    quorum: Option<u32>,
    fail_fast: bool,
) -> MergeEvaluation {
    let incoming_sources = run
        .definition_snapshot
        .edges
        .iter()
        .filter(|edge| edge.to_node_id == node_run.node_id)
        .map(|edge| edge.from_node_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if incoming_sources.is_empty() {
        return MergeEvaluation::Succeeded;
    }

    let statuses = incoming_sources
        .iter()
        .filter_map(|node_id| latest_status_for_node(run, node_id))
        .collect::<Vec<_>>();
    let finished_count = statuses
        .iter()
        .filter(|status| is_finished_status(**status))
        .count();
    let succeeded_count = statuses
        .iter()
        .filter(|status| **status == WorkflowNodeRunStatusDto::Succeeded)
        .count();
    let skipped_count = statuses
        .iter()
        .filter(|status| **status == WorkflowNodeRunStatusDto::Skipped)
        .count();
    let failed_count = statuses
        .iter()
        .filter(|status| is_failed_status(**status))
        .count();
    let resolved_without_failure_count = succeeded_count + skipped_count;
    let expected_count = incoming_sources.len();

    if fail_fast && failed_count > 0 {
        return MergeEvaluation::Failed("merge_branch_failed");
    }

    match wait_policy {
        WorkflowMergeWaitPolicyDto::Any => {
            if succeeded_count > 0 {
                MergeEvaluation::Succeeded
            } else if finished_count == expected_count {
                MergeEvaluation::Failed("merge_no_successful_branch")
            } else {
                MergeEvaluation::Waiting
            }
        }
        WorkflowMergeWaitPolicyDto::Quorum => {
            let required = quorum.unwrap_or(expected_count as u32).max(1) as usize;
            if succeeded_count >= required {
                MergeEvaluation::Succeeded
            } else if finished_count == expected_count {
                MergeEvaluation::Failed("merge_quorum_not_met")
            } else {
                MergeEvaluation::Waiting
            }
        }
        WorkflowMergeWaitPolicyDto::FailFast => {
            if failed_count > 0 {
                MergeEvaluation::Failed("merge_branch_failed")
            } else if resolved_without_failure_count == expected_count && succeeded_count > 0 {
                MergeEvaluation::Succeeded
            } else if finished_count == expected_count {
                MergeEvaluation::Failed("merge_no_successful_branch")
            } else {
                MergeEvaluation::Waiting
            }
        }
        WorkflowMergeWaitPolicyDto::All => {
            if failed_count > 0 && finished_count == expected_count {
                MergeEvaluation::Failed("merge_branch_failed")
            } else if resolved_without_failure_count == expected_count && succeeded_count > 0 {
                MergeEvaluation::Succeeded
            } else if finished_count == expected_count {
                MergeEvaluation::Failed("merge_no_successful_branch")
            } else {
                MergeEvaluation::Waiting
            }
        }
    }
}

fn latest_status_for_node(run: &WorkflowRunDto, node_id: &str) -> Option<WorkflowNodeRunStatusDto> {
    run.nodes
        .iter()
        .filter(|node| node.node_id == node_id)
        .max_by_key(|node| node.attempt_number)
        .map(|node| node.status)
}

fn is_finished_status(status: WorkflowNodeRunStatusDto) -> bool {
    matches!(
        status,
        WorkflowNodeRunStatusDto::Succeeded
            | WorkflowNodeRunStatusDto::Failed
            | WorkflowNodeRunStatusDto::Stalled
            | WorkflowNodeRunStatusDto::Skipped
            | WorkflowNodeRunStatusDto::Cancelled
    )
}

fn is_failed_status(status: WorkflowNodeRunStatusDto) -> bool {
    matches!(
        status,
        WorkflowNodeRunStatusDto::Failed
            | WorkflowNodeRunStatusDto::Stalled
            | WorkflowNodeRunStatusDto::Cancelled
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::contracts::{
            runtime::RuntimeAgentIdDto,
            workflows::{
                WorkflowArtifactRecordDto, WorkflowConditionDto, WorkflowEventDto,
                WorkflowFailureClassificationPolicyDto, WorkflowLoopPolicyDto,
                WorkflowResourceConflictModeDto, WorkflowResourceConflictPolicyDto,
                WorkflowRunPolicyDto, WorkflowStallDetectorDto,
            },
        },
        db::{
            configure_connection, migrations::migrations, project_store,
            register_project_database_path_for_tests,
        },
    };
    use rusqlite::Connection;
    use tempfile::TempDir;

    const NOW: &str = "2026-01-01T00:00:00Z";

    fn terminal_node(id: &str) -> WorkflowNodeDto {
        WorkflowNodeDto::Terminal {
            id: id.into(),
            title: id.into(),
            description: String::new(),
            position: Default::default(),
            terminal_status: WorkflowTerminalStatusDto::Success,
        }
    }

    fn agent_node(id: &str, resource_scopes: Vec<String>) -> WorkflowNodeDto {
        WorkflowNodeDto::Agent {
            id: id.into(),
            title: id.into(),
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
            resource_scopes,
            failure_policy: Default::default(),
        }
    }

    fn merge_node() -> WorkflowNodeDto {
        WorkflowNodeDto::Merge {
            id: "merge".into(),
            title: "Merge".into(),
            description: String::new(),
            position: Default::default(),
            wait_policy: WorkflowMergeWaitPolicyDto::All,
            quorum: None,
            fail_fast: false,
        }
    }

    fn edge(
        id: &str,
        from_node_id: &str,
        to_node_id: &str,
        edge_type: WorkflowEdgeTypeDto,
    ) -> WorkflowEdgeDto {
        WorkflowEdgeDto {
            id: id.into(),
            from_node_id: from_node_id.into(),
            to_node_id: to_node_id.into(),
            r#type: edge_type,
            label: String::new(),
            priority: 10,
            condition: WorkflowConditionDto::Always,
            loop_policy: None,
        }
    }

    fn definition_with_edges(edges: Vec<WorkflowEdgeDto>) -> WorkflowDefinitionDto {
        WorkflowDefinitionDto {
            schema: "xero.workflow_definition.v1".into(),
            id: "workflow-1".into(),
            project_id: "project-1".into(),
            name: "Workflow".into(),
            description: String::new(),
            version: 1,
            start_node_id: "source-a".into(),
            nodes: vec![
                terminal_node("source-a"),
                terminal_node("source-b"),
                terminal_node("source-c"),
                merge_node(),
                terminal_node("done"),
            ],
            edges,
            artifact_contracts: Vec::new(),
            run_policy: WorkflowRunPolicyDto::default(),
            created_at: None,
            updated_at: None,
        }
    }

    fn node_run(
        node_id: &str,
        status: WorkflowNodeRunStatusDto,
        attempt_number: u32,
    ) -> WorkflowRunNodeDto {
        WorkflowRunNodeDto {
            id: format!("run-1:node:{node_id}:attempt:{attempt_number}"),
            workflow_run_id: "run-1".into(),
            node_id: node_id.into(),
            node_type: if node_id == "merge" {
                "merge".into()
            } else {
                "terminal".into()
            },
            status,
            attempt_number,
            runtime_run_id: None,
            agent_session_id: None,
            failure_class: None,
            started_at: None,
            updated_at: NOW.into(),
            completed_at: is_finished_status(status).then(|| NOW.into()),
            idempotency_key: format!("run-1:{node_id}:{attempt_number}"),
        }
    }

    fn artifact_for_node_run(node_run_id: &str, payload: JsonValue) -> WorkflowArtifactRecordDto {
        WorkflowArtifactRecordDto {
            id: format!("artifact-{node_run_id}"),
            workflow_run_id: "run-1".into(),
            producer_node_run_id: node_run_id.into(),
            artifact_type: "review_findings".into(),
            schema_version: 1,
            payload,
            render_text: None,
            created_at: NOW.into(),
        }
    }

    fn workflow_event(event_type: &str, event: JsonValue) -> WorkflowEventDto {
        WorkflowEventDto {
            id: format!("event-{event_type}"),
            workflow_run_id: "run-1".into(),
            node_run_id: None,
            event_type: event_type.into(),
            event,
            created_at: NOW.into(),
        }
    }

    fn run_with_nodes(
        edges: Vec<WorkflowEdgeDto>,
        nodes: Vec<WorkflowRunNodeDto>,
    ) -> WorkflowRunDto {
        run_with_definition(definition_with_edges(edges), nodes)
    }

    fn run_with_definition(
        definition: WorkflowDefinitionDto,
        nodes: Vec<WorkflowRunNodeDto>,
    ) -> WorkflowRunDto {
        WorkflowRunDto {
            id: "run-1".into(),
            project_id: "project-1".into(),
            workflow_version_id: "workflow-version-1".into(),
            workflow_id: definition.id.clone(),
            workflow_version_number: 1,
            status: WorkflowRunStatusDto::Running,
            terminal_status: None,
            definition_snapshot: definition,
            initial_input: None,
            started_at: NOW.into(),
            updated_at: NOW.into(),
            completed_at: None,
            cancellation_reason: None,
            nodes,
            edge_decisions: Vec::new(),
            artifacts: Vec::new(),
            gate_decisions: Vec::new(),
            loop_attempts: Vec::new(),
            events: Vec::new(),
        }
    }

    fn run_for_merge(
        sources: &[&str],
        node_statuses: Vec<(&str, WorkflowNodeRunStatusDto)>,
    ) -> (WorkflowRunDto, WorkflowRunNodeDto) {
        let merge_run = node_run("merge", WorkflowNodeRunStatusDto::Eligible, 0);
        let mut nodes = node_statuses
            .into_iter()
            .map(|(node_id, status)| node_run(node_id, status, 0))
            .collect::<Vec<_>>();
        nodes.push(merge_run.clone());

        let edges = sources
            .iter()
            .map(|source| {
                edge(
                    &format!("edge-{source}-merge"),
                    source,
                    "merge",
                    WorkflowEdgeTypeDto::Success,
                )
            })
            .collect::<Vec<_>>();

        (run_with_nodes(edges, nodes), merge_run)
    }

    #[test]
    fn merge_all_waits_until_every_incoming_source_finishes_successfully() {
        let (run, merge_run) = run_for_merge(
            &["source-a", "source-b"],
            vec![
                ("source-a", WorkflowNodeRunStatusDto::Succeeded),
                ("source-b", WorkflowNodeRunStatusDto::Running),
            ],
        );
        assert_eq!(
            evaluate_merge_node(
                &run,
                &merge_run,
                WorkflowMergeWaitPolicyDto::All,
                None,
                false,
            ),
            MergeEvaluation::Waiting
        );

        let (run, merge_run) = run_for_merge(
            &["source-a", "source-b"],
            vec![
                ("source-a", WorkflowNodeRunStatusDto::Succeeded),
                ("source-b", WorkflowNodeRunStatusDto::Succeeded),
            ],
        );
        assert_eq!(
            evaluate_merge_node(
                &run,
                &merge_run,
                WorkflowMergeWaitPolicyDto::All,
                None,
                false,
            ),
            MergeEvaluation::Succeeded
        );

        let (run, merge_run) = run_for_merge(
            &["source-a", "source-b"],
            vec![
                ("source-a", WorkflowNodeRunStatusDto::Succeeded),
                ("source-b", WorkflowNodeRunStatusDto::Failed),
            ],
        );
        assert_eq!(
            evaluate_merge_node(
                &run,
                &merge_run,
                WorkflowMergeWaitPolicyDto::All,
                None,
                false,
            ),
            MergeEvaluation::Failed("merge_branch_failed")
        );
    }

    #[test]
    fn merge_any_succeeds_on_first_successful_branch() {
        let (run, merge_run) = run_for_merge(
            &["source-a", "source-b"],
            vec![
                ("source-a", WorkflowNodeRunStatusDto::Succeeded),
                ("source-b", WorkflowNodeRunStatusDto::Running),
            ],
        );

        assert_eq!(
            evaluate_merge_node(
                &run,
                &merge_run,
                WorkflowMergeWaitPolicyDto::Any,
                None,
                false,
            ),
            MergeEvaluation::Succeeded
        );
    }

    #[test]
    fn merge_all_treats_skipped_branches_as_resolved_not_successful() {
        let (run, merge_run) = run_for_merge(
            &["source-a", "source-b"],
            vec![
                ("source-a", WorkflowNodeRunStatusDto::Succeeded),
                ("source-b", WorkflowNodeRunStatusDto::Skipped),
            ],
        );
        assert_eq!(
            evaluate_merge_node(
                &run,
                &merge_run,
                WorkflowMergeWaitPolicyDto::All,
                None,
                false,
            ),
            MergeEvaluation::Succeeded
        );

        let (run, merge_run) = run_for_merge(
            &["source-a", "source-b"],
            vec![
                ("source-a", WorkflowNodeRunStatusDto::Skipped),
                ("source-b", WorkflowNodeRunStatusDto::Skipped),
            ],
        );
        assert_eq!(
            evaluate_merge_node(
                &run,
                &merge_run,
                WorkflowMergeWaitPolicyDto::All,
                None,
                false,
            ),
            MergeEvaluation::Failed("merge_no_successful_branch")
        );
    }

    #[test]
    fn resource_conflict_policy_serializes_declared_scopes() {
        let mut definition = definition_with_edges(Vec::new());
        definition.nodes = vec![
            agent_node("agent-a", vec!["repo".into(), "src/lib.rs".into()]),
            agent_node("agent-b", vec!["src/lib.rs".into()]),
        ];
        definition.run_policy.concurrency_limit = 2;
        definition.run_policy.resource_conflict_policy = WorkflowResourceConflictPolicyDto {
            mode: WorkflowResourceConflictModeDto::SerializeConflicts,
            default_scopes: Vec::new(),
        };
        let eligible = node_run("agent-b", WorkflowNodeRunStatusDto::Eligible, 0);
        let run = run_with_definition(
            definition.clone(),
            vec![
                node_run("agent-a", WorkflowNodeRunStatusDto::Running, 0),
                eligible.clone(),
            ],
        );
        let conflict = resource_conflict_for_node(
            &run,
            &eligible,
            find_node(&definition, "agent-b").expect("agent-b exists"),
        )
        .expect("conflict exists");

        assert_eq!(conflict.node_id, "agent-a");
        assert_eq!(conflict.scopes, vec!["src/lib.rs".to_string()]);

        let mut allowed_definition = definition;
        allowed_definition.run_policy.resource_conflict_policy.mode =
            WorkflowResourceConflictModeDto::AllowConflicts;
        let allowed_run = run_with_definition(
            allowed_definition.clone(),
            vec![
                node_run("agent-a", WorkflowNodeRunStatusDto::Running, 0),
                eligible.clone(),
            ],
        );
        assert!(resource_conflict_for_node(
            &allowed_run,
            &eligible,
            find_node(&allowed_definition, "agent-b").expect("agent-b exists"),
        )
        .is_none());
    }

    #[test]
    fn merge_quorum_requires_configured_success_count() {
        let (run, merge_run) = run_for_merge(
            &["source-a", "source-b", "source-c"],
            vec![
                ("source-a", WorkflowNodeRunStatusDto::Succeeded),
                ("source-b", WorkflowNodeRunStatusDto::Succeeded),
                ("source-c", WorkflowNodeRunStatusDto::Running),
            ],
        );
        assert_eq!(
            evaluate_merge_node(
                &run,
                &merge_run,
                WorkflowMergeWaitPolicyDto::Quorum,
                Some(2),
                false,
            ),
            MergeEvaluation::Succeeded
        );

        let (run, merge_run) = run_for_merge(
            &["source-a", "source-b", "source-c"],
            vec![
                ("source-a", WorkflowNodeRunStatusDto::Succeeded),
                ("source-b", WorkflowNodeRunStatusDto::Failed),
                ("source-c", WorkflowNodeRunStatusDto::Cancelled),
            ],
        );
        assert_eq!(
            evaluate_merge_node(
                &run,
                &merge_run,
                WorkflowMergeWaitPolicyDto::Quorum,
                Some(2),
                false,
            ),
            MergeEvaluation::Failed("merge_quorum_not_met")
        );
    }

    #[test]
    fn merge_fail_fast_fails_before_all_sources_finish() {
        let (run, merge_run) = run_for_merge(
            &["source-a", "source-b"],
            vec![
                ("source-a", WorkflowNodeRunStatusDto::Failed),
                ("source-b", WorkflowNodeRunStatusDto::Running),
            ],
        );

        assert_eq!(
            evaluate_merge_node(
                &run,
                &merge_run,
                WorkflowMergeWaitPolicyDto::All,
                None,
                true,
            ),
            MergeEvaluation::Failed("merge_branch_failed")
        );
    }

    #[test]
    fn edge_status_routing_matches_terminal_status_semantics() {
        assert!(edge_applies_to_node_status(
            WorkflowEdgeTypeDto::Success,
            WorkflowNodeRunStatusDto::Succeeded
        ));
        assert!(!edge_applies_to_node_status(
            WorkflowEdgeTypeDto::Success,
            WorkflowNodeRunStatusDto::Failed
        ));

        for status in [
            WorkflowNodeRunStatusDto::Failed,
            WorkflowNodeRunStatusDto::Stalled,
            WorkflowNodeRunStatusDto::Cancelled,
        ] {
            assert!(edge_applies_to_node_status(
                WorkflowEdgeTypeDto::Failure,
                status
            ));
        }
        assert!(!edge_applies_to_node_status(
            WorkflowEdgeTypeDto::Failure,
            WorkflowNodeRunStatusDto::Succeeded
        ));

        assert!(edge_applies_to_node_status(
            WorkflowEdgeTypeDto::Recovery,
            WorkflowNodeRunStatusDto::Failed
        ));
        assert!(edge_applies_to_node_status(
            WorkflowEdgeTypeDto::Recovery,
            WorkflowNodeRunStatusDto::Stalled
        ));
        assert!(!edge_applies_to_node_status(
            WorkflowEdgeTypeDto::Recovery,
            WorkflowNodeRunStatusDto::Cancelled
        ));

        for edge_type in [
            WorkflowEdgeTypeDto::Conditional,
            WorkflowEdgeTypeDto::Loop,
            WorkflowEdgeTypeDto::ManualOverride,
        ] {
            assert!(edge_applies_to_node_status(
                edge_type,
                WorkflowNodeRunStatusDto::Pending
            ));
        }
    }

    #[test]
    fn activity_timeout_prefers_agent_policy_over_run_policy() {
        let mut definition = definition_with_edges(Vec::new());
        definition.run_policy.node_timeout_seconds = Some(60);
        definition.nodes.push(WorkflowNodeDto::Agent {
            id: "agent".into(),
            title: "Agent".into(),
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
            failure_policy: WorkflowFailureClassificationPolicyDto {
                runtime_activity_timeout_seconds: Some(5),
                ..WorkflowFailureClassificationPolicyDto::default()
            },
        });

        assert_eq!(
            activity_timeout_seconds_for_node(&definition, "agent"),
            Some(5)
        );
        assert_eq!(
            activity_timeout_seconds_for_node(&definition, "source-a"),
            None
        );
    }

    #[test]
    fn stale_agent_activity_uses_latest_runtime_activity_timestamp() {
        let now = OffsetDateTime::parse("2026-01-01T00:10:00Z", &Rfc3339).expect("parse now");
        let recent_heartbeat = agent_run_record(
            "2026-01-01T00:00:00Z",
            Some("2026-01-01T00:09:00Z"),
            "2026-01-01T00:01:00Z",
        );
        assert_eq!(stale_agent_activity_at(&recent_heartbeat, 120, now), None);

        let stale_heartbeat = agent_run_record(
            "2026-01-01T00:00:00Z",
            Some("2026-01-01T00:07:00Z"),
            "2026-01-01T00:01:00Z",
        );
        assert_eq!(
            stale_agent_activity_at(&stale_heartbeat, 120, now),
            Some("2026-01-01T00:07:00Z")
        );
    }

    #[test]
    fn stall_detectors_classify_repeated_failures_missing_artifacts_and_flat_findings() {
        let first_failed = WorkflowRunNodeDto {
            failure_class: Some("tool_retry_limit".into()),
            ..node_run("source-a", WorkflowNodeRunStatusDto::Failed, 0)
        };
        let repeated_failed = WorkflowRunNodeDto {
            failure_class: Some("tool_retry_limit".into()),
            ..node_run("source-a", WorkflowNodeRunStatusDto::Failed, 1)
        };
        let run = run_with_nodes(Vec::new(), vec![first_failed, repeated_failed.clone()]);
        assert_eq!(
            stall_failure_class_for_detector(
                &run,
                &repeated_failed,
                WorkflowStallDetectorDto::SameFailureClassRepeated,
            ),
            Some("same_failure_class_repeated")
        );

        let missing_artifact = node_run("agent", WorkflowNodeRunStatusDto::Succeeded, 0);
        let mut definition = definition_with_edges(Vec::new());
        definition.nodes.push(agent_node("agent", Vec::new()));
        let run = run_with_definition(definition, vec![missing_artifact.clone()]);
        assert_eq!(
            stall_failure_class_for_detector(
                &run,
                &missing_artifact,
                WorkflowStallDetectorDto::NoArtifactProgress,
            ),
            Some("no_artifact_progress")
        );

        let previous_review = node_run("source-a", WorkflowNodeRunStatusDto::Succeeded, 0);
        let current_review = node_run("source-a", WorkflowNodeRunStatusDto::Succeeded, 1);
        let mut run = run_with_nodes(
            Vec::new(),
            vec![previous_review.clone(), current_review.clone()],
        );
        run.artifacts = vec![
            artifact_for_node_run(
                &previous_review.id,
                json!({ "findings": { "high_count": 2 } }),
            ),
            artifact_for_node_run(
                &current_review.id,
                json!({ "findings": { "high_count": 2 } }),
            ),
        ];
        assert_eq!(
            stall_failure_class_for_detector(
                &run,
                &current_review,
                WorkflowStallDetectorDto::FindingCountNotDecreasing,
            ),
            Some("finding_count_not_decreasing")
        );
    }

    fn agent_run_record(
        started_at: &str,
        last_heartbeat_at: Option<&str>,
        updated_at: &str,
    ) -> AgentRunRecord {
        AgentRunRecord {
            runtime_agent_id: RuntimeAgentIdDto::Engineer,
            agent_definition_id: "agent-definition".into(),
            agent_definition_version: 1,
            project_id: "project-1".into(),
            agent_session_id: "session-1".into(),
            run_id: "runtime-run-1".into(),
            trace_id: "trace-1".into(),
            lineage_kind: "root".into(),
            parent_run_id: None,
            parent_trace_id: None,
            parent_subagent_id: None,
            subagent_role: None,
            provider_id: "provider".into(),
            model_id: "model".into(),
            status: AgentRunStatus::Running,
            prompt: "prompt".into(),
            system_prompt: "system".into(),
            started_at: started_at.into(),
            last_heartbeat_at: last_heartbeat_at.map(ToOwned::to_owned),
            completed_at: None,
            cancelled_at: None,
            last_error: None,
            updated_at: updated_at.into(),
        }
    }

    #[test]
    fn latest_status_for_node_uses_highest_attempt() {
        let merge_run = node_run("merge", WorkflowNodeRunStatusDto::Eligible, 0);
        let run = run_with_nodes(
            vec![edge(
                "edge-source-a-merge",
                "source-a",
                "merge",
                WorkflowEdgeTypeDto::Success,
            )],
            vec![
                node_run("source-a", WorkflowNodeRunStatusDto::Failed, 0),
                node_run("source-a", WorkflowNodeRunStatusDto::Succeeded, 1),
                merge_run,
            ],
        );

        assert_eq!(
            latest_status_for_node(&run, "source-a"),
            Some(WorkflowNodeRunStatusDto::Succeeded)
        );
    }

    #[test]
    fn event_replay_reconstructs_workflow_observability_counts() {
        let mut run = run_with_nodes(Vec::new(), Vec::new());
        run.events = vec![
            workflow_event("workflow_edge_evaluated", json!({ "matched": true })),
            workflow_event(
                "workflow_agent_start_requested",
                json!({ "nodeId": "agent-a" }),
            ),
            workflow_event(
                "workflow_resource_conflict_wait",
                json!({ "nodeId": "agent-b" }),
            ),
            workflow_event(
                "workflow_metric_recorded",
                json!({ "metric": "loop_exhaustion" }),
            ),
            workflow_event(
                "workflow_metric_recorded",
                json!({ "metric": "checkpoint_pause" }),
            ),
            workflow_event(
                "workflow_metric_recorded",
                json!({ "metric": "recovery_success" }),
            ),
        ];

        assert_eq!(
            replay_workflow_events(&run),
            WorkflowEventReplaySummary {
                edge_evaluations: 1,
                node_start_requests: 1,
                resource_conflict_waits: 1,
                loop_exhaustions: 1,
                checkpoint_pauses: 1,
                recovery_successes: 1,
            }
        );
    }

    fn repo_with_database() -> TempDir {
        let temp = TempDir::new().expect("create temp repo");
        let database_path = temp.path().join("state.db");
        register_project_database_path_for_tests(temp.path(), database_path.clone());
        let mut connection = Connection::open(&database_path).expect("open project db");
        configure_connection(&connection).expect("configure project db");
        migrations()
            .to_latest(&mut connection)
            .expect("migrate project db");
        connection
            .execute(
                r#"
                INSERT INTO projects (
                    id,
                    name,
                    description,
                    milestone,
                    total_phases,
                    completed_phases,
                    active_phase,
                    branch,
                    created_at,
                    updated_at
                )
                VALUES ('project-1', 'Project', '', '', 0, 0, 0, 'main', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
                "#,
                [],
            )
            .expect("seed project");
        temp
    }

    #[test]
    fn exhausted_loop_routes_to_fallback_and_records_exhaustion() {
        let temp = repo_with_database();
        let mut retry_edge = edge(
            "edge-retry",
            "source-a",
            "source-b",
            WorkflowEdgeTypeDto::Loop,
        );
        retry_edge.loop_policy = Some(WorkflowLoopPolicyDto {
            loop_key: "retry".into(),
            max_attempts: 1,
            attempt_scope: Default::default(),
            carryover_policy: Default::default(),
            selected_artifact_refs: Vec::new(),
            reset_policy: Default::default(),
            stall_detector: None,
            on_exhausted: "done".into(),
        });

        let created = project_store::create_workflow_definition(
            temp.path(),
            &definition_with_edges(vec![retry_edge.clone()]),
        )
        .expect("create workflow");
        let run = project_store::create_workflow_run(temp.path(), "project-1", &created.id, None)
            .expect("create run");
        let source_node_run = project_store::insert_workflow_run_node(
            temp.path(),
            "project-1",
            &run.id,
            "source-a",
            "terminal",
            0,
            WorkflowNodeRunStatusDto::Succeeded,
            "run-1:source-a:0",
        )
        .expect("insert source node run");
        project_store::increment_workflow_loop_attempt(
            temp.path(),
            "project-1",
            &run.id,
            "retry",
            &source_node_run.id,
            false,
        )
        .expect("seed first loop attempt");
        let loaded_run = project_store::get_workflow_run(temp.path(), "project-1", &run.id)
            .expect("load run")
            .expect("run exists");

        let target_node_id = loop_target_for_edge(
            temp.path(),
            "project-1",
            &loaded_run,
            &source_node_run,
            &retry_edge,
        )
        .expect("resolve exhausted loop target");

        assert_eq!(target_node_id, "done");
        let reloaded_run = project_store::get_workflow_run(temp.path(), "project-1", &run.id)
            .expect("reload run")
            .expect("run exists");
        let retry_attempt = reloaded_run
            .loop_attempts
            .iter()
            .find(|attempt| attempt.loop_key == "retry")
            .expect("retry attempt exists");
        assert_eq!(retry_attempt.attempt_count, 2);
        assert!(retry_attempt.exhausted);
        let replay = replay_workflow_events(&reloaded_run);
        assert_eq!(replay.loop_exhaustions, 1);
    }
}
