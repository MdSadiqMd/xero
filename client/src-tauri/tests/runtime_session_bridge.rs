use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use cadence_desktop_lib::{
    auth::{
        now_timestamp, persist_openai_codex_session, remove_openai_codex_session,
        OpenAiCodexAuthConfig, StoredOpenAiCodexSession,
    },
    commands::{
        get_runtime_session::get_runtime_session, logout_runtime_session::logout_runtime_session,
        start_openai_login::start_openai_login, start_runtime_session::start_runtime_session,
        ProjectIdRequestDto, RuntimeAuthPhase, StartOpenAiLoginRequestDto,
    },
    configure_builder_with_state,
    db::{self, database_path_for_repo, project_store},
    git::repository::CanonicalRepository,
    registry::{self, RegistryProjectRecord},
    runtime::openai_codex_provider,
    state::DesktopState,
};
use serde_json::json;
use tauri::Manager;
use tempfile::TempDir;

fn build_mock_app(state: DesktopState) -> tauri::App<tauri::test::MockRuntime> {
    configure_builder_with_state(tauri::test::mock_builder(), state)
        .build(tauri::generate_context!())
        .expect("failed to build mock Tauri app")
}

fn create_state(root: &TempDir) -> (DesktopState, PathBuf, PathBuf) {
    let registry_path = root.path().join("app-data").join("project-registry.json");
    let auth_store_path = root.path().join("app-data").join("openai-auth.json");
    (
        DesktopState::default()
            .with_registry_file_override(registry_path.clone())
            .with_auth_store_file_override(auth_store_path.clone()),
        registry_path,
        auth_store_path,
    )
}

fn jwt_with_account_id(account_id: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(
        json!({
            "https://api.openai.com/auth": {
                "chatgpt_account_id": account_id,
            }
        })
        .to_string(),
    );
    format!("{header}.{payload}.")
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
        status_entries: Vec::new(),
        has_staged_changes: false,
        has_unstaged_changes: false,
        has_untracked_changes: false,
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

fn persist_auth_session(
    auth_store_path: &Path,
    session_id: &str,
    account_id: &str,
    expires_at: i64,
    updated_at: &str,
) {
    persist_openai_codex_session(
        auth_store_path,
        StoredOpenAiCodexSession {
            provider_id: "openai_codex".into(),
            session_id: session_id.into(),
            account_id: account_id.into(),
            access_token: jwt_with_account_id(account_id),
            refresh_token: format!("refresh-{account_id}"),
            expires_at,
            updated_at: updated_at.into(),
        },
    )
    .expect("persist auth session");
}

fn seed_runtime_session_record(
    repo_root: &Path,
    project_id: &str,
    account_id: Option<&str>,
    session_id: Option<&str>,
    phase: RuntimeAuthPhase,
) {
    let provider = openai_codex_provider();
    project_store::upsert_runtime_session(
        repo_root,
        &project_store::RuntimeSessionRecord {
            project_id: project_id.into(),
            runtime_kind: provider.runtime_kind.into(),
            provider_id: provider.provider_id.into(),
            flow_id: None,
            session_id: session_id.map(str::to_owned),
            account_id: account_id.map(str::to_owned),
            auth_phase: phase,
            last_error: None,
            updated_at: now_timestamp(),
        },
    )
    .expect("seed runtime session record");
}

fn current_unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_secs() as i64
}

fn auth_config_with_token_url(token_url: String) -> OpenAiCodexAuthConfig {
    let mut config = OpenAiCodexAuthConfig::default();
    config.token_url = token_url;
    config.callback_port = 0;
    config.originator = "cadence-tests".into();
    config.timeout = Duration::from_secs(5);
    config
}

fn spawn_static_http_server(status: u16, body: &str) -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test http server");
    let address = listener.local_addr().expect("test http server addr");
    let body = body.to_owned();

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept test http request");
        let mut reader = BufReader::new(stream.try_clone().expect("clone tcp stream"));
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = reader.read_line(&mut line).expect("read request line");
            if bytes == 0 || line == "\r\n" {
                break;
            }
        }

        write!(
            stream,
            "HTTP/1.1 {status} Test\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body,
        )
        .expect("write test http response");
    });

    format!("http://{address}")
}

