use super::support::*;

pub(crate) fn legacy_repo_local_state_is_upgraded_before_selected_project_snapshot_reads() {
    let root = tempfile::tempdir().expect("temp dir");
    let repo_root = root.path().join("legacy-repo");
    std::fs::create_dir_all(&repo_root).expect("create legacy repo root");
    let project_id = "project-legacy";
    let database_path = create_legacy_state_db(&repo_root, project_id);

    let snapshot = project_store::load_project_snapshot(&repo_root, project_id)
        .expect("load upgraded snapshot")
        .snapshot;

    assert!(snapshot.approval_requests.is_empty());
    assert!(snapshot.verification_records.is_empty());
    assert!(snapshot.resume_history.is_empty());

    let connection = Connection::open(&database_path).expect("reopen upgraded database");
    let tables: Vec<String> = connection
        .prepare(
            r#"
            SELECT name
            FROM sqlite_master
            WHERE type = 'table'
              AND name IN ('operator_approvals', 'operator_verification_records', 'operator_resume_history')
            ORDER BY name ASC
            "#,
        )
        .expect("prepare sqlite_master query")
        .query_map([], |row| row.get(0))
        .expect("query sqlite_master")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect upgraded table names");

    assert_eq!(
        tables,
        vec![
            "operator_approvals".to_string(),
            "operator_resume_history".to_string(),
            "operator_verification_records".to_string(),
        ]
    );
}

pub(crate) fn legacy_repo_local_state_upgrade_adds_workflow_handoff_package_schema() {
    let root = tempfile::tempdir().expect("temp dir");
    let repo_root = root.path().join("legacy-repo-handoff-schema");
    std::fs::create_dir_all(&repo_root).expect("create legacy repo root");
    let project_id = "project-legacy-handoff";
    let database_path = create_legacy_state_db(&repo_root, project_id);

    project_store::load_project_snapshot(&repo_root, project_id)
        .expect("load upgraded snapshot for handoff schema assertions");

    let connection = Connection::open(&database_path).expect("reopen upgraded database");

    let table_sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'workflow_handoff_packages'",
            [],
            |row| row.get(0),
        )
        .expect("workflow_handoff_packages table should exist after migration");

    assert!(table_sql.contains("UNIQUE (project_id, handoff_transition_id)"));
    assert!(table_sql.contains("FOREIGN KEY (project_id, handoff_transition_id)"));
    assert!(table_sql.contains("CHECK (json_valid(package_payload))"));

    let indexes: Vec<String> = connection
        .prepare(
            r#"
            SELECT name
            FROM sqlite_master
            WHERE type = 'index'
              AND tbl_name = 'workflow_handoff_packages'
            ORDER BY name ASC
            "#,
        )
        .expect("prepare workflow_handoff_packages index query")
        .query_map([], |row| row.get(0))
        .expect("query workflow_handoff_packages indexes")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect workflow_handoff_packages indexes");

    assert!(indexes
        .iter()
        .any(|name| name == "idx_workflow_handoff_packages_project_created"));
    assert!(indexes
        .iter()
        .any(|name| name == "idx_workflow_handoff_packages_project_causal"));
}
