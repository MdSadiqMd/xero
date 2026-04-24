use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::{
    auth::now_timestamp,
    commands::{
        get_runtime_settings::{remove_file_if_exists, write_json_file_atomically},
        CommandError, CommandResult,
    },
};

pub const MCP_REGISTRY_FILE_NAME: &str = "mcp-registry.json";
const MCP_REGISTRY_SCHEMA_VERSION: u32 = 1;
const MAX_IMPORT_DIAGNOSTICS: usize = 64;
const MCP_IMPORT_DIAGNOSTIC_CODE: &str = "mcp_registry_import_invalid";

fn mcp_registry_schema_version() -> u32 {
    MCP_REGISTRY_SCHEMA_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpRegistry {
    #[serde(default = "mcp_registry_schema_version")]
    pub version: u32,
    #[serde(default)]
    pub servers: Vec<McpServerRecord>,
    #[serde(default = "now_timestamp")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpServerRecord {
    pub id: String,
    pub name: String,
    pub transport: McpTransport,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<McpEnvironmentReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default = "default_connection_state")]
    pub connection: McpConnectionState,
    #[serde(default = "now_timestamp")]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpTransport {
    Stdio {
        command: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
    },
    Http {
        url: String,
    },
    Sse {
        url: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpEnvironmentReference {
    pub key: String,
    pub from_env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpConnectionStatus {
    Connected,
    Failed,
    Blocked,
    Misconfigured,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpConnectionDiagnostic {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpConnectionState {
    pub status: McpConnectionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<McpConnectionDiagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_healthy_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRegistryImportDiagnostic {
    pub index: u32,
    pub server_id: Option<String>,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRegistryImportResult {
    pub registry: McpRegistry,
    pub diagnostics: Vec<McpRegistryImportDiagnostic>,
}

pub fn default_mcp_registry() -> McpRegistry {
    McpRegistry {
        version: MCP_REGISTRY_SCHEMA_VERSION,
        servers: Vec::new(),
        updated_at: now_timestamp(),
    }
}

pub fn load_mcp_registry_from_path(path: &Path) -> CommandResult<McpRegistry> {
    if !path.exists() {
        return Ok(default_mcp_registry());
    }

    let contents = fs::read_to_string(path).map_err(|error| {
        CommandError::retryable(
            "mcp_registry_read_failed",
            format!(
                "Cadence could not read the app-local MCP registry file at {}: {error}",
                path.display()
            ),
        )
    })?;

    let parsed = serde_json::from_str::<McpRegistry>(&contents).map_err(|error| {
        CommandError::user_fixable(
            "mcp_registry_decode_failed",
            format!(
                "Cadence could not decode the app-local MCP registry file at {}: {error}",
                path.display()
            ),
        )
    })?;

    validate_registry(parsed, &format!("{}", path.display()))
}

pub fn persist_mcp_registry(path: &Path, next: &McpRegistry) -> CommandResult<McpRegistry> {
    let normalized = validate_registry(next.clone(), "requested MCP registry update")?;
    let previous_snapshot = snapshot_existing_file(path)?;

    let json = serde_json::to_vec_pretty(&normalized).map_err(|error| {
        CommandError::system_fault(
            "mcp_registry_serialize_failed",
            format!("Cadence could not serialize the MCP registry update: {error}"),
        )
    })?;

    write_json_file_atomically(path, &json, "mcp_registry")?;

    match load_mcp_registry_from_path(path) {
        Ok(loaded) => Ok(loaded),
        Err(error) => {
            let rollback = restore_file_snapshot(path, previous_snapshot.as_deref());
            if let Err(rollback_error) = rollback {
                return Err(CommandError::retryable(
                    "mcp_registry_rollback_failed",
                    format!(
                        "Cadence rejected the persisted MCP registry at {} and could not restore the previous snapshot: {}. Validation error: {}",
                        path.display(), rollback_error.message, error.message
                    ),
                ));
            }
            Err(error)
        }
    }
}

pub fn parse_mcp_registry_import_file(path: &Path) -> CommandResult<Vec<Value>> {
    let contents = fs::read_to_string(path).map_err(|error| {
        CommandError::user_fixable(
            MCP_IMPORT_DIAGNOSTIC_CODE,
            format!(
                "Cadence could not read MCP import JSON from {}: {error}",
                path.display()
            ),
        )
    })?;

    let value = serde_json::from_str::<Value>(&contents).map_err(|error| {
        CommandError::user_fixable(
            MCP_IMPORT_DIAGNOSTIC_CODE,
            format!(
                "Cadence could not parse MCP import JSON from {}: {error}",
                path.display()
            ),
        )
    })?;

    match value {
        Value::Array(entries) => Ok(entries),
        Value::Object(object) => object
            .get("servers")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| {
                CommandError::user_fixable(
                    MCP_IMPORT_DIAGNOSTIC_CODE,
                    format!(
                        "Cadence expected MCP import JSON at {} to be an array or an object containing `servers`.",
                        path.display()
                    ),
                )
            }),
        _ => Err(CommandError::user_fixable(
            MCP_IMPORT_DIAGNOSTIC_CODE,
            format!(
                "Cadence expected MCP import JSON at {} to be an array or an object containing `servers`.",
                path.display()
            ),
        )),
    }
}

pub fn apply_mcp_registry_import(
    current: &McpRegistry,
    entries: Vec<Value>,
    source_path: &Path,
) -> McpRegistryImportResult {
    let mut diagnostics = Vec::new();
    let mut seen_import_ids = BTreeSet::new();
    let mut next_servers_by_id = current
        .servers
        .iter()
        .cloned()
        .map(|server| (server.id.clone(), server))
        .collect::<BTreeMap<_, _>>();

    for (index, entry) in entries.into_iter().enumerate() {
        let index = index as u32;
        let server_id_hint = entry
            .get("id")
            .and_then(Value::as_str)
            .map(|id| id.trim().to_owned())
            .filter(|id| !id.is_empty());

        let decoded = match serde_json::from_value::<McpServerRecord>(entry) {
            Ok(decoded) => decoded,
            Err(error) => {
                push_import_diagnostic(
                    &mut diagnostics,
                    index,
                    server_id_hint,
                    format!(
                        "Cadence rejected MCP import entry #{index} from {}: {error}",
                        source_path.display()
                    ),
                );
                continue;
            }
        };

        let validated = match validate_server_record(
            decoded,
            &format!("MCP import entry #{index} from {}", source_path.display()),
        ) {
            Ok(validated) => validated,
            Err(error) => {
                push_import_diagnostic(
                    &mut diagnostics,
                    index,
                    server_id_hint,
                    format!(
                        "Cadence rejected MCP import entry #{index} from {}: {}",
                        source_path.display(),
                        error.message
                    ),
                );
                continue;
            }
        };

        if !seen_import_ids.insert(validated.id.clone()) {
            push_import_diagnostic(
                &mut diagnostics,
                index,
                Some(validated.id.clone()),
                format!(
                    "Cadence rejected MCP import entry #{index} from {} because id `{}` was duplicated in the import batch.",
                    source_path.display(), validated.id
                ),
            );
            continue;
        }

        next_servers_by_id.insert(validated.id.clone(), validated);
    }

    let mut merged_servers = next_servers_by_id.into_values().collect::<Vec<_>>();
    merged_servers.sort_by(|left, right| left.id.cmp(&right.id));

    let mut next_registry = current.clone();
    if next_registry.servers != merged_servers {
        next_registry.servers = merged_servers;
        next_registry.updated_at = now_timestamp();
    }

    McpRegistryImportResult {
        registry: next_registry,
        diagnostics,
    }
}

fn push_import_diagnostic(
    diagnostics: &mut Vec<McpRegistryImportDiagnostic>,
    index: u32,
    server_id: Option<String>,
    message: String,
) {
    if diagnostics.len() >= MAX_IMPORT_DIAGNOSTICS {
        return;
    }

    diagnostics.push(McpRegistryImportDiagnostic {
        index,
        server_id,
        code: MCP_IMPORT_DIAGNOSTIC_CODE.into(),
        message,
    });
}

fn validate_registry(registry: McpRegistry, source: &str) -> CommandResult<McpRegistry> {
    if registry.version != MCP_REGISTRY_SCHEMA_VERSION {
        return Err(CommandError::user_fixable(
            "mcp_registry_invalid",
            format!(
                "Cadence rejected MCP registry data from {source} because schema version {} is unsupported.",
                registry.version
            ),
        ));
    }

    let mut seen_ids = BTreeSet::new();
    let mut servers = Vec::with_capacity(registry.servers.len());
    for server in registry.servers {
        let validated = validate_server_record(server, source)?;
        if !seen_ids.insert(validated.id.clone()) {
            return Err(CommandError::user_fixable(
                "mcp_registry_invalid",
                format!(
                    "Cadence rejected MCP registry data from {source} because server id `{}` was duplicated.",
                    validated.id
                ),
            ));
        }
        servers.push(validated);
    }

    servers.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(McpRegistry {
        version: MCP_REGISTRY_SCHEMA_VERSION,
        servers,
        updated_at: normalize_timestamp(registry.updated_at),
    })
}

fn validate_server_record(server: McpServerRecord, source: &str) -> CommandResult<McpServerRecord> {
    let id = normalize_non_empty(server.id, "id", source)?;
    if !is_identifier(&id) {
        return Err(CommandError::user_fixable(
            "mcp_registry_invalid",
            format!(
                "Cadence rejected MCP server `{id}` from {source} because ids may only contain letters, numbers, hyphen, underscore, or dot.",
            ),
        ));
    }

    let name = normalize_non_empty(server.name, "name", source)?;
    let transport = validate_transport(server.transport, &id, source)?;
    let env = validate_environment(server.env, &id, source)?;

    let cwd = server.cwd.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_owned())
        }
    });

    let connection = validate_connection(server.connection, &id, source)?;

    Ok(McpServerRecord {
        id,
        name,
        transport,
        env,
        cwd,
        connection,
        updated_at: normalize_timestamp(server.updated_at),
    })
}

fn validate_transport(
    transport: McpTransport,
    id: &str,
    source: &str,
) -> CommandResult<McpTransport> {
    match transport {
        McpTransport::Stdio { command, args } => {
            let command = normalize_non_empty(command, "transport.command", source)?;
            let mut normalized_args = Vec::with_capacity(args.len());
            for arg in args {
                let trimmed = arg.trim();
                if trimmed.is_empty() {
                    return Err(CommandError::user_fixable(
                        "mcp_registry_invalid",
                        format!(
                            "Cadence rejected MCP server `{id}` from {source} because transport args cannot be blank.",
                        ),
                    ));
                }
                normalized_args.push(trimmed.to_owned());
            }

            Ok(McpTransport::Stdio {
                command,
                args: normalized_args,
            })
        }
        McpTransport::Http { url } => Ok(McpTransport::Http {
            url: validate_transport_url(url, id, source)?,
        }),
        McpTransport::Sse { url } => Ok(McpTransport::Sse {
            url: validate_transport_url(url, id, source)?,
        }),
    }
}

fn validate_transport_url(url: String, id: &str, source: &str) -> CommandResult<String> {
    let url = normalize_non_empty(url, "transport.url", source)?;
    let parsed = Url::parse(&url).map_err(|error| {
        CommandError::user_fixable(
            "mcp_registry_invalid",
            format!(
                "Cadence rejected MCP server `{id}` from {source} because URL `{url}` was invalid: {error}",
            ),
        )
    })?;

    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(CommandError::user_fixable(
                "mcp_registry_invalid",
                format!(
                    "Cadence rejected MCP server `{id}` from {source} because URL `{url}` used unsupported scheme `{other}`.",
                ),
            ));
        }
    }

    if !parsed.username().trim().is_empty() || parsed.password().is_some() {
        return Err(CommandError::user_fixable(
            "mcp_registry_invalid",
            format!(
                "Cadence rejected MCP server `{id}` from {source} because transport URLs must not embed credentials.",
            ),
        ));
    }

    Ok(url)
}