#[test]
fn start_runtime_session_binds_latest_app_local_auth_without_tokens_in_repo_db() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, repo_root) = seed_project(&root, &app);

    persist_auth_session(
        &auth_store_path,
        "session-auth",
        "acct-1",
        current_unix_timestamp() + Duration::from_secs(3600).as_secs() as i64,
        "2026-04-13T14:11:59Z",
    );

    let runtime = start_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("start runtime session");

    assert_eq!(runtime.phase, RuntimeAuthPhase::Authenticated);
    assert_eq!(runtime.account_id.as_deref(), Some("acct-1"));
    assert_eq!(runtime.session_id.as_deref(), Some("session-auth"));
    assert!(runtime.last_error.is_none());

    let status = get_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("get runtime session");
    assert_eq!(status.phase, RuntimeAuthPhase::Authenticated);
    assert_eq!(status.account_id.as_deref(), Some("acct-1"));

    let database_path = database_path_for_repo(&repo_root);
    let database_bytes = std::fs::read(&database_path).expect("read runtime db bytes");
    let database_text = String::from_utf8_lossy(&database_bytes);
    assert!(!database_text.contains("refresh-acct-1"));
    assert!(!database_text.contains("chatgpt_account_id"));
}

#[test]
fn start_runtime_session_binds_explicit_account_instead_of_latest_session() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, repo_root) = seed_project(&root, &app);

    persist_auth_session(
        &auth_store_path,
        "session-explicit",
        "acct-explicit",
        current_unix_timestamp() + Duration::from_secs(3600).as_secs() as i64,
        "2026-04-13T14:11:58Z",
    );
    persist_auth_session(
        &auth_store_path,
        "session-latest",
        "acct-latest",
        current_unix_timestamp() + Duration::from_secs(3600).as_secs() as i64,
        "2026-04-13T14:11:59Z",
    );

    seed_runtime_session_record(
        &repo_root,
        &project_id,
        Some("acct-explicit"),
        None,
        RuntimeAuthPhase::Idle,
    );

    let runtime = start_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("start runtime session with explicit account");

    assert_eq!(runtime.phase, RuntimeAuthPhase::Authenticated);
    assert_eq!(runtime.account_id.as_deref(), Some("acct-explicit"));
    assert_eq!(runtime.session_id.as_deref(), Some("session-explicit"));
}

#[test]
fn start_runtime_session_returns_signed_out_state_when_no_auth_store_entry_exists() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, _auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, _repo_root) = seed_project(&root, &app);

    let runtime = start_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("start runtime session should return signed-out state");

    assert_eq!(runtime.phase, RuntimeAuthPhase::Idle);
    assert_eq!(
        runtime.last_error_code.as_deref(),
        Some("auth_session_not_found")
    );
    assert!(runtime.session_id.is_none());
}

#[test]
fn start_runtime_session_returns_idle_diagnostic_when_auth_store_is_unreadable() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, _repo_root) = seed_project(&root, &app);

    std::fs::create_dir_all(&auth_store_path).expect("create unreadable auth-store directory");

    let runtime = start_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("start runtime session with unreadable auth store");

    assert_eq!(runtime.phase, RuntimeAuthPhase::Idle);
    assert_eq!(
        runtime.last_error_code.as_deref(),
        Some("auth_store_read_failed")
    );
    assert!(runtime.session_id.is_none());
}

#[test]
fn start_runtime_session_preserves_retryable_refresh_state_when_refresh_fails() {
    let token_base_url = spawn_static_http_server(500, "boom");
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let state = state.with_openai_auth_config_override(auth_config_with_token_url(format!(
        "{token_base_url}/oauth/token"
    )));
    let app = build_mock_app(state);
    let (project_id, _repo_root) = seed_project(&root, &app);

    persist_auth_session(
        &auth_store_path,
        "session-refresh",
        "acct-refresh",
        current_unix_timestamp() - Duration::from_secs(60).as_secs() as i64,
        "2026-04-13T14:11:59Z",
    );
    let before = std::fs::read_to_string(&auth_store_path).expect("seed contents");

    let runtime = start_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("start runtime session should surface retryable refresh failure");

    assert_eq!(runtime.phase, RuntimeAuthPhase::Refreshing);
    assert_eq!(
        runtime.last_error_code.as_deref(),
        Some("token_refresh_server_error")
    );
    assert_eq!(runtime.account_id.as_deref(), Some("acct-refresh"));
    assert!(runtime.session_id.is_none());

    let after = std::fs::read_to_string(&auth_store_path).expect("post-refresh contents");
    assert_eq!(
        before, after,
        "failed refresh should not rewrite stored tokens"
    );
}

