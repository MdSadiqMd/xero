use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex},
    thread,
    time::Duration,
};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::{
    auth::now_timestamp,
    commands::{validate_non_empty, CommandError, CommandResult},
    global_db::{
        environment_profile::{
            parse_environment_profile_status, validate_environment_profile_row,
            EnvironmentDiagnostic, EnvironmentDiagnosticSeverity, EnvironmentPathProfile,
            EnvironmentPermissionRequest, EnvironmentPermissionStatus, EnvironmentPlatform,
            EnvironmentProfilePayload, EnvironmentProfileRow, EnvironmentProfileStatus,
            EnvironmentProfileSummary, EnvironmentToolCategory, EnvironmentToolProbeStatus,
            EnvironmentToolSummary, ENVIRONMENT_PROFILE_SCHEMA_VERSION,
        },
        open_global_database,
        user_added_tools::{
            delete_user_added_environment_tool, insert_user_added_environment_tool,
            list_user_added_environment_tools, user_added_environment_tool_exists,
            NewUserAddedToolRow,
        },
    },
};

use super::probe::{
    built_in_environment_probe_catalog, probe_environment_profile_with,
    probe_environment_profile_with_user_tools, EnvironmentBinaryResolver,
    EnvironmentCommandExecutor, EnvironmentProbeCatalogEntry, EnvironmentProbeOptions,
    EnvironmentProbeReport, SystemEnvironmentBinaryResolver, SystemEnvironmentCommandExecutor,
};

const PROFILE_STALE_AFTER: Duration = Duration::from_secs(7 * 24 * 60 * 60);

