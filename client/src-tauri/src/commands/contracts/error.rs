use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type CommandResult<T> = Result<T, CommandError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandErrorClass {
    UserFixable,
    Retryable,
    SystemFault,
    PolicyDenied,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[error("{message}")]
pub struct CommandError {
    pub code: String,
    pub class: CommandErrorClass,
    pub message: String,
    pub retryable: bool,
}

impl CommandError {
    pub fn new(
        code: impl Into<String>,
        class: CommandErrorClass,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            code: code.into(),
            class,
            message: message.into(),
            retryable,
        }
    }

    pub fn invalid_request(field: &'static str) -> Self {
        Self::new(
            "invalid_request",
            CommandErrorClass::UserFixable,
            format!("Field `{field}` must be a non-empty string."),
            false,
        )
    }

    pub fn user_fixable(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(code, CommandErrorClass::UserFixable, message, false)
    }

    pub fn policy_denied(message: impl Into<String>) -> Self {
        Self::new(
            "policy_denied",
            CommandErrorClass::PolicyDenied,
            message,
            false,
        )
    }

    pub fn retryable(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(code, CommandErrorClass::Retryable, message, true)
    }

    pub fn system_fault(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(code, CommandErrorClass::SystemFault, message, false)
    }

    pub fn backend_not_ready(command: &'static str) -> Self {
        Self::system_fault(
            "desktop_backend_not_ready",
            format!("Command {command} is not available from the desktop backend yet."),
        )
    }

    pub fn project_not_found() -> Self {
        Self::user_fixable(
            "project_not_found",
            "Project was not found in the local desktop registry.",
        )
    }
}

pub(crate) fn validate_non_empty(value: &str, field: &'static str) -> CommandResult<()> {
    if value.trim().is_empty() {
        return Err(CommandError::invalid_request(field));
    }

    Ok(())
}
