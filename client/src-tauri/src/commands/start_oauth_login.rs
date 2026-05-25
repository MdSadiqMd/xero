//! Generic app-scoped provider OAuth login entry point. Provider credentials are
//! app-local, so this command intentionally does not require or mutate a project.

use tauri::{AppHandle, Runtime, State};

use crate::{
    auth::{ensure_openai_profile_target, ensure_xai_profile_target, start_provider_auth_flow},
    commands::{
        validate_non_empty, CommandError, CommandResult, ProviderAuthSessionDto,
        StartOAuthLoginRequestDto,
    },
    provider_credentials::{OPENAI_CODEX_DEFAULT_PROFILE_ID, XAI_DEFAULT_PROFILE_ID},
    runtime::{openai_codex_provider, xai_provider, OPENAI_CODEX_PROVIDER_ID, XAI_PROVIDER_ID},
    state::DesktopState,
};

use super::runtime_support::command_error_from_auth;

pub(crate) const PROVIDER_CREDENTIAL_OAUTH_SCOPE_ID: &str = "app-provider-credentials";

#[tauri::command]
pub fn start_oauth_login<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: StartOAuthLoginRequestDto,
) -> CommandResult<ProviderAuthSessionDto> {
    validate_non_empty(&request.provider_id, "providerId")?;

    let (provider, profile_id, action) = match request.provider_id.as_str() {
        OPENAI_CODEX_PROVIDER_ID => (
            openai_codex_provider(),
            OPENAI_CODEX_DEFAULT_PROFILE_ID,
            "start OpenAI login",
        ),
        XAI_PROVIDER_ID => (xai_provider(), XAI_DEFAULT_PROFILE_ID, "start xAI login"),
        _ => {
            return Err(CommandError::user_fixable(
                "oauth_login_provider_unsupported",
                format!(
                    "Xero does not support browser-based OAuth for provider `{}`. Only `{}` and `{}` are wired today.",
                    request.provider_id, OPENAI_CODEX_PROVIDER_ID, XAI_PROVIDER_ID
                ),
            ));
        }
    };

    match request.provider_id.as_str() {
        OPENAI_CODEX_PROVIDER_ID => ensure_openai_profile_target(
            &app,
            state.inner(),
            profile_id,
            crate::commands::RuntimeAuthPhase::Starting,
            action,
        )
        .map_err(command_error_from_auth)?,
        XAI_PROVIDER_ID => ensure_xai_profile_target(
            &app,
            state.inner(),
            profile_id,
            crate::commands::RuntimeAuthPhase::Starting,
            action,
        )
        .map_err(command_error_from_auth)?,
        _ => unreachable!("unsupported OAuth provider was rejected above"),
    }

    let started = start_provider_auth_flow(
        state.inner(),
        provider.provider,
        PROVIDER_CREDENTIAL_OAUTH_SCOPE_ID,
        profile_id,
        request.originator.as_deref(),
    )
    .map_err(command_error_from_auth)?;

    Ok(ProviderAuthSessionDto {
        runtime_kind: provider.runtime_kind.into(),
        provider_id: started.provider_id,
        flow_id: Some(started.flow_id),
        session_id: None,
        account_id: None,
        phase: started.phase,
        callback_bound: Some(started.callback_bound),
        authorization_url: Some(started.authorization_url),
        redirect_uri: Some(started.redirect_uri),
        last_error_code: started.last_error_code.clone(),
        last_error: None,
        updated_at: started.updated_at,
    })
}
