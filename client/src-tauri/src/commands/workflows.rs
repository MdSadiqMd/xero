use tauri::{AppHandle, Runtime, State};

use crate::{
    commands::{
        contracts::workflows::{
            CancelWorkflowRunRequestDto, CreateWorkflowDefinitionRequestDto,
            GetWorkflowDefinitionRequestDto, GetWorkflowRunRequestDto,
            ListWorkflowDefinitionsRequestDto, ListWorkflowDefinitionsResponseDto,
            ListWorkflowRunsRequestDto, ListWorkflowRunsResponseDto,
            ResumeWorkflowCheckpointRequestDto, RetryWorkflowNodeRunRequestDto,
            SkipWorkflowBranchRequestDto, StartWorkflowRunRequestDto,
            UpdateWorkflowDefinitionRequestDto, WorkflowDefinitionResponseDto,
            WorkflowRunResponseDto, WorkflowRunStatusDto, WorkflowTerminalStatusDto,
            WorkflowValidationReportDto,
        },
        runtime_support::resolve_project_root,
        validate_non_empty, CommandError, CommandResult,
    },
    db::project_store,
    runtime::{workflow_orchestrator, DesktopAgentCoreRuntime},
    state::DesktopState,
};

#[tauri::command]
pub fn validate_workflow_definition(
    request: CreateWorkflowDefinitionRequestDto,
) -> CommandResult<WorkflowValidationReportDto> {
    Ok(workflow_orchestrator::validate_workflow_definition(
        &request.definition,
    ))
}

#[tauri::command]
pub fn create_workflow_definition<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: CreateWorkflowDefinitionRequestDto,
) -> CommandResult<WorkflowDefinitionResponseDto> {
    let report = workflow_orchestrator::validate_workflow_definition(&request.definition);
    if matches!(
        report.status,
        crate::commands::contracts::workflows::WorkflowValidationStatusDto::Invalid
    ) {
        return Err(CommandError::user_fixable(
            "workflow_definition_invalid",
            "Xero refused to save the Workflow because the graph has validation errors.",
        ));
    }
    let repo_root = resolve_project_root(&app, state.inner(), &request.definition.project_id)?;
    let definition = project_store::create_workflow_definition(&repo_root, &request.definition)?;
    Ok(WorkflowDefinitionResponseDto { definition })
}

#[tauri::command]
pub fn update_workflow_definition<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: UpdateWorkflowDefinitionRequestDto,
) -> CommandResult<WorkflowDefinitionResponseDto> {
    validate_non_empty(&request.workflow_id, "workflowId")?;
    let report = workflow_orchestrator::validate_workflow_definition(&request.definition);
    if matches!(
        report.status,
        crate::commands::contracts::workflows::WorkflowValidationStatusDto::Invalid
    ) {
        return Err(CommandError::user_fixable(
            "workflow_definition_invalid",
            "Xero refused to save the Workflow because the graph has validation errors.",
        ));
    }
    let repo_root = resolve_project_root(&app, state.inner(), &request.definition.project_id)?;
    let definition = project_store::update_workflow_definition(
        &repo_root,
        &request.workflow_id,
        &request.definition,
    )?;
    Ok(WorkflowDefinitionResponseDto { definition })
}

#[tauri::command]
pub fn list_workflow_definitions<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: ListWorkflowDefinitionsRequestDto,
) -> CommandResult<ListWorkflowDefinitionsResponseDto> {
    validate_non_empty(&request.project_id, "projectId")?;
    let repo_root = resolve_project_root(&app, state.inner(), &request.project_id)?;
    Ok(ListWorkflowDefinitionsResponseDto {
        definitions: project_store::list_workflow_definitions(&repo_root, &request.project_id)?,
    })
}

#[tauri::command]
pub fn get_workflow_definition<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: GetWorkflowDefinitionRequestDto,
) -> CommandResult<WorkflowDefinitionResponseDto> {
    validate_non_empty(&request.project_id, "projectId")?;
    validate_non_empty(&request.workflow_id, "workflowId")?;
    let repo_root = resolve_project_root(&app, state.inner(), &request.project_id)?;
    let definition = project_store::get_workflow_definition(
        &repo_root,
        &request.project_id,
        &request.workflow_id,
    )?
    .ok_or_else(|| {
        CommandError::user_fixable(
            "workflow_definition_not_found",
            format!("Xero could not find Workflow `{}`.", request.workflow_id),
        )
    })?;
    Ok(WorkflowDefinitionResponseDto { definition })
}

#[tauri::command]
pub fn start_workflow_run<R: Runtime + 'static>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: StartWorkflowRunRequestDto,
) -> CommandResult<WorkflowRunResponseDto> {
    validate_non_empty(&request.project_id, "projectId")?;
    validate_non_empty(&request.workflow_id, "workflowId")?;
    let repo_root = resolve_project_root(&app, state.inner(), &request.project_id)?;
    let run = project_store::create_workflow_run(
        &repo_root,
        &request.project_id,
        &request.workflow_id,
        request.initial_input,
    )?;
    let run = workflow_orchestrator::reconcile::reconcile_workflow_run(
        &app,
        state.inner(),
        &request.project_id,
        &run.id,
    )?;
    Ok(WorkflowRunResponseDto { run })
}

