use std::{collections::HashMap, path::Path};

use rusqlite::{params, Connection, Error as SqlError, OptionalExtension, Transaction};
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use crate::{
    commands::{
        CommandError, CommandErrorClass, OperatorApprovalDto, OperatorApprovalStatus, PhaseStatus,
        PhaseStep, PhaseSummaryDto, PlanningLifecycleProjectionDto, PlanningLifecycleStageDto,
        PlanningLifecycleStageKindDto, ResumeHistoryStatus, WorkflowHandoffPackageDto,
    },
    db::database_path_for_repo,
};

use super::{
    decode_optional_non_empty_text, decode_snapshot_row_id, derive_operator_action_id,
    enqueue_notification_dispatches_best_effort_with_connection,
    find_prohibited_runtime_persistence_content, find_prohibited_transition_diagnostic_content,
    format_notification_dispatch_enqueue_outcome, is_retryable_sql_error,
    is_unique_constraint_violation, map_operator_loop_write_error, map_project_query_error,
    map_snapshot_decode_error, open_project_database, parse_phase_status, parse_phase_step,
    planning_lifecycle_stage_label, read_operator_approval_by_action_id, read_operator_approvals,
    read_phase_summaries, read_planning_lifecycle_projection, read_resume_history,
    read_runtime_run_snapshot, read_runtime_session_row, read_selected_agent_session_row,
    require_non_empty_owned, sqlite_path_suffix, NotificationDispatchEnqueueRecord,
    ProjectSummaryRow,
};

const MAX_WORKFLOW_TRANSITION_EVENT_ROWS: i64 = 200;
const MAX_WORKFLOW_HANDOFF_PACKAGE_ROWS: i64 = 200;
pub(crate) const MAX_LIFECYCLE_TRANSITION_EVENT_ROWS: i64 = 64;
const WORKFLOW_HANDOFF_PACKAGE_SCHEMA_VERSION: u32 = 1;
pub(crate) const PLAN_MODE_REQUIRED_GATE_KEY: &str = "plan_mode_required";
const PLAN_MODE_REQUIRED_ACTION_TYPE: &str = "approve_plan_mode";
const PLAN_MODE_REQUIRED_TITLE: &str = "Approve implementation continuation";
const PLAN_MODE_REQUIRED_DETAIL: &str =
    "Plan mode requires explicit approval before implementation can continue.";

mod automatic_dispatch;
mod graph;
mod handoff;
mod queries;
mod sql;
mod transition;
mod types;
mod validation;

pub(crate) use automatic_dispatch::*;
pub use graph::*;
pub use handoff::*;
pub(crate) use queries::*;
pub(crate) use sql::*;
pub(crate) use transition::*;
pub use types::*;
pub(crate) use validation::*;
