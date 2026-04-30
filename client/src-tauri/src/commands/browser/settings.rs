use std::path::Path;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime, State};

use crate::{
    auth::now_timestamp,
    commands::{CommandError, CommandResult},
    state::DesktopState,
};

const BROWSER_CONTROL_SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BrowserControlPreferenceDto {
    #[default]
    Default,
    InAppBrowser,
    NativeBrowser,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserControlSettingsDto {
    pub preference: BrowserControlPreferenceDto,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpsertBrowserControlSettingsRequestDto {
    pub preference: BrowserControlPreferenceDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BrowserControlSettingsFile {
    schema_version: u32,
    preference: BrowserControlPreferenceDto,
    updated_at: String,
}

#[tauri::command]
pub fn browser_control_settings<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
) -> CommandResult<BrowserControlSettingsDto> {
    load_browser_control_settings(&app, state.inner())
}

#[tauri::command]
pub fn browser_control_update_settings<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: UpsertBrowserControlSettingsRequestDto,
) -> CommandResult<BrowserControlSettingsDto> {
    let path = state.global_db_path(&app)?;
    let next = browser_control_settings_file_from_request(request)?;
    persist_browser_control_settings_file(&path, &next)?;
    Ok(next.into_dto())
}

pub(crate) fn load_browser_control_settings<R: Runtime>(
    app: &AppHandle<R>,
    state: &DesktopState,
) -> CommandResult<BrowserControlSettingsDto> {
    load_browser_control_settings_from_path(&state.global_db_path(app)?)
}

fn load_browser_control_settings_from_path(
    path: &Path,
) -> CommandResult<BrowserControlSettingsDto> {
    let connection = crate::global_db::open_global_database(path)?;

    let payload: Option<String> = connection
        .query_row(
            "SELECT payload FROM browser_control_settings WHERE id = 1",
            [],
            |row| row.get(0),
        )
        .ok();

    let Some(payload) = payload else {
        return Ok(default_browser_control_settings());
    };

    let parsed = serde_json::from_str::<BrowserControlSettingsFile>(&payload).map_err(|error| {
        CommandError::user_fixable(
            "browser_control_settings_decode_failed",
            format!(
                "Xero could not decode browser control settings stored in the global database: {error}"
            ),
        )
    })?;

    validate_browser_control_settings_file(parsed, "browser_control_settings_decode_failed")
        .map(BrowserControlSettingsFile::into_dto)
}

fn persist_browser_control_settings_file(
    path: &Path,
    settings: &BrowserControlSettingsFile,
) -> CommandResult<()> {
    let payload = serde_json::to_string(settings).map_err(|error| {
        CommandError::system_fault(
            "browser_control_settings_serialize_failed",
            format!("Xero could not serialize browser control settings: {error}"),
        )
    })?;

    let connection = crate::global_db::open_global_database(path)?;
    connection
        .execute(
            "INSERT INTO browser_control_settings (id, payload, updated_at) VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET
                payload = excluded.payload,
                updated_at = excluded.updated_at",
            rusqlite::params![payload, settings.updated_at],
        )
        .map_err(|error| {
            CommandError::retryable(
                "browser_control_settings_write_failed",
                format!("Xero could not persist browser control settings: {error}"),
            )
        })?;
    Ok(())
}

fn browser_control_settings_file_from_request(
    request: UpsertBrowserControlSettingsRequestDto,
) -> CommandResult<BrowserControlSettingsFile> {
    validate_browser_control_settings_file(
        BrowserControlSettingsFile {
            schema_version: BROWSER_CONTROL_SETTINGS_SCHEMA_VERSION,
            preference: request.preference,
            updated_at: now_timestamp(),
        },
        "browser_control_settings_request_invalid",
    )
}

fn validate_browser_control_settings_file(
    file: BrowserControlSettingsFile,
    error_code: &'static str,
) -> CommandResult<BrowserControlSettingsFile> {
    if file.schema_version != BROWSER_CONTROL_SETTINGS_SCHEMA_VERSION {
        return Err(CommandError::user_fixable(
            error_code,
            format!(
                "Xero rejected browser control settings version `{}` because only version `{BROWSER_CONTROL_SETTINGS_SCHEMA_VERSION}` is supported.",
                file.schema_version
            ),
        ));
    }

    Ok(BrowserControlSettingsFile {
        schema_version: BROWSER_CONTROL_SETTINGS_SCHEMA_VERSION,
        preference: file.preference,
        updated_at: normalize_timestamp(file.updated_at),
    })
}

fn default_browser_control_settings() -> BrowserControlSettingsDto {
    BrowserControlSettingsDto {
        preference: BrowserControlPreferenceDto::Default,
        updated_at: None,
    }
}

fn normalize_timestamp(value: String) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        now_timestamp()
    } else {
        trimmed.to_owned()
    }
}

impl BrowserControlSettingsFile {
    fn into_dto(self) -> BrowserControlSettingsDto {
        BrowserControlSettingsDto {
            preference: self.preference,
            updated_at: Some(self.updated_at),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_path(root: &tempfile::TempDir) -> std::path::PathBuf {
        root.path().join("xero.db")
    }

    #[test]
    fn browser_control_settings_default_to_in_app_first_fallback() {
        let root = tempfile::tempdir().expect("temp dir");
        let settings = load_browser_control_settings_from_path(&settings_path(&root))
            .expect("load default browser control settings");

        assert_eq!(settings.preference, BrowserControlPreferenceDto::Default);
        assert_eq!(settings.updated_at, None);
    }

    #[test]
    fn browser_control_settings_persist_selected_preference() {
        let root = tempfile::tempdir().expect("temp dir");
        let file =
            browser_control_settings_file_from_request(UpsertBrowserControlSettingsRequestDto {
                preference: BrowserControlPreferenceDto::NativeBrowser,
            })
            .expect("valid browser control settings file");

        persist_browser_control_settings_file(&settings_path(&root), &file)
            .expect("persist browser control settings");

        let loaded = load_browser_control_settings_from_path(&settings_path(&root))
            .expect("load persisted browser control settings");
        assert_eq!(
            loaded.preference,
            BrowserControlPreferenceDto::NativeBrowser
        );
        assert!(loaded.updated_at.is_some());
    }

    #[test]
    fn browser_control_settings_reject_unknown_schema_version() {
        let error = validate_browser_control_settings_file(
            BrowserControlSettingsFile {
                schema_version: BROWSER_CONTROL_SETTINGS_SCHEMA_VERSION + 1,
                preference: BrowserControlPreferenceDto::Default,
                updated_at: "2026-04-30T12:00:00Z".into(),
            },
            "browser_control_settings_decode_failed",
        )
        .expect_err("unsupported schema version should fail closed");

        assert_eq!(error.code, "browser_control_settings_decode_failed");
    }
}