fn validate_environment(
    env: Vec<McpEnvironmentReference>,
    id: &str,
    source: &str,
) -> CommandResult<Vec<McpEnvironmentReference>> {
    let mut seen_keys = BTreeSet::new();
    let mut normalized = Vec::with_capacity(env.len());

    for entry in env {
        let key = normalize_non_empty(entry.key, "env.key", source)?;
        let from_env = normalize_non_empty(entry.from_env, "env.fromEnv", source)?;

        if !is_environment_variable_name(&key) {
            return Err(CommandError::user_fixable(
                "mcp_registry_invalid",
                format!(
                    "Cadence rejected MCP server `{id}` from {source} because env key `{key}` was not a valid environment variable name.",
                ),
            ));
        }

        if !is_environment_variable_name(&from_env) {
            return Err(CommandError::user_fixable(
                "mcp_registry_invalid",
                format!(
                    "Cadence rejected MCP server `{id}` from {source} because env source `{from_env}` was not a valid environment variable name.",
                ),
            ));
        }

        if !seen_keys.insert(key.clone()) {
            return Err(CommandError::user_fixable(
                "mcp_registry_invalid",
                format!(
                    "Cadence rejected MCP server `{id}` from {source} because env key `{key}` was duplicated.",
                ),
            ));
        }

        normalized.push(McpEnvironmentReference { key, from_env });
    }

    normalized.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(normalized)
}

