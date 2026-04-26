use std::path::PathBuf;

use cadence_desktop_lib::{
    auth::now_timestamp,
    commands::{
        doctor_report::run_doctor_report, provider_profiles::upsert_provider_profile,
        RunDoctorReportRequestDto, RuntimeAuthPhase, UpsertProviderProfileRequestDto,
    },
    configure_builder_with_state,
    db::{self, project_store},
    git::repository::CanonicalRepository,
    registry::{self, RegistryProjectRecord},
    runtime::{CadenceDiagnosticStatus, CadenceDiagnosticSubject, CadenceDoctorReportMode},
    state::DesktopState,
};
use tauri::Manager;
use tempfile::TempDir;

fn build_mock_app(state: DesktopState) -> tauri::App<tauri::test::MockRuntime> {
    configure_builder_with_state(tauri::test::mock_builder(), state)
        .build(tauri::generate_context!())
        .expect("failed to build mock Tauri app")
}

fn create_state(root: &TempDir) -> DesktopState {
    let app_data = root.path().join("app-data");
    DesktopState::default()
        .with_registry_file_override(app_data.join("project-registry.json"))
        .with_auth_store_file_override(app_data.join("openai-auth.json"))
        .with_provider_profiles_file_override(app_data.join("provider-profiles.json"))
        .with_provider_profile_credential_store_file_override(
            app_data.join("provider-profile-credentials.json"),
        )
        .with_provider_model_catalog_cache_file_override(
            app_data.join("provider-model-catalogs.json"),
        )
        .with_runtime_settings_file_override(app_data.join("runtime-settings.json"))
        .with_mcp_registry_file_override(app_data.join("mcp-registry.json"))
        .with_notification_credential_store_file_override(
            app_data.join("notification-credentials.json"),
        )
        .with_openrouter_credential_file_override(app_data.join("openrouter-credentials.json"))
}

fn seed_openrouter_profile(app: &tauri::App<tauri::test::MockRuntime>) {
    upsert_provider_profile(
        app.handle().clone(),
        app.state::<DesktopState>(),
        UpsertProviderProfileRequestDto {
            profile_id: "openrouter-work".into(),
            provider_id: "openrouter".into(),
            runtime_kind: "openrouter".into(),
            label: "OpenRouter Work".into(),
            model_id: "openai/o4-mini".into(),
            preset_id: Some("openrouter".into()),
            base_url: None,
            api_version: None,
            region: None,
            project_id: None,
            api_key: Some("sk-or-v1-do-not-leak".into()),
            activate: true,
        },
    )
    .expect("seed openrouter profile");
}