static ACTIVE_DISCOVERIES: LazyLock<Mutex<HashSet<PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvironmentDiscoveryStatus {
    pub has_profile: bool,
    pub status: EnvironmentProfileStatus,
    pub stale: bool,
    pub should_start: bool,
    pub refreshed_at: Option<String>,
    pub probe_started_at: Option<String>,
    pub probe_completed_at: Option<String>,
    pub permission_requests: Vec<EnvironmentPermissionRequest>,
    pub diagnostics: Vec<EnvironmentDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EnvironmentPermissionDecision {
    pub id: String,
    pub status: EnvironmentPermissionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerifyUserToolRequest {
    pub id: String,
    pub category: EnvironmentToolCategory,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerifyUserToolResponse {
    pub record: EnvironmentToolSummary,
    #[serde(default)]
    pub diagnostics: Vec<EnvironmentDiagnostic>,
}

pub fn environment_discovery_status(
    database_path: &Path,
) -> CommandResult<EnvironmentDiscoveryStatus> {
    let connection = open_global_database(database_path)?;
    let row = load_environment_profile_row(&connection)?;
    Ok(status_from_row(
        row.as_ref(),
        discovery_is_active(database_path),
    ))
}

pub fn environment_profile_summary(
    database_path: &Path,
) -> CommandResult<Option<EnvironmentProfileSummary>> {
    let connection = open_global_database(database_path)?;
    let Some(row) = load_environment_profile_row(&connection)? else {
        return Ok(None);
    };
    serde_json::from_str(&row.summary_json)
        .map(Some)
        .map_err(|error| {
            CommandError::system_fault(
                "environment_profile_summary_decode_failed",
                format!("Xero could not decode the environment profile summary: {error}"),
            )
        })
}

pub fn start_environment_discovery(
    database_path: PathBuf,
) -> CommandResult<EnvironmentDiscoveryStatus> {
    start_environment_discovery_with_policy(database_path, false)
}

pub fn refresh_environment_discovery(
    database_path: PathBuf,
) -> CommandResult<EnvironmentDiscoveryStatus> {
    start_environment_discovery_with_policy(database_path, true)
}

pub fn resolve_environment_permission_requests(
    database_path: &Path,
    decisions: Vec<EnvironmentPermissionDecision>,
) -> CommandResult<EnvironmentDiscoveryStatus> {
    if decisions.is_empty() {
        return Err(CommandError::invalid_request("decisions"));
    }

    for decision in &decisions {
        validate_non_empty(decision.id.as_str(), "decisions.id")?;
        if decision.status == EnvironmentPermissionStatus::Pending {
            return Err(CommandError::invalid_request("decisions.status"));
        }
    }

    let mut connection = open_global_database(database_path)?;
    let row = load_environment_profile_row(&connection)?.ok_or_else(|| {
        CommandError::user_fixable(
            "environment_profile_missing",
            "Xero cannot resolve environment access decisions before an environment profile exists.",
        )
    })?;

    let mut payload: EnvironmentProfilePayload =
        serde_json::from_str(&row.payload_json).map_err(|error| {
            CommandError::system_fault(
                "environment_profile_payload_decode_failed",
                format!("Xero could not decode the environment profile payload: {error}"),
            )
        })?;
    let mut summary: EnvironmentProfileSummary =
        serde_json::from_str(&row.summary_json).map_err(|error| {
            CommandError::system_fault(
                "environment_profile_summary_decode_failed",
                format!("Xero could not decode the environment profile summary: {error}"),
            )
        })?;

    for decision in decisions {
        let Some(permission) = payload
            .permissions
            .iter_mut()
            .find(|permission| permission.id == decision.id)
        else {
            return Err(CommandError::user_fixable(
                "environment_permission_request_not_found",
                format!(
                    "Xero could not find an environment access request named `{}`.",
                    decision.id
                ),
            ));
        };

        if !permission.optional && decision.status != EnvironmentPermissionStatus::Granted {
            return Err(CommandError::user_fixable(
                "environment_permission_required",
                format!(
                    "Environment access request `{}` must be allowed before onboarding can continue.",
                    permission.title
                ),
            ));
        }

        permission.status = decision.status;
        if let Some(summary_permission) = summary
            .permission_requests
            .iter_mut()
            .find(|permission| permission.id == decision.id)
        {
            summary_permission.status = decision.status;
        }
    }

    let payload_json = serialize_profile_json(&payload)?;
    let summary_json = serialize_profile_json(&summary)?;
    let permission_requests_json = serialize_profile_json(&payload.permissions)?;
    upsert_environment_profile(
        &mut connection,
        row.status,
        &payload.platform,
        row.path_fingerprint.as_deref(),
        &payload_json,
        &summary_json,
        &permission_requests_json,
        &row.diagnostics_json,
        row.probe_started_at.as_deref(),
        row.probe_completed_at.as_deref(),
        &row.refreshed_at,
    )?;

    environment_discovery_status(database_path)
}

pub fn verify_user_environment_tool(
    request: VerifyUserToolRequest,
) -> CommandResult<VerifyUserToolResponse> {
    verify_user_environment_tool_with(
        request,
        Arc::new(SystemEnvironmentBinaryResolver::from_process()),
        Arc::new(SystemEnvironmentCommandExecutor),
        EnvironmentProbeOptions::default(),
    )
}

pub fn save_user_environment_tool(
    database_path: &Path,
    request: VerifyUserToolRequest,
) -> CommandResult<EnvironmentProbeReport> {
    let row = validate_user_tool_request(&request)?;
    reject_builtin_tool_id(&row.id)?;

    let verification = verify_user_environment_tool(request)?;
    if verification.record.probe_status != EnvironmentToolProbeStatus::Ok
        || !verification.record.present
    {
        return Err(CommandError::user_fixable(
            "environment_user_tool_not_verified",
            format!(
                "Xero could not save `{}` because its version probe did not verify successfully.",
                row.id
            ),
        ));
    }

    let mut connection = open_global_database(database_path)?;
    if user_added_environment_tool_exists(&connection, &row.id)? {
        return Err(CommandError::user_fixable(
            "environment_user_tool_exists",
            format!(
                "A custom environment tool named `{}` already exists. Choose a different tool name.",
                row.id
            ),
        ));
    }

    let timestamp = now_timestamp();
    let tx = connection.transaction().map_err(|error| {
        CommandError::system_fault(
            "environment_user_tool_save_failed",
            format!("Xero could not start a transaction for the custom environment tool: {error}"),
        )
    })?;
    insert_user_added_environment_tool(&tx, &row, &timestamp)?;
    tx.commit().map_err(|error| {
        CommandError::system_fault(
            "environment_user_tool_save_failed",
            format!("Xero could not save the custom environment tool: {error}"),
        )
    })?;

    refresh_environment_profile_report(&mut connection)
}

pub fn remove_user_environment_tool(
    database_path: &Path,
    id: String,
) -> CommandResult<EnvironmentProbeReport> {
    let id = validate_user_tool_id(&id)?;
    let mut connection = open_global_database(database_path)?;
    delete_user_added_environment_tool(&connection, &id)?;
    refresh_environment_profile_report(&mut connection)
}

fn start_environment_discovery_with_policy(
    database_path: PathBuf,
    force: bool,
) -> CommandResult<EnvironmentDiscoveryStatus> {
    if !mark_discovery_active(&database_path) {
        return environment_discovery_status(&database_path);
    }

    let mut connection = open_global_database(&database_path)?;
    let current = load_environment_profile_row(&connection)?;
    if !force && !status_from_row(current.as_ref(), false).should_start {
        unmark_discovery_active(&database_path);
        return Ok(status_from_row(current.as_ref(), false));
    }

    persist_marker_profile(&mut connection, EnvironmentProfileStatus::Probing)?;
    let started_status = status_from_row(load_environment_profile_row(&connection)?.as_ref(), true);
    let worker_database_path = database_path.clone();
    thread::spawn(move || {
        let report = probe_environment_profile_for_database(&worker_database_path);
        match open_global_database(&worker_database_path) {
            Ok(mut connection) => {
                let result = match report {
                    Ok(report) => persist_probe_report(&mut connection, &report),
                    Err(error) => persist_failed_profile(
                        &mut connection,
                        "environment_probe_failed",
                        error.message,
                    ),
                };
                if let Err(error) = result {
                    eprintln!("[environment] discovery persistence failed: {error}");
                }
            }
            Err(error) => {
                eprintln!("[environment] discovery could not open global database: {error}");
            }
        }
        unmark_discovery_active(&worker_database_path);
    });

    Ok(started_status)
}

fn verify_user_environment_tool_with(
    request: VerifyUserToolRequest,
    resolver: Arc<dyn EnvironmentBinaryResolver>,
    executor: Arc<dyn EnvironmentCommandExecutor>,
    options: EnvironmentProbeOptions,
) -> CommandResult<VerifyUserToolResponse> {
    let row = validate_user_tool_request(&request)?;
    let report = probe_environment_profile_with(
        vec![EnvironmentProbeCatalogEntry {
            id: row.id,
            category: row.category,
            command: row.command,
            args: row.args,
            custom: true,
        }],
        resolver,
        executor,
        options,
    )
    .map_err(environment_probe_validation_error)?;

    let record = report.summary.tools.into_iter().next().ok_or_else(|| {
        CommandError::system_fault(
            "environment_user_tool_verify_failed",
            "Xero could not read the verification result for the custom environment tool.",
        )
    })?;

    Ok(VerifyUserToolResponse {
        record,
        diagnostics: report.summary.diagnostics,
    })
}

fn probe_environment_profile_for_database(
    database_path: &Path,
) -> CommandResult<EnvironmentProbeReport> {
    let connection = open_global_database(database_path)?;
    let user_tools = list_user_added_environment_tools(&connection)?;
    probe_environment_profile_with_user_tools(user_tools)
        .map_err(environment_probe_validation_error)
}

fn refresh_environment_profile_report(
    connection: &mut Connection,
) -> CommandResult<EnvironmentProbeReport> {
    let user_tools = list_user_added_environment_tools(connection)?;
    let report = probe_environment_profile_with_user_tools(user_tools)
        .map_err(environment_probe_validation_error)?;
    persist_probe_report(connection, &report)?;
    Ok(report)
}

fn validate_user_tool_request(
    request: &VerifyUserToolRequest,
) -> CommandResult<NewUserAddedToolRow> {
    let id = validate_user_tool_id(&request.id)?;
    let command = validate_user_tool_command(&request.command)?;
    let mut args = Vec::with_capacity(request.args.len());
    if request.args.len() > 8 {
        return Err(CommandError::user_fixable(
            "environment_user_tool_args_invalid",
            "Version arguments must include at most 8 items.",
        ));
    }

    for arg in &request.args {
        let trimmed = arg.trim();
        if trimmed.is_empty()
            || trimmed.len() > 128
            || contains_control_character(trimmed)
            || crate::runtime::redaction::find_prohibited_persistence_content(trimmed).is_some()
        {
            return Err(CommandError::user_fixable(
                "environment_user_tool_args_invalid",
                "Each version argument must be a non-empty string under 128 characters without secret-like content.",
            ));
        }
        args.push(trimmed.to_string());
    }

    Ok(NewUserAddedToolRow {
        id,
        category: request.category,
        command,
        args,
    })
}

fn validate_user_tool_id(id: &str) -> CommandResult<String> {
    let id = id.trim();
    if id.is_empty()
        || id.len() > 32
        || id != id.to_ascii_lowercase()
        || !is_slug_safe_tool_id(id)
        || crate::runtime::redaction::find_prohibited_persistence_content(id).is_some()
    {
        return Err(CommandError::user_fixable(
            "environment_user_tool_id_invalid",
            "Tool name must be 1-32 lowercase characters and use only a-z, 0-9, underscore, or dash.",
        ));
    }
    Ok(id.to_string())
}

fn validate_user_tool_command(command: &str) -> CommandResult<String> {
    let command = command.trim();
    if command.is_empty()
        || command.len() > 256
        || contains_control_character(command)
        || crate::runtime::redaction::find_prohibited_persistence_content(command).is_some()
    {
        return Err(CommandError::invalid_request("command"));
    }
    if contains_shell_metacharacter(command) {
        return Err(CommandError::user_fixable(
            "environment_user_tool_command_invalid",
            "Command must be an executable name or absolute path without shell metacharacters.",
        ));
    }

    let looks_like_path = command.contains('/') || command.contains('\\');
    if looks_like_path && !Path::new(command).is_absolute() {
        return Err(CommandError::user_fixable(
            "environment_user_tool_command_invalid",
            "Command paths must be absolute. Use a plain executable name for PATH resolution.",
        ));
    }
    if !looks_like_path && command.chars().any(char::is_whitespace) {
        return Err(CommandError::user_fixable(
            "environment_user_tool_command_invalid",
            "Plain executable names must not contain whitespace.",
        ));
    }

    Ok(command.to_string())
}

fn reject_builtin_tool_id(id: &str) -> CommandResult<()> {
    if built_in_environment_probe_catalog()
        .iter()
        .any(|entry| entry.id == id)
    {
        return Err(CommandError::user_fixable(
            "environment_user_tool_conflict_with_builtin",
            format!("`{id}` is already part of Xero's built-in environment catalog."),
        ));
    }
    Ok(())
}

fn is_slug_safe_tool_id(id: &str) -> bool {
    let mut chars = id.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

fn contains_control_character(value: &str) -> bool {
    value.chars().any(|ch| ch.is_control())
}

fn contains_shell_metacharacter(value: &str) -> bool {
    value.chars().any(|ch| {
        matches!(
            ch,
            ';' | '&' | '|' | '`' | '$' | '<' | '>' | '*' | '?' | '!'
        )
    })
}

fn environment_probe_validation_error(
    error: crate::global_db::environment_profile::EnvironmentProfileValidationError,
) -> CommandError {
    CommandError::system_fault(
        "environment_probe_invalid",
        format!("Xero could not build a valid environment profile: {error}"),
    )
}

fn status_from_row(
    row: Option<&EnvironmentProfileRow>,
    active: bool,
) -> EnvironmentDiscoveryStatus {
    let Some(row) = row else {
        return EnvironmentDiscoveryStatus {
            has_profile: false,
            status: EnvironmentProfileStatus::Pending,
            stale: true,
            should_start: !active,
            refreshed_at: None,
            probe_started_at: None,
            probe_completed_at: None,
            permission_requests: vec![],
            diagnostics: vec![],
        };
    };

    let stale = match row.status {
        EnvironmentProfileStatus::Ready | EnvironmentProfileStatus::Partial => {
            timestamp_is_stale(&row.refreshed_at)
        }
        EnvironmentProfileStatus::Pending
        | EnvironmentProfileStatus::Probing
        | EnvironmentProfileStatus::Failed => true,
    };

    EnvironmentDiscoveryStatus {
        has_profile: true,
        status: row.status,
        stale,
        should_start: !active && stale,
        refreshed_at: Some(row.refreshed_at.clone()),
        probe_started_at: row.probe_started_at.clone(),
        probe_completed_at: row.probe_completed_at.clone(),
        permission_requests: pending_permission_requests(&row.permission_requests_json),
        diagnostics: serde_json::from_str(&row.diagnostics_json).unwrap_or_default(),
    }
}

fn pending_permission_requests(json: &str) -> Vec<EnvironmentPermissionRequest> {
    serde_json::from_str::<Vec<EnvironmentPermissionRequest>>(json)
        .unwrap_or_default()
        .into_iter()
        .filter(|request| request.status == EnvironmentPermissionStatus::Pending)
        .collect()
}

fn timestamp_is_stale(timestamp: &str) -> bool {
    let Ok(parsed) =
        time::OffsetDateTime::parse(timestamp, &time::format_description::well_known::Rfc3339)
    else {
        return true;
    };
    let age = time::OffsetDateTime::now_utc() - parsed;
    age.whole_seconds() < 0 || age.whole_seconds() as u64 > PROFILE_STALE_AFTER.as_secs()
}

fn discovery_is_active(database_path: &Path) -> bool {
    ACTIVE_DISCOVERIES
        .lock()
        .map(|active| active.contains(database_path))
        .unwrap_or(false)
}

fn mark_discovery_active(database_path: &Path) -> bool {
    ACTIVE_DISCOVERIES
        .lock()
        .map(|mut active| active.insert(database_path.to_path_buf()))
        .unwrap_or(false)
}

fn unmark_discovery_active(database_path: &Path) {
    if let Ok(mut active) = ACTIVE_DISCOVERIES.lock() {
        active.remove(database_path);
    }
}

fn load_environment_profile_row(
    connection: &Connection,
) -> CommandResult<Option<EnvironmentProfileRow>> {
    let row = connection
        .query_row(
            "SELECT schema_version, status, os_kind, os_version, arch, default_shell,
                    path_fingerprint, payload_json, summary_json, permission_requests_json,
                    diagnostics_json, probe_started_at, probe_completed_at, refreshed_at
             FROM environment_profile
             WHERE id = 1",
            [],
            |row| {
                let status: String = row.get(1)?;
                Ok((status, row_to_environment_profile(row)?))
            },
        )
        .optional()
        .map_err(|error| {
            CommandError::system_fault(
                "environment_profile_load_failed",
                format!("Xero could not load the environment profile: {error}"),
            )
        })?;

    let Some((status, mut row)) = row else {
        return Ok(None);
    };
    row.status = parse_environment_profile_status(&status).map_err(validation_error)?;
    validate_environment_profile_row(&row).map_err(validation_error)?;
    Ok(Some(row))
}

fn row_to_environment_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<EnvironmentProfileRow> {
    Ok(EnvironmentProfileRow {
        schema_version: row.get(0)?,
        status: EnvironmentProfileStatus::Pending,
        os_kind: row.get(2)?,
        os_version: row.get(3)?,
        arch: row.get(4)?,
        default_shell: row.get(5)?,
        path_fingerprint: row.get(6)?,
        payload_json: row.get(7)?,
        summary_json: row.get(8)?,
        permission_requests_json: row.get(9)?,
        diagnostics_json: row.get(10)?,
        probe_started_at: row.get(11)?,
        probe_completed_at: row.get(12)?,
        refreshed_at: row.get(13)?,
    })
}

fn persist_marker_profile(
    connection: &mut Connection,
    status: EnvironmentProfileStatus,
) -> CommandResult<()> {
    let timestamp = now_timestamp();
    let platform = current_platform();
    let path = EnvironmentPathProfile {
        entry_count: 0,
        fingerprint: None,
        sources: vec![],
    };
    let payload = EnvironmentProfilePayload {
        schema_version: ENVIRONMENT_PROFILE_SCHEMA_VERSION,
        platform: platform.clone(),
        path,
        tools: vec![],
        capabilities: vec![],
        permissions: vec![],
        diagnostics: vec![],
    };
    let summary = EnvironmentProfileSummary {
        schema_version: ENVIRONMENT_PROFILE_SCHEMA_VERSION,
        status,
        platform: platform.clone(),
        refreshed_at: Some(timestamp.clone()),
        tools: vec![],
        capabilities: vec![],
        permission_requests: vec![],
        diagnostics: vec![],
    };

    let payload_json = serialize_profile_json(&payload)?;
    let summary_json = serialize_profile_json(&summary)?;
    upsert_environment_profile(
        connection,
        status,
        &platform,
        None,
        &payload_json,
        &summary_json,
        "[]",
        "[]",
        if status == EnvironmentProfileStatus::Probing {
            Some(timestamp.as_str())
        } else {
            None
        },
        None,
        &timestamp,
    )
}

fn persist_probe_report(
    connection: &mut Connection,
    report: &EnvironmentProbeReport,
) -> CommandResult<()> {
    let payload_json = serialize_profile_json(&report.payload)?;
    let summary_json = serialize_profile_json(&report.summary)?;
    let permission_requests_json = serialize_profile_json(&report.payload.permissions)?;
    let diagnostics_json = serialize_profile_json(&report.payload.diagnostics)?;
    upsert_environment_profile(
        connection,
        report.status,
        &report.payload.platform,
        report.payload.path.fingerprint.as_deref(),
        &payload_json,
        &summary_json,
        &permission_requests_json,
        &diagnostics_json,
        Some(&report.started_at),
        Some(&report.completed_at),
        &report.completed_at,
    )
}

fn persist_failed_profile(
    connection: &mut Connection,
    code: &str,
    message: String,
) -> CommandResult<()> {
    let timestamp = now_timestamp();
    let platform = current_platform();
    let diagnostics = vec![EnvironmentDiagnostic {
        code: code.into(),
        severity: EnvironmentDiagnosticSeverity::Error,
        message,
        retryable: true,
        tool_id: None,
    }];
    let payload = EnvironmentProfilePayload {
        schema_version: ENVIRONMENT_PROFILE_SCHEMA_VERSION,
        platform: platform.clone(),
        path: EnvironmentPathProfile {
            entry_count: 0,
            fingerprint: None,
            sources: vec![],
        },
        tools: vec![],
        capabilities: vec![],
        permissions: vec![],
        diagnostics: diagnostics.clone(),
    };
    let summary = EnvironmentProfileSummary {
        schema_version: ENVIRONMENT_PROFILE_SCHEMA_VERSION,
        status: EnvironmentProfileStatus::Failed,
        platform: platform.clone(),
        refreshed_at: Some(timestamp.clone()),
        tools: vec![],
        capabilities: vec![],
        permission_requests: vec![],
        diagnostics,
    };
    let payload_json = serialize_profile_json(&payload)?;
    let summary_json = serialize_profile_json(&summary)?;
    let diagnostics_json = serialize_profile_json(&payload.diagnostics)?;
    upsert_environment_profile(
        connection,
        EnvironmentProfileStatus::Failed,
        &platform,
        None,
        &payload_json,
        &summary_json,
        "[]",
        &diagnostics_json,
        None,
        Some(&timestamp),
        &timestamp,
    )
}

#[allow(clippy::too_many_arguments)]
fn upsert_environment_profile(
    connection: &mut Connection,
    status: EnvironmentProfileStatus,
    platform: &EnvironmentPlatform,
    path_fingerprint: Option<&str>,
    payload_json: &str,
    summary_json: &str,
    permission_requests_json: &str,
    diagnostics_json: &str,
    probe_started_at: Option<&str>,
    probe_completed_at: Option<&str>,
    refreshed_at: &str,
) -> CommandResult<()> {
    connection
        .execute(
            "INSERT INTO environment_profile (
                id, schema_version, status, os_kind, os_version, arch, default_shell,
                path_fingerprint, payload_json, summary_json, permission_requests_json,
                diagnostics_json, probe_started_at, probe_completed_at, refreshed_at, updated_at
            ) VALUES (
                1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?14
            )
            ON CONFLICT(id) DO UPDATE SET
                schema_version = excluded.schema_version,
                status = excluded.status,
                os_kind = excluded.os_kind,
                os_version = excluded.os_version,
                arch = excluded.arch,
                default_shell = excluded.default_shell,
                path_fingerprint = excluded.path_fingerprint,
                payload_json = excluded.payload_json,
                summary_json = excluded.summary_json,
                permission_requests_json = excluded.permission_requests_json,
                diagnostics_json = excluded.diagnostics_json,
                probe_started_at = excluded.probe_started_at,
                probe_completed_at = excluded.probe_completed_at,
                refreshed_at = excluded.refreshed_at,
                updated_at = excluded.updated_at",
            params![
                ENVIRONMENT_PROFILE_SCHEMA_VERSION,
                status.as_str(),
                &platform.os_kind,
                platform.os_version.as_deref(),
                &platform.arch,
                platform.default_shell.as_deref(),
                path_fingerprint,
                payload_json,
                summary_json,
                permission_requests_json,
                diagnostics_json,
                probe_started_at,
                probe_completed_at,
                refreshed_at,
            ],
        )
        .map(|_| ())
        .map_err(|error| {
            CommandError::system_fault(
                "environment_profile_save_failed",
                format!("Xero could not save the environment profile: {error}"),
            )
        })
}

fn current_platform() -> EnvironmentPlatform {
    EnvironmentPlatform {
        os_kind: env::consts::OS.to_string(),
        os_version: None,
        arch: env::consts::ARCH.to_string(),
        default_shell: None,
    }
}

fn serialize_profile_json<T: Serialize>(value: &T) -> CommandResult<String> {
    serde_json::to_string(value).map_err(|error| {
        CommandError::system_fault(
            "environment_profile_encode_failed",
            format!("Xero could not encode the environment profile: {error}"),
        )
    })
}

fn validation_error(
    error: crate::global_db::environment_profile::EnvironmentProfileValidationError,
) -> CommandError {
    CommandError::system_fault(
        "environment_profile_invalid",
        format!("Xero found an invalid environment profile: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::probe::{EnvironmentCommandExecution, ResolvedEnvironmentBinary};
    use crate::global_db::{
        configure_connection,
        environment_profile::{
            EnvironmentPermissionKind, EnvironmentPermissionStatus, EnvironmentToolSource,
        },
        migrations, open_global_database,
    };
    use std::{collections::HashMap, ffi::OsString};

    fn connection() -> Connection {
        let mut connection = Connection::open_in_memory().expect("open db");
        configure_connection(&connection).expect("configure db");
        migrations::migrations()
            .to_latest(&mut connection)
            .expect("migrate db");
        connection
    }

    #[derive(Debug, Clone)]
    struct FakeResolver {
        binaries: HashMap<String, ResolvedEnvironmentBinary>,
    }

    impl EnvironmentBinaryResolver for FakeResolver {
        fn resolve(&self, command: &str) -> Option<ResolvedEnvironmentBinary> {
            self.binaries.get(command).cloned()
        }

        fn path_profile(&self) -> EnvironmentPathProfile {
            EnvironmentPathProfile {
                entry_count: 1,
                fingerprint: Some("sha256-test".into()),
                sources: vec!["tauri-process-path".into()],
            }
        }

        fn child_envs(&self) -> Vec<(OsString, OsString)> {
            vec![]
        }
    }

    #[derive(Debug, Clone)]
    struct FakeExecutor {
        execution: EnvironmentCommandExecution,
    }

    impl EnvironmentCommandExecutor for FakeExecutor {
        fn run(
            &self,
            _binary: &Path,
            _args: &[String],
            _timeout: Duration,
            _child_envs: &[(OsString, OsString)],
        ) -> EnvironmentCommandExecution {
            self.execution.clone()
        }
    }

    #[test]
    fn missing_profile_requests_silent_start() {
        let status = status_from_row(None, false);

        assert!(!status.has_profile);
        assert_eq!(status.status, EnvironmentProfileStatus::Pending);
        assert!(status.should_start);
        assert!(status.permission_requests.is_empty());
    }

    #[test]
    fn fresh_ready_profile_does_not_restart() {
        let mut connection = connection();
        persist_marker_profile(&mut connection, EnvironmentProfileStatus::Ready)
            .expect("persist profile");
        let row = load_environment_profile_row(&connection)
            .expect("load row")
            .expect("row");
        let status = status_from_row(Some(&row), false);

        assert!(status.has_profile);
        assert_eq!(status.status, EnvironmentProfileStatus::Ready);
        assert!(!status.stale);
        assert!(!status.should_start);
    }

    #[test]
    fn probing_profile_restarts_when_no_worker_is_active() {
        let mut connection = connection();
        persist_marker_profile(&mut connection, EnvironmentProfileStatus::Probing)
            .expect("persist profile");
        let row = load_environment_profile_row(&connection)
            .expect("load row")
            .expect("row");

        assert!(status_from_row(Some(&row), false).should_start);
        assert!(!status_from_row(Some(&row), true).should_start);
    }

    #[test]
    fn resolving_environment_permissions_persists_decisions_and_hides_resolved_requests() {
        let dir = tempfile::tempdir().expect("temp dir");
        let database_path = dir.path().join("xero.db");
        seed_permission_profile(&database_path);

        let status = resolve_environment_permission_requests(
            &database_path,
            vec![
                EnvironmentPermissionDecision {
                    id: "required-toolchain-access".into(),
                    status: EnvironmentPermissionStatus::Granted,
                },
                EnvironmentPermissionDecision {
                    id: "optional-network-access".into(),
                    status: EnvironmentPermissionStatus::Skipped,
                },
            ],
        )
        .expect("resolve permissions");

        assert!(status.permission_requests.is_empty());
        let summary = environment_profile_summary(&database_path)
            .expect("summary")
            .expect("profile");
        assert!(summary
            .permission_requests
            .iter()
            .any(|request| request.id == "required-toolchain-access"
                && request.status == EnvironmentPermissionStatus::Granted));
        assert!(summary
            .permission_requests
            .iter()
            .any(|request| request.id == "optional-network-access"
                && request.status == EnvironmentPermissionStatus::Skipped));
    }

    #[test]
    fn resolving_environment_permissions_rejects_skipped_required_access() {
        let dir = tempfile::tempdir().expect("temp dir");
        let database_path = dir.path().join("xero.db");
        seed_permission_profile(&database_path);

        let error = resolve_environment_permission_requests(
            &database_path,
            vec![EnvironmentPermissionDecision {
                id: "required-toolchain-access".into(),
                status: EnvironmentPermissionStatus::Skipped,
            }],
        )
        .expect_err("required access cannot be skipped");

        assert_eq!(error.code, "environment_permission_required");
    }

    #[test]
    fn saving_user_tool_rejects_builtin_id_before_insert() {
        let dir = tempfile::tempdir().expect("temp dir");
        let database_path = dir.path().join("xero.db");

        let error = save_user_environment_tool(
            &database_path,
            VerifyUserToolRequest {
                id: "git".into(),
                category: EnvironmentToolCategory::BaseDeveloperTool,
                command: "git".into(),
                args: vec!["--version".into()],
            },
        )
        .expect_err("built-in ids are rejected");

        assert_eq!(error.code, "environment_user_tool_conflict_with_builtin");
        let connection = open_global_database(&database_path).expect("open db");
        assert!(
            !user_added_environment_tool_exists(&connection, "git").expect("exists query"),
            "built-in conflict must not insert a custom row"
        );
    }

    #[test]
    fn verify_user_tool_drops_sensitive_output() {
        let mut binaries = HashMap::new();
        binaries.insert(
            "opaque-cli".into(),
            ResolvedEnvironmentBinary {
                path: std::env::temp_dir().join("opaque-cli"),
                source: EnvironmentToolSource::Path,
            },
        );

        let response = verify_user_environment_tool_with(
            VerifyUserToolRequest {
                id: "opaque_cli".into(),
                category: EnvironmentToolCategory::ShellUtility,
                command: "opaque-cli".into(),
                args: vec!["--version".into()],
            },
            Arc::new(FakeResolver { binaries }),
            Arc::new(FakeExecutor {
                execution: EnvironmentCommandExecution::Completed {
                    success: true,
                    stdout: b"opaque-cli sk-demo-token\n".to_vec(),
                    stderr: vec![],
                },
            }),
            EnvironmentProbeOptions {
                timeout: Duration::from_millis(25),
                concurrency: 1,
            },
        )
        .expect("verify response");

        assert_eq!(
            response.record.probe_status,
            EnvironmentToolProbeStatus::Failed
        );
        assert!(response.record.version.is_none());
        assert!(response
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "environment_probe_failed"));
    }

    #[cfg(unix)]
    #[test]
    fn user_tool_save_round_trips_and_remove_is_idempotent() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir");
        let database_path = dir.path().join("xero.db");
        let script = dir.path().join("custom-version");
        std::fs::write(&script, "#!/bin/sh\necho custom-version 4.5.6\n").expect("write fixture");
        let mut permissions = std::fs::metadata(&script)
            .expect("fixture metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).expect("chmod fixture");

        let report = save_user_environment_tool(
            &database_path,
            VerifyUserToolRequest {
                id: "custom_version".into(),
                category: EnvironmentToolCategory::ShellUtility,
                command: script.to_string_lossy().into_owned(),
                args: vec!["--version".into()],
            },
        )
        .expect("save custom tool");

        assert!(report.summary.tools.iter().any(|tool| {
            tool.id == "custom_version"
                && tool.custom
                && tool.version.as_deref() == Some("custom-version 4.5.6")
        }));

        let summary = environment_profile_summary(&database_path)
            .expect("load summary")
            .expect("profile summary");
        assert!(summary
            .tools
            .iter()
            .any(|tool| tool.id == "custom_version" && tool.custom));

        let report = remove_user_environment_tool(&database_path, "missing_custom".into())
            .expect("missing remove is no-op");
        assert!(report
            .summary
            .tools
            .iter()
            .any(|tool| tool.id == "custom_version"));

        let report = remove_user_environment_tool(&database_path, "custom_version".into())
            .expect("remove custom tool");
        assert!(!report
            .summary
            .tools
            .iter()
            .any(|tool| tool.id == "custom_version"));
    }

    fn seed_permission_profile(database_path: &Path) {
        let mut connection = open_global_database(database_path).expect("open global db");
        let timestamp = now_timestamp();
        let platform = current_platform();
        let permissions = vec![
            EnvironmentPermissionRequest {
                id: "required-toolchain-access".into(),
                kind: EnvironmentPermissionKind::ProtectedPath,
                status: EnvironmentPermissionStatus::Pending,
                title: "Required toolchain access".into(),
                reason: "Allow Xero to inspect the selected toolchain directory.".into(),
                optional: false,
            },
            EnvironmentPermissionRequest {
                id: "optional-network-access".into(),
                kind: EnvironmentPermissionKind::NetworkAccess,
                status: EnvironmentPermissionStatus::Pending,
                title: "Optional network access".into(),
                reason: "Allow Xero to refresh extra package-manager metadata.".into(),
                optional: true,
            },
        ];
        let payload = EnvironmentProfilePayload {
            schema_version: ENVIRONMENT_PROFILE_SCHEMA_VERSION,
            platform: platform.clone(),
            path: EnvironmentPathProfile {
                entry_count: 0,
                fingerprint: None,
                sources: vec![],
            },
            tools: vec![],
            capabilities: vec![],
            permissions: permissions.clone(),
            diagnostics: vec![],
        };
        let summary = EnvironmentProfileSummary {
            schema_version: ENVIRONMENT_PROFILE_SCHEMA_VERSION,
            status: EnvironmentProfileStatus::Partial,
            platform: platform.clone(),
            refreshed_at: Some(timestamp.clone()),
            tools: vec![],
            capabilities: vec![],
            permission_requests: permissions,
            diagnostics: vec![],
        };
        let payload_json = serialize_profile_json(&payload).expect("payload json");
        let summary_json = serialize_profile_json(&summary).expect("summary json");
        let permission_requests_json =
            serialize_profile_json(&payload.permissions).expect("permissions json");
        upsert_environment_profile(
            &mut connection,
            EnvironmentProfileStatus::Partial,
            &platform,
            None,
            &payload_json,
            &summary_json,
            &permission_requests_json,
            "[]",
            Some(&timestamp),
            Some(&timestamp),
            &timestamp,
        )
        .expect("seed profile");
    }
}
