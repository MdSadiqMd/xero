use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::commands::{CommandError, CommandResult};

use super::store::{
    normalize_snapshot_for_persist, validate_provider_profiles_contract,
    ProviderApiKeyCredentialEntry, ProviderProfileCredentialLink, ProviderProfileCredentialsFile,
    ProviderProfileRecord, ProviderProfilesMetadataFile, ProviderProfilesMigrationState,
    ProviderProfilesSnapshot,
};

const PROVIDER_PROFILES_SCHEMA_VERSION: u32 = 3;

const SQL_SOURCE_LABEL: &str = "<global_db>";

pub fn load_provider_profiles_from_db(
    connection: &Connection,
) -> CommandResult<Option<ProviderProfilesSnapshot>> {
    let metadata_row = connection
        .query_row(
            "SELECT active_profile_id, updated_at, migration_source, migration_migrated_at, \
             migration_runtime_settings_updated_at, migration_openrouter_credentials_updated_at, \
             migration_openai_auth_updated_at, migration_openrouter_model_inferred \
             FROM provider_profiles_metadata WHERE id = 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<i64>>(7)?,
                ))
            },
        )
        .optional()
        .map_err(|error| {
            CommandError::retryable(
                "provider_profiles_read_failed",
                format!("Cadence could not read provider_profiles_metadata: {error}"),
            )
        })?;

    let Some((
        active_profile_id,
        updated_at,
        migration_source,
        migration_migrated_at,
        migration_runtime_settings_updated_at,
        migration_openrouter_credentials_updated_at,
        migration_openai_auth_updated_at,
        migration_openrouter_model_inferred,
    )) = metadata_row
    else {
        return Ok(None);
    };

    let profiles = load_profiles(connection)?;
    let credentials = load_api_key_credentials(connection)?;

    let migration = match migration_source {
        Some(source) => Some(ProviderProfilesMigrationState {
            source,
            migrated_at: migration_migrated_at.unwrap_or_default(),
            runtime_settings_updated_at: migration_runtime_settings_updated_at,
            openrouter_credentials_updated_at: migration_openrouter_credentials_updated_at,
            openai_auth_updated_at: migration_openai_auth_updated_at,
            openrouter_model_inferred: migration_openrouter_model_inferred.map(|value| value != 0),
        }),
        None => None,
    };

    let metadata = ProviderProfilesMetadataFile {
        version: PROVIDER_PROFILES_SCHEMA_VERSION,
        active_profile_id,
        profiles,
        updated_at,
        migration,
    };

    Ok(Some(validate_provider_profiles_contract(
        metadata,
        ProviderProfileCredentialsFile {
            api_keys: credentials,
        },
        std::path::Path::new(SQL_SOURCE_LABEL),
        std::path::Path::new(SQL_SOURCE_LABEL),
    )?))
}

pub fn persist_provider_profiles_to_db(
    connection: &mut Connection,
    snapshot: &ProviderProfilesSnapshot,
) -> CommandResult<()> {
    let snapshot = normalize_snapshot_for_persist(snapshot.clone())?;

    let tx = connection.transaction().map_err(|error| {
        CommandError::retryable(
            "provider_profiles_transaction_begin_failed",
            format!("Cadence could not start a provider_profiles transaction: {error}"),
        )
    })?;

    write_profiles(&tx, &snapshot.metadata.profiles)?;
    write_metadata(&tx, &snapshot.metadata)?;
    write_credentials(&tx, &snapshot.credentials.api_keys)?;

    tx.commit().map_err(|error| {
        CommandError::retryable(
            "provider_profiles_transaction_commit_failed",
            format!("Cadence could not commit provider_profiles updates: {error}"),
        )
    })
}

