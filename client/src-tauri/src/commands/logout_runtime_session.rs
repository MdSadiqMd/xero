use tauri::{AppHandle, Runtime, State};

use crate::{
    commands::{
        validate_non_empty, CommandResult, ProjectIdRequestDto, RuntimeAuthPhase, RuntimeSessionDto,
    },
    runtime::{
        default_runtime_provider, logout_provider_runtime_session,
        resolve_runtime_provider_identity,
    },
    state::DesktopState,
};

use super::runtime_support::{
    command_error_from_auth, emit_runtime_updated, load_runtime_session_status,
    persist_runtime_session, resolve_project_root, runtime_diagnostic_from_auth,
};

#[tauri::command]
pub fn logout_runtime_session<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: ProjectIdRequestDto,
) -> CommandResult<RuntimeSessionDto> {
    validate_non_empty(&request.project_id, "projectId")?;

    let repo_root = resolve_project_root(&app, state.inner(), &request.project_id)?;
    let current = load_runtime_session_status(state.inner(), &repo_root, &request.project_id)?;

    let provider_result = resolve_runtime_provider_identity(
        Some(current.provider_id.as_str()),
        Some(current.runtime_kind.as_str()),
    );
    let provider_error = provider_result.as_ref().err().cloned();
    let provider = provider_result.unwrap_or_else(|_| default_runtime_provider());

    if let Some(account_id) = current
        .account_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        if let Err(error) =
            logout_provider_runtime_session(&app, state.inner(), provider, account_id)
        {
            return Err(command_error_from_auth(error));
        }
    }

    let signed_out = RuntimeSessionDto {
        project_id: request.project_id,
        runtime_kind: provider.runtime_kind.into(),
        provider_id: provider.provider_id.into(),
        flow_id: None,
        session_id: None,
        account_id: current
            .account_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned),
        phase: RuntimeAuthPhase::Idle,
        callback_bound: None,
        authorization_url: None,
        redirect_uri: None,
        last_error_code: provider_error
            .as_ref()
            .map(|diagnostic| diagnostic.code.clone()),
        last_error: provider_error.map(runtime_diagnostic_from_auth),
        updated_at: crate::auth::now_timestamp(),
    };

    let persisted = persist_runtime_session(&repo_root, &signed_out)?;
    emit_runtime_updated(&app, &persisted)?;
    Ok(persisted)
}
