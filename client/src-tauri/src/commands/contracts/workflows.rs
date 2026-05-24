use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::{runtime::RuntimeRunApprovalModeDto, workflow_agents::AgentRefDto};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowDefinitionDto {
    #[serde(default = "workflow_definition_schema")]
    pub schema: String,
    pub id: String,
    pub project_id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_definition_version")]
    pub version: u32,
    pub start_node_id: String,
    pub nodes: Vec<WorkflowNodeDto>,
    #[serde(default)]
    pub edges: Vec<WorkflowEdgeDto>,
    #[serde(default)]
    pub artifact_contracts: Vec<WorkflowArtifactContractDto>,
    #[serde(default)]
    pub run_policy: WorkflowRunPolicyDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

fn workflow_definition_schema() -> String {
    "xero.workflow_definition.v1".into()
}

fn default_definition_version() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowPositionDto {
    pub x: f64,
    pub y: f64,
}

impl Default for WorkflowPositionDto {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeTypeDto {
    Agent,
    Router,
    Gate,
    HumanCheckpoint,
    Merge,
    Terminal,
}

impl WorkflowNodeTypeDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Router => "router",
            Self::Gate => "gate",
            Self::HumanCheckpoint => "human_checkpoint",
            Self::Merge => "merge",
            Self::Terminal => "terminal",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowEdgeTypeDto {
    Success,
    Failure,
    Conditional,
    Loop,
    Recovery,
    ManualOverride,
}

impl WorkflowEdgeTypeDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Conditional => "conditional",
            Self::Loop => "loop",
            Self::Recovery => "recovery",
            Self::ManualOverride => "manual_override",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeRunStatusDto {
    Pending,
    Eligible,
    Starting,
    Running,
    WaitingOnGate,
    Succeeded,
    Failed,
    Stalled,
    Skipped,
    Cancelled,
}

impl WorkflowNodeRunStatusDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Eligible => "eligible",
            Self::Starting => "starting",
            Self::Running => "running",
            Self::WaitingOnGate => "waiting_on_gate",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Stalled => "stalled",
            Self::Skipped => "skipped",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunStatusDto {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl WorkflowRunStatusDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowTerminalStatusDto {
    Success,
    Failure,
    Cancelled,
    NeedsHuman,
}

impl WorkflowTerminalStatusDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Cancelled => "cancelled",
            Self::NeedsHuman => "needs_human",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowHumanCheckpointTypeDto {
    HumanVerify,
    Decision,
    HumanAction,
}

impl WorkflowHumanCheckpointTypeDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HumanVerify => "human_verify",
            Self::Decision => "decision",
            Self::HumanAction => "human_action",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowAttemptScopeDto {
    Run,
    SourceNode,
    TargetNode,
    ArtifactGroup,
}

impl Default for WorkflowAttemptScopeDto {
    fn default() -> Self {
        Self::Run
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCarryoverPolicyDto {
    All,
    RequiredOnly,
    None,
    Selected,
}

impl Default for WorkflowCarryoverPolicyDto {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowResetPolicyDto {
    Never,
    OnDownstreamSuccess,
    OnTerminalSuccess,
}

impl Default for WorkflowResetPolicyDto {
    fn default() -> Self {
        Self::Never
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStallDetectorDto {
    FindingCountNotDecreasing,
    SameFailureClassRepeated,
    NoArtifactProgress,
    RuntimeActivityTimeout,
    RetryLimitExceeded,
}

impl WorkflowStallDetectorDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FindingCountNotDecreasing => "finding_count_not_decreasing",
            Self::SameFailureClassRepeated => "same_failure_class_repeated",
            Self::NoArtifactProgress => "no_artifact_progress",
            Self::RuntimeActivityTimeout => "runtime_activity_timeout",
            Self::RetryLimitExceeded => "retry_limit_exceeded",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowMergeWaitPolicyDto {
    All,
    Any,
    Quorum,
    FailFast,
}

impl Default for WorkflowMergeWaitPolicyDto {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowResourceConflictModeDto {
    AllowConflicts,
    SerializeConflicts,
}

impl Default for WorkflowResourceConflictModeDto {
    fn default() -> Self {
        Self::SerializeConflicts
    }
}

impl WorkflowResourceConflictModeDto {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AllowConflicts => "allow_conflicts",
            Self::SerializeConflicts => "serialize_conflicts",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNumberCompareOperatorDto {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum WorkflowConditionDto {
    Always,
    All {
        conditions: Vec<WorkflowConditionDto>,
    },
    Any {
        conditions: Vec<WorkflowConditionDto>,
    },
    Not {
        condition: Box<WorkflowConditionDto>,
    },
    NodeStatus {
        node_id: String,
        status: WorkflowNodeRunStatusDto,
    },
    ArtifactExists {
        artifact_ref: String,
    },
    ArtifactFieldEquals {
        artifact_ref: String,
        path: String,
        value: JsonValue,
    },
    ArtifactFieldIn {
        artifact_ref: String,
        path: String,
        values: Vec<JsonValue>,
    },
    ArtifactFieldNumberCompare {
        artifact_ref: String,
        path: String,
        operator: WorkflowNumberCompareOperatorDto,
        value: f64,
    },
    FailureClassIs {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        node_id: Option<String>,
        failure_class: String,
    },
    LoopAttemptLt {
        loop_key: String,
        value: u32,
    },
    LoopAttemptGte {
        loop_key: String,
        value: u32,
    },
    HumanDecisionIs {
        checkpoint_node_id: String,
        decision: String,
    },
}

impl Default for WorkflowConditionDto {
    fn default() -> Self {
        Self::Always
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowRunOverrideDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_mode: Option<RuntimeRunApprovalModeDto>,
    #[serde(default)]
    pub prompt_preface: String,
    #[serde(default)]
    pub plan_mode_required: bool,
    #[serde(default = "default_true")]
    pub auto_compact_enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "source",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum WorkflowInputBindingDto {
    RunInput {
        name: String,
        #[serde(default = "default_true")]
        required: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt_label: Option<String>,
    },
    Artifact {
        name: String,
        #[serde(default = "default_true")]
        required: bool,
        artifact_ref: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prompt_label: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowOutputExtractionDto {
    GenericText,
    JsonObject,
    JsonArray,
}

impl Default for WorkflowOutputExtractionDto {
    fn default() -> Self {
        Self::GenericText
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowOutputContractDto {
    pub artifact_type: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub extraction: WorkflowOutputExtractionDto,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_text_path: Option<String>,
}

impl Default for WorkflowOutputContractDto {
    fn default() -> Self {
        Self {
            artifact_type: "text_output".into(),
            schema_version: 1,
            extraction: WorkflowOutputExtractionDto::GenericText,
            required: true,
            render_text_path: None,
        }
    }
}

fn default_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowFailureClassificationPolicyDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_activity_timeout_seconds: Option<u32>,
    #[serde(default)]
    pub quota_failure_classes: Vec<String>,
    #[serde(default)]
    pub transient_failure_classes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum WorkflowNodeDto {
    Agent {
        id: String,
        title: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        position: WorkflowPositionDto,
        agent_ref: AgentRefDto,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_label: Option<String>,
        #[serde(default)]
        input_bindings: Vec<WorkflowInputBindingDto>,
        #[serde(default)]
        output_contract: WorkflowOutputContractDto,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        run_overrides: Option<WorkflowRunOverrideDto>,
        #[serde(default)]
        resource_scopes: Vec<String>,
        #[serde(default)]
        failure_policy: WorkflowFailureClassificationPolicyDto,
    },
    Router {
        id: String,
        title: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        position: WorkflowPositionDto,
    },
    Gate {
        id: String,
        title: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        position: WorkflowPositionDto,
        #[serde(default)]
        required_checks: Vec<WorkflowConditionDto>,
        #[serde(default = "default_gate_on_blocked")]
        on_blocked: String,
    },
    HumanCheckpoint {
        id: String,
        title: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        position: WorkflowPositionDto,
        checkpoint_type: WorkflowHumanCheckpointTypeDto,
        prompt: String,
        #[serde(default)]
        decision_options: Vec<String>,
    },
    Merge {
        id: String,
        title: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        position: WorkflowPositionDto,
        #[serde(default)]
        wait_policy: WorkflowMergeWaitPolicyDto,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        quorum: Option<u32>,
        #[serde(default)]
        fail_fast: bool,
    },
    Terminal {
        id: String,
        title: String,
        #[serde(default)]
        description: String,
        #[serde(default)]
        position: WorkflowPositionDto,
        terminal_status: WorkflowTerminalStatusDto,
    },
}

fn default_gate_on_blocked() -> String {
    "pause".into()
}

impl WorkflowNodeDto {
    pub fn id(&self) -> &str {
        match self {
            Self::Agent { id, .. }
            | Self::Router { id, .. }
            | Self::Gate { id, .. }
            | Self::HumanCheckpoint { id, .. }
            | Self::Merge { id, .. }
            | Self::Terminal { id, .. } => id,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Self::Agent { title, .. }
            | Self::Router { title, .. }
            | Self::Gate { title, .. }
            | Self::HumanCheckpoint { title, .. }
            | Self::Merge { title, .. }
            | Self::Terminal { title, .. } => title,
        }
    }

    pub fn node_type(&self) -> WorkflowNodeTypeDto {
        match self {
            Self::Agent { .. } => WorkflowNodeTypeDto::Agent,
            Self::Router { .. } => WorkflowNodeTypeDto::Router,
            Self::Gate { .. } => WorkflowNodeTypeDto::Gate,
            Self::HumanCheckpoint { .. } => WorkflowNodeTypeDto::HumanCheckpoint,
            Self::Merge { .. } => WorkflowNodeTypeDto::Merge,
            Self::Terminal { .. } => WorkflowNodeTypeDto::Terminal,
        }
    }

    pub fn output_contract(&self) -> Option<&WorkflowOutputContractDto> {
        match self {
            Self::Agent {
                output_contract, ..
            } => Some(output_contract),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowLoopPolicyDto {
    pub loop_key: String,
    pub max_attempts: u32,
    #[serde(default)]
    pub attempt_scope: WorkflowAttemptScopeDto,
    #[serde(default)]
    pub carryover_policy: WorkflowCarryoverPolicyDto,
    #[serde(default)]
    pub selected_artifact_refs: Vec<String>,
    #[serde(default)]
    pub reset_policy: WorkflowResetPolicyDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stall_detector: Option<WorkflowStallDetectorDto>,
    pub on_exhausted: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowEdgeDto {
    pub id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub r#type: WorkflowEdgeTypeDto,
    #[serde(default)]
    pub label: String,
    #[serde(default = "default_edge_priority")]
    pub priority: u32,
    #[serde(default)]
    pub condition: WorkflowConditionDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loop_policy: Option<WorkflowLoopPolicyDto>,
}

fn default_edge_priority() -> u32 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowArtifactContractDto {
    pub artifact_type: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<JsonValue>,
    pub display_name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowRunPolicyDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_provider_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_mode: Option<RuntimeRunApprovalModeDto>,
    #[serde(default = "default_concurrency_limit")]
    pub concurrency_limit: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_timeout_seconds: Option<u32>,
    #[serde(default)]
    pub resource_conflict_policy: WorkflowResourceConflictPolicyDto,
    #[serde(default)]
    pub recovery_defaults: WorkflowRecoveryDefaultsDto,
}

impl Default for WorkflowRunPolicyDto {
    fn default() -> Self {
        Self {
            default_provider_profile_id: None,
            default_model_id: None,
            approval_mode: None,
            concurrency_limit: 1,
            node_timeout_seconds: None,
            resource_conflict_policy: WorkflowResourceConflictPolicyDto::default(),
            recovery_defaults: WorkflowRecoveryDefaultsDto::default(),
        }
    }
}

fn default_concurrency_limit() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowResourceConflictPolicyDto {
    #[serde(default)]
    pub mode: WorkflowResourceConflictModeDto,
    #[serde(default)]
    pub default_scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowRecoveryDefaultsDto {
    #[serde(default = "default_debug_max_attempts")]
    pub debug_max_attempts: u32,
    #[serde(default = "default_gap_closure_max_attempts")]
    pub gap_closure_max_attempts: u32,
    #[serde(default = "default_review_fix_max_attempts")]
    pub review_fix_max_attempts: u32,
}

impl Default for WorkflowRecoveryDefaultsDto {
    fn default() -> Self {
        Self {
            debug_max_attempts: 2,
            gap_closure_max_attempts: 2,
            review_fix_max_attempts: 3,
        }
    }
}

fn default_debug_max_attempts() -> u32 {
    2
}

fn default_gap_closure_max_attempts() -> u32 {
    2
}

fn default_review_fix_max_attempts() -> u32 {
    3
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowValidationSeverityDto {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowValidationDiagnosticDto {
    pub severity: WorkflowValidationSeverityDto,
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowValidationStatusDto {
    Valid,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowValidationReportDto {
    pub status: WorkflowValidationStatusDto,
    pub diagnostics: Vec<WorkflowValidationDiagnosticDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowDefinitionSummaryDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub description: String,
    pub active_version_id: String,
    pub active_version_number: u32,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowArtifactRecordDto {
    pub id: String,
    pub workflow_run_id: String,
    pub producer_node_run_id: String,
    pub artifact_type: String,
    pub schema_version: u32,
    pub payload: JsonValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_text: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowRunNodeDto {
    pub id: String,
    pub workflow_run_id: String,
    pub node_id: String,
    pub node_type: String,
    pub status: WorkflowNodeRunStatusDto,
    pub attempt_number: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowRunEdgeDecisionDto {
    pub id: String,
    pub workflow_run_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub edge_id: String,
    pub matched: bool,
    pub condition: WorkflowConditionDto,
    pub evidence: JsonValue,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowLoopAttemptDto {
    pub id: String,
    pub workflow_run_id: String,
    pub loop_key: String,
    pub attempt_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_node_run_id: Option<String>,
    pub exhausted: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowGateDecisionDto {
    pub id: String,
    pub workflow_run_id: String,
    pub node_run_id: String,
    pub checkpoint_type: WorkflowHumanCheckpointTypeDto,
    pub decision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_payload: Option<JsonValue>,
    pub decided_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowEventDto {
    pub id: String,
    pub workflow_run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_run_id: Option<String>,
    pub event_type: String,
    pub event: JsonValue,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowRunDto {
    pub id: String,
    pub project_id: String,
    pub workflow_version_id: String,
    pub workflow_id: String,
    pub workflow_version_number: u32,
    pub status: WorkflowRunStatusDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_status: Option<WorkflowTerminalStatusDto>,
    pub definition_snapshot: WorkflowDefinitionDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_input: Option<JsonValue>,
    pub started_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_reason: Option<String>,
    pub nodes: Vec<WorkflowRunNodeDto>,
    pub edge_decisions: Vec<WorkflowRunEdgeDecisionDto>,
    pub artifacts: Vec<WorkflowArtifactRecordDto>,
    pub gate_decisions: Vec<WorkflowGateDecisionDto>,
    pub loop_attempts: Vec<WorkflowLoopAttemptDto>,
    pub events: Vec<WorkflowEventDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateWorkflowDefinitionRequestDto {
    pub definition: WorkflowDefinitionDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateWorkflowDefinitionRequestDto {
    pub workflow_id: String,
    pub definition: WorkflowDefinitionDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListWorkflowDefinitionsRequestDto {
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetWorkflowDefinitionRequestDto {
    pub project_id: String,
    pub workflow_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListWorkflowDefinitionsResponseDto {
    pub definitions: Vec<WorkflowDefinitionSummaryDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowDefinitionResponseDto {
    pub definition: WorkflowDefinitionDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartWorkflowRunRequestDto {
    pub project_id: String,
    pub workflow_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_input: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetWorkflowRunRequestDto {
    pub project_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListWorkflowRunsRequestDto {
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListWorkflowRunsResponseDto {
    pub runs: Vec<WorkflowRunDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowRunResponseDto {
    pub run: WorkflowRunDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelWorkflowRunRequestDto {
    pub project_id: String,
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetryWorkflowNodeRunRequestDto {
    pub project_id: String,
    pub run_id: String,
    pub node_run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkipWorkflowBranchRequestDto {
    pub project_id: String,
    pub run_id: String,
    pub node_run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResumeWorkflowCheckpointRequestDto {
    pub project_id: String,
    pub run_id: String,
    pub node_run_id: String,
    pub decision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<JsonValue>,
}
