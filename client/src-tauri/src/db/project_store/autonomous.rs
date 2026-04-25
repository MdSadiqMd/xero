use std::{collections::HashMap, path::Path};

use rusqlite::{params, Connection, Error as SqlError, Transaction};
use serde::{Deserialize, Serialize};

use crate::{
    commands::CommandError,
    db::database_path_for_repo,
    runtime::protocol::{
        BrowserComputerUseActionStatus, BrowserComputerUseSurface, GitToolResultScope,
        McpCapabilityKind, ToolResultSummary,
    },
};

use super::runtime::{
    decode_runtime_run_bool, decode_runtime_run_checkpoint_sequence,
    decode_runtime_run_optional_non_empty_text, decode_runtime_run_reason,
    find_prohibited_runtime_persistence_content, map_runtime_run_commit_error,
    map_runtime_run_decode_error, map_runtime_run_transaction_error, map_runtime_run_write_error,
    read_runtime_run_row, require_runtime_run_non_empty_owned, RuntimeRunDiagnosticRecord,
};
use super::{
    compute_workflow_handoff_package_hash, open_runtime_database, read_project_row,
    read_transition_event_by_transition_id, read_workflow_handoff_package_by_transition_id,
    validate_non_empty_text, validate_workflow_handoff_package_hash,
    validate_workflow_handoff_package_transition_linkage,
};

const MAX_AUTONOMOUS_HISTORY_UNIT_ROWS: i64 = 16;
const MAX_AUTONOMOUS_HISTORY_ATTEMPT_ROWS: i64 = 32;
const MAX_AUTONOMOUS_HISTORY_ARTIFACT_ROWS: i64 = 64;
const AUTONOMOUS_ARTIFACT_KIND_TOOL_RESULT: &str = "tool_result";
const AUTONOMOUS_ARTIFACT_KIND_VERIFICATION_EVIDENCE: &str = "verification_evidence";
const AUTONOMOUS_ARTIFACT_KIND_POLICY_DENIED: &str = "policy_denied";
const AUTONOMOUS_ARTIFACT_KIND_SKILL_LIFECYCLE: &str = "skill_lifecycle";
const MAX_BROWSER_COMPUTER_USE_SUMMARY_TEXT_CHARS: usize = 512;

mod operations;
mod persistence;
mod queries;
mod sql;
mod types;
mod validation;

pub use operations::*;
pub use types::*;

pub(crate) use persistence::*;
pub(crate) use queries::*;
pub(crate) use sql::*;
pub(crate) use validation::*;
