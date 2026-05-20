//! App-scoped xAI OAuth device-code login commands.

use tauri::{AppHandle, Runtime, State};

use crate::{
    auth::{poll_xai_device_code_flow, start_xai_device_code_flow, XaiDeviceCodeLogin},
    commands::{
        validate_non_empty, CommandError, CommandResult, PollXaiDeviceCodeLoginRequestDto,
        StartXaiDeviceCodeLoginRequestDto, XaiDeviceCodeLoginDto,
    },
    runtime::XAI_PROVIDER_ID,
    state::DesktopState,
};

use super::runtime_support::{command_error_from_auth, runtime_diagnostic_from_auth};

#[tauri::command]
pub fn start_xai_device_code_login(
    state: State<'_, DesktopState>,
    request: StartXaiDeviceCodeLoginRequestDto,
) -> CommandResult<XaiDeviceCodeLoginDto> {
    validate_xai_provider_id(&request.provider_id)?;
    let login = start_xai_device_code_flow(state.inner(), state.xai_auth_config())
        .map_err(command_error_from_auth)?;
    Ok(map_xai_device_code_login(login))
}

#[tauri::command]
pub fn poll_xai_device_code_login<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: PollXaiDeviceCodeLoginRequestDto,
) -> CommandResult<XaiDeviceCodeLoginDto> {
    validate_xai_provider_id(&request.provider_id)?;
    validate_non_empty(&request.flow_id, "flowId")?;
    let login = poll_xai_device_code_flow(
        &app,
        state.inner(),
        &request.flow_id,
        &state.xai_auth_config(),
    )
    .map_err(command_error_from_auth)?;
    Ok(map_xai_device_code_login(login))
}

fn validate_xai_provider_id(provider_id: &str) -> CommandResult<()> {
    validate_non_empty(provider_id, "providerId")?;
    if provider_id != XAI_PROVIDER_ID {
        return Err(CommandError::user_fixable(
            "xai_device_code_provider_unsupported",
            format!("Xero only supports xAI device-code login for provider `{XAI_PROVIDER_ID}`."),
        ));
    }
    Ok(())
}

fn map_xai_device_code_login(login: XaiDeviceCodeLogin) -> XaiDeviceCodeLoginDto {
    XaiDeviceCodeLoginDto {
        provider_id: login.provider_id,
        flow_id: login.flow_id,
        user_code: login.user_code,
        verification_uri: login.verification_uri,
        verification_uri_complete: login.verification_uri_complete,
        interval_seconds: login.interval_seconds,
        expires_at: login.expires_at,
        phase: login.phase,
        session_id: login.session_id,
        account_id: login.account_id,
        last_error_code: login.last_error_code,
        last_error: login.last_error.map(runtime_diagnostic_from_auth),
        updated_at: login.updated_at,
    }
}
