use tauri::{AppHandle, Runtime};

use crate::{
    auth::AuthFlowError,
    commands::{
        get_runtime_settings::runtime_settings_file_from_request, CommandError, CommandResult,
        ProviderProfileDto, ProviderProfileReadinessDto, ProviderProfileReadinessProofDto,
        ProviderProfileReadinessStatusDto, ProviderProfilesDto, ProviderProfilesMigrationDto,
        UpsertProviderProfileRequestDto,
    },
    provider_credentials::{
        delete_provider_credential as cred_delete,
        upsert_provider_credential as cred_upsert, ProviderCredentialKind,
        ProviderCredentialRecord,
    },
    global_db::open_global_database,
    provider_profiles::{
        load_provider_profiles_or_default, ProviderApiKeyCredentialEntry,
        ProviderProfileCredentialLink, ProviderProfileReadinessProof,
        ProviderProfileReadinessStatus, ProviderProfileRecord, ProviderProfilesSnapshot,
    },
    runtime::{
        normalize_openai_codex_model_id, resolve_runtime_provider_identity, BEDROCK_PROVIDER_ID,
        OLLAMA_PROVIDER_ID, OPENAI_API_PROVIDER_ID, OPENAI_CODEX_PROVIDER_ID, VERTEX_PROVIDER_ID,
    },
    state::DesktopState,
};

/// Phase 2.3 (write-through): keep `provider_credentials` aligned with
/// per-provider state whenever the legacy upsert path mutates a profile. The
/// new table is the source of truth for the post-refactor frontend; this
/// mirror keeps it accurate while the legacy frontend is still in flight.
fn mirror_profile_credential_to_new_table(
    connection: &rusqlite::Connection,
    snapshot: &ProviderProfilesSnapshot,
    request: &UpsertProviderProfileRequestDto,
) -> CommandResult<()> {
    // OpenAI Codex is mirrored from the OAuth completion path, not from the
    // profile upsert (the upsert command rejects api keys for that provider).
    let provider_id = request.provider_id.trim();
    if provider_id == OPENAI_CODEX_PROVIDER_ID {
        return Ok(());
    }

    let profile_id = request.profile_id.trim();
    let Some(profile) = snapshot.profile(profile_id) else {
        return Ok(());
    };

    let kind = match profile.credential_link.as_ref() {
        Some(ProviderProfileCredentialLink::ApiKey { .. }) => ProviderCredentialKind::ApiKey,
        Some(ProviderProfileCredentialLink::Local { .. }) => ProviderCredentialKind::Local,
        Some(ProviderProfileCredentialLink::Ambient { .. }) => ProviderCredentialKind::Ambient,
        // Without a credential linkage the user has cleared the profile —
        // drop the credential row so the new readers see "not credentialed".
        Some(ProviderProfileCredentialLink::OpenAiCodex { .. }) | None => {
            return cred_delete(connection, &profile.provider_id);
        }
    };

    let api_key = if matches!(kind, ProviderCredentialKind::ApiKey) {
        snapshot
            .api_key_credential(profile_id)
            .map(|entry| entry.api_key.clone())
    } else {
        None
    };

    if matches!(kind, ProviderCredentialKind::ApiKey) && api_key.is_none() {
        // Linkage says api_key but the secret is missing — treat as cleared.
        return cred_delete(connection, &profile.provider_id);
    }

    let updated_at = profile
        .credential_link
        .as_ref()
        .map(|link| match link {
            ProviderProfileCredentialLink::ApiKey { updated_at }
            | ProviderProfileCredentialLink::Local { updated_at }
            | ProviderProfileCredentialLink::Ambient { updated_at }
            | ProviderProfileCredentialLink::OpenAiCodex { updated_at, .. } => updated_at.clone(),
        })
        .unwrap_or_else(|| profile.updated_at.clone());

    cred_upsert(
        connection,
        &ProviderCredentialRecord {
            provider_id: profile.provider_id.clone(),
            kind,
            api_key,
            oauth_account_id: None,
            oauth_session_id: None,
            oauth_access_token: None,
            oauth_refresh_token: None,
            oauth_expires_at: None,
            base_url: profile.base_url.clone(),
            api_version: profile.api_version.clone(),
            region: profile.region.clone(),
            project_id: profile.project_id.clone(),
            default_model_id: Some(profile.model_id.clone()),
            updated_at,
        },
    )
}

