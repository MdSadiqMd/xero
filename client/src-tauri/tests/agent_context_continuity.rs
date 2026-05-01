use std::{fs, path::PathBuf};

use serde_json::json;
use tempfile::TempDir;
use xero_desktop_lib::{
    commands::RuntimeAgentIdDto,
    db::{self, project_store},
    git::repository::CanonicalRepository,
    state::DesktopState,
};

fn seed_project(root: &TempDir) -> (String, PathBuf) {
    let repo_root = root.path().join("repo");
    fs::create_dir_all(&repo_root).expect("create repo root");
    let canonical_root = fs::canonicalize(&repo_root).expect("canonical repo root");
    let project_id = "project-continuity".to_string();
    let repository = CanonicalRepository {
        project_id: project_id.clone(),
        repository_id: "repo-continuity".into(),
        root_path: canonical_root.clone(),
        root_path_string: canonical_root.to_string_lossy().into_owned(),
        common_git_dir: canonical_root.join(".git"),
        display_name: "repo".into(),
        branch_name: Some("main".into()),
        head_sha: Some("abc123".into()),
        branch: None,
        last_commit: None,
        status_entries: Vec::new(),
        has_staged_changes: false,
        has_unstaged_changes: false,
        has_untracked_changes: false,
        additions: 0,
        deletions: 0,
    };

    db::configure_project_database_paths(&root.path().join("app-data").join("xero.db"));
    let state = DesktopState::default();
    db::import_project(&repository, state.import_failpoints()).expect("import project");
    (project_id, canonical_root)
}

fn seed_agent_run(repo_root: &std::path::Path, project_id: &str, run_id: &str) {
    project_store::insert_agent_run(
        repo_root,
        &project_store::NewAgentRunRecord {
            runtime_agent_id: RuntimeAgentIdDto::Debug,
            project_id: project_id.into(),
            agent_session_id: project_store::DEFAULT_AGENT_SESSION_ID.into(),
            run_id: run_id.into(),
            provider_id: "fake_provider".into(),
            model_id: "fake-model".into(),
            prompt: "Debug the continuity handoff.".into(),
            system_prompt: "system".into(),
            now: "2026-05-01T12:00:00Z".into(),
        },
    )
    .expect("seed agent run");
}

#[test]
fn context_policy_settings_are_db_backed_and_handoff_preserves_agent_type() {
    let root = tempfile::tempdir().expect("temp dir");
    let (project_id, repo_root) = seed_project(&root);

    let defaults = project_store::load_agent_context_policy_settings(&repo_root, &project_id, None)
        .expect("load default settings");
    assert_eq!(defaults.compact_threshold_percent, 75);
    assert_eq!(defaults.handoff_threshold_percent, 90);

    let settings = project_store::upsert_agent_context_policy_settings(
        &repo_root,
        &project_store::NewAgentContextPolicySettingsRecord {
            project_id: project_id.clone(),
            scope: project_store::AgentContextPolicySettingsScope::Project,
            agent_session_id: None,
            auto_compact_enabled: true,
            auto_handoff_enabled: true,
            compact_threshold_percent: 70,
            handoff_threshold_percent: 88,
            raw_tail_message_count: 10,
            updated_at: "2026-05-01T12:01:00Z".into(),
        },
    )
    .expect("upsert settings");
    assert_eq!(settings.compact_threshold_percent, 70);
    assert_eq!(settings.raw_tail_message_count, 10);

    let reloaded = project_store::load_agent_context_policy_settings(&repo_root, &project_id, None)
        .expect("reload settings");
    assert_eq!(reloaded.handoff_threshold_percent, 88);

    for runtime_agent_id in [
        RuntimeAgentIdDto::Ask,
        RuntimeAgentIdDto::Engineer,
        RuntimeAgentIdDto::Debug,
    ] {
        let decision =
            project_store::evaluate_agent_context_policy(project_store::AgentContextPolicyInput {
                runtime_agent_id,
                estimated_tokens: 900,
                budget_tokens: Some(1_000),
                provider_supports_compaction: true,
                active_compaction_present: true,
                compaction_current: false,
                settings: reloaded.clone(),
            });
        assert_eq!(
            decision.action,
            project_store::AgentContextPolicyAction::HandoffNow
        );
        assert_eq!(decision.target_runtime_agent_id, Some(runtime_agent_id));
    }
}

