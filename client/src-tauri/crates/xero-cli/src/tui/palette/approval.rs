//! `approval` — select the active composer approval mode.

use crossterm::event::{KeyCode, KeyEvent};
use serde_json::json;

use crate::GlobalOptions;

use super::{
    super::app::{App, ApprovalMode},
    rows_detail, string_field, DetailOutcome, DetailRow, DetailState, OpenOutcome,
};

const ID: &str = "approval";

pub fn open(_globals: &GlobalOptions, app: &mut App) -> OpenOutcome {
    let selected = app.selected_approval_mode();
    let rows = app
        .approval_modes_for_selected_agent()
        .iter()
        .map(|mode| approval_row(*mode, selected))
        .collect::<Vec<_>>();
    rows_detail(ID, "Approval mode", Some("enter select   esc back"), rows)
}

pub fn handle_key(
    app: &mut App,
    detail: &mut DetailState,
    key: KeyEvent,
    globals: &GlobalOptions,
) -> DetailOutcome {
    if !matches!(key.code, KeyCode::Enter) {
        return DetailOutcome::Stay;
    }
    let super::DetailData::Rows(rows) = &detail.data else {
        return DetailOutcome::Stay;
    };
    let Some(row) = rows.get(detail.selected) else {
        return DetailOutcome::Stay;
    };
    let Some(mode) = ApprovalMode::from_str(&string_field(&row.payload, "approvalMode")) else {
        return DetailOutcome::Stay;
    };
    if !app.set_approval_mode(mode) {
        return DetailOutcome::Stay;
    }
    super::super::app::sync_active_session_to_cloud_best_effort(globals, app);
    DetailOutcome::Close {
        status: Some(format!("Approval mode: {}", mode.display_label())),
    }
}

fn approval_row(mode: ApprovalMode, selected: ApprovalMode) -> DetailRow {
    let mut subtitle = mode.description().to_owned();
    if mode == selected {
        subtitle.push_str(" · selected");
    }
    DetailRow {
        title: mode.display_label().to_owned(),
        subtitle: Some(subtitle),
        payload: json!({ "approvalMode": mode.label() }),
    }
}