fn load_profiles(connection: &Connection) -> CommandResult<Vec<ProviderProfileRecord>> {
    let mut stmt = connection
        .prepare(
            "SELECT profile_id, provider_id, runtime_kind, label, model_id, preset_id, base_url, \
             api_version, region, scope_project_id, credential_link_kind, \
             credential_link_account_id, credential_link_session_id, credential_link_updated_at, \
             migrated_from_legacy, migrated_at, updated_at \
             FROM provider_profiles ORDER BY profile_id",
        )
        .map_err(|error| {
            CommandError::retryable(
                "provider_profiles_read_failed",
                format!("Cadence could not prepare provider_profiles read: {error}"),
            )
        })?;

    let rows = stmt
        .query_map([], |row| {
            let profile_id: String = row.get(0)?;
            let provider_id: String = row.get(1)?;
            let runtime_kind: String = row.get(2)?;
            let label: String = row.get(3)?;
            let model_id: String = row.get(4)?;
            let preset_id: Option<String> = row.get(5)?;
            let base_url: Option<String> = row.get(6)?;
            let api_version: Option<String> = row.get(7)?;
            let region: Option<String> = row.get(8)?;
            let project_id: Option<String> = row.get(9)?;
            let credential_link_kind: Option<String> = row.get(10)?;
            let credential_link_account_id: Option<String> = row.get(11)?;
            let credential_link_session_id: Option<String> = row.get(12)?;
            let credential_link_updated_at: Option<String> = row.get(13)?;
            let migrated_from_legacy: i64 = row.get(14)?;
            let migrated_at: Option<String> = row.get(15)?;
            let updated_at: String = row.get(16)?;

            let credential_link = build_credential_link(
                credential_link_kind.as_deref(),
                credential_link_account_id,
                credential_link_session_id,
                credential_link_updated_at,
            );

            Ok(ProviderProfileRecord {
                profile_id,
                provider_id,
                runtime_kind,
                label,
                model_id,
                preset_id,
                base_url,
                api_version,
                region,
                project_id,
                credential_link,
                migrated_from_legacy: migrated_from_legacy != 0,
                migrated_at,
                updated_at,
            })
        })
        .map_err(|error| {
            CommandError::retryable(
                "provider_profiles_read_failed",
                format!("Cadence could not read provider_profiles rows: {error}"),
            )
        })?;

    let mut profiles = Vec::new();
    for row in rows {
        let profile = row.map_err(|error| {
            CommandError::retryable(
                "provider_profiles_read_failed",
                format!("Cadence could not decode provider_profiles row: {error}"),
            )
        })?;
        profiles.push(profile);
    }

    Ok(profiles)
}

fn load_api_key_credentials(
    connection: &Connection,
) -> CommandResult<Vec<ProviderApiKeyCredentialEntry>> {
    let mut stmt = connection
        .prepare(
            "SELECT profile_id, api_key, updated_at FROM provider_profile_credentials \
             ORDER BY profile_id",
        )
        .map_err(|error| {
            CommandError::retryable(
                "provider_profile_credentials_read_failed",
                format!("Cadence could not prepare provider_profile_credentials read: {error}"),
            )
        })?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ProviderApiKeyCredentialEntry {
                profile_id: row.get(0)?,
                api_key: row.get(1)?,
                updated_at: row.get(2)?,
            })
        })
        .map_err(|error| {
            CommandError::retryable(
                "provider_profile_credentials_read_failed",
                format!("Cadence could not read provider_profile_credentials: {error}"),
            )
        })?;

    let mut credentials = Vec::new();
    for row in rows {
        credentials.push(row.map_err(|error| {
            CommandError::retryable(
                "provider_profile_credentials_read_failed",
                format!("Cadence could not decode provider_profile_credentials row: {error}"),
            )
        })?);
    }

    Ok(credentials)
}

