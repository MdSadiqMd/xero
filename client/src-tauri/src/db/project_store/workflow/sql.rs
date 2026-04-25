use super::*;

pub(crate) fn validate_non_empty_text(
    value: &str,
    field: &str,
    code: &str,
) -> Result<(), CommandError> {
    if value.trim().is_empty() {
        return Err(CommandError::user_fixable(
            code,
            format!("Field `{field}` must be a non-empty string."),
        ));
    }

    Ok(())
}
