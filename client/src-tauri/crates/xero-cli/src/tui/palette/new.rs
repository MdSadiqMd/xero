//! `new` — start or focus a chat surface. One-shot: creates/focuses the
//! target through app-data and dismisses the palette. The next prompt the
//! user sends will use the selected session.

use crate::GlobalOptions;

use super::{
    super::app::{invoke_json, App},
    string_field, OpenOutcome,
};

pub fn open(globals: &GlobalOptions, app: &mut App) -> OpenOutcome {
    open_with_kind(globals, app, "standard", "New Chat", "New session")
}

fn open_with_kind(
    globals: &GlobalOptions,
    app: &mut App,
    session_kind: &str,
    title: &str,
    status_label: &str,
) -> OpenOutcome {
    let Some(project_id) = app.project.project_id.clone() else {
        return OpenOutcome::Closed {
            status: Some("No project bound — `register` this directory first.".to_owned()),
        };
    };
    match invoke_json(
        globals,
        &[
            "session",
            "create",
            "--project-id",
            &project_id,
            "--title",
            title,
            "--session-kind",
            session_kind,
        ],
    ) {
        Ok(value) => {
            let session = value.get("session").cloned().unwrap_or(value);
            let session_id = string_field(&session, "agentSessionId");
            if let Err(error) = app.discard_pending_attachments(globals) {
                return OpenOutcome::Closed {
                    status: Some(format!(
                        "Could not clear pending attachments: {} ({})",
                        error.message, error.code
                    )),
                };
            }
            app.reset_for_new_session((!session_id.is_empty()).then_some(session_id.clone()));
            if !session_id.is_empty() {
                super::super::app::sync_active_session_to_cloud_best_effort(globals, app);
            }
            OpenOutcome::Closed {
                status: Some(if session_id.is_empty() {
                    format!("{status_label}.")
                } else {
                    format!("{status_label}: {}", session_id)
                }),
            }
        }
        Err(error) => OpenOutcome::Closed {
            status: Some(format!(
                "Could not start a new session: {} ({})",
                error.message, error.code
            )),
        },
    }
}