pub(crate) fn load_provider_profiles_snapshot<R: Runtime>(
    app: &AppHandle<R>,
    state: &DesktopState,
) -> CommandResult<ProviderProfilesSnapshot> {
    let connection = open_global_database(&state.global_db_path(app)?)?;
    load_provider_profiles_or_default(&connection)
}

pub(crate) fn provider_profiles_dto_from_snapshot(
    snapshot: &ProviderProfilesSnapshot,
) -> ProviderProfilesDto {
    let mut profiles = snapshot
        .metadata
        .profiles
        .iter()
        .map(|profile| provider_profile_dto(snapshot, profile))
        .collect::<Vec<_>>();
    profiles.sort_by(|left, right| left.profile_id.cmp(&right.profile_id));

    ProviderProfilesDto {
        active_profile_id: snapshot.metadata.active_profile_id.clone(),
        profiles,
        migration: snapshot.metadata.migration.as_ref().map(|migration| {
            ProviderProfilesMigrationDto {
                source: migration.source.clone(),
                migrated_at: migration.migrated_at.clone(),
                runtime_settings_updated_at: migration.runtime_settings_updated_at.clone(),
                openrouter_credentials_updated_at: migration
                    .openrouter_credentials_updated_at
                    .clone(),
                openai_auth_updated_at: migration.openai_auth_updated_at.clone(),
                openrouter_model_inferred: migration.openrouter_model_inferred,
            }
        }),
    }
}

fn provider_profile_dto(
    snapshot: &ProviderProfilesSnapshot,
    profile: &ProviderProfileRecord,
) -> ProviderProfileDto {
    let readiness = profile.readiness(&snapshot.credentials);
    ProviderProfileDto {
        profile_id: profile.profile_id.clone(),
        provider_id: profile.provider_id.clone(),
        runtime_kind: profile.runtime_kind.clone(),
        label: profile.label.clone(),
        model_id: profile.model_id.clone(),
        preset_id: profile.preset_id.clone(),
        base_url: profile.base_url.clone(),
        api_version: profile.api_version.clone(),
        region: profile.region.clone(),
        project_id: profile.project_id.clone(),
        active: profile.profile_id == snapshot.metadata.active_profile_id,
        readiness: ProviderProfileReadinessDto {
            ready: readiness.ready,
            status: map_readiness_status(readiness.status),
            proof: readiness.proof.map(map_readiness_proof),
            proof_updated_at: readiness.proof_updated_at,
        },
        migrated_from_legacy: profile.migrated_from_legacy,
        migrated_at: profile.migrated_at.clone(),
    }
}

fn map_readiness_status(
    status: ProviderProfileReadinessStatus,
) -> ProviderProfileReadinessStatusDto {
    match status {
        ProviderProfileReadinessStatus::Ready => ProviderProfileReadinessStatusDto::Ready,
        ProviderProfileReadinessStatus::Missing => ProviderProfileReadinessStatusDto::Missing,
        ProviderProfileReadinessStatus::Malformed => ProviderProfileReadinessStatusDto::Malformed,
    }
}

fn map_readiness_proof(proof: ProviderProfileReadinessProof) -> ProviderProfileReadinessProofDto {
    match proof {
        ProviderProfileReadinessProof::OAuthSession => {
            ProviderProfileReadinessProofDto::OAuthSession
        }
        ProviderProfileReadinessProof::StoredSecret => {
            ProviderProfileReadinessProofDto::StoredSecret
        }
        ProviderProfileReadinessProof::Local => ProviderProfileReadinessProofDto::Local,
        ProviderProfileReadinessProof::Ambient => ProviderProfileReadinessProofDto::Ambient,
    }
}

