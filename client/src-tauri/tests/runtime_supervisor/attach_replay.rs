use super::support::*;

pub(crate) fn detached_supervisor_attach_replays_buffered_events_after_fresh_host_probe() {
    let _guard = supervisor_test_guard();
    let root = tempfile::tempdir().expect("temp dir");
    let project_id = "project-attach";
    let repo_root = seed_project(&root, project_id, "repo-attach", "repo");
    let state = DesktopState::default();

    let live_lines = vec![
        format!(
            "{STRUCTURED_EVENT_PREFIX}{{\"kind\":\"tool\",\"tool_call_id\":\"tool-1\",\"tool_name\":\"inspect_repository\",\"tool_state\":\"running\",\"detail\":\"Collecting workspace context\"}}"
        ),
        "plain transcript line".to_string(),
        format!(
            "{STRUCTURED_EVENT_PREFIX}{{\"kind\":\"activity\",\"code\":\"phase_progress\",\"title\":\"Planning\",\"detail\":\"Replay buffer ready\"}}"
        ),
    ];

    launch_detached_runtime_supervisor(
        &state,
        launch_request(
            project_id,
            &repo_root,
            "run-attach",
            &runtime_shell::script_print_lines_and_sleep(&live_lines, 5),
        ),
    )
    .expect("launch attachable runtime supervisor");

    wait_for_runtime_run(&state, &repo_root, project_id, |snapshot| {
        snapshot.run.status == project_store::RuntimeRunStatus::Running
            && snapshot.last_checkpoint_sequence >= 2
    });

    let fresh_state = DesktopState::default();
    let recovered = probe_runtime_run(&fresh_state, probe_request(project_id, &repo_root))
        .expect("probe with fresh state")
        .expect("runtime run should exist");

    let mut reader = attach_reader(
        &recovered.run.transport.endpoint,
        SupervisorControlRequest::attach(project_id, "run-attach", None),
    );
    let attached = expect_attach_ack(read_supervisor_response(&mut reader));
    assert_eq!(attached.replayed_count, 3);
    assert_eq!(attached.latest_sequence, Some(3));
    assert_eq!(attached.oldest_available_sequence, Some(1));
    assert!(!attached.replay_truncated);

    let frames = read_event_frames(&mut reader, attached.replayed_count);
    assert_monotonic_sequences(&frames, "run-attach");
    assert!(matches!(
        &frames[0],
        SupervisorControlResponse::Event {
            item:
                SupervisorLiveEventPayload::Tool {
                    tool_call_id,
                    tool_name,
                    tool_state: SupervisorToolCallState::Running,
                    detail,
                    ..
                },
            ..
        } if tool_call_id == "tool-1"
            && tool_name == "inspect_repository"
            && detail.as_deref() == Some("Collecting workspace context")
    ));
    assert!(matches!(
        &frames[1],
        SupervisorControlResponse::Event {
            item: SupervisorLiveEventPayload::Transcript { text },
            ..
        } if text == "plain transcript line"
    ));
    assert!(matches!(
        &frames[2],
        SupervisorControlResponse::Event {
            item: SupervisorLiveEventPayload::Activity { code, title, detail },
            ..
        } if code == "phase_progress"
            && title == "Planning"
            && detail.as_deref() == Some("Replay buffer ready")
    ));

    let stopped = stop_runtime_run(&fresh_state, stop_request(project_id, &repo_root))
        .expect("stop attachable runtime supervisor")
        .expect("stopped runtime run should exist");
    assert_eq!(stopped.run.status, project_store::RuntimeRunStatus::Stopped);
}

pub(crate) fn detached_supervisor_attach_rejects_identity_mismatch_without_mutating_run() {
    let _guard = supervisor_test_guard();
    let root = tempfile::tempdir().expect("temp dir");
    let project_id = "project-mismatch";
    let repo_root = seed_project(&root, project_id, "repo-mismatch", "repo");
    let state = DesktopState::default();

    let launched = launch_detached_runtime_supervisor(
        &state,
        launch_request(
            project_id,
            &repo_root,
            "run-identity",
            &runtime_shell::script_print_line_and_sleep("ready", 5),
        ),
    )
    .expect("launch mismatch runtime supervisor");

    let mut reader = attach_reader(
        &launched.run.transport.endpoint,
        SupervisorControlRequest::attach(project_id, "wrong-run", None),
    );
    let response = read_supervisor_response(&mut reader);
    assert!(matches!(
        response,
        SupervisorControlResponse::Error { code, retryable, .. }
        if code == "runtime_supervisor_identity_mismatch" && !retryable
    ));

    let recovered = probe_runtime_run(&state, probe_request(project_id, &repo_root))
        .expect("probe after mismatch attach")
        .expect("runtime run should still exist");
    assert_eq!(recovered.run.run_id, "run-identity");
    assert_eq!(
        recovered.run.status,
        project_store::RuntimeRunStatus::Running
    );
    assert!(recovered.run.last_error.is_none());

    let stopped = stop_runtime_run(&state, stop_request(project_id, &repo_root))
        .expect("stop mismatch runtime supervisor")
        .expect("runtime run should exist after stop");
    assert_eq!(stopped.run.status, project_store::RuntimeRunStatus::Stopped);
}

