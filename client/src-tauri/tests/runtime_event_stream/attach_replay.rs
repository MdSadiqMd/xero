use super::support::*;

pub(crate) fn runtime_stream_replays_real_supervisor_events_after_fresh_host_reload() {
    let _guard = supervisor_test_guard();
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, repo_root) = seed_project(&root, &app);
    let runtime = seed_authenticated_runtime(&app, &auth_store_path, &project_id);

    let live_lines = vec![
        format!(
            "{STRUCTURED_EVENT_PREFIX}{}",
            json!({
                "kind": "tool",
                "tool_call_id": "tool-1",
                "tool_name": "read",
                "tool_state": "running",
                "detail": "Collecting workspace context",
                "tool_summary": {
                    "kind": "file",
                    "path": "README.md",
                    "scope": null,
                    "lineCount": 12,
                    "matchCount": null,
                    "truncated": true
                }
            })
        ),
        "plain transcript line".to_string(),
        format!(
            "{STRUCTURED_EVENT_PREFIX}{{\"kind\":\"activity\",\"code\":\"phase_progress\",\"title\":\"Planning\",\"detail\":\"Replay buffer ready\"}}"
        ),
    ];

    let launched = launch_supervised_run(
        app.state::<DesktopState>().inner(),
        &project_id,
        &repo_root,
        "run-reload",
        &runtime_shell::script_print_lines_and_sleep(&live_lines, 3),
    );

    wait_for_runtime_run(
        app.state::<DesktopState>().inner(),
        &repo_root,
        &project_id,
        |snapshot| {
            snapshot.run.status == RuntimeRunStatus::Running
                && snapshot.last_checkpoint_sequence >= 2
        },
    );

    let (fresh_state, _fresh_registry_path, _fresh_auth_store_path) = create_state(&root);
    let fresh_app = build_mock_app(fresh_state);
    let (channel, receiver) = capture_stream_channel();
    start_direct_runtime_stream(
        &fresh_app,
        &project_id,
        &repo_root,
        &runtime,
        &launched.run.run_id,
        vec![
            RuntimeStreamItemKind::Transcript,
            RuntimeStreamItemKind::Tool,
            RuntimeStreamItemKind::Activity,
            RuntimeStreamItemKind::Complete,
        ],
        channel,
    );

    let items = collect_until_terminal(receiver);
    assert_monotonic_sequences(&items, &launched.run.run_id);
    assert_eq!(
        items
            .iter()
            .map(|item| item.kind.clone())
            .collect::<Vec<_>>(),
        vec![
            RuntimeStreamItemKind::Tool,
            RuntimeStreamItemKind::Transcript,
            RuntimeStreamItemKind::Activity,
            RuntimeStreamItemKind::Complete,
        ],
        "unexpected replay items: {items:?}"
    );

    assert!(matches!(
        &items[0],
        RuntimeStreamItemDto {
            kind: RuntimeStreamItemKind::Tool,
            tool_call_id: Some(tool_call_id),
            tool_name: Some(tool_name),
            tool_state: Some(RuntimeToolCallState::Running),
            detail: Some(detail),
            tool_summary: Some(ToolResultSummaryDto::File(summary)),
            ..
        } if tool_call_id == "tool-1"
            && tool_name == "read"
            && detail == "Collecting workspace context"
            && summary.path.as_deref() == Some("README.md")
            && summary.line_count == Some(12)
            && summary.truncated
    ));
    assert!(matches!(
        &items[1],
        RuntimeStreamItemDto {
            kind: RuntimeStreamItemKind::Transcript,
            text: Some(text),
            ..
        } if text == "plain transcript line"
    ));
    assert!(matches!(
        &items[2],
        RuntimeStreamItemDto {
            kind: RuntimeStreamItemKind::Activity,
            code: Some(code),
            title: Some(title),
            detail: Some(detail),
            ..
        } if code == "phase_progress"
            && title == "Planning"
            && detail == "Replay buffer ready"
    ));
    assert!(matches!(
        &items[3],
        RuntimeStreamItemDto {
            kind: RuntimeStreamItemKind::Complete,
            detail: Some(detail),
            ..
        } if detail.contains("finished")
    ));

    stop_supervisor_run(
        fresh_app.state::<DesktopState>().inner(),
        &project_id,
        &repo_root,
    );
}