fn validate_connection(
    connection: McpConnectionState,
    id: &str,
    source: &str,
) -> CommandResult<McpConnectionState> {
    let diagnostic = connection.diagnostic.map(|diagnostic| {
        let code = diagnostic.code.trim();
        let message = diagnostic.message.trim();
        if code.is_empty() || message.is_empty() {
            return Err(CommandError::user_fixable(
                "mcp_registry_invalid",
                format!(
                    "Cadence rejected MCP server `{id}` from {source} because connection diagnostics require non-empty code and message.",
                ),
            ));
        }

        Ok(McpConnectionDiagnostic {
            code: code.to_owned(),
            message: message.to_owned(),
            retryable: diagnostic.retryable,
        })
    });

    Ok(McpConnectionState {
        status: connection.status,
        diagnostic: diagnostic.transpose()?,
        last_checked_at: connection.last_checked_at.map(normalize_timestamp),
        last_healthy_at: connection.last_healthy_at.map(normalize_timestamp),
    })
}

fn normalize_non_empty(value: String, field: &str, source: &str) -> CommandResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(CommandError::user_fixable(
            "mcp_registry_invalid",
            format!(
                "Cadence rejected MCP registry data from {source} because `{field}` was blank.",
            ),
        ));
    }
    Ok(trimmed.to_owned())
}

