use std::{fs, path::PathBuf};

use cadence_desktop_lib::{
    configure_builder_with_state, db,
    git::repository::CanonicalRepository,
    registry::{self, RegistryProjectRecord},
    runtime::{
        AutonomousCommandRequest, AutonomousEditRequest, AutonomousReadRequest,
        AutonomousSearchRequest, AutonomousToolOutput, AutonomousToolRuntime,
        AutonomousWriteRequest,
    },
    state::DesktopState,
};
use tauri::Manager;
use tempfile::TempDir;

#[path = "support/runtime_shell.rs"]
mod runtime_shell;

fn build_mock_app(state: DesktopState) -> tauri::App<tauri::test::MockRuntime> {
    configure_builder_with_state(tauri::test::mock_builder(), state)
        .build(tauri::generate_context!())
        .expect("failed to build mock Tauri app")
}

fn create_state(root: &TempDir) -> DesktopState {
    DesktopState::default()
        .with_registry_file_override(root.path().join("app-data").join("project-registry.json"))
}

fn seed_project(root: &TempDir, app: &tauri::App<tauri::test::MockRuntime>) -> (String, PathBuf) {
    let repo_root = root.path().join("repo");
    fs::create_dir_all(repo_root.join("src")).expect("create repo src");
    let canonical_root = fs::canonicalize(&repo_root).expect("canonical repo root");
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

fn shell_argv(script: impl Into<String>) -> Vec<String> {
    let shell = runtime_shell::launch_script(script);
    std::iter::once(shell.program).chain(shell.args).collect()
}

#[test]
fn tool_runtime_executes_repo_scoped_operations_and_returns_stable_envelopes() {
    let root = tempfile::tempdir().expect("temp dir");
    let app = build_mock_app(create_state(&root));
    let (project_id, repo_root) = seed_project(&root, &app);

    fs::write(
        repo_root.join("src").join("app.txt"),
        "alpha\nbeta\ngamma\n",
    )
    .expect("seed repo file");

    let runtime = AutonomousToolRuntime::for_project(
        &app.handle().clone(),
        app.state::<DesktopState>().inner(),
        &project_id,
    )
    .expect("build autonomous tool runtime");

    let read = runtime
        .read(AutonomousReadRequest {
            path: "src/app.txt".into(),
            start_line: Some(2),
            line_count: Some(2),
        })
        .expect("read file inside repo");
    assert_eq!(read.tool_name, "read");
    match read.output {
        AutonomousToolOutput::Read(output) => {
            assert_eq!(output.path, "src/app.txt");
            assert_eq!(output.start_line, 2);
            assert_eq!(output.line_count, 2);
            assert_eq!(output.total_lines, 3);
            assert_eq!(output.content, "beta\ngamma\n");
            assert!(!output.truncated);
        }
        other => panic!("unexpected read output: {other:?}"),
    }

    let search = runtime
        .search(AutonomousSearchRequest {
            query: "beta".into(),
            path: Some("src".into()),
        })
        .expect("search repo text");
    assert_eq!(search.tool_name, "search");
    match search.output {
        AutonomousToolOutput::Search(output) => {
            assert_eq!(output.scope.as_deref(), Some("src"));
            assert_eq!(output.matches.len(), 1);
            assert_eq!(output.matches[0].path, "src/app.txt");
            assert_eq!(output.matches[0].line, 2);
            assert_eq!(output.matches[0].column, 1);
            assert_eq!(output.scanned_files, 1);
        }
        other => panic!("unexpected search output: {other:?}"),
    }

    let written = runtime
        .write(AutonomousWriteRequest {
            path: "notes/output.txt".into(),
            content: "hello from cadence\n".into(),
        })
        .expect("write file inside repo");
    assert_eq!(written.tool_name, "write");
    match written.output {
        AutonomousToolOutput::Write(output) => {
            assert_eq!(output.path, "notes/output.txt");
            assert!(output.created);
            assert_eq!(
                fs::read_to_string(repo_root.join("notes").join("output.txt"))
                    .expect("read written file"),
                "hello from cadence\n"
            );
        }
        other => panic!("unexpected write output: {other:?}"),
    }

    let edited = runtime
        .edit(AutonomousEditRequest {
            path: "src/app.txt".into(),
            start_line: 2,
            end_line: 2,
            expected: "beta\n".into(),
            replacement: "delta\n".into(),
        })
        .expect("edit file inside repo");
    assert_eq!(edited.tool_name, "edit");
    match edited.output {
        AutonomousToolOutput::Edit(output) => {
            assert_eq!(output.path, "src/app.txt");
            assert_eq!(output.start_line, 2);
            assert_eq!(output.end_line, 2);
            assert_eq!(
                fs::read_to_string(repo_root.join("src").join("app.txt"))
                    .expect("read edited file"),
                "alpha\ndelta\ngamma\n"
            );
        }
        other => panic!("unexpected edit output: {other:?}"),
    }

    let command_script = if cfg!(windows) { "cd" } else { "pwd" };
    let command = runtime
        .command(AutonomousCommandRequest {
            argv: shell_argv(command_script),
            cwd: Some("notes".into()),
            timeout_ms: Some(2_000),
        })
        .expect("run repo-scoped command");
    assert_eq!(command.tool_name, "command");
    assert_eq!(
        command
            .command_result
            .as_ref()
            .and_then(|result| result.exit_code),
        Some(0)
    );
    match command.output {
        AutonomousToolOutput::Command(output) => {
            assert_eq!(output.cwd, "notes");
            assert_eq!(output.exit_code, Some(0));
            let stdout = output.stdout.expect("stdout captured");
            assert!(
                stdout.contains("notes"),
                "stdout should include cwd: {stdout}"
            );
        }
        other => panic!("unexpected command output: {other:?}"),
    }
}

#[test]
fn tool_runtime_rejects_malformed_inputs_and_reports_error_paths_deterministically() {
    let root = tempfile::tempdir().expect("temp dir");
    let app = build_mock_app(create_state(&root));
    let (project_id, repo_root) = seed_project(&root, &app);

    fs::write(
        repo_root.join("src").join("app.txt"),
        "alpha\nbeta\ngamma\n",
    )
    .expect("seed repo file");
    fs::write(repo_root.join("binary.bin"), [0xff_u8, 0xfe, 0x00]).expect("seed binary file");

    let runtime = AutonomousToolRuntime::for_project(
        &app.handle().clone(),
        app.state::<DesktopState>().inner(),
        &project_id,
    )
    .expect("build autonomous tool runtime");

    let invalid_read = runtime
        .read(AutonomousReadRequest {
            path: "binary.bin".into(),
            start_line: None,
            line_count: None,
        })
        .expect_err("binary reads should be rejected");
    assert_eq!(invalid_read.code, "autonomous_tool_file_not_text");

    let oversized_query = "x".repeat(257);
    let search_error = runtime
        .search(AutonomousSearchRequest {
            query: oversized_query,
            path: None,
        })
        .expect_err("oversized search query should be rejected");
    assert_eq!(search_error.code, "autonomous_tool_search_query_too_large");

    let empty_search = runtime
        .search(AutonomousSearchRequest {
            query: "missing".into(),
            path: Some("src".into()),
        })
        .expect("zero-match search should still succeed");
    match empty_search.output {
        AutonomousToolOutput::Search(output) => assert!(output.matches.is_empty()),
        other => panic!("unexpected empty-search output: {other:?}"),
    }

    let invalid_range = runtime
        .edit(AutonomousEditRequest {
            path: "src/app.txt".into(),
            start_line: 4,
            end_line: 5,
            expected: "placeholder\n".into(),
            replacement: "noop\n".into(),
        })
        .expect_err("out-of-range edit should be rejected");
    assert_eq!(invalid_range.code, "autonomous_tool_edit_range_invalid");

    runtime
        .edit(AutonomousEditRequest {
            path: "src/app.txt".into(),
            start_line: 2,
            end_line: 2,
            expected: "beta\n".into(),
            replacement: "delta\n".into(),
        })
        .expect("first deterministic edit succeeds");
    let deterministic_mismatch = runtime
        .edit(AutonomousEditRequest {
            path: "src/app.txt".into(),
            start_line: 2,
            end_line: 2,
            expected: "beta\n".into(),
            replacement: "delta\n".into(),
        })
        .expect_err("repeating stale edit should fail deterministically");
    assert_eq!(
        deterministic_mismatch.code,
        "autonomous_tool_edit_expected_text_mismatch"
    );
    assert_eq!(
        fs::read_to_string(repo_root.join("src").join("app.txt")).expect("read edited file"),
        "alpha\ndelta\ngamma\n"
    );

    let nonzero_script = runtime_shell::script_print_line_then_exit("boom", 7);
    let nonzero = runtime
        .command(AutonomousCommandRequest {
            argv: shell_argv(nonzero_script),
            cwd: None,
            timeout_ms: Some(2_000),
        })
        .expect("non-zero exits should return a stable command result");
    assert_eq!(
        nonzero
            .command_result
            .as_ref()
            .and_then(|result| result.exit_code),
        Some(7)
    );
    match nonzero.output {
        AutonomousToolOutput::Command(output) => {
            assert_eq!(output.exit_code, Some(7));
            assert_eq!(output.stderr, None);
            assert_eq!(output.stdout.as_deref(), Some("boom"));
        }
        other => panic!("unexpected non-zero command output: {other:?}"),
    }

    let timeout = runtime
        .command(AutonomousCommandRequest {
            argv: shell_argv(runtime_shell::script_sleep(2)),
            cwd: None,
            timeout_ms: Some(50),
        })
        .expect_err("timed-out command should return a retryable error");
    assert_eq!(timeout.code, "autonomous_tool_command_timeout");
    assert!(timeout.retryable);
}

#[test]
fn tool_runtime_denies_path_traversal_and_out_of_repo_cwds() {
    let root = tempfile::tempdir().expect("temp dir");
    let app = build_mock_app(create_state(&root));
    let (project_id, _repo_root) = seed_project(&root, &app);

    let runtime = AutonomousToolRuntime::for_project(
        &app.handle().clone(),
        app.state::<DesktopState>().inner(),
        &project_id,
    )
    .expect("build autonomous tool runtime");

    let read_error = runtime
        .read(AutonomousReadRequest {
            path: "../outside.txt".into(),
            start_line: None,
            line_count: None,
        })
        .expect_err("path traversal should be denied");
    assert_eq!(read_error.code, "autonomous_tool_path_denied");
    assert_eq!(
        read_error.class,
        cadence_desktop_lib::commands::CommandErrorClass::PolicyDenied
    );

    let write_error = runtime
        .write(AutonomousWriteRequest {
            path: "../outside.txt".into(),
            content: "denied".into(),
        })
        .expect_err("out-of-root write should be denied");
    assert_eq!(write_error.code, "autonomous_tool_path_denied");
    assert_eq!(
        write_error.class,
        cadence_desktop_lib::commands::CommandErrorClass::PolicyDenied
    );

    let cwd_error = runtime
        .command(AutonomousCommandRequest {
            argv: shell_argv(if cfg!(windows) { "cd" } else { "pwd" }),
            cwd: Some("../".into()),
            timeout_ms: Some(1_000),
        })
        .expect_err("out-of-root cwd should be denied");
    assert_eq!(cwd_error.code, "autonomous_tool_path_denied");
    assert_eq!(
        cwd_error.class,
        cadence_desktop_lib::commands::CommandErrorClass::PolicyDenied
    );
}

#[test]
fn tool_runtime_returns_project_not_found_for_unknown_projects() {
    let root = tempfile::tempdir().expect("temp dir");
    let app = build_mock_app(create_state(&root));

    let error = AutonomousToolRuntime::for_project(
        &app.handle().clone(),
        app.state::<DesktopState>().inner(),
        "missing-project",
    )
    .expect_err("unknown projects should not resolve a repo root");
    assert_eq!(error.code, "project_not_found");
}

#[cfg(unix)]
#[test]
fn tool_runtime_denies_symlink_escapes() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("temp dir");
    let app = build_mock_app(create_state(&root));
    let (project_id, repo_root) = seed_project(&root, &app);
    let outside = root.path().join("outside.txt");
    fs::write(&outside, "outside\n").expect("seed outside file");
    symlink(&outside, repo_root.join("linked.txt")).expect("create escape symlink");

    let runtime = AutonomousToolRuntime::for_project(
        &app.handle().clone(),
        app.state::<DesktopState>().inner(),
        &project_id,
    )
    .expect("build autonomous tool runtime");

    let error = runtime
        .read(AutonomousReadRequest {
            path: "linked.txt".into(),
            start_line: None,
            line_count: None,
        })
        .expect_err("symlink escape should be denied");
    assert_eq!(error.code, "autonomous_tool_path_denied");
    assert_eq!(
        error.class,
        cadence_desktop_lib::commands::CommandErrorClass::PolicyDenied
    );
}
