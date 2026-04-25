use super::*;

pub fn load_autonomous_run(
    repo_root: &Path,
    expected_project_id: &str,
    expected_agent_session_id: &str,
) -> Result<Option<AutonomousRunSnapshotRecord>, CommandError> {
    let database_path = database_path_for_repo(repo_root);
    let connection = open_runtime_database(repo_root, &database_path)?;
    read_project_row(&connection, &database_path, repo_root, expected_project_id)?;

    let transaction = connection.unchecked_transaction().map_err(|error| {
        map_runtime_run_transaction_error(
            "autonomous_run_transaction_failed",
            &database_path,
            error,
            "Cadence could not start the durable autonomous-run read transaction.",
        )
    })?;

    let snapshot = read_autonomous_run_snapshot(
        &transaction,
        &database_path,
        expected_project_id,
        expected_agent_session_id,
    )?;
    transaction.rollback().map_err(|error| {
        map_runtime_run_commit_error(
            "autonomous_run_commit_failed",
            &database_path,
            error,
            "Cadence could not close the durable autonomous-run read transaction.",
        )
    })?;

    Ok(snapshot)
}

pub fn upsert_autonomous_run(
    repo_root: &Path,
    payload: &AutonomousRunUpsertRecord,
) -> Result<AutonomousRunSnapshotRecord, CommandError> {
    let payload = normalize_autonomous_run_upsert_payload(payload)?;

    let database_path = database_path_for_repo(repo_root);
    let connection = open_runtime_database(repo_root, &database_path)?;
    read_project_row(
        &connection,
        &database_path,
        repo_root,
        &payload.run.project_id,
    )?;

    let transaction = connection.unchecked_transaction().map_err(|error| {
        map_runtime_run_transaction_error(
            "autonomous_run_transaction_failed",
            &database_path,
            error,
            "Cadence could not start the durable autonomous-run transaction.",
        )
    })?;

    let runtime_row = read_runtime_run_row(
        &transaction,
        &database_path,
        &payload.run.project_id,
        &payload.run.agent_session_id,
    )?
    .ok_or_else(|| {
            CommandError::retryable(
                "autonomous_run_missing_runtime_row",
                format!(
                    "Cadence could not persist autonomous-run metadata in {} because the selected project has no durable runtime-run row.",
                    database_path.display()
                ),
            )
        })?;

    if runtime_row.run_id != payload.run.run_id {
        return Err(CommandError::retryable(
            "autonomous_run_mismatch",
            format!(
                "Cadence refused to persist autonomous-run metadata for run `{}` because the durable runtime-run row currently points at `{}`.",
                payload.run.run_id, runtime_row.run_id
            ),
        ));
    }

    if runtime_row.runtime_kind != payload.run.runtime_kind
        || runtime_row.provider_id != payload.run.provider_id
    {
        return Err(CommandError::retryable(
            "autonomous_run_mismatch",
            format!(
                "Cadence refused to persist autonomous-run metadata for run `{}` because the durable runtime-run identity is `{}`/`{}` instead of `{}`/`{}`.",
                payload.run.run_id,
                runtime_row.provider_id,
                runtime_row.runtime_kind,
                payload.run.provider_id,
                payload.run.runtime_kind
            ),
        ));
    }

    let active_unit_sequence = payload.run.active_unit_sequence.map(i64::from);
    let duplicate_start_detected = if payload.run.duplicate_start_detected {
        1
    } else {
        0
    };
    let pause_reason_code = payload
        .run
        .pause_reason
        .as_ref()
        .map(|reason| reason.code.as_str());
    let pause_reason_message = payload
        .run
        .pause_reason
        .as_ref()
        .map(|reason| reason.message.as_str());
    let cancel_reason_code = payload
        .run
        .cancel_reason
        .as_ref()
        .map(|reason| reason.code.as_str());
    let cancel_reason_message = payload
        .run
        .cancel_reason
        .as_ref()
        .map(|reason| reason.message.as_str());
    let crash_reason_code = payload
        .run
        .crash_reason
        .as_ref()
        .map(|reason| reason.code.as_str());
    let crash_reason_message = payload
        .run
        .crash_reason
        .as_ref()
        .map(|reason| reason.message.as_str());
    let last_error_code = payload
        .run
        .last_error
        .as_ref()
        .map(|reason| reason.code.as_str());
    let last_error_message = payload
        .run
        .last_error
        .as_ref()
        .map(|reason| reason.message.as_str());

    transaction
        .execute(
            r#"
            INSERT INTO autonomous_runs (
                project_id,
                agent_session_id,
                run_id,
                runtime_kind,
                provider_id,
                supervisor_kind,
                status,
                active_unit_sequence,
                duplicate_start_detected,
                duplicate_start_run_id,
                duplicate_start_reason,
                started_at,
                last_heartbeat_at,
                last_checkpoint_at,
                paused_at,
                cancelled_at,
                completed_at,
                crashed_at,
                stopped_at,
                pause_reason_code,
                pause_reason_message,
                cancel_reason_code,
                cancel_reason_message,
                crash_reason_code,
                crash_reason_message,
                last_error_code,
                last_error_message,
                updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)
            ON CONFLICT(project_id, agent_session_id) DO UPDATE SET
                run_id = excluded.run_id,
                runtime_kind = excluded.runtime_kind,
                provider_id = excluded.provider_id,
                supervisor_kind = excluded.supervisor_kind,
                status = excluded.status,
                active_unit_sequence = excluded.active_unit_sequence,
                duplicate_start_detected = excluded.duplicate_start_detected,
                duplicate_start_run_id = excluded.duplicate_start_run_id,
                duplicate_start_reason = excluded.duplicate_start_reason,
                started_at = excluded.started_at,
                last_heartbeat_at = excluded.last_heartbeat_at,
                last_checkpoint_at = excluded.last_checkpoint_at,
                paused_at = excluded.paused_at,
                cancelled_at = excluded.cancelled_at,
                completed_at = excluded.completed_at,
                crashed_at = excluded.crashed_at,
                stopped_at = excluded.stopped_at,
                pause_reason_code = excluded.pause_reason_code,
                pause_reason_message = excluded.pause_reason_message,
                cancel_reason_code = excluded.cancel_reason_code,
                cancel_reason_message = excluded.cancel_reason_message,
                crash_reason_code = excluded.crash_reason_code,
                crash_reason_message = excluded.crash_reason_message,
                last_error_code = excluded.last_error_code,
                last_error_message = excluded.last_error_message,
                updated_at = excluded.updated_at
            "#,
            params![
                payload.run.project_id.as_str(),
                payload.run.agent_session_id.as_str(),
                payload.run.run_id.as_str(),
                payload.run.runtime_kind.as_str(),
                payload.run.provider_id.as_str(),
                payload.run.supervisor_kind.as_str(),
                autonomous_run_status_sql_value(&payload.run.status),
                active_unit_sequence,
                duplicate_start_detected,
                payload.run.duplicate_start_run_id.as_deref(),
                payload.run.duplicate_start_reason.as_deref(),
                payload.run.started_at.as_str(),
                payload.run.last_heartbeat_at.as_deref(),
                payload.run.last_checkpoint_at.as_deref(),
                payload.run.paused_at.as_deref(),
                payload.run.cancelled_at.as_deref(),
                payload.run.completed_at.as_deref(),
                payload.run.crashed_at.as_deref(),
                payload.run.stopped_at.as_deref(),
                pause_reason_code,
                pause_reason_message,
                cancel_reason_code,
                cancel_reason_message,
                crash_reason_code,
                crash_reason_message,
                last_error_code,
                last_error_message,
                payload.run.updated_at.as_str(),
            ],
        )
        .map_err(|error| {
            map_runtime_run_write_error(
                "autonomous_run_persist_failed",
                &database_path,
                error,
                "Cadence could not persist durable autonomous-run metadata.",
            )
        })?;

    let open_unit = read_open_autonomous_unit(
        &transaction,
        &database_path,
        &payload.run.project_id,
        &payload.run.run_id,
    )?;
    let open_attempt = read_open_autonomous_unit_attempt(
        &transaction,
        &database_path,
        &payload.run.project_id,
        &payload.run.run_id,
    )?;
    let rollover_timestamp = payload
        .attempt
        .as_ref()
        .map(|attempt| attempt.started_at.as_str())
        .or_else(|| payload.unit.as_ref().map(|unit| unit.started_at.as_str()))
        .unwrap_or(payload.run.updated_at.as_str());

    if let Some(unit) = payload.unit.as_ref() {
        close_superseded_autonomous_unit_attempt(
            &transaction,
            &database_path,
            open_attempt.as_ref(),
            payload.attempt.as_ref(),
            &payload.run.status,
            rollover_timestamp,
        )?;
        close_superseded_autonomous_unit(
            &transaction,
            &database_path,
            open_unit.as_ref(),
            unit,
            &payload.run.status,
            rollover_timestamp,
        )?;

        persist_autonomous_unit(&transaction, &database_path, unit)?;
        if let Some(linkage) = unit.workflow_linkage.as_ref() {
            validate_autonomous_workflow_linkage_record(
                &transaction,
                &database_path,
                &payload.run.project_id,
                linkage,
                "unit",
                &unit.unit_id,
                "autonomous_run_request_invalid",
            )?;
        }
    }

    if let Some(attempt) = payload.attempt.as_ref() {
        persist_autonomous_unit_attempt(&transaction, &database_path, attempt)?;
        if let Some(linkage) = attempt.workflow_linkage.as_ref() {
            validate_autonomous_workflow_linkage_record(
                &transaction,
                &database_path,
                &payload.run.project_id,
                linkage,
                "attempt",
                &attempt.attempt_id,
                "autonomous_run_request_invalid",
            )?;
        }
    }

    for artifact in &payload.artifacts {
        persist_autonomous_unit_artifact(&transaction, &database_path, artifact)?;
    }

    transaction.commit().map_err(|error| {
        map_runtime_run_commit_error(
            "autonomous_run_commit_failed",
            &database_path,
            error,
            "Cadence could not commit the durable autonomous-run transaction.",
        )
    })?;

    read_autonomous_run_snapshot(
        &connection,
        &database_path,
        &payload.run.project_id,
        &payload.run.agent_session_id,
    )?
    .ok_or_else(|| {
        CommandError::system_fault(
            "autonomous_run_missing_after_persist",
            format!(
                "Cadence persisted durable autonomous-run metadata in {} but could not read it back.",
                database_path.display()
            ),
        )
    })
}