pub(crate) fn runtime_stream_replays_first_class_skill_events_with_source_metadata_after_fresh_host_reload(
) {
    let _guard = supervisor_test_guard();
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, repo_root) = seed_project(&root, &app);
    let runtime = seed_authenticated_runtime(&app, &auth_store_path, &project_id);

    let skill_source = json!({
        "repo": "vercel-labs/skills",
        "path": "skills/find-skills",
        "reference": "main",
        "tree_hash": "0123456789abcdef0123456789abcdef01234567"
    });
    let live_lines = vec![
        format!(
            "{STRUCTURED_EVENT_PREFIX}{}",
            json!({
                "kind": "tool",
                "tool_call_id": "tool-1",
                "tool_name": "read",
                "tool_state": "running",
                "detail": "Collecting workspace context"
            })
        ),
        format!(
            "{STRUCTURED_EVENT_PREFIX}{}",
            json!({
                "kind": "skill",
                "skill_id": "find-skills",
                "stage": "install",
                "result": "succeeded",
                "detail": "Installed autonomous skill `find-skills` from the cached vercel-labs/skills tree.",
                "source": skill_source,
                "cache_status": "refreshed"
            })
        ),
        format!(
            "{STRUCTURED_EVENT_PREFIX}{}",
            json!({
                "kind": "skill",
                "skill_id": "find-skills",
                "stage": "invoke",
                "result": "failed",
                "detail": "Autonomous skill `find-skills` failed during invocation.",
                "source": skill_source,
                "cache_status": "hit",
                "diagnostic": {
                    "code": "autonomous_skill_invoke_failed",
                    "message": "Cadence could not invoke autonomous skill `find-skills`.",
                    "retryable": false
                }
            })
        ),
        format!(
            "{STRUCTURED_EVENT_PREFIX}{{\"kind\":\"activity\",\"code\":\"phase_progress\",\"title\":\"Planning\",\"detail\":\"Replay buffer ready\"}}"
        ),
    ];

    let launched = launch_supervised_run(
        app.state::<DesktopState>().inner(),
        &project_id,
        &repo_root,
        "run-skill-reload",
        &runtime_shell::script_print_lines_and_sleep(&live_lines, 3),
    );

    wait_for_runtime_run(
        app.state::<DesktopState>().inner(),
        &repo_root,
        &project_id,
        |snapshot| {
            snapshot.run.status == RuntimeRunStatus::Running
                && snapshot.last_checkpoint_sequence >= 4
        },
    );

    let (fresh_state, _fresh_registry_path, _fresh_auth_store_path) = create_state(&root);
    let fresh_app = build_mock_app(fresh_state);
    let (channel, receiver) = capture_stream_channel();
    start_direct_runtime_stream(
        &fresh_app,
        &project_id,
        &repo_root,
        &runtime,
        &launched.run.run_id,
        vec![
            RuntimeStreamItemKind::Tool,
            RuntimeStreamItemKind::Skill,
            RuntimeStreamItemKind::Activity,
            RuntimeStreamItemKind::Complete,
        ],
        channel,
    );

    let items = collect_until_terminal(receiver);
    assert_monotonic_sequences(&items, &launched.run.run_id);
    assert_eq!(
        items
            .iter()
            .map(|item| item.kind.clone())
            .collect::<Vec<_>>(),
        vec![
            RuntimeStreamItemKind::Tool,
            RuntimeStreamItemKind::Skill,
            RuntimeStreamItemKind::Skill,
            RuntimeStreamItemKind::Activity,
            RuntimeStreamItemKind::Complete,
        ],
        "unexpected replay items: {items:?}"
    );

    assert!(matches!(
        &items[0],
        RuntimeStreamItemDto {
            kind: RuntimeStreamItemKind::Tool,
            tool_call_id: Some(tool_call_id),
            tool_name: Some(tool_name),
            tool_state: Some(RuntimeToolCallState::Running),
            detail: Some(detail),
            ..
        } if tool_call_id == "tool-1"
            && tool_name == "read"
            && detail == "Collecting workspace context"
    ));

    let install_skill = &items[1];
    assert_eq!(install_skill.skill_id.as_deref(), Some("find-skills"));
    assert_eq!(
        install_skill.skill_stage,
        Some(AutonomousSkillLifecycleStageDto::Install)
    );
    assert_eq!(
        install_skill.skill_result,
        Some(AutonomousSkillLifecycleResultDto::Succeeded)
    );
    assert_eq!(
        install_skill.skill_cache_status,
        Some(AutonomousSkillCacheStatusDto::Refreshed)
    );
    assert_eq!(
        install_skill.detail.as_deref(),
        Some("Installed autonomous skill `find-skills` from the cached vercel-labs/skills tree.")
    );
    let install_source = install_skill
        .skill_source
        .as_ref()
        .expect("install skill source metadata");
    assert_eq!(install_source.repo, "vercel-labs/skills");
    assert_eq!(install_source.path, "skills/find-skills");
    assert_eq!(install_source.reference, "main");
    assert_eq!(
        install_source.tree_hash,
        "0123456789abcdef0123456789abcdef01234567"
    );
    assert_eq!(install_skill.skill_diagnostic, None);

    let invoke_skill = &items[2];
    assert_eq!(invoke_skill.skill_id.as_deref(), Some("find-skills"));
    assert_eq!(
        invoke_skill.skill_stage,
        Some(AutonomousSkillLifecycleStageDto::Invoke)
    );
    assert_eq!(
        invoke_skill.skill_result,
        Some(AutonomousSkillLifecycleResultDto::Failed)
    );
    assert_eq!(
        invoke_skill.skill_cache_status,
        Some(AutonomousSkillCacheStatusDto::Hit)
    );
    assert_eq!(
        invoke_skill.detail.as_deref(),
        Some("Autonomous skill `find-skills` failed during invocation.")
    );
    let invoke_diagnostic = invoke_skill
        .skill_diagnostic
        .as_ref()
        .expect("failed skill diagnostic");
    assert_eq!(invoke_diagnostic.code, "autonomous_skill_invoke_failed");
    assert_eq!(
        invoke_diagnostic.message,
        "Cadence could not invoke autonomous skill `find-skills`."
    );
    assert!(!invoke_diagnostic.retryable);

    assert!(matches!(
        &items[3],
        RuntimeStreamItemDto {
            kind: RuntimeStreamItemKind::Activity,
            code: Some(code),
            title: Some(title),
            detail: Some(detail),
            ..
        } if code == "phase_progress"
            && title == "Planning"
            && detail == "Replay buffer ready"
    ));
    assert!(matches!(
        &items[4],
        RuntimeStreamItemDto {
            kind: RuntimeStreamItemKind::Complete,
            detail: Some(detail),
            ..
        } if detail.contains("finished")
    ));

    stop_supervisor_run(
        fresh_app.state::<DesktopState>().inner(),
        &project_id,
        &repo_root,
    );
}

