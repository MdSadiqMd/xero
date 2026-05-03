use tauri::{AppHandle, Runtime, State};

use crate::{
    commands::CommandResult,
    environment::service::{self, VerifyUserToolRequest, VerifyUserToolResponse},
    state::DesktopState,
};

#[tauri::command]
pub fn environment_verify_user_tool(
    request: VerifyUserToolRequest,
) -> CommandResult<VerifyUserToolResponse> {
    service::verify_user_environment_tool(request)
}

#[tauri::command]
pub fn environment_save_user_tool<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: VerifyUserToolRequest,
) -> CommandResult<crate::environment::probe::EnvironmentProbeReport> {
    service::save_user_environment_tool(&state.global_db_path(&app)?, request)
}

#[tauri::command]
pub fn environment_remove_user_tool<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    id: String,
) -> CommandResult<crate::environment::probe::EnvironmentProbeReport> {
    service::remove_user_environment_tool(&state.global_db_path(&app)?, id)
}