pub(crate) fn detached_supervisor_attach_rejects_invalid_cursor_without_mutating_run() {
    let _guard = supervisor_test_guard();
    let root = tempfile::tempdir().expect("temp dir");
    let project_id = "project-cursor";
    let repo_root = seed_project(&root, project_id, "repo-cursor", "repo");
    let state = DesktopState::default();

    let launched = launch_detached_runtime_supervisor(
        &state,
        launch_request(
            project_id,
            &repo_root,
            "run-cursor",
            &runtime_shell::script_print_line_and_sleep("ready", 5),
        ),
    )
    .expect("launch cursor runtime supervisor");

    let mut reader = attach_reader(
        &launched.run.transport.endpoint,
        SupervisorControlRequest::attach(project_id, "run-cursor", Some(0)),
    );
    let response = read_supervisor_response(&mut reader);
    assert!(matches!(
        response,
        SupervisorControlResponse::Error { code, retryable, .. }
        if code == "runtime_supervisor_attach_cursor_invalid" && !retryable
    ));

    let recovered = probe_runtime_run(&state, probe_request(project_id, &repo_root))
        .expect("probe after invalid cursor attach")
        .expect("runtime run should still exist");
    assert_eq!(recovered.run.run_id, "run-cursor");
    assert_eq!(
        recovered.run.status,
        project_store::RuntimeRunStatus::Running
    );
    assert!(recovered.run.last_error.is_none());

    let stopped = stop_runtime_run(&state, stop_request(project_id, &repo_root))
        .expect("stop cursor runtime supervisor")
        .expect("runtime run should exist after stop");
    assert!(matches!(
        stopped.run.status,
        project_store::RuntimeRunStatus::Stopped | project_store::RuntimeRunStatus::Stale
    ));
}

pub(crate) fn detached_supervisor_attach_replays_only_bounded_ring_window() {
    let _guard = supervisor_test_guard();
    let root = tempfile::tempdir().expect("temp dir");
    let project_id = "project-ring";
    let repo_root = seed_project(&root, project_id, "repo-ring", "repo");
    let state = DesktopState::default();
    let emitted_lines = 160_u32;

    let emitted_runtime_lines = (1..=emitted_lines)
        .map(|index| format!("line-{index:03}"))
        .collect::<Vec<_>>();
    let command = runtime_shell::script_print_lines_and_sleep(&emitted_runtime_lines, 5);

    let launched = launch_detached_runtime_supervisor(
        &state,
        launch_request(project_id, &repo_root, "run-ring", &command),
    )
    .expect("launch ring runtime supervisor");

    wait_for_runtime_run(&state, &repo_root, project_id, |snapshot| {
        snapshot.run.status == project_store::RuntimeRunStatus::Running
            && snapshot.last_checkpoint_sequence >= 10
    });

    let mut reader = attach_reader(
        &launched.run.transport.endpoint,
        SupervisorControlRequest::attach(project_id, "run-ring", None),
    );
    let attached = expect_attach_ack(read_supervisor_response(&mut reader));
    assert_eq!(attached.replayed_count, 128);
    assert!(attached.replay_truncated);
    assert_eq!(attached.oldest_available_sequence, Some(33));
    assert_eq!(attached.latest_sequence, Some(160));

    let frames = read_event_frames(&mut reader, attached.replayed_count);
    assert_monotonic_sequences(&frames, "run-ring");
    assert!(matches!(
        &frames.first(),
        Some(SupervisorControlResponse::Event {
            sequence,
            item: SupervisorLiveEventPayload::Transcript { text },
            ..
        }) if *sequence == 33 && text == "line-033"
    ));
    assert!(matches!(
        &frames.last(),
        Some(SupervisorControlResponse::Event {
            sequence,
            item: SupervisorLiveEventPayload::Transcript { text },
            ..
        }) if *sequence == 160 && text == "line-160"
    ));

    let stopped = stop_runtime_run(&state, stop_request(project_id, &repo_root))
        .expect("stop ring runtime supervisor")
        .expect("runtime run should exist after stop");
    assert_eq!(stopped.run.status, project_store::RuntimeRunStatus::Stopped);
}

pub(crate) fn detached_supervisor_attach_rejects_finished_run() {
    let _guard = supervisor_test_guard();
    let root = tempfile::tempdir().expect("temp dir");
    let project_id = "project-finished";
    let repo_root = seed_project(&root, project_id, "repo-finished", "repo");
    let state = DesktopState::default();

    let launched = launch_detached_runtime_supervisor(
        &state,
        launch_request(
            project_id,
            &repo_root,
            "run-finished",
            &runtime_shell::script_print_line_then_exit("done", 0),
        ),
    )
    .expect("launch finished runtime supervisor");

    wait_for_runtime_run(&state, &repo_root, project_id, |snapshot| {
        snapshot.run.status == project_store::RuntimeRunStatus::Stopped
    });

    match TcpStream::connect(&launched.run.transport.endpoint) {
        Ok(mut stream) => {
            stream
                .set_read_timeout(Some(ATTACH_READ_TIMEOUT))
                .expect("set attach read timeout");
            stream
                .set_write_timeout(Some(ATTACH_READ_TIMEOUT))
                .expect("set attach write timeout");
            serde_json::to_writer(
                &mut stream,
                &SupervisorControlRequest::attach(project_id, "run-finished", None),
            )
            .expect("write finished attach request");
            stream.write_all(b"\n").expect("write attach newline");
            stream.flush().expect("flush attach request");
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {}
                Ok(_) => {
                    let response: SupervisorControlResponse =
                        serde_json::from_str(line.trim()).expect("decode finished attach response");
                    assert!(matches!(
                        response,
                        SupervisorControlResponse::Error { code, retryable, .. }
                        if code == "runtime_supervisor_attach_unavailable" && !retryable
                    ));
                }
                Err(error) => {
                    assert!(matches!(
                        error.kind(),
                        std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::UnexpectedEof
                    ));
                }
            }
        }
        Err(error) => {
            assert_eq!(error.kind(), std::io::ErrorKind::ConnectionRefused);
        }
    }
}
