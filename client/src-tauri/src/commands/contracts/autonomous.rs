use serde::{Deserialize, Serialize};

use super::runtime::RuntimeRunDiagnosticDto;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousRunStatusDto {
    Starting,
    Running,
    Paused,
    Cancelling,
    Cancelled,
    Stale,
    Failed,
    Stopped,
    Crashed,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousRunRecoveryStateDto {
    Healthy,
    RecoveryRequired,
    Terminal,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousSkillLifecycleStageDto {
    Discovery,
    Install,
    Invoke,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousSkillLifecycleResultDto {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousSkillCacheStatusDto {
    Miss,
    Hit,
    Refreshed,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitToolResultScopeDto {
    Staged,
    Unstaged,
    Worktree,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WebToolResultContentKindDto {
    Html,
    PlainText,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandToolResultSummaryDto {
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub stdout_redacted: bool,
    pub stderr_redacted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileToolResultSummaryDto {
    pub path: Option<String>,
    pub scope: Option<String>,
    pub line_count: Option<usize>,
    pub match_count: Option<usize>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitToolResultSummaryDto {
    pub scope: Option<GitToolResultScopeDto>,
    pub changed_files: usize,
    pub truncated: bool,
    pub base_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WebToolResultSummaryDto {
    pub target: String,
    pub result_count: Option<usize>,
    pub final_url: Option<String>,
    pub content_kind: Option<WebToolResultContentKindDto>,
    pub content_type: Option<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrowserComputerUseSurfaceDto {
    Browser,
    ComputerUse,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BrowserComputerUseActionStatusDto {
    Pending,
    Running,
    Succeeded,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserComputerUseToolResultSummaryDto {
    pub surface: BrowserComputerUseSurfaceDto,
    pub action: String,
    pub status: BrowserComputerUseActionStatusDto,
    pub target: Option<String>,
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum McpCapabilityKindDto {
    Tool,
    Resource,
    Prompt,
    Command,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpCapabilityToolResultSummaryDto {
    pub server_id: String,
    pub capability_kind: McpCapabilityKindDto,
    pub capability_id: String,
    pub capability_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ToolResultSummaryDto {
    Command(CommandToolResultSummaryDto),
    File(FileToolResultSummaryDto),
    Git(GitToolResultSummaryDto),
    Web(WebToolResultSummaryDto),
    BrowserComputerUse(BrowserComputerUseToolResultSummaryDto),
    McpCapability(McpCapabilityToolResultSummaryDto),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousSkillLifecycleSourceDto {
    pub repo: String,
    pub path: String,
    pub reference: String,
    pub tree_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousSkillLifecycleCacheDto {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AutonomousSkillCacheStatusDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousSkillLifecycleDiagnosticDto {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousLifecycleReasonDto {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousRunDto {
    pub project_id: String,
    pub agent_session_id: String,
    pub run_id: String,
    pub runtime_kind: String,
    pub provider_id: String,
    pub supervisor_kind: String,
    pub status: AutonomousRunStatusDto,
    pub recovery_state: AutonomousRunRecoveryStateDto,
    pub duplicate_start_detected: bool,
    pub duplicate_start_run_id: Option<String>,
    pub duplicate_start_reason: Option<String>,
    pub started_at: String,
    pub last_heartbeat_at: Option<String>,
    pub last_checkpoint_at: Option<String>,
    pub paused_at: Option<String>,
    pub cancelled_at: Option<String>,
    pub completed_at: Option<String>,
    pub crashed_at: Option<String>,
    pub stopped_at: Option<String>,
    pub pause_reason: Option<AutonomousLifecycleReasonDto>,
    pub cancel_reason: Option<AutonomousLifecycleReasonDto>,
    pub crash_reason: Option<AutonomousLifecycleReasonDto>,
    pub last_error_code: Option<String>,
    pub last_error: Option<RuntimeRunDiagnosticDto>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousRunStateDto {
    pub run: Option<AutonomousRunDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetAutonomousRunRequestDto {
    pub project_id: String,
    pub agent_session_id: String,
}

use super::runtime::RuntimeRunControlInputDto;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartAutonomousRunRequestDto {
    pub project_id: String,
    pub agent_session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_controls: Option<RuntimeRunControlInputDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelAutonomousRunRequestDto {
    pub project_id: String,
    pub agent_session_id: String,
    pub run_id: String,
}
