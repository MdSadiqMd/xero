use super::support::*;

pub(crate) fn notification_dispatch_claim_flow_is_idempotent_for_pending_operator_approvals() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path) = create_state(&root);
    let app = build_mock_app(state);
    let project_id = "project-notification-loop-1";
    let repo_root = seed_project(
        &root,
        &app,
        project_id,
        "repo-notification-loop-1",
        "repo-notification-loop",
    );

    project_store::upsert_notification_route(
        &repo_root,
        &project_store::NotificationRouteUpsertRecord {
            project_id: project_id.into(),
            route_id: "route-discord".into(),
            route_kind: "discord".into(),
            route_target: "discord:ops-room".into(),
            enabled: true,
            metadata_json: Some("{\"label\":\"ops\"}".into()),
            updated_at: "2026-04-16T20:00:00Z".into(),
        },
    )
    .expect("persist notification route");

    let pending = project_store::upsert_pending_operator_approval(
        &repo_root,
        project_id,
        "session-1",
        Some("flow-1"),
        "terminal_input_required",
        "Terminal input required",
        "Runtime paused and requires a coarse operator answer.",
        "2026-04-16T20:00:01Z",
    )
    .expect("persist pending approval");

    let first = project_store::enqueue_notification_dispatches(
        &repo_root,
        &project_store::NotificationDispatchEnqueueRecord {
            project_id: project_id.into(),
            action_id: pending.action_id.clone(),
            enqueued_at: "2026-04-16T20:00:02Z".into(),
        },
    )
    .expect("enqueue notification dispatch");
    let second = project_store::enqueue_notification_dispatches(
        &repo_root,
        &project_store::NotificationDispatchEnqueueRecord {
            project_id: project_id.into(),
            action_id: pending.action_id.clone(),
            enqueued_at: "2026-04-16T20:00:03Z".into(),
        },
    )
    .expect("re-enqueue notification dispatch");

    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(first[0].id, second[0].id);

    let claim = project_store::claim_notification_reply(
        &repo_root,
        &project_store::NotificationReplyClaimRequestRecord {
            project_id: project_id.into(),
            action_id: pending.action_id.clone(),
            route_id: second[0].route_id.clone(),
            correlation_key: second[0].correlation_key.clone(),
            responder_id: Some("operator-a".into()),
            reply_text: "approved".into(),
            received_at: "2026-04-16T20:00:04Z".into(),
        },
    )
    .expect("first reply claim should succeed");

    assert_eq!(
        claim.dispatch.status,
        project_store::NotificationDispatchStatus::Claimed
    );

    let duplicate = project_store::claim_notification_reply(
        &repo_root,
        &project_store::NotificationReplyClaimRequestRecord {
            project_id: project_id.into(),
            action_id: pending.action_id.clone(),
            route_id: second[0].route_id.clone(),
            correlation_key: second[0].correlation_key.clone(),
            responder_id: Some("operator-b".into()),
            reply_text: "late answer".into(),
            received_at: "2026-04-16T20:00:05Z".into(),
        },
    )
    .expect_err("duplicate claim should be rejected");
    assert_eq!(duplicate.code, "notification_reply_already_claimed");

    let approval = project_store::load_project_snapshot(&repo_root, project_id)
        .expect("load project snapshot after claim flow")
        .snapshot
        .approval_requests
        .into_iter()
        .find(|approval| approval.action_id == pending.action_id)
        .expect("pending approval should still exist");
    assert_eq!(approval.status, OperatorApprovalStatus::Pending);

    let connection = open_state_connection(&repo_root);
    let claim_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM notification_reply_claims WHERE project_id = ?1 AND action_id = ?2",
            params![project_id, pending.action_id.as_str()],
            |row| row.get(0),
        )
        .expect("count reply claim rows");
    assert_eq!(claim_count, 2);
}
