use super::support::*;

pub(crate) fn skill_lifecycle_payload_contracts_fail_closed_on_unknown_stage_or_extra_fields() {
    assert!(
        serde_json::from_value::<AutonomousArtifactPayloadDto>(json!({
            "kind": "skill_lifecycle",
            "projectId": "project-1",
            "runId": "run-1",
            "unitId": "run-1:unit:researcher",
            "attemptId": "run-1:unit:researcher:attempt:1",
            "artifactId": "artifact-skill-lifecycle",
            "stage": "discover",
            "result": "succeeded",
            "skillId": "find-skills",
            "source": {
                "repo": "vercel-labs/skills",
                "path": "skills/find-skills",
                "reference": "main",
                "treeHash": "0123456789abcdef0123456789abcdef01234567"
            },
            "cache": {
                "key": "find-skills-576b45048241"
            }
        }))
        .is_err(),
        "skill lifecycle payload should reject unknown stage enums"
    );

    assert!(
        serde_json::from_value::<AutonomousArtifactPayloadDto>(json!({
            "kind": "skill_lifecycle",
            "projectId": "project-1",
            "runId": "run-1",
            "unitId": "run-1:unit:researcher",
            "attemptId": "run-1:unit:researcher:attempt:1",
            "artifactId": "artifact-skill-lifecycle",
            "stage": "invoke",
            "result": "failed",
            "skillId": "find-skills",
            "source": {
                "repo": "vercel-labs/skills",
                "path": "skills/find-skills",
                "reference": "main",
                "treeHash": "0123456789abcdef0123456789abcdef01234567",
                "unexpected": true
            },
            "cache": {
                "key": "find-skills-576b45048241",
                "status": "hit"
            },
            "diagnostic": {
                "code": "autonomous_skill_cache_read_failed",
                "message": "Cadence could not read the autonomous skill cache manifest.",
                "retryable": true
            }
        }))
        .is_err(),
        "skill lifecycle payload should reject unknown nested source fields"
    );
}