#[test]
fn get_runtime_session_returns_idle_when_bound_auth_row_disappears() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, _repo_root) = seed_project(&root, &app);

    persist_auth_session(
        &auth_store_path,
        "session-auth",
        "acct-1",
        current_unix_timestamp() + Duration::from_secs(3600).as_secs() as i64,
        "2026-04-13T14:11:59Z",
    );

    start_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("seed runtime session");

    remove_openai_codex_session(&auth_store_path, "acct-1").expect("remove auth session");

    let runtime = get_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("reconcile missing auth row");

    assert_eq!(runtime.phase, RuntimeAuthPhase::Idle);
    assert_eq!(
        runtime.last_error_code.as_deref(),
        Some("auth_session_not_found")
    );
    assert!(runtime.session_id.is_none());
}

#[test]
fn get_runtime_session_returns_idle_when_authenticated_row_is_missing_account_id() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, repo_root) = seed_project(&root, &app);

    persist_auth_session(
        &auth_store_path,
        "session-auth",
        "acct-1",
        current_unix_timestamp() + Duration::from_secs(3600).as_secs() as i64,
        "2026-04-13T14:11:59Z",
    );

    start_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("seed runtime session");

    let database_path = database_path_for_repo(&repo_root);
    let connection = rusqlite::Connection::open(&database_path).expect("open runtime db");
    connection
        .execute(
            "UPDATE runtime_sessions SET account_id = NULL WHERE project_id = ?1",
            [&project_id],
        )
        .expect("clear runtime account id");

    let runtime = get_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("reconcile missing runtime account id");

    assert_eq!(runtime.phase, RuntimeAuthPhase::Idle);
    assert_eq!(
        runtime.last_error_code.as_deref(),
        Some("runtime_account_missing")
    );
    assert!(runtime.session_id.is_none());
}

#[test]
fn get_runtime_session_returns_idle_when_authenticated_binding_is_stale() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, _repo_root) = seed_project(&root, &app);

    persist_auth_session(
        &auth_store_path,
        "session-auth",
        "acct-1",
        current_unix_timestamp() + Duration::from_secs(3600).as_secs() as i64,
        "2026-04-13T14:11:59Z",
    );

    start_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("seed runtime session");

    persist_auth_session(
        &auth_store_path,
        "session-new",
        "acct-1",
        current_unix_timestamp() + Duration::from_secs(3600).as_secs() as i64,
        "2026-04-13T14:12:59Z",
    );

    let runtime = get_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("reconcile stale runtime binding");

    assert_eq!(runtime.phase, RuntimeAuthPhase::Idle);
    assert_eq!(
        runtime.last_error_code.as_deref(),
        Some("auth_session_stale")
    );
    assert!(runtime.session_id.is_none());
}

#[test]
fn get_runtime_session_returns_failed_when_transient_flow_snapshot_is_missing_after_reload() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, registry_path, auth_store_path) = create_state(&root);
    let state = state.with_openai_auth_config_override(auth_config_with_token_url(
        "http://127.0.0.1:9/oauth/token".into(),
    ));
    let app = build_mock_app(state);
    let (project_id, _repo_root) = seed_project(&root, &app);

    let started = start_openai_login(
        app.handle().clone(),
        app.state::<DesktopState>(),
        StartOpenAiLoginRequestDto {
            project_id: project_id.clone(),
            originator: Some("cadence-tests".into()),
        },
    )
    .expect("start login flow");
    assert!(started.flow_id.is_some());

    let reloaded = build_mock_app(
        DesktopState::default()
            .with_registry_file_override(registry_path)
            .with_auth_store_file_override(auth_store_path)
            .with_openai_auth_config_override(auth_config_with_token_url(
                "http://127.0.0.1:9/oauth/token".into(),
            )),
    );

    let runtime = get_runtime_session(
        reloaded.handle().clone(),
        reloaded.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("reconcile missing in-memory flow");

    assert_eq!(runtime.phase, RuntimeAuthPhase::Failed);
    assert_eq!(
        runtime.last_error_code.as_deref(),
        Some("auth_flow_unavailable")
    );
    assert!(runtime.flow_id.is_none());
}

