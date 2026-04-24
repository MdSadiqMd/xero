use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpTransportKindDto {
    Stdio,
    Http,
    Sse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpTransportDto {
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

impl McpTransportDto {
    pub const fn kind(&self) -> McpTransportKindDto {
        match self {
            Self::Stdio { .. } => McpTransportKindDto::Stdio,
            Self::Http { .. } => McpTransportKindDto::Http,
            Self::Sse { .. } => McpTransportKindDto::Sse,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpConnectionStatusDto {
    Connected,
    Failed,
    Blocked,
    Misconfigured,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpConnectionDiagnosticDto {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpConnectionStateDto {
    pub status: McpConnectionStatusDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<McpConnectionDiagnosticDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_healthy_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpEnvironmentReferenceDto {
    pub key: String,
    pub from_env: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpServerDto {
    pub id: String,
    pub name: String,
    pub transport: McpTransportDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<McpEnvironmentReferenceDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    pub connection: McpConnectionStateDto,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpRegistryDto {
    #[serde(default)]
    pub servers: Vec<McpServerDto>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpImportDiagnosticDto {
    pub index: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_id: Option<String>,
    pub code: String,
    pub message: String,
}
