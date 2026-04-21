use serde::{Deserialize, Serialize};

use super::{
    autonomous::{AutonomousRunDto, AutonomousUnitDto},
    workflow::{
        OperatorApprovalDto, PhaseSummaryDto, PlanningLifecycleProjectionDto,
        ResumeHistoryEntryDto, VerificationRecordDto, WorkflowHandoffPackageDto,
    },
};

pub const IMPORT_REPOSITORY_COMMAND: &str = "import_repository";
pub const LIST_PROJECTS_COMMAND: &str = "list_projects";
pub const REMOVE_PROJECT_COMMAND: &str = "remove_project";
pub const GET_AUTONOMOUS_RUN_COMMAND: &str = "get_autonomous_run";
pub const GET_PROJECT_SNAPSHOT_COMMAND: &str = "get_project_snapshot";
pub const GET_REPOSITORY_STATUS_COMMAND: &str = "get_repository_status";
pub const GET_REPOSITORY_DIFF_COMMAND: &str = "get_repository_diff";
pub const GET_RUNTIME_RUN_COMMAND: &str = "get_runtime_run";
pub const GET_RUNTIME_SESSION_COMMAND: &str = "get_runtime_session";
pub const GET_RUNTIME_SETTINGS_COMMAND: &str = "get_runtime_settings";
pub const START_AUTONOMOUS_RUN_COMMAND: &str = "start_autonomous_run";
pub const START_OPENAI_LOGIN_COMMAND: &str = "start_openai_login";
pub const SUBMIT_OPENAI_CALLBACK_COMMAND: &str = "submit_openai_callback";
pub const LOGOUT_RUNTIME_SESSION_COMMAND: &str = "logout_runtime_session";
pub const START_RUNTIME_RUN_COMMAND: &str = "start_runtime_run";
pub const CANCEL_AUTONOMOUS_RUN_COMMAND: &str = "cancel_autonomous_run";
pub const START_RUNTIME_SESSION_COMMAND: &str = "start_runtime_session";
pub const STOP_RUNTIME_RUN_COMMAND: &str = "stop_runtime_run";
pub const SUBSCRIBE_RUNTIME_STREAM_COMMAND: &str = "subscribe_runtime_stream";
pub const RESOLVE_OPERATOR_ACTION_COMMAND: &str = "resolve_operator_action";
pub const RESUME_OPERATOR_RUN_COMMAND: &str = "resume_operator_run";
pub const LIST_NOTIFICATION_ROUTES_COMMAND: &str = "list_notification_routes";
pub const LIST_NOTIFICATION_DISPATCHES_COMMAND: &str = "list_notification_dispatches";
pub const UPSERT_NOTIFICATION_ROUTE_COMMAND: &str = "upsert_notification_route";
pub const UPSERT_NOTIFICATION_ROUTE_CREDENTIALS_COMMAND: &str =
    "upsert_notification_route_credentials";
pub const RECORD_NOTIFICATION_DISPATCH_OUTCOME_COMMAND: &str =
    "record_notification_dispatch_outcome";
pub const SUBMIT_NOTIFICATION_REPLY_COMMAND: &str = "submit_notification_reply";
pub const SYNC_NOTIFICATION_ADAPTERS_COMMAND: &str = "sync_notification_adapters";
pub const UPSERT_RUNTIME_SETTINGS_COMMAND: &str = "upsert_runtime_settings";
pub const UPSERT_WORKFLOW_GRAPH_COMMAND: &str = "upsert_workflow_graph";
pub const APPLY_WORKFLOW_TRANSITION_COMMAND: &str = "apply_workflow_transition";

pub const REGISTERED_COMMAND_NAMES: &[&str] = &[
    IMPORT_REPOSITORY_COMMAND,
    LIST_PROJECTS_COMMAND,
    REMOVE_PROJECT_COMMAND,
    GET_AUTONOMOUS_RUN_COMMAND,
    GET_PROJECT_SNAPSHOT_COMMAND,
    GET_REPOSITORY_STATUS_COMMAND,
    GET_REPOSITORY_DIFF_COMMAND,
    GET_RUNTIME_RUN_COMMAND,
    GET_RUNTIME_SESSION_COMMAND,
    GET_RUNTIME_SETTINGS_COMMAND,
    START_AUTONOMOUS_RUN_COMMAND,
    START_OPENAI_LOGIN_COMMAND,
    SUBMIT_OPENAI_CALLBACK_COMMAND,
    LOGOUT_RUNTIME_SESSION_COMMAND,
    START_RUNTIME_RUN_COMMAND,
    CANCEL_AUTONOMOUS_RUN_COMMAND,
    START_RUNTIME_SESSION_COMMAND,
    STOP_RUNTIME_RUN_COMMAND,
    SUBSCRIBE_RUNTIME_STREAM_COMMAND,
    RESOLVE_OPERATOR_ACTION_COMMAND,
    RESUME_OPERATOR_RUN_COMMAND,
    LIST_NOTIFICATION_ROUTES_COMMAND,
    LIST_NOTIFICATION_DISPATCHES_COMMAND,
    UPSERT_NOTIFICATION_ROUTE_COMMAND,
    UPSERT_NOTIFICATION_ROUTE_CREDENTIALS_COMMAND,
    RECORD_NOTIFICATION_DISPATCH_OUTCOME_COMMAND,
    SUBMIT_NOTIFICATION_REPLY_COMMAND,
    SYNC_NOTIFICATION_ADAPTERS_COMMAND,
    UPSERT_RUNTIME_SETTINGS_COMMAND,
    UPSERT_WORKFLOW_GRAPH_COMMAND,
    APPLY_WORKFLOW_TRANSITION_COMMAND,
];

