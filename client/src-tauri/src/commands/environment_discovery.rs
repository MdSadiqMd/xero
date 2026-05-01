use tauri::{AppHandle, Runtime, State};

use crate::{
    commands::{CommandResult, EnvironmentDiscoveryStatus},
    environment::service,
    global_db::environment_profile::EnvironmentProfileSummary,
    state::DesktopState,
};

#[tauri::command]
pub fn get_environment_discovery_status<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
) -> CommandResult<EnvironmentDiscoveryStatus> {
    service::environment_discovery_status(&state.global_db_path(&app)?)
}

#[tauri::command]
pub fn start_environment_discovery<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
) -> CommandResult<EnvironmentDiscoveryStatus> {
    service::start_environment_discovery(state.global_db_path(&app)?)
}

#[tauri::command]
pub fn refresh_environment_discovery<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
) -> CommandResult<EnvironmentDiscoveryStatus> {
    service::refresh_environment_discovery(state.global_db_path(&app)?)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveEnvironmentPermissionRequestsRequest {
    pub decisions: Vec<service::EnvironmentPermissionDecision>,
}

#[tauri::command]
pub fn resolve_environment_permission_requests<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: ResolveEnvironmentPermissionRequestsRequest,
) -> CommandResult<EnvironmentDiscoveryStatus> {
    service::resolve_environment_permission_requests(
        &state.global_db_path(&app)?,
        request.decisions,
    )
}

#[tauri::command]
pub fn get_environment_profile_summary<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
) -> CommandResult<Option<EnvironmentProfileSummary>> {
    service::environment_profile_summary(&state.global_db_path(&app)?)
}