fn write_profiles(tx: &Transaction<'_>, profiles: &[ProviderProfileRecord]) -> CommandResult<()> {
    let surviving_ids: Vec<String> = profiles
        .iter()
        .map(|profile| profile.profile_id.clone())
        .collect();

    if surviving_ids.is_empty() {
        tx.execute("DELETE FROM provider_profiles", [])
            .map_err(map_persist_error)?;
        return Ok(());
    }

    let placeholders = surviving_ids
        .iter()
        .enumerate()
        .map(|(index, _)| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let delete_sql = format!(
        "DELETE FROM provider_profiles WHERE profile_id NOT IN ({placeholders})"
    );
    tx.execute(
        delete_sql.as_str(),
        rusqlite::params_from_iter(surviving_ids.iter()),
    )
    .map_err(map_persist_error)?;

    for profile in profiles {
        let (link_kind, link_account_id, link_session_id, link_updated_at) =
            credential_link_columns(profile.credential_link.as_ref());

        tx.execute(
            "INSERT INTO provider_profiles (
                profile_id, provider_id, runtime_kind, label, model_id, preset_id, base_url,
                api_version, region, scope_project_id, credential_link_kind,
                credential_link_account_id, credential_link_session_id,
                credential_link_updated_at, migrated_from_legacy, migrated_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17
            ) ON CONFLICT(profile_id) DO UPDATE SET
                provider_id = excluded.provider_id,
                runtime_kind = excluded.runtime_kind,
                label = excluded.label,
                model_id = excluded.model_id,
                preset_id = excluded.preset_id,
                base_url = excluded.base_url,
                api_version = excluded.api_version,
                region = excluded.region,
                scope_project_id = excluded.scope_project_id,
                credential_link_kind = excluded.credential_link_kind,
                credential_link_account_id = excluded.credential_link_account_id,
                credential_link_session_id = excluded.credential_link_session_id,
                credential_link_updated_at = excluded.credential_link_updated_at,
                migrated_from_legacy = excluded.migrated_from_legacy,
                migrated_at = excluded.migrated_at,
                updated_at = excluded.updated_at",
            params![
                profile.profile_id,
                profile.provider_id,
                profile.runtime_kind,
                profile.label,
                profile.model_id,
                profile.preset_id,
                profile.base_url,
                profile.api_version,
                profile.region,
                profile.project_id,
                link_kind,
                link_account_id,
                link_session_id,
                link_updated_at,
                if profile.migrated_from_legacy { 1_i64 } else { 0_i64 },
                profile.migrated_at,
                profile.updated_at,
            ],
        )
        .map_err(map_persist_error)?;
    }

    Ok(())
}

fn write_metadata(
    tx: &Transaction<'_>,
    metadata: &ProviderProfilesMetadataFile,
) -> CommandResult<()> {
    let migration = metadata.migration.as_ref();
    tx.execute(
        "INSERT INTO provider_profiles_metadata (
            id, active_profile_id, updated_at, migration_source, migration_migrated_at,
            migration_runtime_settings_updated_at,
            migration_openrouter_credentials_updated_at,
            migration_openai_auth_updated_at,
            migration_openrouter_model_inferred
        ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(id) DO UPDATE SET
            active_profile_id = excluded.active_profile_id,
            updated_at = excluded.updated_at,
            migration_source = excluded.migration_source,
            migration_migrated_at = excluded.migration_migrated_at,
            migration_runtime_settings_updated_at = excluded.migration_runtime_settings_updated_at,
            migration_openrouter_credentials_updated_at = excluded.migration_openrouter_credentials_updated_at,
            migration_openai_auth_updated_at = excluded.migration_openai_auth_updated_at,
            migration_openrouter_model_inferred = excluded.migration_openrouter_model_inferred",
        params![
            metadata.active_profile_id,
            metadata.updated_at,
            migration.map(|m| m.source.clone()),
            migration.map(|m| m.migrated_at.clone()),
            migration.and_then(|m| m.runtime_settings_updated_at.clone()),
            migration.and_then(|m| m.openrouter_credentials_updated_at.clone()),
            migration.and_then(|m| m.openai_auth_updated_at.clone()),
            migration
                .and_then(|m| m.openrouter_model_inferred)
                .map(|value| if value { 1_i64 } else { 0_i64 }),
        ],
    )
    .map_err(map_persist_error)?;

    Ok(())
}