fn apply_provider_profile_upsert(
    current: &ProviderProfilesSnapshot,
    request: &UpsertProviderProfileRequestDto,
) -> CommandResult<ProviderProfilesSnapshot> {
    let profile_id = request.profile_id.trim();
    if profile_id.is_empty() {
        return Err(CommandError::invalid_request("profileId"));
    }

    let provider_id = request.provider_id.trim();
    if provider_id.is_empty() {
        return Err(CommandError::invalid_request("providerId"));
    }

    let runtime_kind = request.runtime_kind.trim();
    if runtime_kind.is_empty() {
        return Err(CommandError::invalid_request("runtimeKind"));
    }

    let label = request.label.trim();
    if label.is_empty() {
        return Err(CommandError::invalid_request("label"));
    }

    let model_id = request.model_id.trim();
    if model_id.is_empty() {
        return Err(CommandError::invalid_request("modelId"));
    }

    let provider = resolve_runtime_provider_identity(Some(provider_id), Some(runtime_kind))
        .map_err(|diagnostic| {
            CommandError::user_fixable("provider_profiles_invalid", diagnostic.message)
        })?;

    if let Some(existing) = current
        .metadata
        .profiles
        .iter()
        .find(|profile| profile.provider_id == provider.provider_id)
    {
        if existing.profile_id != profile_id {
            return Err(CommandError::user_fixable(
                "provider_profile_already_exists",
                format!(
                    "Cadence already has a `{}` provider profile (`{}`); only one profile per provider is supported. Update the existing profile instead of creating `{}`.",
                    provider.provider_id, existing.profile_id, profile_id,
                ),
            ));
        }
    }

    if provider.provider_id == OPENAI_CODEX_PROVIDER_ID {
        let _ = runtime_settings_file_from_request(provider_id, model_id, false)?;
        if request
            .api_key
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            return Err(CommandError::invalid_request("apiKey"));
        }
    }

    let normalized_model_id = if provider.provider_id == OPENAI_CODEX_PROVIDER_ID {
        normalize_openai_codex_model_id(model_id)
    } else {
        model_id.to_owned()
    };

    let supports_api_key = !matches!(
        provider.provider_id,
        "openai_codex" | "ollama" | "bedrock" | "vertex"
    );
    if !supports_api_key
        && request
            .api_key
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    {
        return Err(CommandError::invalid_request("apiKey"));
    }

    let now = crate::auth::now_timestamp();
    let current_profile = current.profile(profile_id).cloned();
    let current_api_key_secret = current.api_key_credential(profile_id).cloned();
    let current_openai_auth_link =
        current.metadata.profiles.iter().find_map(|profile| {
            match profile.credential_link.as_ref() {
                Some(ProviderProfileCredentialLink::OpenAiCodex { .. })
                    if profile.provider_id == OPENAI_CODEX_PROVIDER_ID =>
                {
                    profile.credential_link.clone()
                }
                _ => None,
            }
        });
    let requested_api_key = request
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let explicit_api_key_clear = request
        .api_key
        .as_deref()
        .is_some_and(|value| value.trim().is_empty());

    if explicit_api_key_clear
        && current_profile.is_none()
        && provider.provider_id != OPENAI_CODEX_PROVIDER_ID
    {
        return Err(CommandError::invalid_request("apiKey"));
    }

    let next_api_key_secret = if !supports_api_key
        || provider.provider_id == OPENAI_CODEX_PROVIDER_ID
        || explicit_api_key_clear
    {
        None
    } else if let Some(api_key) = requested_api_key {
        Some(ProviderApiKeyCredentialEntry {
            profile_id: profile_id.to_owned(),
            api_key: api_key.to_owned(),
            updated_at: api_key_updated_at(current_api_key_secret.as_ref(), Some(api_key)),
        })
    } else if current_profile
        .as_ref()
        .is_some_and(|profile| profile.provider_id != OPENAI_CODEX_PROVIDER_ID)
    {
        current_api_key_secret.clone()
    } else {
        None
    };

    let next_credential_link = next_provider_profile_credential_link(
        provider.provider_id,
        current_profile.as_ref(),
        next_api_key_secret.as_ref(),
        request.base_url.as_deref(),
        current_openai_auth_link.as_ref(),
    );

    let mut next = current.clone();
    let next_profile = ProviderProfileRecord {
        profile_id: profile_id.to_owned(),
        provider_id: provider.provider_id.to_owned(),
        runtime_kind: provider.runtime_kind.to_owned(),
        label: label.to_owned(),
        model_id: normalized_model_id.clone(),
        preset_id: normalize_optional_text(request.preset_id.clone()),
        base_url: normalize_optional_text(request.base_url.clone()),
        api_version: normalize_optional_text(request.api_version.clone()),
        region: normalize_optional_text(request.region.clone()),
        project_id: normalize_optional_text(request.project_id.clone()),
        credential_link: next_credential_link,
        migrated_from_legacy: current_profile
            .as_ref()
            .is_some_and(|profile| profile.migrated_from_legacy),
        migrated_at: current_profile
            .as_ref()
            .and_then(|profile| profile.migrated_at.clone()),
        updated_at: current_profile
            .as_ref()
            .filter(|profile| {
                profile.provider_id == provider.provider_id
                    && profile.runtime_kind == provider.runtime_kind
                    && profile.label == label
                    && profile.model_id == normalized_model_id
                    && profile.preset_id == normalize_optional_text(request.preset_id.clone())
                    && profile.base_url == normalize_optional_text(request.base_url.clone())
                    && profile.api_version == normalize_optional_text(request.api_version.clone())
                    && profile.region == normalize_optional_text(request.region.clone())
                    && profile.project_id == normalize_optional_text(request.project_id.clone())
                    && profile.credential_link
                        == next_provider_profile_credential_link(
                            provider.provider_id,
                            current_profile.as_ref(),
                            next_api_key_secret.as_ref(),
                            request.base_url.as_deref(),
                            current_openai_auth_link.as_ref(),
                        )
            })
            .map(|profile| profile.updated_at.clone())
            .unwrap_or_else(|| now.clone()),
    };

    upsert_profile(&mut next.metadata.profiles, next_profile);

    if let Some(secret) = next_api_key_secret {
        upsert_api_key_secret(&mut next, secret);
    } else {
        next.credentials
            .api_keys
            .retain(|entry| entry.profile_id != profile_id);
    }

    if request.activate {
        next.metadata.active_profile_id = profile_id.to_owned();
    }

    if next == *current {
        return Ok(current.clone());
    }

    next.metadata.updated_at = now;
    Ok(next)
}

