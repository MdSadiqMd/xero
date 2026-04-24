use super::support::*;

pub(crate) fn get_autonomous_run_returns_transient_state_when_initial_persist_hits_write_lock() {
    let _guard = supervisor_test_guard();
    let root = tempfile::tempdir().expect("temp dir");
    let app = build_mock_app(create_state(&root));
    let (project_id, repo_root) = seed_project(&root, &app);
    let runtime_session = seed_authenticated_runtime(&app, &root, &project_id);

    let launched = launch_scripted_runtime_run(
        app.state::<DesktopState>().inner(),
        &repo_root,
        &project_id,
        "run-autonomous-observe-write-lock",
        runtime_session
            .session_id
            .as_deref()
            .expect("authenticated runtime session id"),
        runtime_session.flow_id.as_deref(),
        &runtime_shell::script_print_line_and_sleep("hold", 2),
    );

    wait_for_runtime_run(&app, &project_id, |runtime_run| {
        runtime_run.run_id == launched.run.run_id
            && runtime_run.status == RuntimeRunStatusDto::Running
            && runtime_run.transport.liveness == RuntimeRunTransportLivenessDto::Reachable
    });

    let database_path = database_path_for_repo(&repo_root);
    let locking_connection =
        rusqlite::Connection::open(&database_path).expect("open runtime db for write lock");
    locking_connection
        .execute_batch("PRAGMA journal_mode = WAL; BEGIN IMMEDIATE;")
        .expect("acquire write lock");

    let observed = get_autonomous_run(
        app.handle().clone(),
        app.state::<DesktopState>(),
        GetAutonomousRunRequestDto {
            project_id: project_id.clone(),
            agent_session_id: "agent-session-main".into(),
        },
    )
    .expect("get autonomous run should fall back to the transient snapshot while the durable autonomous row is locked");

    locking_connection
        .execute_batch("ROLLBACK;")
        .expect("release write lock");

    let observed_run = observed
        .run
        .as_ref()
        .expect("observed autonomous run should exist");
    assert_eq!(observed_run.run_id, launched.run.run_id);
    assert!(matches!(
        observed_run.status,
        AutonomousRunStatusDto::Starting | AutonomousRunStatusDto::Running
    ));
    assert!(!observed_run.duplicate_start_detected);

    let recovered = wait_for_autonomous_run(&app, &project_id, |autonomous_state| {
        autonomous_state
            .run
            .as_ref()
            .is_some_and(|run| run.run_id == launched.run.run_id)
    });
    assert_eq!(
        recovered
            .run
            .as_ref()
            .expect("recovered autonomous run should exist")
            .run_id,
        launched.run.run_id
    );
}

pub(crate) fn get_autonomous_run_reuses_unchanged_snapshot_without_write_lock_contention() {
    let _guard = supervisor_test_guard();
    let root = tempfile::tempdir().expect("temp dir");
    let app = build_mock_app(create_state(&root));
    let (project_id, repo_root) = seed_project(&root, &app);
    let runtime_session = seed_authenticated_runtime(&app, &root, &project_id);

    let launched = launch_scripted_runtime_run(
        app.state::<DesktopState>().inner(),
        &repo_root,
        &project_id,
        "run-autonomous-observe-noop",
        runtime_session
            .session_id
            .as_deref()
            .expect("authenticated runtime session id"),
        runtime_session.flow_id.as_deref(),
        &runtime_shell::script_join_steps(&[
            runtime_shell::script_print_line("noop"),
            runtime_shell::script_exit(0),
        ]),
    );

    let stopped_runtime = wait_for_runtime_run(&app, &project_id, |runtime_run| {
        runtime_run.run_id == launched.run.run_id
            && runtime_run.status == RuntimeRunStatusDto::Stopped
    });
    assert_eq!(stopped_runtime.status, RuntimeRunStatusDto::Stopped);

    let initial = get_autonomous_run(
        app.handle().clone(),
        app.state::<DesktopState>(),
        GetAutonomousRunRequestDto {
            project_id: project_id.clone(),
            agent_session_id: "agent-session-main".into(),
        },
    )
    .expect("seed autonomous snapshot before lock");
    let initial_run = initial.run.as_ref().expect("autonomous run should exist");
    assert_eq!(initial_run.run_id, launched.run.run_id);
    assert_eq!(initial_run.status, AutonomousRunStatusDto::Stopped);

    let database_path = database_path_for_repo(&repo_root);
    let locking_connection =
        rusqlite::Connection::open(&database_path).expect("open runtime db for write lock");
    locking_connection
        .execute_batch("PRAGMA journal_mode = WAL; BEGIN IMMEDIATE;")
        .expect("acquire write lock");

    let observed = get_autonomous_run(
        app.handle().clone(),
        app.state::<DesktopState>(),
        GetAutonomousRunRequestDto {
            project_id: project_id.clone(),
            agent_session_id: "agent-session-main".into(),
        },
    )
    .expect("get autonomous run should reuse existing snapshot without writes");

    locking_connection
        .execute_batch("ROLLBACK;")
        .expect("release write lock");

    let observed_run = observed
        .run
        .as_ref()
        .expect("observed autonomous run should exist");
    assert_eq!(observed_run.run_id, launched.run.run_id);
    assert_eq!(observed_run.status, AutonomousRunStatusDto::Stopped);
    assert_eq!(
        observed
            .history
            .first()
            .and_then(|entry| entry.latest_attempt.as_ref())
            .map(|attempt| attempt.attempt_id.clone()),
        initial
            .history
            .first()
            .and_then(|entry| entry.latest_attempt.as_ref())
            .map(|attempt| attempt.attempt_id.clone())
    );
}
