use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::{
    commands::{CommandError, CommandResult},
    global_db::environment_profile::{
        parse_environment_tool_category, EnvironmentProfileValidationError, EnvironmentToolCategory,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAddedToolRow {
    pub id: String,
    pub category: EnvironmentToolCategory,
    pub command: String,
    pub args: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUserAddedToolRow {
    pub id: String,
    pub category: EnvironmentToolCategory,
    pub command: String,
    pub args: Vec<String>,
}

#[derive(Debug)]
struct RawUserAddedToolRow {
    id: String,
    category: String,
    command: String,
    args_json: String,
    created_at: String,
    updated_at: String,
}

pub fn list_user_added_environment_tools(
    connection: &Connection,
) -> CommandResult<Vec<UserAddedToolRow>> {
    let mut stmt = connection
        .prepare(
            "SELECT id, category, command, args_json, created_at, updated_at
             FROM user_added_environment_tools
             ORDER BY created_at ASC, id ASC",
        )
        .map_err(load_error)?;

    let rows = stmt
        .query_map([], |row| {
            Ok(RawUserAddedToolRow {
                id: row.get(0)?,
                category: row.get(1)?,
                command: row.get(2)?,
                args_json: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(load_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(load_error)?;

    rows.into_iter().map(decode_row).collect()
}

pub fn user_added_environment_tool_exists(
    connection: &Connection,
    id: &str,
) -> CommandResult<bool> {
    connection
        .query_row(
            "SELECT 1 FROM user_added_environment_tools WHERE id = ?1",
            [id],
            |_| Ok(()),
        )
        .optional()
        .map(|row| row.is_some())
        .map_err(load_error)
}

pub fn insert_user_added_environment_tool(
    tx: &Transaction<'_>,
    row: &NewUserAddedToolRow,
    timestamp: &str,
) -> CommandResult<()> {
    let args_json = serde_json::to_string(&row.args).map_err(|error| {
        CommandError::system_fault(
            "environment_user_tool_encode_failed",
            format!("Xero could not encode the tool version arguments: {error}"),
        )
    })?;

    tx.execute(
        "INSERT INTO user_added_environment_tools (
            id, category, command, args_json, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![
            &row.id,
            row.category.as_str(),
            &row.command,
            args_json,
            timestamp,
        ],
    )
    .map(|_| ())
    .map_err(|error| {
        CommandError::user_fixable(
            "environment_user_tool_exists",
            format!(
                "A custom environment tool named `{}` already exists. Choose a different tool name.",
                row.id
            ),
        )
        .or_sqlite_constraint(error)
    })
}

pub fn delete_user_added_environment_tool(connection: &Connection, id: &str) -> CommandResult<()> {
    connection
        .execute(
            "DELETE FROM user_added_environment_tools WHERE id = ?1",
            [id],
        )
        .map(|_| ())
        .map_err(|error| {
            CommandError::system_fault(
                "environment_user_tool_delete_failed",
                format!("Xero could not remove the custom environment tool `{id}`: {error}"),
            )
        })
}

fn decode_row(raw: RawUserAddedToolRow) -> CommandResult<UserAddedToolRow> {
    let category = parse_environment_tool_category(&raw.category).map_err(invalid_row)?;
    let args = serde_json::from_str::<Vec<String>>(&raw.args_json).map_err(|error| {
        CommandError::system_fault(
            "environment_user_tool_invalid",
            format!(
                "Xero found invalid version arguments for custom environment tool `{}`: {error}",
                raw.id
            ),
        )
    })?;

    Ok(UserAddedToolRow {
        id: raw.id,
        category,
        command: raw.command,
        args,
        created_at: raw.created_at,
        updated_at: raw.updated_at,
    })
}

fn load_error(error: rusqlite::Error) -> CommandError {
    CommandError::system_fault(
        "environment_user_tools_load_failed",
        format!("Xero could not load custom environment tools: {error}"),
    )
}

fn invalid_row(error: EnvironmentProfileValidationError) -> CommandError {
    CommandError::system_fault(
        "environment_user_tool_invalid",
        format!("Xero found an invalid custom environment tool row: {error}"),
    )
}

trait ConstraintFallback {
    fn or_sqlite_constraint(self, error: rusqlite::Error) -> CommandError;
}

impl ConstraintFallback for CommandError {
    fn or_sqlite_constraint(self, error: rusqlite::Error) -> CommandError {
        match error {
            rusqlite::Error::SqliteFailure(_, _) => self,
            other => CommandError::system_fault(
                "environment_user_tool_save_failed",
                format!("Xero could not save the custom environment tool: {other}"),
            ),
        }
    }
}