#[test]
fn logout_runtime_session_succeeds_when_backing_auth_row_is_already_gone() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, _repo_root) = seed_project(&root, &app);

    persist_auth_session(
        &auth_store_path,
        "session-auth",
        "acct-1",
        current_unix_timestamp() + Duration::from_secs(3600).as_secs() as i64,
        "2026-04-13T14:11:59Z",
    );

    start_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("seed runtime session");

    remove_openai_codex_session(&auth_store_path, "acct-1").expect("remove auth session");

    let runtime = logout_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("logout runtime session");

    assert_eq!(runtime.phase, RuntimeAuthPhase::Idle);
    assert_eq!(runtime.account_id.as_deref(), Some("acct-1"));
    assert!(runtime.session_id.is_none());
    assert!(runtime.last_error.is_none());
}

#[test]
fn start_runtime_session_rejects_empty_project_id() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, _auth_store_path) = create_state(&root);
    let app = build_mock_app(state);

    let error = start_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: "   ".into(),
        },
    )
    .expect_err("empty project id should be rejected");

    assert_eq!(error.code, "invalid_request");
}

#[test]
fn corrupted_runtime_rows_fail_with_typed_decode_errors() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, repo_root) = seed_project(&root, &app);

    persist_auth_session(
        &auth_store_path,
        "session-auth",
        "acct-1",
        current_unix_timestamp() + Duration::from_secs(3600).as_secs() as i64,
        "2026-04-13T14:11:59Z",
    );

    start_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("seed runtime session");

    let database_path = database_path_for_repo(&repo_root);
    let connection = rusqlite::Connection::open(&database_path).expect("open runtime db");
    connection
        .execute(
            "UPDATE runtime_sessions SET auth_phase = 'bogus_phase' WHERE project_id = ?1",
            [&project_id],
        )
        .expect("corrupt runtime phase");

    let error = get_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect_err("corrupted runtime row should fail");
    assert_eq!(error.code, "runtime_session_decode_failed");
}

#[test]
fn stale_registry_roots_are_pruned_before_runtime_lookup() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, registry_path, _auth_store_path) = create_state(&root);
    let app = build_mock_app(state);

    registry::replace_projects(
        &registry_path,
        vec![RegistryProjectRecord {
            project_id: "project-1".into(),
            repository_id: "repo-1".into(),
            root_path: root.path().join("missing-repo").display().to_string(),
        }],
    )
    .expect("write stale registry entry");

    let error = get_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: "project-1".into(),
        },
    )
    .expect_err("stale registry root should be pruned");
    assert_eq!(error.code, "project_not_found");

    let contents = std::fs::read_to_string(&registry_path).expect("read pruned registry");
    assert!(contents.contains("\"projects\": []"));
}

#[test]
fn start_runtime_session_does_not_create_durable_runtime_run_rows() {
    let root = tempfile::tempdir().expect("temp dir");
    let (state, _registry_path, auth_store_path) = create_state(&root);
    let app = build_mock_app(state);
    let (project_id, repo_root) = seed_project(&root, &app);

    persist_auth_session(
        &auth_store_path,
        "session-auth",
        "acct-1",
        current_unix_timestamp() + Duration::from_secs(3600).as_secs() as i64,
        "2026-04-13T14:11:59Z",
    );

    start_runtime_session(
        app.handle().clone(),
        app.state::<DesktopState>(),
        ProjectIdRequestDto {
            project_id: project_id.clone(),
        },
    )
    .expect("start runtime session");

    let database_path = database_path_for_repo(&repo_root);
    let connection = rusqlite::Connection::open(&database_path).expect("open runtime db");
    let run_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM runtime_runs", [], |row| row.get(0))
        .expect("count runtime runs");
    let checkpoint_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM runtime_run_checkpoints", [], |row| {
            row.get(0)
        })
        .expect("count runtime checkpoints");

    assert_eq!(run_count, 0);
    assert_eq!(checkpoint_count, 0);
}
