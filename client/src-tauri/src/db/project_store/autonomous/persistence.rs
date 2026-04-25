use super::*;

pub(crate) fn read_open_autonomous_unit(
    connection: &Connection,
    database_path: &Path,
    project_id: &str,
    run_id: &str,
) -> Result<Option<AutonomousUnitRecord>, CommandError> {
    let mut open_units = read_autonomous_units(connection, database_path, project_id, run_id)?
        .into_iter()
        .filter(|unit| autonomous_unit_status_is_open(&unit.status))
        .collect::<Vec<_>>();

    if open_units.len() > 1 {
        return Err(CommandError::system_fault(
            "autonomous_unit_conflict",
            format!(
                "Cadence refused to persist autonomous unit rollover because run `{run_id}` already has {} open durable unit rows in {}.",
                open_units.len(),
                database_path.display()
            ),
        ));
    }

    Ok(open_units.pop())
}

pub(crate) fn read_open_autonomous_unit_attempt(
    connection: &Connection,
    database_path: &Path,
    project_id: &str,
    run_id: &str,
) -> Result<Option<AutonomousUnitAttemptRecord>, CommandError> {
    let mut open_attempts =
        read_autonomous_unit_attempts(connection, database_path, project_id, run_id)?
            .into_iter()
            .filter(|attempt| autonomous_unit_status_is_open(&attempt.status))
            .collect::<Vec<_>>();

    if open_attempts.len() > 1 {
        return Err(CommandError::system_fault(
            "autonomous_unit_attempt_conflict",
            format!(
                "Cadence refused to persist autonomous attempt rollover because run `{run_id}` already has {} open durable attempt rows in {}.",
                open_attempts.len(),
                database_path.display()
            ),
        ));
    }

    Ok(open_attempts.pop())
}

pub(crate) fn close_superseded_autonomous_unit(
    transaction: &Transaction<'_>,
    database_path: &Path,
    existing: Option<&AutonomousUnitRecord>,
    incoming: &AutonomousUnitRecord,
    run_status: &AutonomousRunStatus,
    closed_at: &str,
) -> Result<(), CommandError> {
    let Some(existing) = existing else {
        return Ok(());
    };
    if existing.unit_id == incoming.unit_id {
        return Ok(());
    }
    if existing.boundary_id.is_some() {
        return Err(CommandError::user_fixable(
            "autonomous_unit_boundary_drift",
            format!(
                "Cadence refused to roll durable autonomous unit `{}` to `{}` because the existing unit is still attached to boundary `{}`.",
                existing.unit_id,
                incoming.unit_id,
                existing.boundary_id.as_deref().unwrap_or_default()
            ),
        ));
    }

    transaction
        .execute(
            r#"
            UPDATE autonomous_units
            SET status = ?1,
                finished_at = COALESCE(finished_at, ?2),
                updated_at = ?3
            WHERE project_id = ?4
              AND run_id = ?5
              AND unit_id = ?6
            "#,
            params![
                autonomous_unit_status_sql_value(&rollover_autonomous_unit_status(run_status)),
                closed_at,
                closed_at,
                existing.project_id.as_str(),
                existing.run_id.as_str(),
                existing.unit_id.as_str(),
            ],
        )
        .map_err(|error| {
            map_runtime_run_write_error(
                "autonomous_unit_persist_failed",
                database_path,
                error,
                "Cadence could not close the superseded durable autonomous-unit row.",
            )
        })?;

    Ok(())
}