fn normalize_timestamp(value: String) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        now_timestamp()
    } else {
        trimmed.to_owned()
    }
}

fn is_identifier(value: &str) -> bool {
    value.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || character == '-'
            || character == '_'
            || character == '.'
    })
}

fn is_environment_variable_name(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }

    characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn default_connection_state() -> McpConnectionState {
    McpConnectionState {
        status: McpConnectionStatus::Stale,
        diagnostic: Some(McpConnectionDiagnostic {
            code: "mcp_status_unchecked".into(),
            message: "Cadence has not checked this MCP server yet.".into(),
            retryable: true,
        }),
        last_checked_at: None,
        last_healthy_at: None,
    }
}

fn snapshot_existing_file(path: &Path) -> CommandResult<Option<Vec<u8>>> {
    if !path.exists() || path.is_dir() {
        return Ok(None);
    }

    fs::read(path).map(Some).map_err(|error| {
        CommandError::retryable(
            "mcp_registry_read_failed",
            format!(
                "Cadence could not snapshot the app-local MCP registry file at {} before updating it: {error}",
                path.display()
            ),
        )
    })
}

fn restore_file_snapshot(path: &Path, snapshot: Option<&[u8]>) -> CommandResult<()> {
    match snapshot {
        Some(bytes) => write_json_file_atomically(path, bytes, "mcp_registry_rollback"),
        None => remove_file_if_exists(path, "mcp_registry_rollback"),
    }
}