pub const PROJECT_UPDATED_EVENT: &str = "project:updated";
pub const REPOSITORY_STATUS_CHANGED_EVENT: &str = "repository:status_changed";
pub const RUNTIME_UPDATED_EVENT: &str = "runtime:updated";
pub const RUNTIME_RUN_UPDATED_EVENT: &str = "runtime_run:updated";

pub const START_OPENAI_CODEX_AUTH_COMMAND: &str = START_OPENAI_LOGIN_COMMAND;
pub const COMPLETE_OPENAI_CODEX_AUTH_COMMAND: &str = SUBMIT_OPENAI_CALLBACK_COMMAND;
pub const CANCEL_OPENAI_CODEX_AUTH_COMMAND: &str = "cancel_openai_codex_auth";
pub const GET_RUNTIME_AUTH_STATUS_COMMAND: &str = GET_RUNTIME_SESSION_COMMAND;
pub const REFRESH_OPENAI_CODEX_AUTH_COMMAND: &str = START_RUNTIME_SESSION_COMMAND;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChange,
    Conflicted,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryDiffScope {
    Staged,
    Unstaged,
    Worktree,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectUpdateReason {
    Imported,
    Refreshed,
    MetadataChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportRepositoryRequestDto {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectIdRequestDto {
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepositoryDiffRequestDto {
    pub project_id: String,
    pub scope: RepositoryDiffScope,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSummaryDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub milestone: String,
    pub total_phases: u32,
    pub completed_phases: u32,
    pub active_phase: u32,
    pub branch: Option<String>,
    pub runtime: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepositorySummaryDto {
    pub id: String,
    pub project_id: String,
    pub root_path: String,
    pub display_name: String,
    pub branch: Option<String>,
    pub head_sha: Option<String>,
    pub is_git_repo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BranchSummaryDto {
    pub name: String,
    pub head_sha: Option<String>,
    pub detached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepositoryStatusEntryDto {
    pub path: String,
    pub staged: Option<ChangeKind>,
    pub unstaged: Option<ChangeKind>,
    pub untracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportRepositoryResponseDto {
    pub project: ProjectSummaryDto,
    pub repository: RepositorySummaryDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListProjectsResponseDto {
    pub projects: Vec<ProjectSummaryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSnapshotResponseDto {
    pub project: ProjectSummaryDto,
    pub repository: Option<RepositorySummaryDto>,
    pub phases: Vec<PhaseSummaryDto>,
    pub lifecycle: PlanningLifecycleProjectionDto,
    pub approval_requests: Vec<OperatorApprovalDto>,
    pub verification_records: Vec<VerificationRecordDto>,
    pub resume_history: Vec<ResumeHistoryEntryDto>,
    #[serde(default)]
    pub handoff_packages: Vec<WorkflowHandoffPackageDto>,
    #[serde(default)]
    pub autonomous_run: Option<AutonomousRunDto>,
    #[serde(default)]
    pub autonomous_unit: Option<AutonomousUnitDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepositoryStatusResponseDto {
    pub repository: RepositorySummaryDto,
    pub branch: Option<BranchSummaryDto>,
    pub entries: Vec<RepositoryStatusEntryDto>,
    pub has_staged_changes: bool,
    pub has_unstaged_changes: bool,
    pub has_untracked_changes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepositoryDiffResponseDto {
    pub repository: RepositorySummaryDto,
    pub scope: RepositoryDiffScope,
    pub patch: String,
    pub truncated: bool,
    pub base_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectUpdatedPayloadDto {
    pub project: ProjectSummaryDto,
    pub reason: ProjectUpdateReason,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepositoryStatusChangedPayloadDto {
    pub project_id: String,
    pub repository_id: String,
    pub status: RepositoryStatusResponseDto,
}