pub(crate) fn close_superseded_autonomous_unit_attempt(
    transaction: &Transaction<'_>,
    database_path: &Path,
    existing: Option<&AutonomousUnitAttemptRecord>,
    incoming: Option<&AutonomousUnitAttemptRecord>,
    run_status: &AutonomousRunStatus,
    closed_at: &str,
) -> Result<(), CommandError> {
    let Some(existing) = existing else {
        return Ok(());
    };
    let Some(incoming) = incoming else {
        return Ok(());
    };
    if existing.attempt_id == incoming.attempt_id {
        return Ok(());
    }
    if existing.boundary_id.is_some() {
        return Err(CommandError::user_fixable(
            "autonomous_unit_attempt_boundary_drift",
            format!(
                "Cadence refused to roll durable autonomous attempt `{}` to `{}` because the existing attempt is still attached to boundary `{}`.",
                existing.attempt_id,
                incoming.attempt_id,
                existing.boundary_id.as_deref().unwrap_or_default()
            ),
        ));
    }

    transaction
        .execute(
            r#"
            UPDATE autonomous_unit_attempts
            SET status = ?1,
                finished_at = COALESCE(finished_at, ?2),
                updated_at = ?3
            WHERE project_id = ?4
              AND run_id = ?5
              AND attempt_id = ?6
            "#,
            params![
                autonomous_unit_status_sql_value(&rollover_autonomous_unit_status(run_status)),
                closed_at,
                closed_at,
                existing.project_id.as_str(),
                existing.run_id.as_str(),
                existing.attempt_id.as_str(),
            ],
        )
        .map_err(|error| {
            map_runtime_run_write_error(
                "autonomous_unit_attempt_persist_failed",
                database_path,
                error,
                "Cadence could not close the superseded durable autonomous-attempt row.",
            )
        })?;

    Ok(())
}

pub(crate) fn autonomous_unit_status_is_open(status: &AutonomousUnitStatus) -> bool {
    matches!(
        status,
        AutonomousUnitStatus::Pending
            | AutonomousUnitStatus::Active
            | AutonomousUnitStatus::Blocked
            | AutonomousUnitStatus::Paused
    )
}

pub(crate) fn rollover_autonomous_unit_status(
    run_status: &AutonomousRunStatus,
) -> AutonomousUnitStatus {
    match run_status {
        AutonomousRunStatus::Cancelled => AutonomousUnitStatus::Cancelled,
        AutonomousRunStatus::Failed | AutonomousRunStatus::Crashed => AutonomousUnitStatus::Failed,
        _ => AutonomousUnitStatus::Completed,
    }
}