fn apply_active_profile_switch(
    current: &ProviderProfilesSnapshot,
    profile_id: &str,
) -> CommandResult<ProviderProfilesSnapshot> {
    let profile_id = profile_id.trim();
    if profile_id.is_empty() {
        return Err(CommandError::invalid_request("profileId"));
    }

    if !current
        .metadata
        .profiles
        .iter()
        .any(|profile| profile.profile_id == profile_id)
    {
        return Err(CommandError::user_fixable(
            "provider_profile_not_found",
            format!("Cadence could not find provider profile `{profile_id}`."),
        ));
    }

    if current.metadata.active_profile_id == profile_id {
        return Ok(current.clone());
    }

    let mut next = current.clone();
    next.metadata.active_profile_id = profile_id.to_owned();
    next.metadata.updated_at = crate::auth::now_timestamp();
    Ok(next)
}

fn next_provider_profile_credential_link(
    provider_id: &str,
    current_profile: Option<&ProviderProfileRecord>,
    next_api_key_secret: Option<&ProviderApiKeyCredentialEntry>,
    base_url: Option<&str>,
    current_openai_auth_link: Option<&ProviderProfileCredentialLink>,
) -> Option<ProviderProfileCredentialLink> {
    if provider_id == OPENAI_CODEX_PROVIDER_ID {
        return current_profile
            .and_then(|profile| match profile.credential_link.as_ref() {
                Some(ProviderProfileCredentialLink::OpenAiCodex { .. }) => {
                    profile.credential_link.clone()
                }
                _ => None,
            })
            .or_else(|| current_openai_auth_link.cloned());
    }

    if provider_uses_local_readiness(provider_id, base_url) && next_api_key_secret.is_none() {
        let updated_at = current_profile
            .and_then(|profile| match profile.credential_link.as_ref() {
                Some(ProviderProfileCredentialLink::Local { updated_at }) => {
                    Some(updated_at.clone())
                }
                _ => None,
            })
            .unwrap_or_else(crate::auth::now_timestamp);
        return Some(ProviderProfileCredentialLink::Local { updated_at });
    }

    if provider_uses_ambient_readiness(provider_id) {
        let updated_at = current_profile
            .and_then(|profile| match profile.credential_link.as_ref() {
                Some(ProviderProfileCredentialLink::Ambient { updated_at }) => {
                    Some(updated_at.clone())
                }
                _ => None,
            })
            .unwrap_or_else(crate::auth::now_timestamp);
        return Some(ProviderProfileCredentialLink::Ambient { updated_at });
    }

    next_api_key_secret.map(|entry| ProviderProfileCredentialLink::ApiKey {
        updated_at: entry.updated_at.clone(),
    })
}

