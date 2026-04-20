use std::path::Path;

use rusqlite::Connection;

use crate::{
    commands::CommandError,
    db::{configure_connection, migrations::migrations},
};

fn open_state_database(repo_root: &Path, database_path: &Path) -> Result<Connection, CommandError> {
    if !repo_root.is_dir() {
        return Err(CommandError::user_fixable(
            "project_root_unavailable",
            format!(
                "Imported project root {} is no longer available.",
                repo_root.display()
            ),
        ));
    }

    if !database_path.exists() {
        return Err(CommandError::retryable(
            "project_state_unavailable",
            format!(
                "Imported project at {} is missing repo-local state at {}.",
                repo_root.display(),
                database_path.display()
            ),
        ));
    }

    let connection = Connection::open(database_path).map_err(|error| {
        CommandError::retryable(
            "project_state_open_failed",
            format!(
                "Cadence could not open the repo-local database at {} for {}: {error}",
                database_path.display(),
                repo_root.display()
            ),
        )
    })?;

    configure_connection(&connection)?;
    Ok(connection)
}

pub(crate) fn open_project_database(
    repo_root: &Path,
    database_path: &Path,
) -> Result<Connection, CommandError> {
    let mut connection = open_state_database(repo_root, database_path)?;
    migrations().to_latest(&mut connection).map_err(|error| {
        CommandError::retryable(
            "project_state_migration_failed",
            format!(
                "Cadence could not migrate the repo-local selected-project state at {}: {error}",
                database_path.display()
            ),
        )
    })?;
    Ok(connection)
}

pub(crate) fn open_runtime_database(
    repo_root: &Path,
    database_path: &Path,
) -> Result<Connection, CommandError> {
    let mut connection = open_state_database(repo_root, database_path)?;
    migrations().to_latest(&mut connection).map_err(|error| {
        CommandError::retryable(
            "runtime_session_migration_failed",
            format!(
                "Cadence could not migrate the repo-local runtime-session tables at {}: {error}",
                database_path.display()
            ),
        )
    })?;
    Ok(connection)
}
