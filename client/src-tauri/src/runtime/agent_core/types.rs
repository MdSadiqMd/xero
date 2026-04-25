use super::*;

#[derive(Debug, Clone)]
pub struct OwnedAgentRunRequest {
    pub repo_root: PathBuf,
    pub project_id: String,
    pub agent_session_id: String,
    pub run_id: String,
    pub prompt: String,
    pub controls: Option<RuntimeRunControlInputDto>,
    pub tool_runtime: AutonomousToolRuntime,
    pub provider_config: AgentProviderConfig,
}

#[derive(Debug, Clone)]
pub struct ContinueOwnedAgentRunRequest {
    pub repo_root: PathBuf,
    pub project_id: String,
    pub run_id: String,
    pub prompt: String,
    pub controls: Option<RuntimeRunControlInputDto>,
    pub tool_runtime: AutonomousToolRuntime,
    pub provider_config: AgentProviderConfig,
    pub answer_pending_actions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: JsonValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRegistry {
    descriptors: Vec<AgentToolDescriptor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ToolRegistryOptions {
    pub skill_tool_enabled: bool,
}

impl ToolRegistry {
    pub fn builtin() -> Self {
        Self::builtin_with_options(ToolRegistryOptions::default())
    }

    pub fn builtin_with_options(options: ToolRegistryOptions) -> Self {
        Self {
            descriptors: builtin_tool_descriptors()
                .into_iter()
                .filter(|descriptor| {
                    options.skill_tool_enabled || descriptor.name != AUTONOMOUS_TOOL_SKILL
                })
                .collect(),
        }
    }

    pub fn for_prompt(
        repo_root: &Path,
        prompt: &str,
        controls: &RuntimeRunControlStateDto,
    ) -> Self {
        Self::for_prompt_with_options(repo_root, prompt, controls, ToolRegistryOptions::default())
    }

    pub fn for_prompt_with_options(
        repo_root: &Path,
        prompt: &str,
        controls: &RuntimeRunControlStateDto,
        options: ToolRegistryOptions,
    ) -> Self {
        let mut names = select_tool_names_for_prompt(repo_root, prompt, controls);
        if !options.skill_tool_enabled {
            names.remove(AUTONOMOUS_TOOL_SKILL);
        }
        Self::for_tool_names_with_options(names, options)
    }

    pub fn for_tool_names(tool_names: BTreeSet<String>) -> Self {
        Self::for_tool_names_with_options(tool_names, ToolRegistryOptions::default())
    }

    pub fn for_tool_names_with_options(
        tool_names: BTreeSet<String>,
        options: ToolRegistryOptions,
    ) -> Self {
        let descriptors = builtin_tool_descriptors()
            .into_iter()
            .filter(|descriptor| {
                tool_names.contains(descriptor.name.as_str())
                    && (options.skill_tool_enabled || descriptor.name != AUTONOMOUS_TOOL_SKILL)
            })
            .collect();
        Self { descriptors }
    }

    pub fn descriptors(&self) -> &[AgentToolDescriptor] {
        &self.descriptors
    }

    pub fn into_descriptors(self) -> Vec<AgentToolDescriptor> {
        self.descriptors
    }

    pub fn descriptor(&self, name: &str) -> Option<&AgentToolDescriptor> {
        self.descriptors
            .iter()
            .find(|descriptor| descriptor.name == name)
    }

    pub fn descriptor_names(&self) -> BTreeSet<String> {
        self.descriptors
            .iter()
            .map(|descriptor| descriptor.name.clone())
            .collect()
    }

    pub fn expand_with_tool_names<I, S>(&mut self, tool_names: I)
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut names = self.descriptor_names();
        for tool_name in tool_names {
            names.insert(tool_name.as_ref().to_owned());
        }
        *self = Self::for_tool_names(names);
    }

    pub fn decode_call(&self, tool_call: &AgentToolCall) -> CommandResult<AutonomousToolRequest> {
        if self.descriptor(&tool_call.tool_name).is_none() {
            return Err(CommandError::user_fixable(
                "agent_tool_call_unknown",
                format!(
                    "The owned-agent model requested unregistered tool `{}`.",
                    tool_call.tool_name
                ),
            ));
        }

        let request_value = json!({
            "tool": tool_call.tool_name,
            "input": tool_call.input,
        });
        serde_json::from_value::<AutonomousToolRequest>(request_value).map_err(|error| {
            CommandError::user_fixable(
                "agent_tool_call_invalid",
                format!(
                    "Cadence could not decode owned-agent tool call `{}` for `{}`: {error}",
                    tool_call.tool_call_id, tool_call.tool_name
                ),
            )
        })
    }

    pub fn validate_call(&self, tool_call: &AgentToolCall) -> CommandResult<()> {
        self.decode_call(tool_call).map(|_| ())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentToolCall {
    pub tool_call_id: String,
    pub tool_name: String,
    pub input: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentToolResult {
    pub tool_call_id: String,
    pub tool_name: String,
    pub ok: bool,
    pub summary: String,
    pub output: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AgentSafetyDecision {
    Allow { reason: String },
    RequireApproval { reason: String },
    Deny { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderStreamEvent {
    MessageDelta(String),
    ReasoningSummary(String),
    ToolDelta {
        tool_call_id: Option<String>,
        tool_name: Option<String>,
        arguments_delta: String,
    },
    Usage(ProviderUsage),
}

pub trait ProviderAdapter {
    fn provider_id(&self) -> &str;
    fn model_id(&self) -> &str;
    fn stream_turn(
        &self,
        request: &ProviderTurnRequest,
        emit: &mut dyn FnMut(ProviderStreamEvent) -> CommandResult<()>,
    ) -> CommandResult<ProviderTurnOutcome>;
}

#[derive(Debug, Clone)]
pub struct ProviderTurnRequest {
    pub system_prompt: String,
    pub messages: Vec<ProviderMessage>,
    pub tools: Vec<AgentToolDescriptor>,
    pub turn_index: usize,
    pub controls: RuntimeRunControlStateDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "role")]
pub enum ProviderMessage {
    User {
        content: String,
    },
    Assistant {
        content: String,
        tool_calls: Vec<AgentToolCall>,
    },
    Tool {
        tool_call_id: String,
        tool_name: String,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderTurnOutcome {
    Complete {
        message: String,
        usage: Option<ProviderUsage>,
    },
    ToolCalls {
        message: String,
        tool_calls: Vec<AgentToolCall>,
        usage: Option<ProviderUsage>,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct FakeProviderAdapter;

impl ProviderAdapter for FakeProviderAdapter {
    fn provider_id(&self) -> &str {
        OPENAI_CODEX_PROVIDER_ID
    }

    fn model_id(&self) -> &str {
        OPENAI_CODEX_PROVIDER_ID
    }

    fn stream_turn(
        &self,
        request: &ProviderTurnRequest,
        emit: &mut dyn FnMut(ProviderStreamEvent) -> CommandResult<()>,
    ) -> CommandResult<ProviderTurnOutcome> {
        emit(ProviderStreamEvent::ReasoningSummary(format!(
            "Loaded {} owned tool descriptor(s) under {}.",
            request.tools.len(),
            SYSTEM_PROMPT_VERSION
        )))?;

        if request
            .messages
            .iter()
            .any(|message| matches!(message, ProviderMessage::Tool { .. }))
        {
            let message =
                "Owned agent run completed through the Cadence model-loop scaffold.".to_string();
            emit(ProviderStreamEvent::MessageDelta(message.clone()))?;
            return Ok(ProviderTurnOutcome::Complete {
                message,
                usage: Some(ProviderUsage::default()),
            });
        }

        let user_prompt = request
            .messages
            .iter()
            .find_map(|message| match message {
                ProviderMessage::User { content } => Some(content.as_str()),
                _ => None,
            })
            .unwrap_or_default();
        let tool_calls = parse_fake_tool_directives(user_prompt);
        let message = "Cadence owned-agent runtime accepted the task.".to_string();
        emit(ProviderStreamEvent::MessageDelta(message.clone()))?;
        if tool_calls.is_empty() {
            Ok(ProviderTurnOutcome::Complete {
                message,
                usage: Some(ProviderUsage::default()),
            })
        } else {
            Ok(ProviderTurnOutcome::ToolCalls {
                message,
                tool_calls,
                usage: Some(ProviderUsage::default()),
            })
        }
    }
}