fn seed_project(root: &TempDir, app: &tauri::App<tauri::test::MockRuntime>) -> (String, PathBuf) {
    let repo_root = root.path().join("repo");
    std::fs::create_dir_all(&repo_root).expect("create repo root");
    let canonical_root = std::fs::canonicalize(&repo_root).expect("canonical repo root");
    let root_path_string = canonical_root.to_string_lossy().into_owned();

    let repository = CanonicalRepository {
        project_id: "project-1".into(),
        repository_id: "repo-1".into(),
        root_path: canonical_root.clone(),
        root_path_string: root_path_string.clone(),
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

    db::import_project(&repository, app.state::<DesktopState>().import_failpoints())
        .expect("import project into repo-local db");

    let registry_path = app
        .state::<DesktopState>()
        .registry_file(&app.handle().clone())
        .expect("registry path");
    registry::replace_projects(
        &registry_path,
        vec![RegistryProjectRecord {
            project_id: repository.project_id.clone(),
            repository_id: repository.repository_id.clone(),
            root_path: root_path_string,
        }],
    )
    .expect("persist registry entry");

    (repository.project_id, canonical_root)
}

#[test]
fn run_doctor_report_returns_quick_local_contract_with_redacted_dependencies() {
    let root = tempfile::tempdir().expect("temp dir");
    let app = build_mock_app(create_state(&root));
    seed_openrouter_profile(&app);

    let mcp_registry_path = app
        .state::<DesktopState>()
        .mcp_registry_file(&app.handle().clone())
        .expect("mcp registry path");
    std::fs::create_dir_all(mcp_registry_path.parent().expect("mcp parent"))
        .expect("create mcp parent");
    std::fs::write(
        &mcp_registry_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "version": 1,
            "updatedAt": "2026-04-26T12:00:00Z",
            "servers": [{
                "id": "linear",
                "name": "Linear",
                "transport": { "kind": "stdio", "command": "linear-mcp", "args": [] },
                "connection": {
                    "status": "failed",
                    "diagnostic": {
                        "code": "mcp_server_secret_probe_failed",
                        "message": "Server launch failed with token sk-mcp-do-not-leak at /Users/sn0w/.config/linear",
                        "retryable": true
                    }
                },
                "updatedAt": "2026-04-26T12:00:00Z"
            }]
        }))
        .expect("serialize mcp registry"),
    )
    .expect("write mcp registry");

    let report = run_doctor_report(
        app.handle().clone(),
        app.state::<DesktopState>(),
        RunDoctorReportRequestDto {
            mode: Some(CadenceDoctorReportMode::QuickLocal),
        },
    )
    .expect("doctor report");

    assert_eq!(report.mode, CadenceDoctorReportMode::QuickLocal);
    assert_eq!(
        report.summary.total as usize,
        report.profile_checks.len()
            + report.model_catalog_checks.len()
            + report.runtime_supervisor_checks.len()
            + report.mcp_dependency_checks.len()
            + report.settings_dependency_checks.len()
    );
    assert!(report
        .model_catalog_checks
        .iter()
        .any(|check| check.status == CadenceDiagnosticStatus::Skipped
            && check.code == "provider_model_catalog_network_skipped"));
    assert!(report
        .mcp_dependency_checks
        .iter()
        .any(|check| check.status == CadenceDiagnosticStatus::Failed
            && check.code == "mcp_server_secret_probe_failed"));
    assert!(report
        .profile_checks
        .iter()
        .any(|check| check.code == "provider_profile_ready"));

    let serialized = serde_json::to_string(&report).expect("serialize report");
    assert!(!serialized.contains("sk-or-v1-do-not-leak"));
    assert!(!serialized.contains("sk-mcp-do-not-leak"));
    assert!(!serialized.contains(root.path().to_string_lossy().as_ref()));
    assert!(serialized.contains("[redacted"));
}

#[test]
fn run_doctor_report_threads_runtime_session_failures_into_runtime_checks() {
    let root = tempfile::tempdir().expect("temp dir");
    let app = build_mock_app(create_state(&root));
    seed_openrouter_profile(&app);
    let (project_id, repo_root) = seed_project(&root, &app);

    project_store::upsert_runtime_session(
        &repo_root,
        &project_store::RuntimeSessionRecord {
            project_id: project_id.clone(),
            runtime_kind: "openrouter".into(),
            provider_id: "openrouter".into(),
            flow_id: None,
            session_id: None,
            account_id: None,
            auth_phase: RuntimeAuthPhase::Failed,
            last_error: Some(project_store::RuntimeSessionDiagnosticRecord {
                code: "provider_profile_credentials_missing".into(),
                message: "Selected OpenRouter profile is missing credentials for runtime startup."
                    .into(),
                retryable: false,
            }),
            updated_at: now_timestamp(),
        },
    )
    .expect("seed failed runtime session");

    let report = run_doctor_report(
        app.handle().clone(),
        app.state::<DesktopState>(),
        RunDoctorReportRequestDto { mode: None },
    )
    .expect("doctor report");

    let runtime_failure = report
        .runtime_supervisor_checks
        .iter()
        .find(|check| {
            check.subject == CadenceDiagnosticSubject::RuntimeBinding
                && check.code == "provider_profile_credentials_missing"
        })
        .expect("runtime binding failure check");

    assert_eq!(runtime_failure.status, CadenceDiagnosticStatus::Failed);
    assert_eq!(
        runtime_failure.affected_provider_id.as_deref(),
        Some("openrouter")
    );
    assert!(runtime_failure
        .remediation
        .as_deref()
        .is_some_and(|value| value.contains("Providers settings")));
}
