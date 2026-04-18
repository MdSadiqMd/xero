use tauri::{AppHandle, Runtime, State};

use crate::{
    commands::{
        validate_non_empty, AutonomousRunStateDto, CommandError, CommandResult,
        StartAutonomousRunRequestDto,
    },
    runtime::{
        launch_detached_runtime_supervisor, resolve_runtime_shell_selection,
        RuntimeSupervisorLaunchRequest,
    },
    state::DesktopState,
};

use super::{
    get_runtime_session::reconcile_runtime_session,
    runtime_support::{
        emit_runtime_run_updated, emit_runtime_run_updated_if_changed, generate_runtime_run_id,
        load_persisted_runtime_run, load_runtime_run_status, load_runtime_session_status,
        resolve_project_root, runtime_run_dto_from_snapshot, sync_autonomous_run_state,
        AutonomousSyncIntent, DEFAULT_RUNTIME_RUN_CONTROL_TIMEOUT,
        DEFAULT_RUNTIME_RUN_STARTUP_TIMEOUT, OPENAI_RUNTIME_KIND,
    },
    start_runtime_run::{ensure_runtime_run_auth_ready, is_reconnectable_runtime_run},
};

#[tauri::command]
pub fn start_autonomous_run<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: StartAutonomousRunRequestDto,
) -> CommandResult<AutonomousRunStateDto> {
    validate_non_empty(&request.project_id, "projectId")?;

    let repo_root = resolve_project_root(&app, state.inner(), &request.project_id)?;
    let before = load_persisted_runtime_run(&repo_root, &request.project_id)?;
    let current = load_runtime_run_status(state.inner(), &repo_root, &request.project_id)?;
    emit_runtime_run_updated_if_changed(&app, &request.project_id, &before, &current)?;

    if let Some(existing) = current
        .as_ref()
        .filter(|snapshot| is_reconnectable_runtime_run(snapshot))
    {
        return sync_autonomous_run_state(
            &repo_root,
            &request.project_id,
            Some(existing),
            AutonomousSyncIntent::DuplicateStart,
        );
    }

    let runtime = load_runtime_session_status(state.inner(), &repo_root, &request.project_id)?;
    let runtime = reconcile_runtime_session(&app, state.inner(), &repo_root, runtime)?;
    ensure_runtime_run_auth_ready(&runtime.phase)?;
    let session_id = runtime.session_id.clone().ok_or_else(|| {
        CommandError::retryable(
            "runtime_run_session_missing",
            "Cadence cannot start an autonomous run until the selected project's authenticated runtime session exposes a stable session id.",
        )
    })?;
    let flow_id = runtime.flow_id.clone();

    let shell = resolve_runtime_shell_selection();
    let launch_repo_root = repo_root.clone();

    let launched = launch_detached_runtime_supervisor(
        state.inner(),
        RuntimeSupervisorLaunchRequest {
            project_id: request.project_id,
            repo_root: launch_repo_root,
            runtime_kind: OPENAI_RUNTIME_KIND.into(),
            run_id: generate_runtime_run_id(),
            session_id,
            flow_id,
            program: shell.program,
            args: shell.args,
            startup_timeout: DEFAULT_RUNTIME_RUN_STARTUP_TIMEOUT,
            control_timeout: DEFAULT_RUNTIME_RUN_CONTROL_TIMEOUT,
            supervisor_binary: state.inner().runtime_supervisor_binary_override().cloned(),
        },
    )?;

    let runtime_run = runtime_run_dto_from_snapshot(&launched);
    emit_runtime_run_updated(&app, Some(&runtime_run))?;

    sync_autonomous_run_state(
        &repo_root,
        &launched.run.project_id,
        Some(&launched),
        AutonomousSyncIntent::Observe,
    )
}
