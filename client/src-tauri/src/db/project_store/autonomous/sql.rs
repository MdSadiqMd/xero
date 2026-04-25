use super::*;

pub(crate) fn parse_autonomous_run_status(value: &str) -> Result<AutonomousRunStatus, String> {
    match value {
        "starting" => Ok(AutonomousRunStatus::Starting),
        "running" => Ok(AutonomousRunStatus::Running),
        "paused" => Ok(AutonomousRunStatus::Paused),
        "cancelling" => Ok(AutonomousRunStatus::Cancelling),
        "cancelled" => Ok(AutonomousRunStatus::Cancelled),
        "stale" => Ok(AutonomousRunStatus::Stale),
        "failed" => Ok(AutonomousRunStatus::Failed),
        "stopped" => Ok(AutonomousRunStatus::Stopped),
        "crashed" => Ok(AutonomousRunStatus::Crashed),
        "completed" => Ok(AutonomousRunStatus::Completed),
        other => Err(format!(
            "must be a known autonomous-run status, found `{other}`."
        )),
    }
}

pub(crate) fn autonomous_run_status_sql_value(value: &AutonomousRunStatus) -> &'static str {
    match value {
        AutonomousRunStatus::Starting => "starting",
        AutonomousRunStatus::Running => "running",
        AutonomousRunStatus::Paused => "paused",
        AutonomousRunStatus::Cancelling => "cancelling",
        AutonomousRunStatus::Cancelled => "cancelled",
        AutonomousRunStatus::Stale => "stale",
        AutonomousRunStatus::Failed => "failed",
        AutonomousRunStatus::Stopped => "stopped",
        AutonomousRunStatus::Crashed => "crashed",
        AutonomousRunStatus::Completed => "completed",
    }
}

pub(crate) fn parse_autonomous_unit_kind(value: &str) -> Result<AutonomousUnitKind, String> {
    match value {
        "researcher" => Ok(AutonomousUnitKind::Researcher),
        "planner" => Ok(AutonomousUnitKind::Planner),
        "executor" => Ok(AutonomousUnitKind::Executor),
        "verifier" => Ok(AutonomousUnitKind::Verifier),
        other => Err(format!(
            "must be a known autonomous-unit kind, found `{other}`."
        )),
    }
}

pub(crate) fn autonomous_unit_kind_sql_value(value: &AutonomousUnitKind) -> &'static str {
    match value {
        AutonomousUnitKind::Researcher => "researcher",
        AutonomousUnitKind::Planner => "planner",
        AutonomousUnitKind::Executor => "executor",
        AutonomousUnitKind::Verifier => "verifier",
    }
}

pub(crate) fn parse_autonomous_unit_status(value: &str) -> Result<AutonomousUnitStatus, String> {
    match value {
        "pending" => Ok(AutonomousUnitStatus::Pending),
        "active" => Ok(AutonomousUnitStatus::Active),
        "blocked" => Ok(AutonomousUnitStatus::Blocked),
        "paused" => Ok(AutonomousUnitStatus::Paused),
        "completed" => Ok(AutonomousUnitStatus::Completed),
        "cancelled" => Ok(AutonomousUnitStatus::Cancelled),
        "failed" => Ok(AutonomousUnitStatus::Failed),
        other => Err(format!(
            "must be a known autonomous-unit status, found `{other}`."
        )),
    }
}

pub(crate) fn autonomous_unit_status_sql_value(value: &AutonomousUnitStatus) -> &'static str {
    match value {
        AutonomousUnitStatus::Pending => "pending",
        AutonomousUnitStatus::Active => "active",
        AutonomousUnitStatus::Blocked => "blocked",
        AutonomousUnitStatus::Paused => "paused",
        AutonomousUnitStatus::Completed => "completed",
        AutonomousUnitStatus::Cancelled => "cancelled",
        AutonomousUnitStatus::Failed => "failed",
    }
}

pub(crate) fn parse_autonomous_unit_artifact_status(
    value: &str,
) -> Result<AutonomousUnitArtifactStatus, String> {
    match value {
        "pending" => Ok(AutonomousUnitArtifactStatus::Pending),
        "recorded" => Ok(AutonomousUnitArtifactStatus::Recorded),
        "rejected" => Ok(AutonomousUnitArtifactStatus::Rejected),
        "redacted" => Ok(AutonomousUnitArtifactStatus::Redacted),
        other => Err(format!(
            "must be a known autonomous-artifact status, found `{other}`."
        )),
    }
}

pub(crate) fn autonomous_unit_artifact_status_sql_value(
    value: &AutonomousUnitArtifactStatus,
) -> &'static str {
    match value {
        AutonomousUnitArtifactStatus::Pending => "pending",
        AutonomousUnitArtifactStatus::Recorded => "recorded",
        AutonomousUnitArtifactStatus::Rejected => "rejected",
        AutonomousUnitArtifactStatus::Redacted => "redacted",
    }
}