#[tauri::command]
pub fn get_workflow_run<R: Runtime + 'static>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: GetWorkflowRunRequestDto,
) -> CommandResult<WorkflowRunResponseDto> {
    validate_non_empty(&request.project_id, "projectId")?;
    validate_non_empty(&request.run_id, "runId")?;
    let run = workflow_orchestrator::reconcile::reconcile_workflow_run(
        &app,
        state.inner(),
        &request.project_id,
        &request.run_id,
    )?;
    Ok(WorkflowRunResponseDto { run })
}

#[tauri::command]
pub fn list_workflow_runs<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: ListWorkflowRunsRequestDto,
) -> CommandResult<ListWorkflowRunsResponseDto> {
    validate_non_empty(&request.project_id, "projectId")?;
    let repo_root = resolve_project_root(&app, state.inner(), &request.project_id)?;
    Ok(ListWorkflowRunsResponseDto {
        runs: project_store::list_workflow_runs(
            &repo_root,
            &request.project_id,
            request.workflow_id.as_deref(),
        )?,
    })
}

#[tauri::command]
pub fn cancel_workflow_run<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: CancelWorkflowRunRequestDto,
) -> CommandResult<WorkflowRunResponseDto> {
    validate_non_empty(&request.project_id, "projectId")?;
    validate_non_empty(&request.run_id, "runId")?;
    let repo_root = resolve_project_root(&app, state.inner(), &request.project_id)?;
    let run = project_store::get_workflow_run(&repo_root, &request.project_id, &request.run_id)?
        .ok_or_else(|| {
            CommandError::user_fixable(
                "workflow_run_not_found",
                format!("Xero could not find Workflow run `{}`.", request.run_id),
            )
        })?;

    let runtime = DesktopAgentCoreRuntime::new(state.inner().agent_run_supervisor().clone());
    for node in run.nodes.iter().filter(|node| {
        node.status == crate::commands::contracts::workflows::WorkflowNodeRunStatusDto::Running
    }) {
        if let Some(runtime_run_id) = node.runtime_run_id.as_ref() {
            let _ = runtime.cancel_run(
                repo_root.clone(),
                request.project_id.clone(),
                runtime_run_id.clone(),
            );
        }
        project_store::update_workflow_run_node(
            &repo_root,
            &request.project_id,
            &node.id,
            crate::commands::contracts::workflows::WorkflowNodeRunStatusDto::Cancelled,
            None,
            None,
            Some("cancelled"),
        )?;
    }
    project_store::update_workflow_run_status(
        &repo_root,
        &request.project_id,
        &request.run_id,
        WorkflowRunStatusDto::Cancelled,
        Some(WorkflowTerminalStatusDto::Cancelled),
        request.reason.as_deref(),
    )?;
    let run = project_store::get_workflow_run(&repo_root, &request.project_id, &request.run_id)?
        .ok_or_else(|| {
            CommandError::system_fault(
                "workflow_run_missing_after_cancel",
                format!(
                    "Workflow run `{}` disappeared during cancellation.",
                    request.run_id
                ),
            )
        })?;
    Ok(WorkflowRunResponseDto { run })
}

#[tauri::command]
pub fn retry_workflow_node_run<R: Runtime + 'static>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: RetryWorkflowNodeRunRequestDto,
) -> CommandResult<WorkflowRunResponseDto> {
    validate_non_empty(&request.project_id, "projectId")?;
    validate_non_empty(&request.run_id, "runId")?;
    validate_non_empty(&request.node_run_id, "nodeRunId")?;
    let run = workflow_orchestrator::reconcile::retry_workflow_node_run(
        &app,
        state.inner(),
        &request.project_id,
        &request.run_id,
        &request.node_run_id,
    )?;
    Ok(WorkflowRunResponseDto { run })
}

#[tauri::command]
pub fn skip_workflow_branch<R: Runtime + 'static>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: SkipWorkflowBranchRequestDto,
) -> CommandResult<WorkflowRunResponseDto> {
    validate_non_empty(&request.project_id, "projectId")?;
    validate_non_empty(&request.run_id, "runId")?;
    validate_non_empty(&request.node_run_id, "nodeRunId")?;
    let run = workflow_orchestrator::reconcile::skip_workflow_branch(
        &app,
        state.inner(),
        &request.project_id,
        &request.run_id,
        &request.node_run_id,
        request.reason.as_deref(),
    )?;
    Ok(WorkflowRunResponseDto { run })
}

#[tauri::command]
pub fn resume_workflow_checkpoint<R: Runtime + 'static>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: ResumeWorkflowCheckpointRequestDto,
) -> CommandResult<WorkflowRunResponseDto> {
    validate_non_empty(&request.project_id, "projectId")?;
    validate_non_empty(&request.run_id, "runId")?;
    validate_non_empty(&request.node_run_id, "nodeRunId")?;
    validate_non_empty(&request.decision, "decision")?;
    let run = workflow_orchestrator::reconcile::resume_workflow_checkpoint(
        &app,
        state.inner(),
        &request.project_id,
        &request.run_id,
        &request.node_run_id,
        &request.decision,
        request.payload,
    )?;
    Ok(WorkflowRunResponseDto { run })
}
