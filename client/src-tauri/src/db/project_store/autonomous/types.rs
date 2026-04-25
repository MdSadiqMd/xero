use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutonomousRunStatus {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutonomousUnitKind {
    Researcher,
    Planner,
    Executor,
    Verifier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutonomousUnitStatus {
    Pending,
    Active,
    Blocked,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutonomousUnitArtifactStatus {
    Pending,
    Recorded,
    Rejected,
    Redacted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousToolCallStateRecord {
    Pending,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousVerificationOutcomeRecord {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousArtifactCommandResultRecord {
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousToolResultPayloadRecord {
    pub project_id: String,
    pub run_id: String,
    pub unit_id: String,
    pub attempt_id: String,
    pub artifact_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    pub tool_state: AutonomousToolCallStateRecord,
    pub command_result: Option<AutonomousArtifactCommandResultRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_summary: Option<ToolResultSummary>,
    pub action_id: Option<String>,
    pub boundary_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousVerificationEvidencePayloadRecord {
    pub project_id: String,
    pub run_id: String,
    pub unit_id: String,
    pub attempt_id: String,
    pub artifact_id: String,
    pub evidence_kind: String,
    pub label: String,
    pub outcome: AutonomousVerificationOutcomeRecord,
    pub command_result: Option<AutonomousArtifactCommandResultRecord>,
    pub action_id: Option<String>,
    pub boundary_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousPolicyDeniedPayloadRecord {
    pub project_id: String,
    pub run_id: String,
    pub unit_id: String,
    pub attempt_id: String,
    pub artifact_id: String,
    pub diagnostic_code: String,
    pub message: String,
    pub tool_name: Option<String>,
    pub action_id: Option<String>,
    pub boundary_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousSkillLifecycleStageRecord {
    Discovery,
    Install,
    Invoke,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousSkillLifecycleResultRecord {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomousSkillCacheStatusRecord {
    Miss,
    Hit,
    Refreshed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousSkillLifecycleSourceRecord {
    pub repo: String,
    pub path: String,
    pub reference: String,
    pub tree_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousSkillLifecycleCacheRecord {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<AutonomousSkillCacheStatusRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousSkillLifecycleDiagnosticRecord {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousSkillLifecyclePayloadRecord {
    pub project_id: String,
    pub run_id: String,
    pub unit_id: String,
    pub attempt_id: String,
    pub artifact_id: String,
    pub stage: AutonomousSkillLifecycleStageRecord,
    pub result: AutonomousSkillLifecycleResultRecord,
    pub skill_id: String,
    pub source: AutonomousSkillLifecycleSourceRecord,
    pub cache: AutonomousSkillLifecycleCacheRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<AutonomousSkillLifecycleDiagnosticRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AutonomousArtifactPayloadRecord {
    ToolResult(AutonomousToolResultPayloadRecord),
    VerificationEvidence(AutonomousVerificationEvidencePayloadRecord),
    PolicyDenied(AutonomousPolicyDeniedPayloadRecord),
    SkillLifecycle(AutonomousSkillLifecyclePayloadRecord),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomousRunRecord {
    pub project_id: String,
    pub agent_session_id: String,
    pub run_id: String,
    pub runtime_kind: String,
    pub provider_id: String,
    pub supervisor_kind: String,
    pub status: AutonomousRunStatus,
    pub active_unit_sequence: Option<u32>,
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
    pub pause_reason: Option<RuntimeRunDiagnosticRecord>,
    pub cancel_reason: Option<RuntimeRunDiagnosticRecord>,
    pub crash_reason: Option<RuntimeRunDiagnosticRecord>,
    pub last_error: Option<RuntimeRunDiagnosticRecord>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomousWorkflowLinkageRecord {
    pub workflow_node_id: String,
    pub transition_id: String,
    pub causal_transition_id: Option<String>,
    pub handoff_transition_id: String,
    pub handoff_package_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomousUnitRecord {
    pub project_id: String,
    pub run_id: String,
    pub unit_id: String,
    pub sequence: u32,
    pub kind: AutonomousUnitKind,
    pub status: AutonomousUnitStatus,
    pub summary: String,
    pub boundary_id: Option<String>,
    pub workflow_linkage: Option<AutonomousWorkflowLinkageRecord>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub updated_at: String,
    pub last_error: Option<RuntimeRunDiagnosticRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomousUnitAttemptRecord {
    pub project_id: String,
    pub run_id: String,
    pub unit_id: String,
    pub attempt_id: String,
    pub attempt_number: u32,
    pub child_session_id: String,
    pub status: AutonomousUnitStatus,
    pub boundary_id: Option<String>,
    pub workflow_linkage: Option<AutonomousWorkflowLinkageRecord>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub updated_at: String,
    pub last_error: Option<RuntimeRunDiagnosticRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomousUnitArtifactRecord {
    pub project_id: String,
    pub run_id: String,
    pub unit_id: String,
    pub attempt_id: String,
    pub artifact_id: String,
    pub artifact_kind: String,
    pub status: AutonomousUnitArtifactStatus,
    pub summary: String,
    pub content_hash: Option<String>,
    pub payload: Option<AutonomousArtifactPayloadRecord>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomousUnitHistoryRecord {
    pub unit: AutonomousUnitRecord,
    pub latest_attempt: Option<AutonomousUnitAttemptRecord>,
    pub artifacts: Vec<AutonomousUnitArtifactRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomousRunUpsertRecord {
    pub run: AutonomousRunRecord,
    pub unit: Option<AutonomousUnitRecord>,
    pub attempt: Option<AutonomousUnitAttemptRecord>,
    pub artifacts: Vec<AutonomousUnitArtifactRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomousRunSnapshotRecord {
    pub run: AutonomousRunRecord,
    pub unit: Option<AutonomousUnitRecord>,
    pub attempt: Option<AutonomousUnitAttemptRecord>,
    pub history: Vec<AutonomousUnitHistoryRecord>,
}