pub(crate) fn runtime_stream_dropped_channel_does_not_poison_resubscribe() {
    let _guard = supervisor_test_guard();
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, repo_root) = seed_project(&root, &app);
    let runtime = seed_authenticated_runtime(&app, &auth_store_path, &project_id);

    let launched = launch_supervised_run(
        app.state::<DesktopState>().inner(),
        &project_id,
        &repo_root,
        "run-dropped-channel",
        &runtime_shell::script_print_lines_and_sleep(
            &[
                "first replay line".to_string(),
                "second replay line".to_string(),
            ],
            3,
        ),
    );

    wait_for_runtime_run(
        app.state::<DesktopState>().inner(),
        &repo_root,
        &project_id,
        |snapshot| {
            snapshot.run.status == RuntimeRunStatus::Running
                && snapshot.last_checkpoint_sequence >= 1
        },
    );

    let delivery_attempts = Mutex::new(0_usize);
    let dropped_channel = tauri::ipc::Channel::<RuntimeStreamItemDto>::new(move |_body| {
        let mut attempts = delivery_attempts.lock().expect("delivery attempts lock");
        *attempts += 1;
        if *attempts >= 2 {
            Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "channel dropped").into())
        } else {
            Ok(())
        }
    });

    start_direct_runtime_stream(
        &app,
        &project_id,
        &repo_root,
        &runtime,
        &launched.run.run_id,
        vec![
            RuntimeStreamItemKind::Transcript,
            RuntimeStreamItemKind::Complete,
            RuntimeStreamItemKind::Failure,
        ],
        dropped_channel,
    );

    thread::sleep(Duration::from_millis(250));

    let (channel, receiver) = capture_stream_channel();
    start_direct_runtime_stream(
        &app,
        &project_id,
        &repo_root,
        &runtime,
        &launched.run.run_id,
        vec![
            RuntimeStreamItemKind::Transcript,
            RuntimeStreamItemKind::Complete,
        ],
        channel,
    );

    let items = collect_until_terminal(receiver);
    assert_monotonic_sequences(&items, &launched.run.run_id);
    assert_eq!(
        items
            .iter()
            .map(|item| item.kind.clone())
            .collect::<Vec<_>>(),
        vec![
            RuntimeStreamItemKind::Transcript,
            RuntimeStreamItemKind::Transcript,
            RuntimeStreamItemKind::Complete,
        ]
    );
    assert_eq!(items[0].text.as_deref(), Some("first replay line"));
    assert_eq!(items[1].text.as_deref(), Some("second replay line"));

    stop_supervisor_run(app.state::<DesktopState>().inner(), &project_id, &repo_root);
}

