use std::{collections::HashMap, fs, path::Path};

use rusqlite::Connection;
use serde::Deserialize;

use crate::commands::{CommandError, CommandResult};

use super::{
    sql::upsert_openai_codex_session, AuthFlowError, StoredOpenAiCodexSession,
    OPENAI_CODEX_PROVIDER_ID,
};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LegacyAuthStoreFile {
    #[serde(default)]
    openai_codex_sessions: HashMap<String, StoredOpenAiCodexSession>,
    #[allow(dead_code)]
    #[serde(default)]
    updated_at: String,
}

/// Imports legacy `openai-auth.json` into the `openai_codex_sessions` table.
///
/// Idempotent: if the table already has rows, this is a no-op. Otherwise the JSON file is read,
/// rows are upserted, and the JSON file is deleted only after successful writes.
pub fn import_legacy_openai_codex_sessions(
    connection: &Connection,
    legacy_path: &Path,
) -> CommandResult<()> {
    if table_has_rows(connection)? {
        return Ok(());
    }

    if !legacy_path.exists() {
        return Ok(());
    }

    let contents = fs::read_to_string(legacy_path).map_err(|error| {
        CommandError::retryable(
            "auth_store_read_failed",
            format!(
                "Cadence could not read the legacy auth store at {}: {error}",
                legacy_path.display()
            ),
        )
    })?;

    let parsed: LegacyAuthStoreFile = serde_json::from_str(&contents).map_err(|error| {
        CommandError::user_fixable(
            "auth_store_decode_failed",
            format!(
                "Cadence could not decode the legacy auth store at {}: {error}",
                legacy_path.display()
            ),
        )
    })?;

    if parsed.openai_codex_sessions.is_empty() {
        // File present but empty; remove it to keep the migration idempotent on subsequent boots.
        remove_legacy_file(legacy_path)?;
        return Ok(());
    }

    for mut session in parsed.openai_codex_sessions.into_values() {
        if session.provider_id.trim().is_empty() {
            session.provider_id = OPENAI_CODEX_PROVIDER_ID.into();
        }
        upsert_openai_codex_session(connection, &session).map_err(map_auth_error)?;
    }

    remove_legacy_file(legacy_path)
}

fn table_has_rows(connection: &Connection) -> CommandResult<bool> {
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM openai_codex_sessions", [], |row| {
            row.get(0)
        })
        .map_err(|error| {
            CommandError::retryable(
                "auth_store_read_failed",
                format!("Cadence could not probe openai_codex_sessions: {error}"),
            )
        })?;
    Ok(count > 0)
}

fn remove_legacy_file(path: &Path) -> CommandResult<()> {
    fs::remove_file(path).map_err(|error| {
        CommandError::retryable(
            "auth_store_legacy_cleanup_failed",
            format!(
                "Cadence imported {} into the global database but could not delete the legacy file: {error}",
                path.display()
            ),
        )
    })
}

fn map_auth_error(error: AuthFlowError) -> CommandError {
    if error.retryable {
        CommandError::retryable(error.code, error.message)
    } else {
        CommandError::user_fixable(error.code, error.message)
    }
}