pub(crate) fn malformed_inputs_fail_fast_before_runtime_logic() {
    assert!(serde_json::from_value::<ImportRepositoryRequestDto>(json!({})).is_err());
    assert!(
        serde_json::from_value::<ProjectIdRequestDto>(json!({ "projectID": "project-1" })).is_err()
    );
    assert!(
        serde_json::from_value::<GetRuntimeRunRequestDto>(json!({
            "projectId": "project-1",
            "unexpected": true
        }))
        .is_err(),
        "get runtime run request should reject unknown fields"
    );
    assert!(
        serde_json::from_value::<StopRuntimeRunRequestDto>(json!({
            "projectId": "project-1",
            "runID": "run-1"
        }))
        .is_err(),
        "stop runtime run request should require camelCase runId"
    );
    assert!(serde_json::from_value::<RepositoryDiffRequestDto>(json!({
        "projectId": "project-1",
        "scope": "UNSTAGED"
    }))
    .is_err());
    assert!(
        serde_json::from_value::<ResolveOperatorActionRequestDto>(json!({
            "projectId": "project-1",
            "actionId": "session:session-1:review_worktree",
            "decision": "approve",
            "decisionNote": "legacy-field-should-be-rejected"
        }))
        .is_err(),
        "resolve request should reject legacy decisionNote payloads"
    );
    assert!(
        serde_json::from_value::<ResumeOperatorRunRequestDto>(json!({
            "projectId": "project-1",
            "actionId": "session:session-1:review_worktree",
            "decisionNote": "unexpected"
        }))
        .is_err(),
        "resume request should reject unknown fields"
    );
    assert!(
        serde_json::from_value::<ListNotificationRoutesRequestDto>(json!({
            "projectId": "project-1",
            "unexpected": true
        }))
        .is_err(),
        "list notification routes request should reject unknown fields"
    );
    assert!(
        serde_json::from_value::<UpsertNotificationRouteRequestDto>(json!({
            "projectId": "project-1",
            "routeId": "route-discord",
            "routeKind": "discord",
            "routeTarget": "123456789012345678",
            "enabled": true,
            "metadataJson": "{\"channel\":\"ops\"}",
            "updatedAt": "2026-04-16T20:06:33Z",
            "unexpected": true
        }))
        .is_err(),
        "upsert notification route request should reject unknown fields"
    );
    assert!(
        serde_json::from_value::<UpsertNotificationRouteRequestDto>(json!({
            "projectId": "project-1",
            "routeId": "route-discord",
            "routeKind": 7,
            "routeTarget": "123456789012345678",
            "enabled": true,
            "metadataJson": "{\"channel\":\"ops\"}",
            "updatedAt": "2026-04-16T20:06:33Z"
        }))
        .is_err(),
        "upsert notification route request should reject non-string routeKind payloads"
    );
    assert!(
        serde_json::from_value::<UpsertNotificationRouteCredentialsRequestDto>(json!({
            "projectId": "project-1",
            "routeId": "route-discord",
            "routeKind": "discord",
            "credentials": {
                "botToken": "discord-bot-token",
                "webhookUrl": "https://discord.com/api/webhooks/1/2",
                "unexpected": true
            },
            "updatedAt": "2026-04-16T20:06:33Z"
        }))
        .is_err(),
        "upsert notification route credentials request should reject unknown credential fields"
    );
    assert!(
        serde_json::from_value::<UpsertNotificationRouteCredentialsRequestDto>(json!({
            "projectId": "project-1",
            "routeId": "route-discord",
            "routeKind": "discord",
            "credentials": {
                "botToken": 7,
                "webhookUrl": "https://discord.com/api/webhooks/1/2"
            },
            "updatedAt": "2026-04-16T20:06:33Z"
        }))
        .is_err(),
        "upsert notification route credentials request should reject non-string credential payloads"
    );
    assert!(
        serde_json::from_value::<UpsertNotificationRouteCredentialsResponseDto>(json!({
            "projectId": "project-1",
            "routeId": "route-discord",
            "routeKind": "email",
            "credentialScope": "app_local",
            "hasBotToken": true,
            "hasChatId": false,
            "hasWebhookUrl": true,
            "updatedAt": "2026-04-16T20:06:33Z"
        }))
        .is_err(),
        "upsert notification route credentials response should reject unsupported route-kind enums"
    );
    assert!(
        serde_json::from_value::<ListNotificationDispatchesRequestDto>(json!({
            "projectId": "project-1",
            "actionId": "session:session-1:review_worktree",
            "unexpected": true
        }))
        .is_err(),
        "list notification dispatches request should reject unknown fields"
    );
    assert!(
        serde_json::from_value::<SyncNotificationAdaptersRequestDto>(json!({
            "projectId": "project-1",
            "unexpected": true
        }))
        .is_err(),
        "sync notification adapters request should reject unknown fields"
    );
    assert!(
        serde_json::from_value::<SyncNotificationAdaptersResponseDto>(json!({
            "projectId": "project-1",
            "dispatch": {
                "projectId": "project-1",
                "pendingCount": 1,
                "attemptedCount": 1,
                "sentCount": 0,
                "failedCount": 1,
                "attemptLimit": 64,
                "attemptsTruncated": false,
                "attempts": [
                    {
                        "dispatchId": 1,
                        "actionId": "session:session-1:review_worktree",
                        "routeId": "route-discord",
                        "routeKind": "discord",
                        "outcomeStatus": "queued",
                        "diagnosticCode": "notification_adapter_dispatch_failed",
                        "diagnosticMessage": "failed",
                        "durableErrorCode": "notification_adapter_transport_failed",
                        "durableErrorMessage": "transport error"
                    }
                ],
                "errorCodeCounts": []
            },
            "replies": {
                "projectId": "project-1",
                "routeCount": 1,
                "polledRouteCount": 1,
                "messageCount": 0,
                "acceptedCount": 0,
                "rejectedCount": 1,
                "attemptLimit": 256,
                "attemptsTruncated": false,
                "attempts": [],
                "errorCodeCounts": []
            },
            "syncedAt": "2026-04-16T20:06:35Z"
        }))
        .is_err(),
        "sync notification adapters response should reject unsupported dispatch status enums"
    );
    assert!(
        serde_json::from_value::<RecordNotificationDispatchOutcomeRequestDto>(json!({
            "projectId": "project-1",
            "actionId": "session:session-1:review_worktree",
            "routeId": "route-discord",
            "status": "pending",
            "attemptedAt": "2026-04-16T20:06:33Z",
            "errorCode": null,
            "errorMessage": null
        }))
        .is_err(),
        "record notification dispatch outcome request should reject unsupported status enums"
    );
    assert!(
        serde_json::from_value::<SubmitNotificationReplyRequestDto>(json!({
            "projectId": "project-1",
            "actionId": "session:session-1:review_worktree",
            "routeId": "route-discord",
            "correlationKey": "nfy:11111111111111111111111111111111",
            "responderId": "operator-a",
            "replyText": "Looks good",
            "decision": "approve",
            "receivedAt": "2026-04-16T20:06:34Z",
            "unexpected": true
        }))
        .is_err(),
        "submit notification reply request should reject unknown fields"
    );
    assert!(
        serde_json::from_value::<ListNotificationRoutesResponseDto>(json!({
            "routes": [
                {
                    "projectId": "project-1",
                    "routeId": "route-discord",
                    "routeKind": "email",
                    "routeTarget": "123456789012345678",
                    "enabled": true,
                    "metadataJson": "{\"channel\":\"ops\"}",
                    "createdAt": "2026-04-16T20:05:30Z",
                    "updatedAt": "2026-04-16T20:06:33Z"
                }
            ]
        }))
        .is_err(),
        "list notification routes response should reject unsupported route kind enums"
    );

    assert!(
        serde_json::from_value::<UpsertWorkflowGraphRequestDto>(json!({
            "projectId": "project-1",
            "nodes": [
                {
                    "nodeId": "plan",
                    "phaseId": 1,
                    "sortOrder": 1,
                    "name": "Plan",
                    "description": "Plan phase",
                    "status": "active",
                    "currentStep": "plan",
                    "taskCount": 1,
                    "completedTasks": 0,
                    "summary": null,
                    "extra": true
                }
            ],
            "edges": [],
            "gates": []
        }))
        .is_err(),
        "upsert request should reject unknown node fields"
    );

    assert!(
        serde_json::from_value::<ApplyWorkflowTransitionRequestDto>(json!({
            "projectId": "project-1",
            "transitionId": "txn-1",
            "causalTransitionId": null,
            "fromNodeId": "plan",
            "toNodeId": "execute",
            "transitionKind": "advance",
            "gateDecision": "approved",
            "gateDecisionContext": null,
            "gateUpdates": [
                {
                    "gateKey": "execution_gate",
                    "gateState": "satisfied",
                    "decisionContext": null,
                    "unexpected": true
                }
            ],
            "occurredAt": "2026-04-13T20:01:00Z"
        }))
        .is_err(),
        "transition request should reject unknown gate-update fields"
    );

    assert!(
        serde_json::from_value::<ApplyWorkflowTransitionRequestDto>(json!({
            "projectId": "project-1",
            "transitionId": "txn-1",
            "causalTransitionId": null,
            "fromNodeId": "plan",
            "toNodeId": "execute",
            "transitionKind": "advance",
            "gateDecision": 7,
            "gateDecisionContext": null,
            "gateUpdates": [],
            "occurredAt": "2026-04-13T20:01:00Z"
        }))
        .is_err(),
        "transition request should reject non-string gateDecision payloads"
    );

    let mut snapshot_with_unknown_lifecycle_stage =
        serde_json::to_value(sample_snapshot()).expect("serialize snapshot fixture");
    snapshot_with_unknown_lifecycle_stage["lifecycle"]["stages"][0]["stage"] = json!("discovery");
    assert!(
        serde_json::from_value::<ProjectSnapshotResponseDto>(snapshot_with_unknown_lifecycle_stage)
            .is_err(),
        "snapshot payload should reject unknown lifecycle stage enums"
    );

    let mut snapshot_with_unknown_lifecycle_field =
        serde_json::to_value(sample_snapshot()).expect("serialize snapshot fixture");
    snapshot_with_unknown_lifecycle_field["lifecycle"]["stages"][0]["unexpected"] = json!(true);
    assert!(
        serde_json::from_value::<ProjectSnapshotResponseDto>(snapshot_with_unknown_lifecycle_field)
            .is_err(),
        "snapshot payload should reject unknown lifecycle stage fields"
    );

    let mut snapshot_with_unknown_handoff_field =
        serde_json::to_value(sample_snapshot()).expect("serialize snapshot fixture");
    snapshot_with_unknown_handoff_field["handoffPackages"][0]["unexpected"] = json!(true);
    assert!(
        serde_json::from_value::<ProjectSnapshotResponseDto>(snapshot_with_unknown_handoff_field)
            .is_err(),
        "snapshot payload should reject unknown handoff package fields"
    );

    let mut snapshot_with_malformed_handoff_hash =
        serde_json::to_value(sample_snapshot()).expect("serialize snapshot fixture");
    snapshot_with_malformed_handoff_hash["handoffPackages"][0]["packageHash"] = json!(7);
    assert!(
        serde_json::from_value::<ProjectSnapshotResponseDto>(snapshot_with_malformed_handoff_hash)
            .is_err(),
        "snapshot payload should reject malformed handoff package hashes"
    );

    let mut transition_response_with_unknown_dispatch_status =
        serde_json::to_value(ApplyWorkflowTransitionResponseDto {
            transition_event: sample_transition_event(),
            automatic_dispatch: sample_automatic_dispatch_outcome(),
            phases: Vec::new(),
        })
        .expect("serialize apply transition response fixture");
    transition_response_with_unknown_dispatch_status["automaticDispatch"]["status"] =
        json!("continued");
    assert!(
        serde_json::from_value::<ApplyWorkflowTransitionResponseDto>(
            transition_response_with_unknown_dispatch_status,
        )
        .is_err(),
        "transition response should reject unknown automatic-dispatch status values"
    );

    let mut transition_response_with_malformed_skipped_dispatch =
        serde_json::to_value(ApplyWorkflowTransitionResponseDto {
            transition_event: sample_transition_event(),
            automatic_dispatch: sample_skipped_automatic_dispatch_outcome(),
            phases: Vec::new(),
        })
        .expect("serialize skipped apply transition response fixture");
    transition_response_with_malformed_skipped_dispatch["automaticDispatch"]["code"] = json!(7);
    assert!(
        serde_json::from_value::<ApplyWorkflowTransitionResponseDto>(
            transition_response_with_malformed_skipped_dispatch,
        )
        .is_err(),
        "transition response should reject malformed skipped automatic-dispatch diagnostics"
    );

    let mut resume_response_with_malformed_handoff_package =
        serde_json::to_value(ResumeOperatorRunResponseDto {
            approval_request: sample_snapshot()
                .approval_requests
                .into_iter()
                .next()
                .expect("sample approval exists"),
            resume_entry: sample_snapshot()
                .resume_history
                .into_iter()
                .next()
                .expect("sample resume entry exists"),
            automatic_dispatch: Some(sample_automatic_dispatch_outcome()),
        })
        .expect("serialize resume response fixture");
    resume_response_with_malformed_handoff_package["automaticDispatch"]["handoffPackage"]
        ["package"]["handoffTransitionId"] = json!(null);
    assert!(
        serde_json::from_value::<ResumeOperatorRunResponseDto>(
            resume_response_with_malformed_handoff_package,
        )
        .is_err(),
        "resume response should reject malformed automatic-dispatch handoff package payloads"
    );

    let malformed_runtime_run = json!({
        "projectId": "project-1",
        "runId": "run-1",
        "runtimeKind": "openai_codex",
        "supervisorKind": "detached_pty",
        "status": "awaiting_operator",
        "transport": {
            "kind": "tcp",
            "endpoint": "127.0.0.1:45123",
            "liveness": "reachable"
        },
        "startedAt": "2026-04-15T23:10:00Z",
        "lastHeartbeatAt": "2026-04-15T23:10:01Z",
        "lastCheckpointSequence": 2,
        "lastCheckpointAt": "2026-04-15T23:10:02Z",
        "stoppedAt": null,
        "lastErrorCode": null,
        "lastError": null,
        "updatedAt": "2026-04-15T23:10:02Z",
        "checkpoints": []
    });
    assert!(
        serde_json::from_value::<RuntimeRunDto>(malformed_runtime_run).is_err(),
        "runtime run payload should reject unknown status enums"
    );

    let malformed_runtime_run_event = json!({
        "projectId": "project-1",
        "run": {
            "projectId": "project-1",
            "runId": "run-1",
            "runtimeKind": "openai_codex",
            "supervisorKind": "detached_pty",
            "status": "running",
            "transport": {
                "kind": "tcp",
                "endpoint": "127.0.0.1:45123",
                "liveness": "reachable"
            },
            "startedAt": "2026-04-15T23:10:00Z",
            "lastHeartbeatAt": "2026-04-15T23:10:01Z",
            "lastCheckpointSequence": 2,
            "lastCheckpointAt": "2026-04-15T23:10:02Z",
            "stoppedAt": null,
            "lastErrorCode": null,
            "lastError": null,
            "updatedAt": "2026-04-15T23:10:02Z",
            "checkpoints": [],
            "unexpected": true
        }
    });
    assert!(
        serde_json::from_value::<RuntimeRunUpdatedPayloadDto>(malformed_runtime_run_event).is_err(),
        "runtime run updated payload should reject malformed nested run payloads"
    );
}
