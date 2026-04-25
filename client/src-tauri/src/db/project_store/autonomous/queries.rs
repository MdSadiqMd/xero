use super::*;

#[derive(Debug)]
struct RawAutonomousRunRow {
    project_id: String,
    agent_session_id: String,
    run_id: String,
    runtime_kind: String,
    provider_id: String,
    supervisor_kind: String,
    status: String,
    active_unit_sequence: Option<i64>,
    duplicate_start_detected: i64,
    duplicate_start_run_id: Option<String>,
    duplicate_start_reason: Option<String>,
    started_at: String,
    last_heartbeat_at: Option<String>,
    last_checkpoint_at: Option<String>,
    paused_at: Option<String>,
    cancelled_at: Option<String>,
    completed_at: Option<String>,
    crashed_at: Option<String>,
    stopped_at: Option<String>,
    pause_reason_code: Option<String>,
    pause_reason_message: Option<String>,
    cancel_reason_code: Option<String>,
    cancel_reason_message: Option<String>,
    crash_reason_code: Option<String>,
    crash_reason_message: Option<String>,
    last_error_code: Option<String>,
    last_error_message: Option<String>,
    updated_at: String,
}

#[derive(Debug)]
struct RawAutonomousUnitRow {
    project_id: String,
    run_id: String,
    unit_id: String,
    sequence: i64,
    kind: String,
    status: String,
    summary: String,
    boundary_id: Option<String>,
    workflow_node_id: Option<String>,
    workflow_transition_id: Option<String>,
    workflow_causal_transition_id: Option<String>,
    workflow_handoff_transition_id: Option<String>,
    workflow_handoff_package_hash: Option<String>,
    started_at: String,
    finished_at: Option<String>,
    last_error_code: Option<String>,
    last_error_message: Option<String>,
    updated_at: String,
}

#[derive(Debug)]
struct RawAutonomousUnitAttemptRow {
    project_id: String,
    run_id: String,
    unit_id: String,
    attempt_id: String,
    attempt_number: i64,
    child_session_id: String,
    status: String,
    boundary_id: Option<String>,
    workflow_node_id: Option<String>,
    workflow_transition_id: Option<String>,
    workflow_causal_transition_id: Option<String>,
    workflow_handoff_transition_id: Option<String>,
    workflow_handoff_package_hash: Option<String>,
    started_at: String,
    finished_at: Option<String>,
    last_error_code: Option<String>,
    last_error_message: Option<String>,
    updated_at: String,
}

#[derive(Debug)]
struct RawAutonomousUnitArtifactRow {
    project_id: String,
    run_id: String,
    unit_id: String,
    attempt_id: String,
    artifact_id: String,
    artifact_kind: String,
    status: String,
    summary: String,
    content_hash: Option<String>,
    payload_json: Option<String>,
    created_at: String,
    updated_at: String,
}

pub(crate) fn read_autonomous_run_snapshot(
    connection: &Connection,
    database_path: &Path,
    expected_project_id: &str,
    expected_agent_session_id: &str,
) -> Result<Option<AutonomousRunSnapshotRecord>, CommandError> {
    let row = connection.query_row(
        r#"
            SELECT
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
            FROM autonomous_runs
            WHERE project_id = ?1
              AND agent_session_id = ?2
            "#,
        params![expected_project_id, expected_agent_session_id],
        |row| {
            Ok(RawAutonomousRunRow {
                project_id: row.get(0)?,
                agent_session_id: row.get(1)?,
                run_id: row.get(2)?,
                runtime_kind: row.get(3)?,
                provider_id: row.get(4)?,
                supervisor_kind: row.get(5)?,
                status: row.get(6)?,
                active_unit_sequence: row.get(7)?,
                duplicate_start_detected: row.get(8)?,
                duplicate_start_run_id: row.get(9)?,
                duplicate_start_reason: row.get(10)?,
                started_at: row.get(11)?,
                last_heartbeat_at: row.get(12)?,
                last_checkpoint_at: row.get(13)?,
                paused_at: row.get(14)?,
                cancelled_at: row.get(15)?,
                completed_at: row.get(16)?,
                crashed_at: row.get(17)?,
                stopped_at: row.get(18)?,
                pause_reason_code: row.get(19)?,
                pause_reason_message: row.get(20)?,
                cancel_reason_code: row.get(21)?,
                cancel_reason_message: row.get(22)?,
                crash_reason_code: row.get(23)?,
                crash_reason_message: row.get(24)?,
                last_error_code: row.get(25)?,
                last_error_message: row.get(26)?,
                updated_at: row.get(27)?,
            })
        },
    );

    let raw_row = match row {
        Ok(row) => row,
        Err(SqlError::QueryReturnedNoRows) => return Ok(None),
        Err(other) => {
            return Err(CommandError::system_fault(
                "autonomous_run_query_failed",
                format!(
                    "Cadence could not read durable autonomous-run metadata from {}: {other}",
                    database_path.display()
                ),
            ))
        }
    };

    let run = decode_autonomous_run_row(raw_row, database_path)?;
    let units = read_autonomous_units(connection, database_path, expected_project_id, &run.run_id)?;
    let attempts =
        read_autonomous_unit_attempts(connection, database_path, expected_project_id, &run.run_id)?;
    let artifacts = read_autonomous_unit_artifacts(
        connection,
        database_path,
        expected_project_id,
        &run.run_id,
    )?;
    let history = build_autonomous_unit_history(database_path, &run, units, attempts, artifacts)?;

    let unit = history
        .iter()
        .find(|entry| {
            matches!(
                entry.unit.status,
                AutonomousUnitStatus::Active
                    | AutonomousUnitStatus::Blocked
                    | AutonomousUnitStatus::Paused
            )
        })
        .or_else(|| history.first())
        .map(|entry| entry.unit.clone());
    let attempt = unit.as_ref().and_then(|unit| {
        history
            .iter()
            .find(|entry| entry.unit.unit_id == unit.unit_id)
            .and_then(|entry| entry.latest_attempt.clone())
    });

    if let (Some(active_unit_sequence), Some(unit)) = (run.active_unit_sequence, unit.as_ref()) {
        if active_unit_sequence != unit.sequence {
            return Err(map_runtime_run_decode_error(
                database_path,
                format!(
                    "Autonomous run active_unit_sequence {} does not match durable unit `{}` sequence {}.",
                    active_unit_sequence, unit.unit_id, unit.sequence
                ),
            ));
        }
    }

    Ok(Some(AutonomousRunSnapshotRecord {
        run,
        unit,
        attempt,
        history,
    }))
}