fn provider_uses_local_readiness(provider_id: &str, base_url: Option<&str>) -> bool {
    provider_id == OLLAMA_PROVIDER_ID
        || (provider_id == OPENAI_API_PROVIDER_ID && base_url.is_some_and(is_local_openai_base_url))
}

fn provider_uses_ambient_readiness(provider_id: &str) -> bool {
    matches!(provider_id, BEDROCK_PROVIDER_ID | VERTEX_PROVIDER_ID)
}

fn is_local_openai_base_url(base_url: &str) -> bool {
    url::Url::parse(base_url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(|host| host.to_ascii_lowercase()))
        .is_some_and(|host| matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1"))
}

fn upsert_profile(profiles: &mut Vec<ProviderProfileRecord>, next: ProviderProfileRecord) {
    if let Some(existing) = profiles
        .iter_mut()
        .find(|profile| profile.profile_id == next.profile_id)
    {
        *existing = next;
    } else {
        profiles.push(next);
    }
}

fn upsert_api_key_secret(
    snapshot: &mut ProviderProfilesSnapshot,
    next: ProviderApiKeyCredentialEntry,
) {
    if let Some(existing) = snapshot
        .credentials
        .api_keys
        .iter_mut()
        .find(|entry| entry.profile_id == next.profile_id)
    {
        *existing = next;
    } else {
        snapshot.credentials.api_keys.push(next);
    }
}

fn api_key_updated_at(
    current: Option<&ProviderApiKeyCredentialEntry>,
    next_api_key: Option<&str>,
) -> String {
    match (current, next_api_key) {
        (Some(current), Some(next_api_key)) if current.api_key == next_api_key => {
            current.updated_at.clone()
        }
        _ => crate::auth::now_timestamp(),
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    })
}

