use tauri::{AppHandle, Runtime, State};

use crate::{
    commands::{
        validate_non_empty, AutonomousRunStateDto, CommandResult, GetAutonomousRunRequestDto,
    },
    state::DesktopState,
};

use super::runtime_support::{
    emit_runtime_run_updated_if_changed, load_runtime_run_status, resolve_project_root,
    sync_autonomous_run_state, AutonomousSyncIntent,
};

#[tauri::command]
pub fn get_autonomous_run<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: GetAutonomousRunRequestDto,
) -> CommandResult<AutonomousRunStateDto> {
    validate_non_empty(&request.project_id, "projectId")?;

    let repo_root = resolve_project_root(&app, state.inner(), &request.project_id)?;
    let before =
        super::runtime_support::load_persisted_runtime_run(&repo_root, &request.project_id)?;
    let after = load_runtime_run_status(state.inner(), &repo_root, &request.project_id)?;
    emit_runtime_run_updated_if_changed(&app, &request.project_id, &before, &after)?;

    sync_autonomous_run_state(
        &repo_root,
        &request.project_id,
        after.as_ref(),
        AutonomousSyncIntent::Observe,
    )
}
