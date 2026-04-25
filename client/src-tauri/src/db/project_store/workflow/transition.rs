use super::*;

pub(crate) fn resolve_operator_approval_gate_link(
    _transaction: &Transaction<'_>,
    _database_path: &Path,
    _project_id: &str,
    _action_type: &str,
    _title: &str,
    _detail: &str,
) -> Result<Option<OperatorApprovalGateLink>, CommandError> {
    Ok(None)
}
