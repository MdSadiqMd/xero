use tauri::{AppHandle, Runtime, State};

use crate::{
    commands::{
        validate_non_empty, AutonomousRunStateDto, CancelAutonomousRunRequestDto, CommandError,
        CommandResult,
    },
    runtime::{stop_runtime_run as stop_supervised_runtime_run, RuntimeSupervisorStopRequest},
    state::DesktopState,
};

use super::runtime_support::{
    emit_runtime_run_updated_if_changed, load_persisted_runtime_run, resolve_project_root,
    sync_autonomous_run_state, AutonomousSyncIntent, DEFAULT_RUNTIME_RUN_CONTROL_TIMEOUT,
    DEFAULT_RUNTIME_RUN_SHUTDOWN_TIMEOUT,
};

#[tauri::command]
pub fn cancel_autonomous_run<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: CancelAutonomousRunRequestDto,
) -> CommandResult<AutonomousRunStateDto> {
    validate_non_empty(&request.project_id, "projectId")?;
    validate_non_empty(&request.agent_session_id, "agentSessionId")?;
    validate_non_empty(&request.run_id, "runId")?;

    let repo_root = resolve_project_root(&app, state.inner(), &request.project_id)?;
    let before =
        load_persisted_runtime_run(&repo_root, &request.project_id, &request.agent_session_id)?;

    if let Some(snapshot) = before.as_ref() {
        if snapshot.run.run_id != request.run_id {
            return Err(CommandError::user_fixable(
                "autonomous_run_mismatch",
                format!(
                    "Cadence refused to cancel autonomous run `{}` because project `{}` is currently bound to durable run `{}`.",
                    request.run_id, request.project_id, snapshot.run.run_id
                ),
            ));
        }
    } else {
        return Ok(AutonomousRunStateDto { run: None });
    }

    let after = stop_supervised_runtime_run(
        state.inner(),
        RuntimeSupervisorStopRequest {
            project_id: request.project_id.clone(),
            agent_session_id: request.agent_session_id.clone(),
            repo_root: repo_root.clone(),
            control_timeout: DEFAULT_RUNTIME_RUN_CONTROL_TIMEOUT,
            shutdown_timeout: DEFAULT_RUNTIME_RUN_SHUTDOWN_TIMEOUT,
        },
    )?;
    emit_runtime_run_updated_if_changed(
        &app,
        &request.project_id,
        &request.agent_session_id,
        &before,
        &after,
    )?;

    sync_autonomous_run_state(
        &repo_root,
        &request.project_id,
        &request.agent_session_id,
        after.as_ref(),
        AutonomousSyncIntent::CancelRequested,
    )
}
