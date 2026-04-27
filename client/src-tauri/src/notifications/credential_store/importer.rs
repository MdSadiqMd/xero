use std::{fs, path::Path};

use rusqlite::Connection;

use crate::commands::{CommandError, CommandResult};

use super::{
    file_store::NotificationCredentialStoreFile,
    sql::{load_store, write_store},
};

/// Imports the legacy `notification-credentials.json` file into the global SQLite database.
///
/// Idempotent: if either `notification_credentials` or `notification_inbound_cursors` already
/// holds rows the importer treats Phase 2.3 as complete and returns Ok(()). Otherwise the JSON
/// is read, materialized into the two tables in a single transaction, and the JSON file is
/// deleted only after the transaction commits.
pub fn import_legacy_notification_credentials(
    connection: &mut Connection,
    legacy_path: &Path,
) -> CommandResult<()> {
    if global_db_already_populated(connection)? {
        return Ok(());
    }

    if !legacy_path.exists() {
        return Ok(());
    }

    let contents = fs::read_to_string(legacy_path).map_err(|error| {
        CommandError::retryable(
            "notification_adapter_credentials_read_failed",
            format!(
                "Cadence could not read legacy notification credentials at {}: {error}",
                legacy_path.display()
            ),
        )
    })?;

    let parsed: NotificationCredentialStoreFile =
        serde_json::from_str(&contents).map_err(|error| {
            CommandError::user_fixable(
                "notification_adapter_credentials_malformed",
                format!(
                    "Cadence could not decode legacy notification credentials at {}: {error}",
                    legacy_path.display()
                ),
            )
        })?;

    if parsed.routes.is_empty() && parsed.inbound_cursors.is_empty() {
        remove_legacy(legacy_path)?;
        return Ok(());
    }

    write_store(connection, &parsed).map_err(|error| {
        CommandError::retryable(
            error.code,
            format!(
                "Cadence could not import legacy notification credentials from {}: {}",
                legacy_path.display(),
                error.message
            ),
        )
    })?;

    remove_legacy(legacy_path)
}

fn global_db_already_populated(connection: &Connection) -> CommandResult<bool> {
    let store = load_store(connection).map_err(|error| {
        CommandError::retryable(
            error.code,
            format!(
                "Cadence could not probe notification credential tables before importing: {}",
                error.message
            ),
        )
    })?;
    Ok(!store.routes.is_empty() || !store.inbound_cursors.is_empty())
}

fn remove_legacy(path: &Path) -> CommandResult<()> {
    fs::remove_file(path).map_err(|error| {
        CommandError::retryable(
            "notification_adapter_credentials_legacy_cleanup_failed",
            format!(
                "Cadence imported {} into the global database but could not delete the legacy file: {error}",
                path.display()
            ),
        )
    })
}