pub(crate) fn map_auth_store_error_to_command_error(error: AuthFlowError) -> CommandError {
    if error.retryable {
        CommandError::retryable(error.code, error.message)
    } else {
        CommandError::user_fixable(error.code, error.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::global_db::migrations::migrations;
    use crate::provider_credentials::{
        load_all_provider_credentials, load_provider_credential, ProviderCredentialKind,
    };
    use rusqlite::Connection;

    fn open_in_memory() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open in-memory db");
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");
        migrations()
            .to_latest(&mut connection)
            .expect("walk migrations");
        connection
    }

    fn snapshot_with_openrouter_api_key() -> ProviderProfilesSnapshot {
        let timestamp = "2026-04-01T00:00:00Z";
        ProviderProfilesSnapshot {
            metadata: crate::provider_profiles::ProviderProfilesMetadataFile {
                version: 3,
                active_profile_id: "openrouter-default".into(),
                profiles: vec![ProviderProfileRecord {
                    profile_id: "openrouter-default".into(),
                    provider_id: "openrouter".into(),
                    runtime_kind: "openrouter".into(),
                    label: "OpenRouter".into(),
                    model_id: "openai/gpt-4.1-mini".into(),
                    preset_id: Some("openrouter".into()),
                    base_url: None,
                    api_version: None,
                    region: None,
                    project_id: None,
                    credential_link: Some(ProviderProfileCredentialLink::ApiKey {
                        updated_at: timestamp.into(),
                    }),
                    migrated_from_legacy: false,
                    migrated_at: None,
                    updated_at: timestamp.into(),
                }],
                updated_at: timestamp.into(),
                migration: None,
            },
            credentials: crate::provider_profiles::ProviderProfileCredentialsFile {
                api_keys: vec![ProviderApiKeyCredentialEntry {
                    profile_id: "openrouter-default".into(),
                    api_key: "sk-or-test".into(),
                    updated_at: timestamp.into(),
                }],
            },
        }
    }

    fn upsert_request_for_openrouter(api_key: Option<&str>) -> UpsertProviderProfileRequestDto {
        UpsertProviderProfileRequestDto {
            profile_id: "openrouter-default".into(),
            provider_id: "openrouter".into(),
            runtime_kind: "openrouter".into(),
            label: "OpenRouter".into(),
            model_id: "openai/gpt-4.1-mini".into(),
            preset_id: Some("openrouter".into()),
            base_url: None,
            api_version: None,
            region: None,
            project_id: None,
            api_key: api_key.map(str::to_owned),
            activate: false,
        }
    }

    #[test]
    fn mirror_writes_api_key_into_provider_credentials() {
        let connection = open_in_memory();
        let snapshot = snapshot_with_openrouter_api_key();
        let request = upsert_request_for_openrouter(Some("sk-or-test"));
        mirror_profile_credential_to_new_table(&connection, &snapshot, &request)
            .expect("mirror succeeds");

        let row = load_provider_credential(&connection, "openrouter")
            .expect("load")
            .expect("row present");
        assert_eq!(row.kind, ProviderCredentialKind::ApiKey);
        assert_eq!(row.api_key.as_deref(), Some("sk-or-test"));
        assert_eq!(row.default_model_id.as_deref(), Some("openai/gpt-4.1-mini"));
    }

    #[test]
    fn mirror_skips_openai_codex_writes() {
        let connection = open_in_memory();
        let snapshot = snapshot_with_openrouter_api_key();
        let request = UpsertProviderProfileRequestDto {
            provider_id: "openai_codex".into(),
            ..upsert_request_for_openrouter(None)
        };
        mirror_profile_credential_to_new_table(&connection, &snapshot, &request)
            .expect("mirror succeeds (no-op for openai_codex)");

        let rows = load_all_provider_credentials(&connection).expect("load");
        assert!(
            rows.iter().all(|row| row.provider_id != "openai_codex"),
            "OpenAI Codex must not be mirrored from the legacy api-key path"
        );
    }

    #[test]
    fn mirror_clears_credential_when_link_dropped() {
        let connection = open_in_memory();
        // Pre-populate the new table.
        let snapshot = snapshot_with_openrouter_api_key();
        mirror_profile_credential_to_new_table(
            &connection,
            &snapshot,
            &upsert_request_for_openrouter(Some("sk-or-test")),
        )
        .expect("seed mirror");
        assert!(load_provider_credential(&connection, "openrouter")
            .expect("load")
            .is_some());

        // Now mutate snapshot to drop the credential link & secret.
        let mut cleared = snapshot;
        cleared.metadata.profiles[0].credential_link = None;
        cleared.credentials.api_keys.clear();
        mirror_profile_credential_to_new_table(
            &connection,
            &cleared,
            &upsert_request_for_openrouter(None),
        )
        .expect("mirror clear");

        let row = load_provider_credential(&connection, "openrouter").expect("load");
        assert!(row.is_none(), "credential row must be removed when linkage drops");
    }
}
