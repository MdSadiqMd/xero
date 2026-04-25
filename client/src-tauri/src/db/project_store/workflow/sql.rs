use super::*;

pub(crate) fn validate_non_empty_text(
    value: &str,
    field: &str,
    code: &str,
) -> Result<(), CommandError> {
    if value.trim().is_empty() {
        return Err(CommandError::user_fixable(
            code,
            format!("Field `{field}` must be a non-empty string."),
        ));
    }

    Ok(())
}

pub(crate) fn parse_workflow_gate_state(value: &str) -> Result<WorkflowGateState, String> {
    match value {
        "pending" => Ok(WorkflowGateState::Pending),
        "satisfied" => Ok(WorkflowGateState::Satisfied),
        "blocked" => Ok(WorkflowGateState::Blocked),
        "skipped" => Ok(WorkflowGateState::Skipped),
        other => Err(format!(
            "Field `gate_state` must be a known workflow gate state, found `{other}`."
        )),
    }
}

pub(crate) fn workflow_gate_state_sql_value(value: &WorkflowGateState) -> &'static str {
    match value {
        WorkflowGateState::Pending => "pending",
        WorkflowGateState::Satisfied => "satisfied",
        WorkflowGateState::Blocked => "blocked",
        WorkflowGateState::Skipped => "skipped",
    }
}

pub(crate) fn parse_workflow_transition_gate_decision(
    value: &str,
) -> Result<WorkflowTransitionGateDecision, String> {
    match value {
        "approved" => Ok(WorkflowTransitionGateDecision::Approved),
        "rejected" => Ok(WorkflowTransitionGateDecision::Rejected),
        "blocked" => Ok(WorkflowTransitionGateDecision::Blocked),
        "not_applicable" => Ok(WorkflowTransitionGateDecision::NotApplicable),
        other => Err(format!(
            "Field `gate_decision` must be a known transition gate decision, found `{other}`."
        )),
    }
}

pub(crate) fn workflow_transition_gate_decision_sql_value(
    value: &WorkflowTransitionGateDecision,
) -> &'static str {
    match value {
        WorkflowTransitionGateDecision::Approved => "approved",
        WorkflowTransitionGateDecision::Rejected => "rejected",
        WorkflowTransitionGateDecision::Blocked => "blocked",
        WorkflowTransitionGateDecision::NotApplicable => "not_applicable",
    }
}

pub(crate) fn phase_status_sql_value(value: &PhaseStatus) -> &'static str {
    match value {
        PhaseStatus::Complete => "complete",
        PhaseStatus::Active => "active",
        PhaseStatus::Pending => "pending",
        PhaseStatus::Blocked => "blocked",
    }
}

pub(crate) fn phase_step_sql_value(value: &PhaseStep) -> &'static str {
    match value {
        PhaseStep::Discuss => "discuss",
        PhaseStep::Plan => "plan",
        PhaseStep::Execute => "execute",
        PhaseStep::Verify => "verify",
        PhaseStep::Ship => "ship",
    }
}

pub(crate) fn map_workflow_graph_transaction_error(
    code: &str,
    database_path: &Path,
    error: SqlError,
    message: &str,
) -> CommandError {
    if is_retryable_sql_error(&error) {
        CommandError::retryable(
            code,
            format!("{message} {}", sqlite_path_suffix(database_path)),
        )
    } else {
        CommandError::system_fault(
            code,
            format!("{message} {}: {error}", sqlite_path_suffix(database_path)),
        )
    }
}

pub(crate) fn map_workflow_graph_write_error(
    code: &str,
    database_path: &Path,
    error: SqlError,
    message: &str,
) -> CommandError {
    if is_retryable_sql_error(&error) {
        CommandError::retryable(
            code,
            format!("{message} {}", sqlite_path_suffix(database_path)),
        )
    } else {
        CommandError::system_fault(
            code,
            format!("{message} {}: {error}", sqlite_path_suffix(database_path)),
        )
    }
}

pub(crate) fn map_workflow_graph_commit_error(
    code: &str,
    database_path: &Path,
    error: SqlError,
    message: &str,
) -> CommandError {
    if is_retryable_sql_error(&error) {
        CommandError::retryable(
            code,
            format!("{message} {}", sqlite_path_suffix(database_path)),
        )
    } else {
        CommandError::system_fault(
            code,
            format!("{message} {}: {error}", sqlite_path_suffix(database_path)),
        )
    }
}

pub(crate) fn map_workflow_handoff_transaction_error(
    code: &str,
    database_path: &Path,
    error: SqlError,
    message: &str,
) -> CommandError {
    if is_retryable_sql_error(&error) {
        CommandError::retryable(
            code,
            format!("{message} {}", sqlite_path_suffix(database_path)),
        )
    } else {
        CommandError::system_fault(
            code,
            format!("{message} {}: {error}", sqlite_path_suffix(database_path)),
        )
    }
}

pub(crate) fn map_workflow_handoff_write_error(
    code: &str,
    database_path: &Path,
    error: SqlError,
    message: &str,
) -> CommandError {
    if is_retryable_sql_error(&error) {
        CommandError::retryable(
            code,
            format!("{message} {}", sqlite_path_suffix(database_path)),
        )
    } else {
        CommandError::system_fault(
            code,
            format!("{message} {}: {error}", sqlite_path_suffix(database_path)),
        )
    }
}

pub(crate) fn map_workflow_handoff_commit_error(
    code: &str,
    database_path: &Path,
    error: SqlError,
    message: &str,
) -> CommandError {
    if is_retryable_sql_error(&error) {
        CommandError::retryable(
            code,
            format!("{message} {}", sqlite_path_suffix(database_path)),
        )
    } else {
        CommandError::system_fault(
            code,
            format!("{message} {}: {error}", sqlite_path_suffix(database_path)),
        )
    }
}

pub(crate) fn map_workflow_handoff_insert_error(
    database_path: &Path,
    error: SqlError,
    project_id: &str,
    handoff_transition_id: &str,
) -> CommandError {
    if let SqlError::SqliteFailure(inner, message) = &error {
        if inner.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY {
            return CommandError::user_fixable(
                "workflow_handoff_linkage_missing",
                format!(
                    "Cadence cannot persist workflow handoff package `{handoff_transition_id}` for project `{project_id}` because the linked workflow transition or node rows are missing in {}.",
                    database_path.display()
                ),
            );
        }

        if inner.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_CHECK {
            return CommandError::user_fixable(
                "workflow_handoff_request_invalid",
                format!(
                    "Workflow handoff package `{handoff_transition_id}` violated table validation rules in {}: {}.",
                    database_path.display(),
                    message
                        .as_deref()
                        .unwrap_or("SQLite CHECK constraint failed")
                ),
            );
        }
    }

    map_workflow_handoff_write_error(
        "workflow_handoff_persist_failed",
        database_path,
        error,
        "Cadence could not persist the workflow handoff-package row.",
    )
}
