//! Events emitted by the Solana workbench backend. Shape mirrors the
//! emulator sidebar so the frontend hook can listen in the same way.

use serde::{Deserialize, Serialize};

pub const SOLANA_VALIDATOR_STATUS_EVENT: &str = "solana:validator:status";
pub const SOLANA_VALIDATOR_LOG_EVENT: &str = "solana:validator:log";
pub const SOLANA_TOOLCHAIN_STATUS_CHANGED_EVENT: &str = "solana:toolchain:changed";
pub const SOLANA_RPC_HEALTH_EVENT: &str = "solana:rpc:health";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorPhase {
    Idle,
    Booting,
    Ready,
    Stopping,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorStatusPayload {
    pub phase: ValidatorPhase,
    pub kind: Option<String>,
    pub rpc_url: Option<String>,
    pub ws_url: Option<String>,
    pub message: Option<String>,
}

impl ValidatorStatusPayload {
    pub fn new(phase: ValidatorPhase) -> Self {
        Self {
            phase,
            kind: None,
            rpc_url: None,
            ws_url: None,
            message: None,
        }
    }

    pub fn with_kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    pub fn with_rpc_url(mut self, url: impl Into<String>) -> Self {
        self.rpc_url = Some(url.into());
        self
    }

    pub fn with_ws_url(mut self, url: impl Into<String>) -> Self {
        self.ws_url = Some(url.into());
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorLogPayload {
    pub level: ValidatorLogLevel,
    pub message: String,
    pub ts_ms: u64,
}
