use super::support::*;

pub(crate) fn start_autonomous_run_reuses_existing_boundary_and_persists_duplicate_start_visibility(
) {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let recorder = attach_event_recorders(&app);
    let (project_id, repo_root) = seed_project(&root, &app);

    seed_authenticated_runtime(&app, &auth_store_path, &project_id);
    recorder.clear();

    let first = start_autonomous_run(
        app.handle().clone(),
        app.state::<DesktopState>(),
        StartAutonomousRunRequestDto {
            project_id: project_id.clone(),
            initial_controls: None,
            initial_prompt: None,
        },
    )
    .expect("start autonomous run");
    let running = first.run.expect("autonomous start should return run state");
    assert_eq!(running.project_id, project_id);
    assert!(matches!(
        running.status,
        AutonomousRunStatusDto::Starting | AutonomousRunStatusDto::Running
    ));
    assert!(!running.duplicate_start_detected);

    wait_for_runtime_run(&app, &running.project_id, |runtime_run| {
        runtime_run.status == RuntimeRunStatusDto::Running
            && runtime_run.transport.liveness == RuntimeRunTransportLivenessDto::Reachable
    });

    let second = start_autonomous_run(
        app.handle().clone(),
        app.state::<DesktopState>(),
        StartAutonomousRunRequestDto {
            project_id: project_id.clone(),
            initial_controls: None,
            initial_prompt: None,
        },
    )
    .expect("second autonomous start should reconnect");
    let duplicate = second
        .run
        .expect("duplicate autonomous start should return run state");
    assert_eq!(duplicate.run_id, running.run_id);
    assert!(duplicate.duplicate_start_detected);
    assert_eq!(
        duplicate.duplicate_start_run_id.as_deref(),
        Some(running.run_id.as_str())
    );
    assert_eq!(
        duplicate.duplicate_start_reason.as_deref(),
        Some(
            "Cadence reused the already-active autonomous run for this project instead of launching a duplicate supervisor."
        )
    );
    assert_eq!(count_runtime_run_rows(&repo_root), 1);
    assert_eq!(recorder.runtime_update_count(), 0);
    assert!(recorder.runtime_run_update_count() >= 1);

    let persisted = get_autonomous_run(
        app.handle().clone(),
        app.state::<DesktopState>(),
        GetAutonomousRunRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("get autonomous run after duplicate start")
    .run
    .expect("persisted autonomous run should exist");
    assert_eq!(persisted.run_id, running.run_id);
    assert!(persisted.duplicate_start_detected);
    assert_eq!(
        persisted.recovery_state,
        AutonomousRunRecoveryStateDto::Healthy
    );

    let cancelled = cancel_autonomous_run(
        app.handle().clone(),
        app.state::<DesktopState>(),
        CancelAutonomousRunRequestDto {
            project_id,
            run_id: running.run_id,
        },
    )
    .expect("cancel autonomous run should succeed")
    .run
    .expect("cancelled autonomous run should still exist");
    assert_eq!(cancelled.status, AutonomousRunStatusDto::Cancelled);
    assert!(cancelled.cancelled_at.is_some());
    assert_eq!(
        cancelled.recovery_state,
        AutonomousRunRecoveryStateDto::Terminal
    );
}

pub(crate) fn autonomous_run_rehydrates_same_boundary_after_reload_and_prevents_duplicate_continuation(
) {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, repo_root) = seed_project(&root, &app);

    seed_authenticated_runtime(&app, &auth_store_path, &project_id);

    let started = start_autonomous_run(
        app.handle().clone(),
        app.state::<DesktopState>(),
        StartAutonomousRunRequestDto {
            project_id: project_id.clone(),
            initial_controls: None,
            initial_prompt: None,
        },
    )
    .expect("start autonomous run for reload proof");
    let started_run = started
        .run
        .expect("autonomous start should return run state");
    assert!(matches!(
        started_run.status,
        AutonomousRunStatusDto::Starting | AutonomousRunStatusDto::Running
    ));

    wait_for_runtime_run(&app, &project_id, |runtime_run| {
        runtime_run.run_id == started_run.run_id
            && runtime_run.status == RuntimeRunStatusDto::Running
            && runtime_run.transport.liveness == RuntimeRunTransportLivenessDto::Reachable
    });
    let initial_autonomous = wait_for_autonomous_run(&app, &project_id, |autonomous_state| {
        let Some(run) = autonomous_state.run.as_ref() else {
            return false;
        };
        let Some(unit) = autonomous_state.unit.as_ref() else {
            return false;
        };

        run.run_id == started_run.run_id
            && matches!(
                run.status,
                AutonomousRunStatusDto::Starting | AutonomousRunStatusDto::Running
            )
            && run.recovery_state == AutonomousRunRecoveryStateDto::Healthy
            && run.active_unit_id.as_deref() == Some(unit.unit_id.as_str())
            && unit.sequence >= 1
            && unit.status == AutonomousUnitStatusDto::Active
    });
    let initial_run = initial_autonomous
        .run
        .as_ref()
        .expect("initial autonomous run should exist");
    let initial_unit = initial_autonomous
        .unit
        .as_ref()
        .expect("initial autonomous unit should exist");
    assert_eq!(initial_run.run_id, started_run.run_id);
    assert_eq!(
        initial_run.active_unit_id.as_deref(),
        Some(initial_unit.unit_id.as_str())
    );

    let (fresh_state, _fresh_registry_path, _fresh_auth_store_path) = create_state(&root);
    let fresh_app = build_mock_app(fresh_state);

    let recovered_runtime = wait_for_runtime_run(&fresh_app, &project_id, |runtime_run| {
        runtime_run.run_id == started_run.run_id
            && runtime_run.status == RuntimeRunStatusDto::Running
            && runtime_run.transport.liveness == RuntimeRunTransportLivenessDto::Reachable
    });
    assert_eq!(recovered_runtime.run_id, started_run.run_id);

    let recovered = wait_for_autonomous_run(&fresh_app, &project_id, |autonomous_state| {
        let Some(run) = autonomous_state.run.as_ref() else {
            return false;
        };
        let Some(unit) = autonomous_state.unit.as_ref() else {
            return false;
        };

        run.run_id == started_run.run_id
            && run.recovery_state == AutonomousRunRecoveryStateDto::Healthy
            && run.active_unit_id.as_deref() == Some(unit.unit_id.as_str())
            && matches!(
                run.status,
                AutonomousRunStatusDto::Starting | AutonomousRunStatusDto::Running
            )
            && unit.sequence >= initial_unit.sequence
            && unit.status == AutonomousUnitStatusDto::Active
    });
    let recovered_run = recovered
        .run
        .as_ref()
        .expect("recovered autonomous run should exist");
    let recovered_unit = recovered
        .unit
        .as_ref()
        .expect("recovered autonomous unit should exist");
    assert_eq!(recovered_run.run_id, started_run.run_id);
    assert_eq!(
        recovered_run.active_unit_id.as_deref(),
        Some(recovered_unit.unit_id.as_str())
    );
    assert_eq!(count_runtime_run_rows(&repo_root), 1);
    assert_eq!(count_autonomous_run_rows(&repo_root), 1);

    let snapshot = get_project_snapshot(
        fresh_app.handle().clone(),
        fresh_app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("project snapshot should expose the same autonomous boundary after reload");
    assert_eq!(
        snapshot
            .autonomous_run
            .as_ref()
            .map(|autonomous| autonomous.run_id.as_str()),
        Some(started_run.run_id.as_str())
    );
    assert_eq!(
        snapshot
            .autonomous_unit
            .as_ref()
            .map(|autonomous| autonomous.unit_id.as_str()),
        Some(recovered_unit.unit_id.as_str())
    );

    let duplicate = start_autonomous_run(
        fresh_app.handle().clone(),
        fresh_app.state::<DesktopState>(),
        StartAutonomousRunRequestDto {
            project_id: project_id.clone(),
            initial_controls: None,
            initial_prompt: None,
        },
    )
    .expect("duplicate autonomous start after reload should reconnect");
    let duplicate_run = duplicate
        .run
        .expect("duplicate autonomous start should return run state");
    assert_eq!(duplicate_run.run_id, started_run.run_id);
    assert!(duplicate_run.duplicate_start_detected);
    assert_eq!(
        duplicate_run.duplicate_start_run_id.as_deref(),
        Some(started_run.run_id.as_str())
    );
    assert_eq!(count_runtime_run_rows(&repo_root), 1);
    assert_eq!(count_autonomous_run_rows(&repo_root), 1);

    let cancelled = cancel_autonomous_run(
        fresh_app.handle().clone(),
        fresh_app.state::<DesktopState>(),
        CancelAutonomousRunRequestDto {
            project_id: project_id.clone(),
            run_id: started_run.run_id.clone(),
        },
    )
    .expect("cancel autonomous run after reload should succeed")
    .run
    .expect("cancelled autonomous run should still exist");
    assert_eq!(cancelled.status, AutonomousRunStatusDto::Cancelled);
    assert_eq!(
        cancelled.recovery_state,
        AutonomousRunRecoveryStateDto::Terminal
    );

    let stopped_runtime = wait_for_runtime_run(&fresh_app, &project_id, |runtime_run| {
        runtime_run.run_id == started_run.run_id
            && runtime_run.status == RuntimeRunStatusDto::Stopped
    });
    assert!(stopped_runtime.stopped_at.is_some());
}

pub(crate) fn get_autonomous_run_recovers_stale_boundary_after_fresh_host_reload() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, repo_root) = seed_project(&root, &app);

    seed_authenticated_runtime(&app, &auth_store_path, &project_id);
    seed_unreachable_runtime_run(&repo_root, &project_id, "run-unreachable");

    let (fresh_state, _fresh_registry_path, _fresh_auth_store_path) = create_state(&root);
    let fresh_app = build_mock_app(fresh_state);

    let recovered = get_autonomous_run(
        fresh_app.handle().clone(),
        fresh_app.state::<DesktopState>(),
        GetAutonomousRunRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("get autonomous run after fresh-host restart");
    let run = recovered
        .run
        .expect("autonomous run should exist after restart");
    let unit = recovered
        .unit
        .expect("autonomous unit should exist after restart");
    assert_eq!(run.run_id, "run-unreachable");
    assert_eq!(run.status, AutonomousRunStatusDto::Stale);
    assert_eq!(
        run.recovery_state,
        AutonomousRunRecoveryStateDto::RecoveryRequired
    );
    assert!(run.crashed_at.is_some());
    assert_eq!(
        run.crash_reason.as_ref().map(|reason| reason.code.as_str()),
        Some("runtime_supervisor_connect_failed")
    );
    assert_eq!(unit.run_id, run.run_id);
    assert_eq!(unit.status, AutonomousUnitStatusDto::Active);

    let snapshot = get_project_snapshot(
        fresh_app.handle().clone(),
        fresh_app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("project snapshot should expose recovered autonomous state");
    assert_eq!(
        snapshot
            .autonomous_run
            .as_ref()
            .map(|autonomous| autonomous.run_id.as_str()),
        Some("run-unreachable")
    );
    assert_eq!(
        snapshot
            .autonomous_unit
            .as_ref()
            .map(|autonomous| autonomous.sequence),
        Some(1)
    );
}
