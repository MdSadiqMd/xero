use super::support::*;

pub(crate) fn project_snapshot_returns_empty_operator_loop_arrays_when_no_rows_exist() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path) = create_state(&root);
    let app = build_mock_app(state);
    let project_id = "project-1";
    seed_project(&root, &app, project_id, "repo-1", "repo");

    let snapshot = get_project_snapshot(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.into(),
        },
    )
    .expect("load project snapshot");

    assert!(snapshot.approval_requests.is_empty());
    assert!(snapshot.verification_records.is_empty());
    assert!(snapshot.resume_history.is_empty());
}

pub(crate) fn project_snapshot_persists_operator_loop_metadata_across_reopens_in_stable_order() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path) = create_state(&root);
    let app = build_mock_app(state);
    let project_id = "project-1";
    let repo_root = seed_project(&root, &app, project_id, "repo-1", "repo");
    insert_operator_loop_rows(&repo_root, project_id);

    let first = project_store::load_project_snapshot(&repo_root, project_id)
        .expect("load first snapshot")
        .snapshot;
    let reopened = project_store::load_project_snapshot(&repo_root, project_id)
        .expect("load reopened snapshot")
        .snapshot;

    assert_eq!(first, reopened, "snapshot should be durable across reloads");
    assert_eq!(first.approval_requests.len(), 2);
    assert_eq!(first.approval_requests[0].action_id, "approve-plan");
    assert_eq!(
        first.approval_requests[0].status,
        OperatorApprovalStatus::Pending
    );
    assert_eq!(first.approval_requests[1].action_id, "review-worktree");
    assert_eq!(
        first.approval_requests[1].decision_note.as_deref(),
        Some("Changes reviewed and accepted.")
    );
    assert_eq!(first.verification_records.len(), 1);
    assert_eq!(
        first.verification_records[0].summary,
        "Reviewed repository status before resume."
    );
    assert_eq!(first.resume_history.len(), 1);
    assert_eq!(
        first.resume_history[0].summary,
        "Operator resumed the selected project runtime."
    );
}

pub(crate) fn project_snapshot_scopes_operator_loop_rows_to_the_selected_project() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path) = create_state(&root);
    let app = build_mock_app(state);
    let project_id = "project-1";
    let repo_root = seed_project(&root, &app, project_id, "repo-1", "repo");
    insert_operator_loop_rows(&repo_root, project_id);
    insert_other_project_rows(&repo_root);

    let snapshot = project_store::load_project_snapshot(&repo_root, project_id)
        .expect("load scoped snapshot")
        .snapshot;

    assert_eq!(snapshot.approval_requests.len(), 2);
    assert!(snapshot
        .approval_requests
        .iter()
        .all(|approval| approval.action_id != "other-action"));
    assert_eq!(snapshot.verification_records.len(), 1);
    assert!(snapshot
        .verification_records
        .iter()
        .all(|record| record.source_action_id.as_deref() != Some("other-action")));
    assert_eq!(snapshot.resume_history.len(), 1);
    assert!(snapshot
        .resume_history
        .iter()
        .all(|entry| entry.source_action_id.as_deref() != Some("other-action")));
}

pub(crate) fn malformed_operator_loop_rows_fail_closed_during_snapshot_decode() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path) = create_state(&root);
    let app = build_mock_app(state);
    let project_id = "project-1";
    let repo_root = seed_project(&root, &app, project_id, "repo-1", "repo");
    insert_operator_loop_rows(&repo_root, project_id);

    let connection = open_state_connection(&repo_root);
    connection
        .execute_batch("PRAGMA ignore_check_constraints = 1;")
        .expect("disable check constraints for corruption test");
    connection
        .execute(
            "UPDATE operator_approvals SET status = 'bogus_status' WHERE project_id = ?1 AND action_id = 'approve-plan'",
            [project_id],
        )
        .expect("corrupt approval status");

    let error = project_store::load_project_snapshot(&repo_root, project_id)
        .expect_err("malformed snapshot rows should fail closed");
    assert_eq!(error.code, "operator_approval_decode_failed");
}
