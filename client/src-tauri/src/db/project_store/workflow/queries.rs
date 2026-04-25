use super::*;

pub(crate) fn read_project_row(
    connection: &Connection,
    database_path: &Path,
    repo_root: &Path,
    expected_project_id: &str,
) -> Result<ProjectSummaryRow, CommandError> {
    connection
        .query_row(
            r#"
            SELECT
                id,
                name,
                description,
                milestone,
                branch,
                runtime
            FROM projects
            WHERE id = ?1
            "#,
            [expected_project_id],
            |row| {
                Ok(ProjectSummaryRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    milestone: row.get(3)?,
                    branch: row.get(4)?,
                    runtime: row.get(5)?,
                })
            },
        )
        .map_err(|error| {
            map_project_query_error(error, database_path, repo_root, expected_project_id)
        })
}