fn write_credentials(
    tx: &Transaction<'_>,
    credentials: &[ProviderApiKeyCredentialEntry],
) -> CommandResult<()> {
    if credentials.is_empty() {
        tx.execute("DELETE FROM provider_profile_credentials", [])
            .map_err(map_persist_error)?;
        return Ok(());
    }

    let surviving_ids: Vec<String> = credentials
        .iter()
        .map(|entry| entry.profile_id.clone())
        .collect();
    let placeholders = surviving_ids
        .iter()
        .enumerate()
        .map(|(index, _)| format!("?{}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let delete_sql = format!(
        "DELETE FROM provider_profile_credentials WHERE profile_id NOT IN ({placeholders})"
    );
    tx.execute(
        delete_sql.as_str(),
        rusqlite::params_from_iter(surviving_ids.iter()),
    )
    .map_err(map_persist_error)?;

    for entry in credentials {
        tx.execute(
            "INSERT INTO provider_profile_credentials (profile_id, api_key, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(profile_id) DO UPDATE SET
                api_key = excluded.api_key,
                updated_at = excluded.updated_at",
            params![entry.profile_id, entry.api_key, entry.updated_at],
        )
        .map_err(map_persist_error)?;
    }

    Ok(())
}

fn build_credential_link(
    kind: Option<&str>,
    account_id: Option<String>,
    session_id: Option<String>,
    updated_at: Option<String>,
) -> Option<ProviderProfileCredentialLink> {
    let kind = kind?;
    let updated_at = updated_at.unwrap_or_default();
    match kind {
        "openai_codex" => Some(ProviderProfileCredentialLink::OpenAiCodex {
            account_id: account_id.unwrap_or_default(),
            session_id: session_id.unwrap_or_default(),
            updated_at,
        }),
        "api_key" => Some(ProviderProfileCredentialLink::ApiKey { updated_at }),
        "local" => Some(ProviderProfileCredentialLink::Local { updated_at }),
        "ambient" => Some(ProviderProfileCredentialLink::Ambient { updated_at }),
        _ => None,
    }
}

fn credential_link_columns(
    link: Option<&ProviderProfileCredentialLink>,
) -> (Option<String>, Option<String>, Option<String>, Option<String>) {
    match link {
        Some(ProviderProfileCredentialLink::OpenAiCodex {
            account_id,
            session_id,
            updated_at,
        }) => (
            Some("openai_codex".into()),
            Some(account_id.clone()),
            Some(session_id.clone()),
            Some(updated_at.clone()),
        ),
        Some(ProviderProfileCredentialLink::ApiKey { updated_at }) => (
            Some("api_key".into()),
            None,
            None,
            Some(updated_at.clone()),
        ),
        Some(ProviderProfileCredentialLink::Local { updated_at }) => (
            Some("local".into()),
            None,
            None,
            Some(updated_at.clone()),
        ),
        Some(ProviderProfileCredentialLink::Ambient { updated_at }) => (
            Some("ambient".into()),
            None,
            None,
            Some(updated_at.clone()),
        ),
        None => (None, None, None, None),
    }
}

fn map_persist_error(error: rusqlite::Error) -> CommandError {
    CommandError::retryable(
        "provider_profiles_write_failed",
        format!("Cadence could not write provider_profiles: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::global_db::migrations::migrations;
    use crate::provider_profiles::store::default_provider_profiles_snapshot;

    fn open_in_memory() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open in-memory db");
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .expect("enable foreign keys");
        migrations()
            .to_latest(&mut connection)
            .expect("walk migrations to latest");
        connection
    }

    #[test]
    fn load_returns_none_when_metadata_missing() {
        let connection = open_in_memory();
        let snapshot = load_provider_profiles_from_db(&connection).expect("load");
        assert!(snapshot.is_none());
    }

    #[test]
    fn persist_then_load_roundtrips_default_snapshot() {
        let mut connection = open_in_memory();
        let snapshot = default_provider_profiles_snapshot();
        persist_provider_profiles_to_db(&mut connection, &snapshot).expect("persist");
        let loaded = load_provider_profiles_from_db(&connection)
            .expect("load")
            .expect("snapshot present");
        assert_eq!(loaded.metadata.active_profile_id, snapshot.metadata.active_profile_id);
        assert_eq!(loaded.metadata.profiles.len(), snapshot.metadata.profiles.len());
    }
}