pub(crate) fn runtime_stream_emits_typed_failure_when_supervisor_sequence_is_invalid() {
    let _guard = supervisor_test_guard();
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, repo_root) = seed_project(&root, &app);
    let runtime = seed_authenticated_runtime(&app, &auth_store_path, &project_id);

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind fake supervisor listener");
    let endpoint = listener
        .local_addr()
        .expect("read fake supervisor endpoint")
        .to_string();
    let server = thread::spawn({
        let project_id = project_id.clone();
        move || {
            let (mut stream, _) = listener.accept().expect("accept fake supervisor attach");
            let mut line = String::new();
            BufReader::new(stream.try_clone().expect("clone fake supervisor stream"))
                .read_line(&mut line)
                .expect("read attach request");
            let request: SupervisorControlRequest =
                serde_json::from_str(line.trim()).expect("decode attach request");
            assert!(matches!(
                request,
                SupervisorControlRequest::Attach {
                    project_id: requested_project_id,
                    run_id,
                    after_sequence: None,
                    ..
                } if requested_project_id == project_id && run_id == "run-invalid-sequence"
            ));

            write_json_line(
                &mut stream,
                &SupervisorControlResponse::Attached {
                    protocol_version: SUPERVISOR_PROTOCOL_VERSION,
                    project_id: project_id.clone(),
                    run_id: "run-invalid-sequence".into(),
                    after_sequence: None,
                    replayed_count: 1,
                    replay_truncated: false,
                    oldest_available_sequence: Some(1),
                    latest_sequence: Some(1),
                },
            );
            write_json_line(
                &mut stream,
                &SupervisorControlResponse::Event {
                    protocol_version: SUPERVISOR_PROTOCOL_VERSION,
                    project_id,
                    run_id: "run-invalid-sequence".into(),
                    sequence: 0,
                    created_at: "2026-04-15T23:10:02Z".into(),
                    replay: true,
                    item: SupervisorLiveEventPayload::Transcript {
                        text: "bad sequence".into(),
                    },
                },
            );
            thread::sleep(Duration::from_millis(250));
        }
    });

    seed_fake_runtime_run(&repo_root, &project_id, "run-invalid-sequence", &endpoint);

    let (channel, receiver) = capture_stream_channel();
    start_direct_runtime_stream(
        &app,
        &project_id,
        &repo_root,
        &runtime,
        "run-invalid-sequence",
        vec![RuntimeStreamItemKind::Failure],
        channel,
    );

    let items = collect_until_terminal(receiver);
    eprintln!("invalid sequence items: {items:?}");
    assert_eq!(
        items.len(),
        1,
        "expected a single failure item, got {items:?}"
    );
    let failure = &items[0];
    assert_eq!(failure.kind, RuntimeStreamItemKind::Failure);
    assert_eq!(
        failure.code.as_deref(),
        Some("runtime_stream_sequence_invalid")
    );
    assert_eq!(failure.retryable, Some(false));
    assert!(failure
        .message
        .as_deref()
        .expect("failure message")
        .contains("sequence 0"));

    server.join().expect("join fake supervisor thread");
}