pub(crate) fn persist_autonomous_unit(
    transaction: &Transaction<'_>,
    database_path: &Path,
    unit: &AutonomousUnitRecord,
) -> Result<(), CommandError> {
    let (last_error_code, last_error_message) = unit
        .last_error
        .as_ref()
        .map(|error| (Some(error.code.as_str()), Some(error.message.as_str())))
        .unwrap_or((None, None));

    let (
        workflow_node_id,
        workflow_transition_id,
        workflow_causal_transition_id,
        workflow_handoff_transition_id,
        workflow_handoff_package_hash,
    ) = unit
        .workflow_linkage
        .as_ref()
        .map(|linkage| {
            (
                Some(linkage.workflow_node_id.as_str()),
                Some(linkage.transition_id.as_str()),
                linkage.causal_transition_id.as_deref(),
                Some(linkage.handoff_transition_id.as_str()),
                Some(linkage.handoff_package_hash.as_str()),
            )
        })
        .unwrap_or((None, None, None, None, None));

    transaction
        .execute(
            r#"
            INSERT INTO autonomous_units (
                unit_id,
                project_id,
                run_id,
                sequence,
                kind,
                status,
                summary,
                boundary_id,
                workflow_node_id,
                workflow_transition_id,
                workflow_causal_transition_id,
                workflow_handoff_transition_id,
                workflow_handoff_package_hash,
                started_at,
                finished_at,
                last_error_code,
                last_error_message,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT(unit_id) DO UPDATE SET
                sequence = excluded.sequence,
                kind = excluded.kind,
                status = excluded.status,
                summary = excluded.summary,
                boundary_id = excluded.boundary_id,
                workflow_node_id = excluded.workflow_node_id,
                workflow_transition_id = excluded.workflow_transition_id,
                workflow_causal_transition_id = excluded.workflow_causal_transition_id,
                workflow_handoff_transition_id = excluded.workflow_handoff_transition_id,
                workflow_handoff_package_hash = excluded.workflow_handoff_package_hash,
                started_at = excluded.started_at,
                finished_at = excluded.finished_at,
                last_error_code = excluded.last_error_code,
                last_error_message = excluded.last_error_message,
                updated_at = excluded.updated_at
            "#,
            params![
                unit.unit_id.as_str(),
                unit.project_id.as_str(),
                unit.run_id.as_str(),
                i64::from(unit.sequence),
                autonomous_unit_kind_sql_value(&unit.kind),
                autonomous_unit_status_sql_value(&unit.status),
                unit.summary.as_str(),
                unit.boundary_id.as_deref(),
                workflow_node_id,
                workflow_transition_id,
                workflow_causal_transition_id,
                workflow_handoff_transition_id,
                workflow_handoff_package_hash,
                unit.started_at.as_str(),
                unit.finished_at.as_deref(),
                last_error_code,
                last_error_message,
                unit.updated_at.as_str(),
            ],
        )
        .map_err(|error| {
            if matches!(error, SqlError::SqliteFailure(_, _)) {
                return CommandError::system_fault(
                    "autonomous_unit_conflict",
                    format!(
                        "Cadence refused to persist autonomous unit `{}` because it would violate the one-active-unit invariant in {}: {error}",
                        unit.unit_id,
                        database_path.display()
                    ),
                );
            }

            map_runtime_run_write_error(
                "autonomous_unit_persist_failed",
                database_path,
                error,
                "Cadence could not persist the durable autonomous-unit row.",
            )
        })?;

    Ok(())
}

pub(crate) fn persist_autonomous_unit_attempt(
    transaction: &Transaction<'_>,
    database_path: &Path,
    attempt: &AutonomousUnitAttemptRecord,
) -> Result<(), CommandError> {
    let existing = read_autonomous_unit_attempt_by_id(
        transaction,
        database_path,
        &attempt.project_id,
        &attempt.run_id,
        &attempt.attempt_id,
    )?;
    if let Some(existing) = existing.as_ref() {
        if existing == attempt {
            return Ok(());
        }

        if matches!(
            existing.status,
            AutonomousUnitStatus::Completed
                | AutonomousUnitStatus::Cancelled
                | AutonomousUnitStatus::Failed
        ) {
            return Err(CommandError::system_fault(
                "autonomous_unit_attempt_immutable",
                format!(
                    "Cadence refused to mutate completed autonomous attempt `{}` in {}.",
                    attempt.attempt_id,
                    database_path.display()
                ),
            ));
        }
    }

    let (last_error_code, last_error_message) = attempt
        .last_error
        .as_ref()
        .map(|error| (Some(error.code.as_str()), Some(error.message.as_str())))
        .unwrap_or((None, None));

    let (
        workflow_node_id,
        workflow_transition_id,
        workflow_causal_transition_id,
        workflow_handoff_transition_id,
        workflow_handoff_package_hash,
    ) = attempt
        .workflow_linkage
        .as_ref()
        .map(|linkage| {
            (
                Some(linkage.workflow_node_id.as_str()),
                Some(linkage.transition_id.as_str()),
                linkage.causal_transition_id.as_deref(),
                Some(linkage.handoff_transition_id.as_str()),
                Some(linkage.handoff_package_hash.as_str()),
            )
        })
        .unwrap_or((None, None, None, None, None));

    transaction
        .execute(
            r#"
            INSERT INTO autonomous_unit_attempts (
                attempt_id,
                project_id,
                run_id,
                unit_id,
                attempt_number,
                child_session_id,
                status,
                boundary_id,
                workflow_node_id,
                workflow_transition_id,
                workflow_causal_transition_id,
                workflow_handoff_transition_id,
                workflow_handoff_package_hash,
                started_at,
                finished_at,
                last_error_code,
                last_error_message,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
            ON CONFLICT(attempt_id) DO UPDATE SET
                attempt_number = excluded.attempt_number,
                child_session_id = excluded.child_session_id,
                status = excluded.status,
                boundary_id = excluded.boundary_id,
                workflow_node_id = excluded.workflow_node_id,
                workflow_transition_id = excluded.workflow_transition_id,
                workflow_causal_transition_id = excluded.workflow_causal_transition_id,
                workflow_handoff_transition_id = excluded.workflow_handoff_transition_id,
                workflow_handoff_package_hash = excluded.workflow_handoff_package_hash,
                started_at = excluded.started_at,
                finished_at = excluded.finished_at,
                last_error_code = excluded.last_error_code,
                last_error_message = excluded.last_error_message,
                updated_at = excluded.updated_at
            "#,
            params![
                attempt.attempt_id.as_str(),
                attempt.project_id.as_str(),
                attempt.run_id.as_str(),
                attempt.unit_id.as_str(),
                i64::from(attempt.attempt_number),
                attempt.child_session_id.as_str(),
                autonomous_unit_status_sql_value(&attempt.status),
                attempt.boundary_id.as_deref(),
                workflow_node_id,
                workflow_transition_id,
                workflow_causal_transition_id,
                workflow_handoff_transition_id,
                workflow_handoff_package_hash,
                attempt.started_at.as_str(),
                attempt.finished_at.as_deref(),
                last_error_code,
                last_error_message,
                attempt.updated_at.as_str(),
            ],
        )
        .map_err(|error| {
            if matches!(error, SqlError::SqliteFailure(_, _)) {
                return CommandError::system_fault(
                    "autonomous_unit_attempt_conflict",
                    format!(
                        "Cadence refused to persist autonomous attempt `{}` because it would violate the active-attempt or parent-link invariants in {}: {error}",
                        attempt.attempt_id,
                        database_path.display()
                    ),
                );
            }

            map_runtime_run_write_error(
                "autonomous_unit_attempt_persist_failed",
                database_path,
                error,
                "Cadence could not persist the durable autonomous attempt row.",
            )
        })?;

    Ok(())
}

pub(crate) fn persist_autonomous_unit_artifact(
    transaction: &Transaction<'_>,
    database_path: &Path,
    artifact: &AutonomousUnitArtifactRecord,
) -> Result<(), CommandError> {
    let payload_json = artifact
        .payload
        .as_ref()
        .map(canonicalize_autonomous_artifact_payload_json)
        .transpose()?;

    transaction
        .execute(
            r#"
            INSERT INTO autonomous_unit_artifacts (
                artifact_id,
                project_id,
                run_id,
                unit_id,
                attempt_id,
                artifact_kind,
                status,
                summary,
                content_hash,
                payload_json,
                created_at,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            ON CONFLICT(artifact_id) DO UPDATE SET
                artifact_kind = excluded.artifact_kind,
                status = excluded.status,
                summary = excluded.summary,
                content_hash = excluded.content_hash,
                payload_json = excluded.payload_json,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at
            "#,
            params![
                artifact.artifact_id.as_str(),
                artifact.project_id.as_str(),
                artifact.run_id.as_str(),
                artifact.unit_id.as_str(),
                artifact.attempt_id.as_str(),
                artifact.artifact_kind.as_str(),
                autonomous_unit_artifact_status_sql_value(&artifact.status),
                artifact.summary.as_str(),
                artifact.content_hash.as_deref(),
                payload_json.as_deref(),
                artifact.created_at.as_str(),
                artifact.updated_at.as_str(),
            ],
        )
        .map_err(|error| {
            if matches!(error, SqlError::SqliteFailure(_, _)) {
                return CommandError::system_fault(
                    "autonomous_unit_artifact_conflict",
                    format!(
                        "Cadence refused to persist autonomous artifact `{}` because its parent linkage is invalid in {}: {error}",
                        artifact.artifact_id,
                        database_path.display()
                    ),
                );
            }

            map_runtime_run_write_error(
                "autonomous_unit_artifact_persist_failed",
                database_path,
                error,
                "Cadence could not persist the durable autonomous artifact row.",
            )
        })?;

    Ok(())
}