#[test]
fn context_manifest_persists_without_provider_call_and_retrieval_logs_round_trip() {
    let root = tempfile::tempdir().expect("temp dir");
    let (project_id, repo_root) = seed_project(&root);

    let manifest = project_store::insert_agent_context_manifest(
        &repo_root,
        &project_store::NewAgentContextManifestRecord {
            manifest_id: "manifest-pre-provider".into(),
            project_id: project_id.clone(),
            agent_session_id: project_store::DEFAULT_AGENT_SESSION_ID.into(),
            run_id: None,
            runtime_agent_id: RuntimeAgentIdDto::Ask,
            provider_id: None,
            model_id: None,
            request_kind: project_store::AgentContextManifestRequestKind::Test,
            policy_action: project_store::AgentContextPolicyAction::ContinueNow,
            policy_reason_code: "schema_test".into(),
            budget_tokens: None,
            estimated_tokens: 42,
            pressure: project_store::AgentContextBudgetPressure::Unknown,
            context_hash: "a".repeat(64),
            included_contributors: vec![project_store::AgentContextManifestContributorRecord {
                contributor_id: "runtime_policy".into(),
                kind: "policy".into(),
                source_id: Some("xero".into()),
                estimated_tokens: 42,
                reason: None,
            }],
            excluded_contributors: Vec::new(),
            retrieval_query_ids: Vec::new(),
            retrieval_result_ids: Vec::new(),
            compaction_id: None,
            handoff_id: None,
            redaction_state: project_store::AgentContextRedactionState::Clean,
            manifest: json!({
                "kind": "pre_provider_context_manifest",
                "contributors": ["runtime_policy"]
            }),
            created_at: "2026-05-01T12:02:00Z".into(),
        },
    )
    .expect("persist manifest without provider call");
    assert!(manifest.run_id.is_none());
    assert_eq!(manifest.included_contributors.len(), 1);

    let reloaded =
        project_store::get_agent_context_manifest(&repo_root, &project_id, "manifest-pre-provider")
            .expect("reload manifest")
            .expect("manifest exists");
    assert_eq!(reloaded.manifest["kind"], "pre_provider_context_manifest");

    let query = project_store::insert_agent_retrieval_query_log(
        &repo_root,
        &project_store::NewAgentRetrievalQueryLogRecord {
            query_id: "retrieval-query-1".into(),
            project_id: project_id.clone(),
            agent_session_id: Some(project_store::DEFAULT_AGENT_SESSION_ID.into()),
            run_id: None,
            runtime_agent_id: RuntimeAgentIdDto::Ask,
            query_text: "recent handoffs".into(),
            search_scope: project_store::AgentRetrievalSearchScope::Handoffs,
            filters: json!({"kind": "agent_handoff"}),
            limit_count: 5,
            status: project_store::AgentRetrievalQueryStatus::Succeeded,
            diagnostic: None,
            created_at: "2026-05-01T12:03:00Z".into(),
            completed_at: Some("2026-05-01T12:03:01Z".into()),
        },
    )
    .expect("persist retrieval query");
    assert_eq!(
        query.query_hash,
        project_store::retrieval_query_hash("recent   handoffs")
    );

    project_store::insert_agent_retrieval_result_log(
        &repo_root,
        &project_store::NewAgentRetrievalResultLogRecord {
            project_id: project_id.clone(),
            query_id: "retrieval-query-1".into(),
            result_id: "retrieval-result-1".into(),
            source_kind: project_store::AgentRetrievalResultSourceKind::ContextManifest,
            source_id: "manifest-pre-provider".into(),
            rank: 1,
            score: Some(1.0),
            snippet: "Pre-provider context manifest was persisted.".into(),
            redaction_state: project_store::AgentContextRedactionState::Clean,
            metadata: Some(json!({"manifestId": "manifest-pre-provider"})),
            created_at: "2026-05-01T12:03:01Z".into(),
        },
    )
    .expect("persist retrieval result");

    let results =
        project_store::list_agent_retrieval_results(&repo_root, &project_id, "retrieval-query-1")
            .expect("list retrieval results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source_id, "manifest-pre-provider");
}

#[test]
fn handoff_lineage_requires_same_type_and_deduplicates_by_idempotency_key() {
    let root = tempfile::tempdir().expect("temp dir");
    let (project_id, repo_root) = seed_project(&root);
    seed_agent_run(&repo_root, &project_id, "run-handoff-source");

    let record = project_store::NewAgentHandoffLineageRecord {
        handoff_id: "handoff-1".into(),
        project_id: project_id.clone(),
        source_agent_session_id: project_store::DEFAULT_AGENT_SESSION_ID.into(),
        source_run_id: "run-handoff-source".into(),
        source_runtime_agent_id: RuntimeAgentIdDto::Debug,
        target_agent_session_id: None,
        target_run_id: None,
        target_runtime_agent_id: RuntimeAgentIdDto::Debug,
        provider_id: "fake_provider".into(),
        model_id: "fake-model".into(),
        source_context_hash: "b".repeat(64),
        status: project_store::AgentHandoffLineageStatus::Pending,
        idempotency_key: "source-run-context-debug".into(),
        handoff_record_id: None,
        bundle: json!({
            "sourceRunId": "run-handoff-source",
            "targetRuntimeAgentId": "debug"
        }),
        diagnostic: None,
        created_at: "2026-05-01T12:04:00Z".into(),
        updated_at: "2026-05-01T12:04:00Z".into(),
        completed_at: None,
    };
    let inserted = project_store::insert_agent_handoff_lineage(&repo_root, &record)
        .expect("insert handoff lineage");
    assert_eq!(inserted.source_runtime_agent_id, RuntimeAgentIdDto::Debug);
    assert_eq!(inserted.target_runtime_agent_id, RuntimeAgentIdDto::Debug);

    let duplicate = project_store::insert_agent_handoff_lineage(
        &repo_root,
        &project_store::NewAgentHandoffLineageRecord {
            handoff_id: "handoff-retry".into(),
            ..record.clone()
        },
    )
    .expect("idempotent retry returns existing handoff");
    assert_eq!(duplicate.handoff_id, "handoff-1");
    assert_eq!(duplicate.id, inserted.id);

    let mismatch = project_store::insert_agent_handoff_lineage(
        &repo_root,
        &project_store::NewAgentHandoffLineageRecord {
            target_runtime_agent_id: RuntimeAgentIdDto::Engineer,
            idempotency_key: "source-run-context-engineer".into(),
            handoff_id: "handoff-invalid".into(),
            ..record
        },
    )
    .expect_err("cross-agent handoff should be rejected");
    assert_eq!(mismatch.code, "agent_handoff_lineage_target_agent_mismatch");
}
