use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Runtime};

use super::{AuthDiagnostic, AuthFlowError};
use crate::{
    commands::{get_runtime_settings::RuntimeSettingsSnapshot, RuntimeAuthPhase},
    runtime::{anthropic_provider, ANTHROPIC_PROVIDER_ID},
    state::DesktopState,
};

const DEFAULT_MODELS_URL: &str = "https://api.anthropic.com/v1/models";
const DEFAULT_ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone)]
pub struct AnthropicAuthConfig {
    pub models_url: String,
    pub anthropic_version: String,
    pub timeout: Duration,
}

impl Default for AnthropicAuthConfig {
    fn default() -> Self {
        Self {
            models_url: DEFAULT_MODELS_URL.into(),
            anthropic_version: DEFAULT_ANTHROPIC_VERSION.into(),
            timeout: Duration::from_secs(10),
        }
    }
}

impl AnthropicAuthConfig {
    pub fn for_platform() -> Self {
        Self::default()
    }

    fn http_client(&self) -> Result<Client, AuthFlowError> {
        Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|error| {
                AuthFlowError::terminal(
                    "anthropic_http_client_unavailable",
                    RuntimeAuthPhase::Failed,
                    format!(
                        "Cadence could not build the Anthropic HTTP client for the models probe: {error}"
                    ),
                )
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnthropicRuntimeSessionBinding {
    pub provider_id: String,
    pub session_id: String,
    pub account_id: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnthropicBindOutcome {
    Ready(AnthropicRuntimeSessionBinding),
    SignedOut(AuthDiagnostic),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnthropicReconcileOutcome {
    Authenticated(AnthropicRuntimeSessionBinding),
    SignedOut(AuthDiagnostic),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnthropicDiscoveredModel {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelsResponse {
    data: Vec<ModelSummary>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelSummary {
    id: String,
    display_name: Option<String>,
}

pub(crate) fn bind_anthropic_runtime_session<R: Runtime>(
    _app: &AppHandle<R>,
    state: &DesktopState,
    settings: &RuntimeSettingsSnapshot,
) -> Result<AnthropicBindOutcome, AuthFlowError> {
    let Some(api_key) = settings.anthropic_api_key.as_deref() else {
        return Ok(AnthropicBindOutcome::SignedOut(AuthDiagnostic {
            code: "anthropic_api_key_missing".into(),
            message: "Cadence cannot bind the selected Anthropic runtime because no app-local API key is configured for the active provider profile.".into(),
            retryable: false,
        }));
    };

    validate_anthropic_models_probe(
        api_key,
        &settings.settings.model_id,
        &state.anthropic_auth_config(),
    )?;
    Ok(AnthropicBindOutcome::Ready(synthetic_binding(
        settings, api_key,
    )))
}

pub(crate) fn reconcile_anthropic_runtime_session<R: Runtime>(
    _app: &AppHandle<R>,
    state: &DesktopState,
    account_id: Option<&str>,
    session_id: Option<&str>,
    settings: &RuntimeSettingsSnapshot,
) -> Result<AnthropicReconcileOutcome, AuthFlowError> {
    let Some(api_key) = settings.anthropic_api_key.as_deref() else {
        return Ok(AnthropicReconcileOutcome::SignedOut(AuthDiagnostic {
            code: "anthropic_api_key_missing".into(),
            message: "Cadence cannot reconcile the selected Anthropic runtime because no app-local API key is configured for the active provider profile.".into(),
            retryable: false,
        }));
    };

    let expected = synthetic_binding(settings, api_key);
    let account_id = normalized(account_id);
    let session_id = normalized(session_id);
    if account_id != Some(expected.account_id.as_str())
        || session_id != Some(expected.session_id.as_str())
    {
        return Ok(AnthropicReconcileOutcome::SignedOut(AuthDiagnostic {
            code: "anthropic_binding_stale".into(),
            message: "Cadence rejected the persisted Anthropic runtime binding because the selected provider profile, model, or API key changed. Rebind the runtime session from the active profile.".into(),
            retryable: false,
        }));
    }

    validate_anthropic_models_probe(
        api_key,
        &settings.settings.model_id,
        &state.anthropic_auth_config(),
    )?;

    Ok(AnthropicReconcileOutcome::Authenticated(expected))
}

pub(crate) fn fetch_anthropic_models(
    api_key: &str,
    config: &AnthropicAuthConfig,
) -> Result<Vec<AnthropicDiscoveredModel>, AuthFlowError> {
    let client = config.http_client()?;
    let response = client
        .get(&config.models_url)
        .header("x-api-key", api_key)
        .header("anthropic-version", &config.anthropic_version)
        .send()
        .map_err(map_probe_transport_error)?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_default();
        return Err(map_probe_status_error(status.as_u16(), body.trim()));
    }

    let models: ModelsResponse = response.json().map_err(|error| {
        AuthFlowError::terminal(
            "anthropic_models_decode_failed",
            RuntimeAuthPhase::Failed,
            format!("Cadence could not decode the Anthropic models response: {error}"),
        )
    })?;

    normalize_anthropic_models(models)
}

fn validate_anthropic_models_probe(
    api_key: &str,
    model_id: &str,
    config: &AnthropicAuthConfig,
) -> Result<(), AuthFlowError> {
    let models = fetch_anthropic_models(api_key, config)?;

    if !models.iter().any(|model| model.id.trim() == model_id) {
        return Err(AuthFlowError::terminal(
            "anthropic_model_unavailable",
            RuntimeAuthPhase::Failed,
            format!(
                "Cadence could not find the configured Anthropic model `{model_id}` in the provider models response."
            ),
        ));
    }

    Ok(())
}

fn normalize_anthropic_models(
    response: ModelsResponse,
) -> Result<Vec<AnthropicDiscoveredModel>, AuthFlowError> {
    response
        .data
        .into_iter()
        .map(|model| {
            let id = model.id.trim();
            if id.is_empty() {
                return Err(AuthFlowError::terminal(
                    "anthropic_models_decode_failed",
                    RuntimeAuthPhase::Failed,
                    "Cadence could not decode the Anthropic models response because one model id was blank.",
                ));
            }

            let display_name = model
                .display_name
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(id)
                .to_owned();

            Ok(AnthropicDiscoveredModel {
                id: id.to_owned(),
                display_name,
            })
        })
        .collect()
}

fn map_probe_transport_error(error: reqwest::Error) -> AuthFlowError {
    if error.is_timeout() {
        return AuthFlowError::retryable(
            "anthropic_provider_unavailable",
            RuntimeAuthPhase::Failed,
            "The Anthropic models probe timed out. Try again once the provider is reachable.",
        );
    }

    AuthFlowError::retryable(
        "anthropic_provider_unavailable",
        RuntimeAuthPhase::Failed,
        format!("Cadence could not reach the Anthropic models endpoint: {error}"),
    )
}

fn map_probe_status_error(status: u16, body: &str) -> AuthFlowError {
    let suffix = if body.is_empty() {
        String::new()
    } else {
        format!(" Response: {body}")
    };

    match status {
        401 | 403 => AuthFlowError::terminal(
            "anthropic_invalid_api_key",
            RuntimeAuthPhase::Failed,
            format!("Anthropic rejected the configured API key with HTTP {status}.{suffix}"),
        ),
        429 => AuthFlowError::retryable(
            "anthropic_rate_limited",
            RuntimeAuthPhase::Failed,
            format!("Anthropic rate limited the models probe with HTTP 429.{suffix}"),
        ),
        500..=599 => AuthFlowError::retryable(
            "anthropic_provider_unavailable",
            RuntimeAuthPhase::Failed,
            format!(
                "Anthropic returned HTTP {status} while validating the configured API key.{suffix}"
            ),
        ),
        _ => AuthFlowError::terminal(
            "anthropic_provider_unavailable",
            RuntimeAuthPhase::Failed,
            format!(
                "Anthropic returned HTTP {status} while validating the configured API key.{suffix}"
            ),
        ),
    }
}

fn synthetic_binding(
    settings: &RuntimeSettingsSnapshot,
    api_key: &str,
) -> AnthropicRuntimeSessionBinding {
    let provider = anthropic_provider();
    let key_fingerprint = sha256_hex(format!("{ANTHROPIC_PROVIDER_ID}:{api_key}"));
    let effective_timestamp = settings
        .anthropic_credentials_updated_at
        .as_deref()
        .unwrap_or(settings.settings.updated_at.as_str());
    let session_fingerprint = sha256_hex(format!(
        "{}:{}:{}:{}",
        key_fingerprint,
        settings.settings.provider_id,
        settings.settings.model_id,
        effective_timestamp,
    ));

    AnthropicRuntimeSessionBinding {
        provider_id: provider.provider_id.into(),
        account_id: format!("anthropic-acct-{}", &key_fingerprint[..16]),
        session_id: format!("anthropic-session-{}", &session_fingerprint[..16]),
        updated_at: crate::auth::now_timestamp(),
    }
}

fn sha256_hex(value: String) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn normalized(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}
