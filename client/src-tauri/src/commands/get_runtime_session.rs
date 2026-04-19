use std::path::Path;

use tauri::{AppHandle, Runtime, State};

use crate::{
    commands::{
        validate_non_empty, CommandResult, ProjectIdRequestDto, RuntimeAuthPhase,
        RuntimeDiagnosticDto, RuntimeSessionDto,
    },
    runtime::{
        reconcile_provider_runtime_session, resolve_runtime_provider_identity,
        RuntimeProviderReconcileOutcome,
    },
    state::DesktopState,
};

use super::runtime_support::{
    emit_runtime_updated, load_runtime_session_status, persist_runtime_session,
    resolve_project_root,
};

#[tauri::command]
pub fn get_runtime_session<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: ProjectIdRequestDto,
) -> CommandResult<RuntimeSessionDto> {
    validate_non_empty(&request.project_id, "projectId")?;

    let repo_root = resolve_project_root(&app, state.inner(), &request.project_id)?;
    let runtime = load_runtime_session_status(state.inner(), &repo_root, &request.project_id)?;
    reconcile_runtime_session(&app, state.inner(), &repo_root, runtime)
}

pub(crate) fn reconcile_runtime_session<R: Runtime>(
    app: &AppHandle<R>,
    state: &DesktopState,
    repo_root: &Path,
    runtime: RuntimeSessionDto,
) -> CommandResult<RuntimeSessionDto> {
    if is_transient_phase(&runtime.phase)
        && runtime.flow_id.is_some()
        && runtime.authorization_url.is_none()
        && runtime.redirect_uri.is_none()
    {
        let updated = RuntimeSessionDto {
            flow_id: None,
            phase: RuntimeAuthPhase::Failed,
            callback_bound: None,
            authorization_url: None,
            redirect_uri: None,
            last_error_code: Some("auth_flow_unavailable".into()),
            last_error: Some(RuntimeDiagnosticDto {
                code: "auth_flow_unavailable".into(),
                message: format!(
                    "Cadence no longer has the in-memory {} login flow for this project. Start login again.",
                    runtime_provider_label(&runtime)
                ),
                retryable: false,
            }),
            updated_at: crate::auth::now_timestamp(),
            ..runtime
        };
        let persisted = persist_runtime_session(repo_root, &updated)?;
        emit_runtime_updated(app, &persisted)?;
        return Ok(persisted);
    }

    if runtime.phase != RuntimeAuthPhase::Authenticated {
        return Ok(runtime);
    }

    let provider = match resolve_runtime_provider_identity(
        Some(runtime.provider_id.as_str()),
        Some(runtime.runtime_kind.as_str()),
    ) {
        Ok(provider) => provider,
        Err(diagnostic) => {
            let updated = signed_out_runtime(
                runtime,
                &diagnostic.code,
                &diagnostic.message,
                diagnostic.retryable,
            );
            let persisted = persist_runtime_session(repo_root, &updated)?;
            emit_runtime_updated(app, &persisted)?;
            return Ok(persisted);
        }
    };

    match reconcile_provider_runtime_session(
        app,
        state,
        provider,
        runtime.account_id.as_deref(),
        runtime.session_id.as_deref(),
    ) {
        Ok(RuntimeProviderReconcileOutcome::Authenticated(_binding)) => Ok(runtime),
        Ok(RuntimeProviderReconcileOutcome::SignedOut(diagnostic)) => {
            let updated = signed_out_runtime(
                runtime,
                &diagnostic.code,
                &diagnostic.message,
                diagnostic.retryable,
            );
            let persisted = persist_runtime_session(repo_root, &updated)?;
            emit_runtime_updated(app, &persisted)?;
            Ok(persisted)
        }
        Err(error) => {
            let updated = signed_out_runtime(runtime, &error.code, &error.message, error.retryable);
            let persisted = persist_runtime_session(repo_root, &updated)?;
            emit_runtime_updated(app, &persisted)?;
            Ok(persisted)
        }
    }
}

fn signed_out_runtime(
    runtime: RuntimeSessionDto,
    code: &str,
    message: &str,
    retryable: bool,
) -> RuntimeSessionDto {
    RuntimeSessionDto {
        flow_id: None,
        session_id: None,
        phase: RuntimeAuthPhase::Idle,
        callback_bound: None,
        authorization_url: None,
        redirect_uri: None,
        last_error_code: Some(code.into()),
        last_error: Some(RuntimeDiagnosticDto {
            code: code.into(),
            message: message.into(),
            retryable,
        }),
        updated_at: crate::auth::now_timestamp(),
        ..runtime
    }
}

fn is_transient_phase(phase: &RuntimeAuthPhase) -> bool {
    matches!(
        phase,
        RuntimeAuthPhase::Starting
            | RuntimeAuthPhase::AwaitingBrowserCallback
            | RuntimeAuthPhase::AwaitingManualInput
            | RuntimeAuthPhase::ExchangingCode
            | RuntimeAuthPhase::Refreshing
    )
}

fn runtime_provider_label(runtime: &RuntimeSessionDto) -> String {
    resolve_runtime_provider_identity(
        Some(runtime.provider_id.as_str()),
        Some(runtime.runtime_kind.as_str()),
    )
    .map(|provider| provider.provider_id.into())
    .unwrap_or_else(|_| {
        let provider_id = runtime.provider_id.trim();
        if provider_id.is_empty() {
            let runtime_kind = runtime.runtime_kind.trim();
            if runtime_kind.is_empty() {
                "runtime".into()
            } else {
                runtime_kind.into()
            }
        } else {
            provider_id.into()
        }
    })
}