pub(crate) fn read_autonomous_units(
    connection: &Connection,
    database_path: &Path,
    project_id: &str,
    run_id: &str,
) -> Result<Vec<AutonomousUnitRecord>, CommandError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                project_id,
                run_id,
                unit_id,
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
            FROM autonomous_units
            WHERE project_id = ?1
              AND run_id = ?2
            ORDER BY sequence DESC, updated_at DESC, unit_id ASC
            LIMIT ?3
            "#,
        )
        .map_err(|error| {
            CommandError::system_fault(
                "autonomous_unit_query_failed",
                format!(
                    "Cadence could not prepare the durable autonomous-unit query against {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let rows = statement
        .query_map(
            params![project_id, run_id, MAX_AUTONOMOUS_HISTORY_UNIT_ROWS],
            |row| {
                Ok(RawAutonomousUnitRow {
                    project_id: row.get(0)?,
                    run_id: row.get(1)?,
                    unit_id: row.get(2)?,
                    sequence: row.get(3)?,
                    kind: row.get(4)?,
                    status: row.get(5)?,
                    summary: row.get(6)?,
                    boundary_id: row.get(7)?,
                    workflow_node_id: row.get(8)?,
                    workflow_transition_id: row.get(9)?,
                    workflow_causal_transition_id: row.get(10)?,
                    workflow_handoff_transition_id: row.get(11)?,
                    workflow_handoff_package_hash: row.get(12)?,
                    started_at: row.get(13)?,
                    finished_at: row.get(14)?,
                    last_error_code: row.get(15)?,
                    last_error_message: row.get(16)?,
                    updated_at: row.get(17)?,
                })
            },
        )
        .map_err(|error| {
            CommandError::system_fault(
                "autonomous_unit_query_failed",
                format!(
                    "Cadence could not query durable autonomous-unit rows from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let mut units = Vec::new();
    let mut last_sequence = u32::MAX;
    for row in rows {
        let unit = decode_autonomous_unit_row(
            row.map_err(|error| {
                CommandError::system_fault(
                    "autonomous_unit_query_failed",
                    format!(
                        "Cadence could not read a durable autonomous-unit row from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            database_path,
        )?;

        if let Some(linkage) = unit.workflow_linkage.as_ref() {
            validate_autonomous_workflow_linkage_record(
                connection,
                database_path,
                project_id,
                linkage,
                "unit",
                &unit.unit_id,
                "runtime_run_decode_failed",
            )?;
        }

        if !units.is_empty() && unit.sequence >= last_sequence {
            return Err(map_runtime_run_decode_error(
                database_path,
                format!(
                    "Autonomous unit sequences must decrease strictly in bounded history order, but sequence {} followed {}.",
                    unit.sequence, last_sequence
                ),
            ));
        }

        last_sequence = unit.sequence;
        units.push(unit);
    }

    Ok(units)
}

pub(crate) fn read_autonomous_unit_attempts(
    connection: &Connection,
    database_path: &Path,
    project_id: &str,
    run_id: &str,
) -> Result<Vec<AutonomousUnitAttemptRecord>, CommandError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                project_id,
                run_id,
                unit_id,
                attempt_id,
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
            FROM autonomous_unit_attempts
            WHERE project_id = ?1
              AND run_id = ?2
            ORDER BY attempt_number DESC, updated_at DESC, attempt_id ASC
            LIMIT ?3
            "#,
        )
        .map_err(|error| {
            CommandError::system_fault(
                "autonomous_unit_attempt_query_failed",
                format!(
                    "Cadence could not prepare the durable autonomous attempt query against {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let rows = statement
        .query_map(
            params![project_id, run_id, MAX_AUTONOMOUS_HISTORY_ATTEMPT_ROWS],
            |row| {
                Ok(RawAutonomousUnitAttemptRow {
                    project_id: row.get(0)?,
                    run_id: row.get(1)?,
                    unit_id: row.get(2)?,
                    attempt_id: row.get(3)?,
                    attempt_number: row.get(4)?,
                    child_session_id: row.get(5)?,
                    status: row.get(6)?,
                    boundary_id: row.get(7)?,
                    workflow_node_id: row.get(8)?,
                    workflow_transition_id: row.get(9)?,
                    workflow_causal_transition_id: row.get(10)?,
                    workflow_handoff_transition_id: row.get(11)?,
                    workflow_handoff_package_hash: row.get(12)?,
                    started_at: row.get(13)?,
                    finished_at: row.get(14)?,
                    last_error_code: row.get(15)?,
                    last_error_message: row.get(16)?,
                    updated_at: row.get(17)?,
                })
            },
        )
        .map_err(|error| {
            CommandError::system_fault(
                "autonomous_unit_attempt_query_failed",
                format!(
                    "Cadence could not query durable autonomous attempts from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let mut attempts = Vec::new();
    for row in rows {
        let attempt = decode_autonomous_unit_attempt_row(
            row.map_err(|error| {
                CommandError::system_fault(
                    "autonomous_unit_attempt_query_failed",
                    format!(
                        "Cadence could not read a durable autonomous-attempt row from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            database_path,
        )?;

        if let Some(linkage) = attempt.workflow_linkage.as_ref() {
            validate_autonomous_workflow_linkage_record(
                connection,
                database_path,
                project_id,
                linkage,
                "attempt",
                &attempt.attempt_id,
                "runtime_run_decode_failed",
            )?;
        }

        attempts.push(attempt);
    }

    Ok(attempts)
}

pub(crate) fn read_autonomous_unit_artifacts(
    connection: &Connection,
    database_path: &Path,
    project_id: &str,
    run_id: &str,
) -> Result<Vec<AutonomousUnitArtifactRecord>, CommandError> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                project_id,
                run_id,
                unit_id,
                attempt_id,
                artifact_id,
                artifact_kind,
                status,
                summary,
                content_hash,
                payload_json,
                created_at,
                updated_at
            FROM autonomous_unit_artifacts
            WHERE project_id = ?1
              AND run_id = ?2
            ORDER BY created_at DESC, artifact_id ASC
            LIMIT ?3
            "#,
        )
        .map_err(|error| {
            CommandError::system_fault(
                "autonomous_unit_artifact_query_failed",
                format!(
                    "Cadence could not prepare the durable autonomous artifact query against {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let rows = statement
        .query_map(
            params![project_id, run_id, MAX_AUTONOMOUS_HISTORY_ARTIFACT_ROWS],
            |row| {
                Ok(RawAutonomousUnitArtifactRow {
                    project_id: row.get(0)?,
                    run_id: row.get(1)?,
                    unit_id: row.get(2)?,
                    attempt_id: row.get(3)?,
                    artifact_id: row.get(4)?,
                    artifact_kind: row.get(5)?,
                    status: row.get(6)?,
                    summary: row.get(7)?,
                    content_hash: row.get(8)?,
                    payload_json: row.get(9)?,
                    created_at: row.get(10)?,
                    updated_at: row.get(11)?,
                })
            },
        )
        .map_err(|error| {
            CommandError::system_fault(
                "autonomous_unit_artifact_query_failed",
                format!(
                    "Cadence could not query durable autonomous artifacts from {}: {error}",
                    database_path.display()
                ),
            )
        })?;

    let mut artifacts = Vec::new();
    for row in rows {
        artifacts.push(decode_autonomous_unit_artifact_row(
            row.map_err(|error| {
                CommandError::system_fault(
                    "autonomous_unit_artifact_query_failed",
                    format!(
                        "Cadence could not read a durable autonomous-artifact row from {}: {error}",
                        database_path.display()
                    ),
                )
            })?,
            database_path,
        )?);
    }

    Ok(artifacts)
}

pub(crate) fn build_autonomous_unit_history(
    database_path: &Path,
    run: &AutonomousRunRecord,
    units: Vec<AutonomousUnitRecord>,
    attempts: Vec<AutonomousUnitAttemptRecord>,
    artifacts: Vec<AutonomousUnitArtifactRecord>,
) -> Result<Vec<AutonomousUnitHistoryRecord>, CommandError> {
    if units.is_empty() {
        return Err(map_runtime_run_decode_error(
            database_path,
            format!(
                "Autonomous run `{}` has no durable unit ledger rows.",
                run.run_id
            ),
        ));
    }

    let active_unit_count = units
        .iter()
        .filter(|unit| unit.status == AutonomousUnitStatus::Active)
        .count();
    if active_unit_count > 1 {
        return Err(map_runtime_run_decode_error(
            database_path,
            format!(
                "Autonomous run `{}` has {} active unit rows; expected at most one.",
                run.run_id, active_unit_count
            ),
        ));
    }

    let open_unit_count = units
        .iter()
        .filter(|unit| autonomous_unit_status_is_open(&unit.status))
        .count();
    if open_unit_count > 1 {
        return Err(map_runtime_run_decode_error(
            database_path,
            format!(
                "Autonomous run `{}` has {} open unit rows; expected at most one active, blocked, paused, or pending row.",
                run.run_id, open_unit_count
            ),
        ));
    }

    let active_attempt_count = attempts
        .iter()
        .filter(|attempt| attempt.status == AutonomousUnitStatus::Active)
        .count();
    if active_attempt_count > 1 {
        return Err(map_runtime_run_decode_error(
            database_path,
            format!(
                "Autonomous run `{}` has {} active attempt rows; expected at most one.",
                run.run_id, active_attempt_count
            ),
        ));
    }

    let open_attempt_count = attempts
        .iter()
        .filter(|attempt| autonomous_unit_status_is_open(&attempt.status))
        .count();
    if open_attempt_count > 1 {
        return Err(map_runtime_run_decode_error(
            database_path,
            format!(
                "Autonomous run `{}` has {} open attempt rows; expected at most one active, blocked, paused, or pending row.",
                run.run_id, open_attempt_count
            ),
        ));
    }

    let mut attempts_by_unit: HashMap<String, Vec<AutonomousUnitAttemptRecord>> = HashMap::new();
    for attempt in attempts {
        if !units.iter().any(|unit| unit.unit_id == attempt.unit_id) {
            return Err(map_runtime_run_decode_error(
                database_path,
                format!(
                    "Autonomous attempt `{}` points at missing durable unit `{}` for run `{}`.",
                    attempt.attempt_id, attempt.unit_id, run.run_id
                ),
            ));
        }
        attempts_by_unit
            .entry(attempt.unit_id.clone())
            .or_default()
            .push(attempt);
    }

    let mut artifacts_by_attempt: HashMap<String, Vec<AutonomousUnitArtifactRecord>> =
        HashMap::new();
    for artifact in artifacts {
        if !units.iter().any(|unit| unit.unit_id == artifact.unit_id) {
            return Err(map_runtime_run_decode_error(
                database_path,
                format!(
                    "Autonomous artifact `{}` points at missing durable unit `{}` for run `{}`.",
                    artifact.artifact_id, artifact.unit_id, run.run_id
                ),
            ));
        }

        let attempt_known = attempts_by_unit
            .get(&artifact.unit_id)
            .map(|attempts| {
                attempts
                    .iter()
                    .any(|attempt| attempt.attempt_id == artifact.attempt_id)
            })
            .unwrap_or(false);
        if !attempt_known {
            return Err(map_runtime_run_decode_error(
                database_path,
                format!(
                    "Autonomous artifact `{}` points at missing durable attempt `{}` for unit `{}`.",
                    artifact.artifact_id, artifact.attempt_id, artifact.unit_id
                ),
            ));
        }

        artifacts_by_attempt
            .entry(artifact.attempt_id.clone())
            .or_default()
            .push(artifact);
    }

    let mut history = Vec::new();
    for unit in units {
        let latest_attempt =
            attempts_by_unit
                .remove(&unit.unit_id)
                .and_then(|mut unit_attempts| {
                    unit_attempts.sort_by(|left, right| {
                        right
                            .attempt_number
                            .cmp(&left.attempt_number)
                            .then_with(|| right.updated_at.cmp(&left.updated_at))
                            .then_with(|| right.attempt_id.cmp(&left.attempt_id))
                    });
                    unit_attempts.into_iter().next()
                });

        if let Some(attempt) = latest_attempt.as_ref() {
            match (&unit.workflow_linkage, &attempt.workflow_linkage) {
                (None, None) => {}
                (Some(_), Some(_)) if unit.workflow_linkage == attempt.workflow_linkage => {}
                (None, Some(_)) => {
                    return Err(map_runtime_run_decode_error(
                        database_path,
                        format!(
                            "Autonomous attempt `{}` retained workflow linkage while parent unit `{}` did not.",
                            attempt.attempt_id, unit.unit_id
                        ),
                    ));
                }
                (Some(_), None) => {
                    return Err(map_runtime_run_decode_error(
                        database_path,
                        format!(
                            "Autonomous attempt `{}` is missing workflow linkage while parent unit `{}` retained durable linkage.",
                            attempt.attempt_id, unit.unit_id
                        ),
                    ));
                }
                (Some(_), Some(_)) => {
                    return Err(map_runtime_run_decode_error(
                        database_path,
                        format!(
                            "Autonomous attempt `{}` workflow linkage does not match parent unit `{}` linkage.",
                            attempt.attempt_id, unit.unit_id
                        ),
                    ));
                }
            }
        }

        let unit_artifacts = latest_attempt
            .as_ref()
            .and_then(|attempt| artifacts_by_attempt.remove(&attempt.attempt_id))
            .unwrap_or_default();

        history.push(AutonomousUnitHistoryRecord {
            unit,
            latest_attempt,
            artifacts: unit_artifacts,
        });
    }

    Ok(history)
}

fn decode_autonomous_run_row(
    raw_row: RawAutonomousRunRow,
    database_path: &Path,
) -> Result<AutonomousRunRecord, CommandError> {
    let project_id =
        require_runtime_run_non_empty_owned(raw_row.project_id, "project_id", database_path)?;
    let agent_session_id = require_runtime_run_non_empty_owned(
        raw_row.agent_session_id,
        "agent_session_id",
        database_path,
    )?;
    let run_id = require_runtime_run_non_empty_owned(raw_row.run_id, "run_id", database_path)?;
    let runtime_kind =
        require_runtime_run_non_empty_owned(raw_row.runtime_kind, "runtime_kind", database_path)?;
    let provider_id =
        require_runtime_run_non_empty_owned(raw_row.provider_id, "provider_id", database_path)?;
    crate::runtime::resolve_runtime_provider_identity(
        Some(provider_id.as_str()),
        Some(runtime_kind.as_str()),
    )
    .map_err(|diagnostic| {
        map_runtime_run_decode_error(
            database_path,
            format!(
                "Autonomous run provider identity is invalid because {}",
                diagnostic.message
            ),
        )
    })?;
    let supervisor_kind = require_runtime_run_non_empty_owned(
        raw_row.supervisor_kind,
        "supervisor_kind",
        database_path,
    )?;
    let status = parse_autonomous_run_status(&raw_row.status).map_err(|details| {
        map_runtime_run_decode_error(database_path, format!("Field `status` {details}"))
    })?;
    let active_unit_sequence = raw_row
        .active_unit_sequence
        .map(|value| {
            decode_runtime_run_checkpoint_sequence(value, "active_unit_sequence", database_path)
        })
        .transpose()?;
    let duplicate_start_detected = decode_runtime_run_bool(
        raw_row.duplicate_start_detected,
        "duplicate_start_detected",
        database_path,
    )?;
    let duplicate_start_run_id = decode_runtime_run_optional_non_empty_text(
        raw_row.duplicate_start_run_id,
        "duplicate_start_run_id",
        database_path,
    )?;
    let duplicate_start_reason = decode_runtime_run_optional_non_empty_text(
        raw_row.duplicate_start_reason,
        "duplicate_start_reason",
        database_path,
    )?;
    let started_at =
        require_runtime_run_non_empty_owned(raw_row.started_at, "started_at", database_path)?;
    let last_heartbeat_at = decode_runtime_run_optional_non_empty_text(
        raw_row.last_heartbeat_at,
        "last_heartbeat_at",
        database_path,
    )?;
    let last_checkpoint_at = decode_runtime_run_optional_non_empty_text(
        raw_row.last_checkpoint_at,
        "last_checkpoint_at",
        database_path,
    )?;
    let paused_at =
        decode_runtime_run_optional_non_empty_text(raw_row.paused_at, "paused_at", database_path)?;
    let cancelled_at = decode_runtime_run_optional_non_empty_text(
        raw_row.cancelled_at,
        "cancelled_at",
        database_path,
    )?;
    let completed_at = decode_runtime_run_optional_non_empty_text(
        raw_row.completed_at,
        "completed_at",
        database_path,
    )?;
    let crashed_at = decode_runtime_run_optional_non_empty_text(
        raw_row.crashed_at,
        "crashed_at",
        database_path,
    )?;
    let stopped_at = decode_runtime_run_optional_non_empty_text(
        raw_row.stopped_at,
        "stopped_at",
        database_path,
    )?;
    let pause_reason = decode_runtime_run_reason(
        raw_row.pause_reason_code,
        raw_row.pause_reason_message,
        "pause_reason",
        database_path,
    )?;
    let cancel_reason = decode_runtime_run_reason(
        raw_row.cancel_reason_code,
        raw_row.cancel_reason_message,
        "cancel_reason",
        database_path,
    )?;
    let crash_reason = decode_runtime_run_reason(
        raw_row.crash_reason_code,
        raw_row.crash_reason_message,
        "crash_reason",
        database_path,
    )?;
    let last_error = decode_runtime_run_reason(
        raw_row.last_error_code,
        raw_row.last_error_message,
        "last_error",
        database_path,
    )?;
    let updated_at =
        require_runtime_run_non_empty_owned(raw_row.updated_at, "updated_at", database_path)?;

    if duplicate_start_detected
        && (duplicate_start_run_id.is_none() || duplicate_start_reason.is_none())
    {
        return Err(map_runtime_run_decode_error(
            database_path,
            "Autonomous run duplicate-start fields must be fully populated when duplicate_start_detected is true.".into(),
        ));
    }

    if !duplicate_start_detected
        && (duplicate_start_run_id.is_some() || duplicate_start_reason.is_some())
    {
        return Err(map_runtime_run_decode_error(
            database_path,
            "Autonomous run duplicate-start fields must be null when duplicate_start_detected is false.".into(),
        ));
    }

    Ok(AutonomousRunRecord {
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
        pause_reason,
        cancel_reason,
        crash_reason,
        last_error,
        updated_at,
    })
}

fn decode_autonomous_workflow_linkage_row(
    workflow_node_id: Option<String>,
    transition_id: Option<String>,
    causal_transition_id: Option<String>,
    handoff_transition_id: Option<String>,
    handoff_package_hash: Option<String>,
    database_path: &Path,
) -> Result<Option<AutonomousWorkflowLinkageRecord>, CommandError> {
    let populated_fields = [
        workflow_node_id.is_some(),
        transition_id.is_some(),
        handoff_transition_id.is_some(),
        handoff_package_hash.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();

    if populated_fields == 0 && causal_transition_id.is_none() {
        return Ok(None);
    }

    if populated_fields != 4 {
        return Err(map_runtime_run_decode_error(
            database_path,
            "Autonomous workflow linkage rows must either omit all linkage fields or persist non-empty `workflow_node_id`, `transition_id`, `handoff_transition_id`, and `handoff_package_hash` values."
                .into(),
        ));
    }

    let handoff_package_hash = require_runtime_run_non_empty_owned(
        handoff_package_hash.ok_or_else(|| {
            map_runtime_run_decode_error(
                database_path,
                "Field `workflow_handoff_package_hash` must be a non-empty string when workflow linkage is present."
                    .into(),
            )
        })?,
        "workflow_handoff_package_hash",
        database_path,
    )?;
    validate_workflow_handoff_package_hash(
        &handoff_package_hash,
        "workflow_handoff_package_hash",
        database_path,
        "runtime_run_decode_failed",
    )?;

    Ok(Some(AutonomousWorkflowLinkageRecord {
        workflow_node_id: require_runtime_run_non_empty_owned(
            workflow_node_id.ok_or_else(|| {
                map_runtime_run_decode_error(
                    database_path,
                    "Field `workflow_node_id` must be a non-empty string when workflow linkage is present."
                        .into(),
                )
            })?,
            "workflow_node_id",
            database_path,
        )?,
        transition_id: require_runtime_run_non_empty_owned(
            transition_id.ok_or_else(|| {
                map_runtime_run_decode_error(
                    database_path,
                    "Field `workflow_transition_id` must be a non-empty string when workflow linkage is present."
                        .into(),
                )
            })?,
            "workflow_transition_id",
            database_path,
        )?,
        causal_transition_id: decode_runtime_run_optional_non_empty_text(
            causal_transition_id,
            "workflow_causal_transition_id",
            database_path,
        )?,
        handoff_transition_id: require_runtime_run_non_empty_owned(
            handoff_transition_id.ok_or_else(|| {
                map_runtime_run_decode_error(
                    database_path,
                    "Field `workflow_handoff_transition_id` must be a non-empty string when workflow linkage is present."
                        .into(),
                )
            })?,
            "workflow_handoff_transition_id",
            database_path,
        )?,
        handoff_package_hash,
    }))
}

fn decode_autonomous_unit_row(
    raw_row: RawAutonomousUnitRow,
    database_path: &Path,
) -> Result<AutonomousUnitRecord, CommandError> {
    Ok(AutonomousUnitRecord {
        project_id: require_runtime_run_non_empty_owned(
            raw_row.project_id,
            "project_id",
            database_path,
        )?,
        run_id: require_runtime_run_non_empty_owned(raw_row.run_id, "run_id", database_path)?,
        unit_id: require_runtime_run_non_empty_owned(raw_row.unit_id, "unit_id", database_path)?,
        sequence: decode_runtime_run_checkpoint_sequence(
            raw_row.sequence,
            "sequence",
            database_path,
        )?,
        kind: parse_autonomous_unit_kind(&raw_row.kind).map_err(|details| {
            map_runtime_run_decode_error(database_path, format!("Field `kind` {details}"))
        })?,
        status: parse_autonomous_unit_status(&raw_row.status).map_err(|details| {
            map_runtime_run_decode_error(database_path, format!("Field `status` {details}"))
        })?,
        summary: require_runtime_run_non_empty_owned(raw_row.summary, "summary", database_path)?,
        boundary_id: decode_runtime_run_optional_non_empty_text(
            raw_row.boundary_id,
            "boundary_id",
            database_path,
        )?,
        workflow_linkage: decode_autonomous_workflow_linkage_row(
            raw_row.workflow_node_id,
            raw_row.workflow_transition_id,
            raw_row.workflow_causal_transition_id,
            raw_row.workflow_handoff_transition_id,
            raw_row.workflow_handoff_package_hash,
            database_path,
        )?,
        started_at: require_runtime_run_non_empty_owned(
            raw_row.started_at,
            "started_at",
            database_path,
        )?,
        finished_at: decode_runtime_run_optional_non_empty_text(
            raw_row.finished_at,
            "finished_at",
            database_path,
        )?,
        updated_at: require_runtime_run_non_empty_owned(
            raw_row.updated_at,
            "updated_at",
            database_path,
        )?,
        last_error: decode_runtime_run_reason(
            raw_row.last_error_code,
            raw_row.last_error_message,
            "last_error",
            database_path,
        )?,
    })
}

fn decode_autonomous_unit_attempt_row(
    raw_row: RawAutonomousUnitAttemptRow,
    database_path: &Path,
) -> Result<AutonomousUnitAttemptRecord, CommandError> {
    Ok(AutonomousUnitAttemptRecord {
        project_id: require_runtime_run_non_empty_owned(
            raw_row.project_id,
            "project_id",
            database_path,
        )?,
        run_id: require_runtime_run_non_empty_owned(raw_row.run_id, "run_id", database_path)?,
        unit_id: require_runtime_run_non_empty_owned(raw_row.unit_id, "unit_id", database_path)?,
        attempt_id: require_runtime_run_non_empty_owned(
            raw_row.attempt_id,
            "attempt_id",
            database_path,
        )?,
        attempt_number: decode_runtime_run_checkpoint_sequence(
            raw_row.attempt_number,
            "attempt_number",
            database_path,
        )?,
        child_session_id: require_runtime_run_non_empty_owned(
            raw_row.child_session_id,
            "child_session_id",
            database_path,
        )?,
        status: parse_autonomous_unit_status(&raw_row.status).map_err(|details| {
            map_runtime_run_decode_error(database_path, format!("Field `status` {details}"))
        })?,
        boundary_id: decode_runtime_run_optional_non_empty_text(
            raw_row.boundary_id,
            "boundary_id",
            database_path,
        )?,
        workflow_linkage: decode_autonomous_workflow_linkage_row(
            raw_row.workflow_node_id,
            raw_row.workflow_transition_id,
            raw_row.workflow_causal_transition_id,
            raw_row.workflow_handoff_transition_id,
            raw_row.workflow_handoff_package_hash,
            database_path,
        )?,
        started_at: require_runtime_run_non_empty_owned(
            raw_row.started_at,
            "started_at",
            database_path,
        )?,
        finished_at: decode_runtime_run_optional_non_empty_text(
            raw_row.finished_at,
            "finished_at",
            database_path,
        )?,
        updated_at: require_runtime_run_non_empty_owned(
            raw_row.updated_at,
            "updated_at",
            database_path,
        )?,
        last_error: decode_runtime_run_reason(
            raw_row.last_error_code,
            raw_row.last_error_message,
            "last_error",
            database_path,
        )?,
    })
}

fn decode_autonomous_unit_artifact_row(
    raw_row: RawAutonomousUnitArtifactRow,
    database_path: &Path,
) -> Result<AutonomousUnitArtifactRecord, CommandError> {
    let project_id =
        require_runtime_run_non_empty_owned(raw_row.project_id, "project_id", database_path)?;
    let run_id = require_runtime_run_non_empty_owned(raw_row.run_id, "run_id", database_path)?;
    let unit_id = require_runtime_run_non_empty_owned(raw_row.unit_id, "unit_id", database_path)?;
    let attempt_id =
        require_runtime_run_non_empty_owned(raw_row.attempt_id, "attempt_id", database_path)?;
    let artifact_id =
        require_runtime_run_non_empty_owned(raw_row.artifact_id, "artifact_id", database_path)?;
    let artifact_kind =
        require_runtime_run_non_empty_owned(raw_row.artifact_kind, "artifact_kind", database_path)?;
    let summary = require_runtime_run_non_empty_owned(raw_row.summary, "summary", database_path)?;
    let content_hash = decode_runtime_run_optional_non_empty_text(
        raw_row.content_hash,
        "content_hash",
        database_path,
    )?;
    if let Some(content_hash) = content_hash.as_deref() {
        validate_workflow_handoff_package_hash(
            content_hash,
            "content_hash",
            database_path,
            "runtime_run_decode_failed",
        )?;
    }

    let payload = raw_row
        .payload_json
        .map(|payload_json| {
            decode_autonomous_artifact_payload_json(
                &payload_json,
                &project_id,
                &run_id,
                &unit_id,
                &attempt_id,
                &artifact_id,
                &artifact_kind,
                database_path,
            )
        })
        .transpose()?;

    if payload.is_some() && content_hash.is_none() {
        return Err(map_runtime_run_decode_error(
            database_path,
            format!(
                "Autonomous artifact `{artifact_id}` stored structured payload JSON without a matching content_hash."
            ),
        ));
    }

    if let (Some(payload), Some(content_hash)) = (payload.as_ref(), content_hash.as_deref()) {
        let canonical_payload = canonicalize_autonomous_artifact_payload_json(payload)?;
        let expected_hash = compute_workflow_handoff_package_hash(&canonical_payload);
        if content_hash != expected_hash {
            return Err(map_runtime_run_decode_error(
                database_path,
                format!(
                    "Autonomous artifact `{artifact_id}` stored content_hash `{content_hash}` but canonical payload hash is `{expected_hash}`."
                ),
            ));
        }
    }

    if payload.is_none() && autonomous_artifact_kind_requires_payload(&artifact_kind) {
        return Err(map_runtime_run_decode_error(
            database_path,
            format!(
                "Autonomous artifact `{artifact_id}` of kind `{artifact_kind}` must persist a structured payload JSON value."
            ),
        ));
    }

    Ok(AutonomousUnitArtifactRecord {
        project_id,
        run_id,
        unit_id,
        attempt_id,
        artifact_id,
        artifact_kind,
        status: parse_autonomous_unit_artifact_status(&raw_row.status).map_err(|details| {
            map_runtime_run_decode_error(database_path, format!("Field `status` {details}"))
        })?,
        summary,
        content_hash,
        payload,
        created_at: require_runtime_run_non_empty_owned(
            raw_row.created_at,
            "created_at",
            database_path,
        )?,
        updated_at: require_runtime_run_non_empty_owned(
            raw_row.updated_at,
            "updated_at",
            database_path,
        )?,
    })
}

pub(crate) fn read_autonomous_unit_attempt_by_id(
    connection: &Connection,
    database_path: &Path,
    project_id: &str,
    run_id: &str,
    attempt_id: &str,
) -> Result<Option<AutonomousUnitAttemptRecord>, CommandError> {
    let row = connection.query_row(
        r#"
        SELECT
            project_id,
            run_id,
            unit_id,
            attempt_id,
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
        FROM autonomous_unit_attempts
        WHERE project_id = ?1
          AND run_id = ?2
          AND attempt_id = ?3
        "#,
        params![project_id, run_id, attempt_id],
        |row| {
            Ok(RawAutonomousUnitAttemptRow {
                project_id: row.get(0)?,
                run_id: row.get(1)?,
                unit_id: row.get(2)?,
                attempt_id: row.get(3)?,
                attempt_number: row.get(4)?,
                child_session_id: row.get(5)?,
                status: row.get(6)?,
                boundary_id: row.get(7)?,
                workflow_node_id: row.get(8)?,
                workflow_transition_id: row.get(9)?,
                workflow_causal_transition_id: row.get(10)?,
                workflow_handoff_transition_id: row.get(11)?,
                workflow_handoff_package_hash: row.get(12)?,
                started_at: row.get(13)?,
                finished_at: row.get(14)?,
                last_error_code: row.get(15)?,
                last_error_message: row.get(16)?,
                updated_at: row.get(17)?,
            })
        },
    );

    match row {
        Ok(row) => Ok(Some(decode_autonomous_unit_attempt_row(
            row,
            database_path,
        )?)),
        Err(SqlError::QueryReturnedNoRows) => Ok(None),
        Err(other) => Err(CommandError::system_fault(
            "autonomous_unit_attempt_query_failed",
            format!(
                "Cadence could not read autonomous attempt `{attempt_id}` from {}: {other}",
                database_path.display()
            ),
        )),
    }
}
