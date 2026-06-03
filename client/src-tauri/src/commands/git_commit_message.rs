use std::{collections::BTreeSet, path::PathBuf};

use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use tauri::{AppHandle, Runtime, State};

use crate::{
    auth::now_timestamp,
    commands::{
        backend_jobs::BackendCancellationToken,
        runtime_support::resolve_owned_agent_provider_config, validate_non_empty, CommandError,
        CommandResult, GitGenerateCommitMessageRequestDto, GitGenerateCommitMessageResponseDto,
        RepositoryDiffFileDto, RepositoryDiffScope, RuntimeAgentIdDto,
        RuntimeRunActiveControlSnapshotDto, RuntimeRunApprovalModeDto, RuntimeRunControlInputDto,
        RuntimeRunControlStateDto,
    },
    git::diff,
    runtime::{
        create_provider_adapter, AgentToolCall, AgentToolDescriptor, ProviderAdapter,
        ProviderMessage, ProviderStreamEvent, ProviderTurnOutcome, ProviderTurnRequest,
    },
    state::DesktopState,
};

const COMMIT_MESSAGE_SYSTEM_PROMPT: &str = "You write polished Git commit messages from staged diffs. Return only the commit message text, with no markdown, quotes, labels, or explanation. Use a concise Conventional Commit subject when the change clearly fits, such as feat:, fix:, refactor:, docs:, test:, or chore:. Use imperative mood and keep the first line at 72 characters or less. Add a short body only when it clarifies important behavior, risk, or migration context. If changes are broad or unrelated, use a neutral subject that reflects the dominant user-visible outcome. Do not mention AI, the prompt, the model, or the diff.";
const COMMIT_MESSAGE_LIST_STAGED_FILES_TOOL: &str = "list_staged_files";
const COMMIT_MESSAGE_READ_STAGED_DIFF_TOOL: &str = "read_staged_diff";
const COMMIT_MESSAGE_MAX_PROVIDER_TURNS: usize = 6;
const COMMIT_MESSAGE_MAX_TOOL_CALLS: usize = 8;
const COMMIT_MESSAGE_MAX_PATHS_PER_DIFF_CALL: usize = 8;
const COMMIT_MESSAGE_MAX_DIFF_BYTES_PER_CALL: usize = 64 * 1024;
const COMMIT_MESSAGE_MAX_DIFF_BYTES_TOTAL: usize = 192 * 1024;

#[tauri::command]
pub async fn git_generate_commit_message<R: Runtime + 'static>(
    app: AppHandle<R>,
    state: State<'_, DesktopState>,
    request: GitGenerateCommitMessageRequestDto,
) -> CommandResult<GitGenerateCommitMessageResponseDto> {
    validate_non_empty(&request.project_id, "projectId")?;
    validate_non_empty(&request.model_id, "modelId")?;

    let jobs = state.backend_jobs().clone();
    let state = state.inner().clone();
    let project_id = request.project_id.clone();

    jobs.run_blocking_latest(
        format!("git-commit-message:{project_id}"),
        "commit message generation",
        move |cancellation| git_generate_commit_message_blocking(app, state, request, cancellation),
    )
    .await
}

fn git_generate_commit_message_blocking<R: Runtime + 'static>(
    app: AppHandle<R>,
    state: DesktopState,
    request: GitGenerateCommitMessageRequestDto,
    cancellation: BackendCancellationToken,
) -> CommandResult<GitGenerateCommitMessageResponseDto> {
    cancellation.check_cancelled("commit message generation")?;
    let registry_path = state.global_db_path(&app)?;
    cancellation.check_cancelled("commit message generation")?;
    let diff = diff::load_repository_diff_with_patch_budget(
        &request.project_id,
        RepositoryDiffScope::Staged,
        0,
        &registry_path,
    )?;
    cancellation.check_cancelled("commit message generation")?;
    if diff.files.is_empty() {
        return Err(CommandError::user_fixable(
            "git_commit_message_no_staged_changes",
            "Stage changes before generating a commit message.",
        ));
    }

    let controls = RuntimeRunControlInputDto {
        runtime_agent_id: RuntimeAgentIdDto::Engineer,
        agent_definition_id: None,
        agent_definition_version: None,
        provider_profile_id: normalize_optional_text(request.provider_profile_id),
        model_id: request.model_id.trim().to_owned(),
        thinking_effort: request.thinking_effort.clone(),
        approval_mode: RuntimeRunApprovalModeDto::Yolo,
        plan_mode_required: false,
        auto_compact_enabled: false,
    };
    let provider_config = resolve_owned_agent_provider_config(&app, &state, Some(&controls))?;
    let provider = create_provider_adapter(provider_config)?;
    let provider_id = provider.provider_id().to_owned();
    let provider_model_id = provider.model_id().to_owned();
    let controls_state = RuntimeRunControlStateDto {
        active: RuntimeRunActiveControlSnapshotDto {
            runtime_agent_id: RuntimeAgentIdDto::Engineer,
            agent_definition_id: None,
            agent_definition_version: None,
            provider_profile_id: controls.provider_profile_id.clone(),
            model_id: provider_model_id.clone(),
            thinking_effort: controls.thinking_effort.clone(),
            approval_mode: RuntimeRunApprovalModeDto::Yolo,
            plan_mode_required: false,
            auto_compact_enabled: false,
            revision: 1,
            applied_at: now_timestamp(),
        },
        pending: None,
    };

    let outcome = generate_commit_message_with_git_tools(
        provider.as_ref(),
        &request.project_id,
        registry_path,
        diff.files,
        controls_state,
        &cancellation,
    )?;

    Ok(GitGenerateCommitMessageResponseDto {
        message: outcome.message,
        provider_id,
        model_id: provider_model_id,
        diff_truncated: outcome.diff_truncated,
    })
}

struct CommitMessageGenerationOutcome {
    message: String,
    diff_truncated: bool,
}

struct CommitMessageToolContext {
    project_id: String,
    registry_path: PathBuf,
    staged_files: Vec<RepositoryDiffFileDto>,
    tool_calls_used: usize,
    diff_read_calls_used: usize,
    diff_bytes_sent: usize,
    diff_truncated: bool,
}

impl CommitMessageToolContext {
    fn new(
        project_id: &str,
        registry_path: PathBuf,
        staged_files: Vec<RepositoryDiffFileDto>,
    ) -> Self {
        Self {
            project_id: project_id.to_owned(),
            registry_path,
            staged_files,
            tool_calls_used: 0,
            diff_read_calls_used: 0,
            diff_bytes_sent: 0,
            diff_truncated: false,
        }
    }

    fn execute_tool(&mut self, tool_call: &AgentToolCall) -> CommandResult<String> {
        if self.tool_calls_used >= COMMIT_MESSAGE_MAX_TOOL_CALLS {
            return model_visible_tool_result(
                false,
                "Commit-message git tool budget exhausted.",
                json!({
                    "remainingToolCalls": 0,
                    "remainingDiffBytes": self.remaining_diff_bytes(),
                }),
            );
        }
        self.tool_calls_used += 1;

        match tool_call.tool_name.as_str() {
            COMMIT_MESSAGE_LIST_STAGED_FILES_TOOL => self.list_staged_files(),
            COMMIT_MESSAGE_READ_STAGED_DIFF_TOOL => self.read_staged_diff(&tool_call.input),
            _ => model_visible_tool_result(
                false,
                format!("Unknown commit-message git tool `{}`.", tool_call.tool_name),
                json!({
                    "availableTools": [
                        COMMIT_MESSAGE_LIST_STAGED_FILES_TOOL,
                        COMMIT_MESSAGE_READ_STAGED_DIFF_TOOL,
                    ],
                    "remainingToolCalls": self.remaining_tool_calls(),
                    "remainingDiffBytes": self.remaining_diff_bytes(),
                }),
            ),
        }
    }

    fn list_staged_files(&self) -> CommandResult<String> {
        model_visible_tool_result(
            true,
            format!("Listed {} staged file(s).", self.staged_files.len()),
            json!({
                "files": staged_file_tool_entries(&self.staged_files),
                "fileCount": self.staged_files.len(),
                "readDiffLimits": {
                    "maxPathsPerCall": COMMIT_MESSAGE_MAX_PATHS_PER_DIFF_CALL,
                    "maxBytesPerCall": COMMIT_MESSAGE_MAX_DIFF_BYTES_PER_CALL,
                    "remainingToolCalls": self.remaining_tool_calls(),
                    "remainingDiffBytes": self.remaining_diff_bytes(),
                }
            }),
        )
    }

    fn read_staged_diff(&mut self, input: &JsonValue) -> CommandResult<String> {
        let request = match serde_json::from_value::<ReadStagedDiffInput>(input.clone()) {
            Ok(request) => request,
            Err(error) => {
                return model_visible_tool_result(
                    false,
                    "Invalid read_staged_diff input.",
                    json!({
                        "error": error.to_string(),
                        "expected": {
                            "paths": ["exact/staged/path/from/list_staged_files"]
                        },
                        "remainingToolCalls": self.remaining_tool_calls(),
                        "remainingDiffBytes": self.remaining_diff_bytes(),
                    }),
                );
            }
        };
        let requested_paths = normalized_requested_paths(request.paths);
        if requested_paths.is_empty() {
            return model_visible_tool_result(
                false,
                "read_staged_diff requires at least one staged path.",
                json!({
                    "remainingToolCalls": self.remaining_tool_calls(),
                    "remainingDiffBytes": self.remaining_diff_bytes(),
                }),
            );
        }
        if requested_paths.len() > COMMIT_MESSAGE_MAX_PATHS_PER_DIFF_CALL {
            return model_visible_tool_result(
                false,
                format!(
                    "read_staged_diff accepts at most {COMMIT_MESSAGE_MAX_PATHS_PER_DIFF_CALL} path(s) per call."
                ),
                json!({
                    "requestedPathCount": requested_paths.len(),
                    "maxPathsPerCall": COMMIT_MESSAGE_MAX_PATHS_PER_DIFF_CALL,
                    "remainingToolCalls": self.remaining_tool_calls(),
                    "remainingDiffBytes": self.remaining_diff_bytes(),
                }),
            );
        }

        let mut selected_labels = Vec::new();
        let mut pathspecs = BTreeSet::new();
        let mut unknown_paths = Vec::new();
        for path in requested_paths {
            match staged_file_pathspecs_for_request(&self.staged_files, &path) {
                Some((label, specs)) => {
                    selected_labels.push(label);
                    pathspecs.extend(specs);
                }
                None => unknown_paths.push(path),
            }
        }
        if !unknown_paths.is_empty() {
            return model_visible_tool_result(
                false,
                "read_staged_diff can only read exact paths from the staged file list.",
                json!({
                    "unknownPaths": unknown_paths,
                    "knownPaths": staged_file_labels(&self.staged_files),
                    "remainingToolCalls": self.remaining_tool_calls(),
                    "remainingDiffBytes": self.remaining_diff_bytes(),
                }),
            );
        }

        let remaining_bytes = self.remaining_diff_bytes();
        if remaining_bytes == 0 {
            self.diff_truncated = true;
            return model_visible_tool_result(
                false,
                "Commit-message diff byte budget exhausted.",
                json!({
                    "requestedPaths": selected_labels,
                    "remainingToolCalls": self.remaining_tool_calls(),
                    "remainingDiffBytes": 0,
                }),
            );
        }

        let max_patch_bytes = remaining_bytes.min(COMMIT_MESSAGE_MAX_DIFF_BYTES_PER_CALL);
        let pathspecs = pathspecs.into_iter().collect::<Vec<_>>();
        let response = diff::load_repository_diff_for_paths(
            &self.project_id,
            RepositoryDiffScope::Staged,
            &pathspecs,
            max_patch_bytes,
            &self.registry_path,
        )?;
        self.diff_bytes_sent =
            (self.diff_bytes_sent + response.patch.len()).min(COMMIT_MESSAGE_MAX_DIFF_BYTES_TOTAL);
        self.diff_read_calls_used += 1;
        self.diff_truncated |= response.truncated;

        model_visible_tool_result(
            true,
            format!("Read staged diff for {} path(s).", selected_labels.len()),
            json!({
                "requestedPaths": selected_labels,
                "files": staged_file_tool_entries(&response.files),
                "patch": response.patch,
                "patchBytes": response.patch.len(),
                "truncated": response.truncated,
                "remainingToolCalls": self.remaining_tool_calls(),
                "remainingDiffBytes": self.remaining_diff_bytes(),
            }),
        )
    }

    fn remaining_tool_calls(&self) -> usize {
        COMMIT_MESSAGE_MAX_TOOL_CALLS.saturating_sub(self.tool_calls_used)
    }

    fn remaining_diff_bytes(&self) -> usize {
        COMMIT_MESSAGE_MAX_DIFF_BYTES_TOTAL.saturating_sub(self.diff_bytes_sent)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadStagedDiffInput {
    paths: Vec<String>,
}

fn generate_commit_message_with_git_tools(
    provider: &dyn ProviderAdapter,
    project_id: &str,
    registry_path: PathBuf,
    staged_files: Vec<RepositoryDiffFileDto>,
    controls: RuntimeRunControlStateDto,
    cancellation: &BackendCancellationToken,
) -> CommandResult<CommitMessageGenerationOutcome> {
    let tools = commit_message_tool_descriptors();
    let mut context = CommitMessageToolContext::new(project_id, registry_path, staged_files);
    let mut messages = vec![ProviderMessage::User {
        content: build_commit_message_prompt(project_id, &context.staged_files),
        attachments: Vec::new(),
    }];
    let mut requested_diff_read_after_early_completion = false;

    for turn_index in 0..COMMIT_MESSAGE_MAX_PROVIDER_TURNS {
        cancellation.check_cancelled("commit message generation")?;
        let turn = ProviderTurnRequest {
            system_prompt: COMMIT_MESSAGE_SYSTEM_PROMPT.into(),
            messages: messages.clone(),
            tools: tools.clone(),
            turn_index,
            controls: controls.clone(),
        };
        let mut emit = |_event: ProviderStreamEvent| Ok(());
        match provider.stream_turn(&turn, &mut emit)? {
            ProviderTurnOutcome::Complete { message, .. } => {
                if context.diff_read_calls_used == 0
                    && context.remaining_tool_calls() > 0
                    && !requested_diff_read_after_early_completion
                {
                    requested_diff_read_after_early_completion = true;
                    messages.push(ProviderMessage::User {
                        content: "Before returning the commit message, inspect at least one staged diff with `read_staged_diff`. Use the complete file list to choose the most relevant path(s).".into(),
                        attachments: Vec::new(),
                    });
                    continue;
                }
                return Ok(CommitMessageGenerationOutcome {
                    message: sanitize_provider_commit_message(&message)?,
                    diff_truncated: context.diff_truncated,
                });
            }
            ProviderTurnOutcome::ToolCalls {
                message,
                reasoning_content,
                reasoning_details,
                tool_calls,
                ..
            } => {
                if tool_calls.is_empty() {
                    return Err(CommandError::system_fault(
                        "git_commit_message_provider_turn_invalid",
                        "Xero received a commit-message tool-turn outcome without tool calls.",
                    ));
                }
                messages.push(ProviderMessage::Assistant {
                    content: message,
                    reasoning_content,
                    reasoning_details,
                    tool_calls: tool_calls.clone(),
                });
                for tool_call in tool_calls {
                    cancellation.check_cancelled("commit message generation")?;
                    let content = context.execute_tool(&tool_call)?;
                    messages.push(ProviderMessage::Tool {
                        tool_call_id: tool_call.tool_call_id,
                        tool_name: tool_call.tool_name,
                        content,
                    });
                }
            }
        }
    }

    Err(CommandError::retryable(
        "git_commit_message_provider_turn_limit_exceeded",
        format!(
            "Xero stopped commit-message generation after {COMMIT_MESSAGE_MAX_PROVIDER_TURNS} provider turns to prevent an infinite git-tool loop."
        ),
    ))
}

fn build_commit_message_prompt(project_id: &str, files: &[RepositoryDiffFileDto]) -> String {
    let file_overview = staged_file_overview(files);

    format!(
        "Generate a Git commit message for the staged changes in project `{}`.\nThe staged file list below is complete. Xero has not provided a capped all-file diff; instead, decide which staged diffs to inspect with the read-only git tools. Use `read_staged_diff` before making behavior-specific claims. Prefer reading every staged diff when it fits the budget; when the change set is too large, inspect the files that determine the dominant change and keep the final message broad and truthful about uninspected paths.\n\nTool limits: at most {} tool call(s), {} path(s) per diff call, {} total diff byte(s), and {} byte(s) per diff call.\n\nStaged files ({}):\n{}",
        project_id.trim(),
        COMMIT_MESSAGE_MAX_TOOL_CALLS,
        COMMIT_MESSAGE_MAX_PATHS_PER_DIFF_CALL,
        COMMIT_MESSAGE_MAX_DIFF_BYTES_TOTAL,
        COMMIT_MESSAGE_MAX_DIFF_BYTES_PER_CALL,
        files.len(),
        file_overview
    )
}

fn commit_message_tool_descriptors() -> Vec<AgentToolDescriptor> {
    vec![
        AgentToolDescriptor {
            name: COMMIT_MESSAGE_LIST_STAGED_FILES_TOOL.into(),
            description: "List every staged file path and change kind for commit-message generation. This is read-only and never returns file contents.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false,
            }),
        },
        AgentToolDescriptor {
            name: COMMIT_MESSAGE_READ_STAGED_DIFF_TOOL.into(),
            description: "Read the staged git diff for one or more exact paths from the staged file list. This is read-only, staged-only, and byte-budgeted.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "description": "Exact staged file path labels from the staged file list.",
                        "items": { "type": "string" },
                        "minItems": 1,
                        "maxItems": COMMIT_MESSAGE_MAX_PATHS_PER_DIFF_CALL,
                    },
                },
                "required": ["paths"],
                "additionalProperties": false,
            }),
        },
    ]
}

fn model_visible_tool_result(
    ok: bool,
    summary: impl Into<String>,
    output: JsonValue,
) -> CommandResult<String> {
    serde_json::to_string(&json!({
        "ok": ok,
        "summary": summary.into(),
        "output": output,
    }))
    .map_err(|error| {
        CommandError::system_fault(
            "git_commit_message_tool_result_serialize_failed",
            format!("Xero could not serialize a commit-message git tool result: {error}"),
        )
    })
}

fn staged_file_tool_entries(files: &[RepositoryDiffFileDto]) -> Vec<JsonValue> {
    files
        .iter()
        .map(|file| {
            json!({
                "path": file_display_path(file),
                "status": change_kind_label(&file.status),
                "oldPath": file.old_path.as_deref(),
                "newPath": file.new_path.as_deref(),
                "truncated": file.truncated,
            })
        })
        .collect()
}

fn staged_file_labels(files: &[RepositoryDiffFileDto]) -> Vec<String> {
    files.iter().map(file_display_path).collect()
}

fn normalized_requested_paths(paths: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for path in paths {
        let path = path.trim().replace('\\', "/");
        if path.is_empty() || !seen.insert(path.clone()) {
            continue;
        }
        normalized.push(path);
    }
    normalized
}

fn staged_file_pathspecs_for_request(
    files: &[RepositoryDiffFileDto],
    requested_path: &str,
) -> Option<(String, Vec<String>)> {
    files.iter().find_map(|file| {
        let aliases = staged_file_aliases(file);
        if !aliases.iter().any(|alias| alias == requested_path) {
            return None;
        }

        let mut pathspecs = BTreeSet::new();
        if let Some(old_path) = file.old_path.as_deref().filter(|path| !path.is_empty()) {
            pathspecs.insert(old_path.to_owned());
        }
        if let Some(new_path) = file.new_path.as_deref().filter(|path| !path.is_empty()) {
            pathspecs.insert(new_path.to_owned());
        }
        if pathspecs.is_empty() {
            pathspecs.insert(file.display_path.clone());
        }

        Some((file_display_path(file), pathspecs.into_iter().collect()))
    })
}

fn staged_file_aliases(file: &RepositoryDiffFileDto) -> Vec<String> {
    let mut aliases = BTreeSet::new();
    aliases.insert(file.display_path.clone());
    aliases.insert(file_display_path(file));
    if let Some(old_path) = &file.old_path {
        aliases.insert(old_path.clone());
    }
    if let Some(new_path) = &file.new_path {
        aliases.insert(new_path.clone());
    }
    aliases.into_iter().collect()
}

fn staged_file_overview(files: &[RepositoryDiffFileDto]) -> String {
    if files.is_empty() {
        return "No staged files were reported.".to_owned();
    }

    files
        .iter()
        .enumerate()
        .map(|(index, file)| {
            format!(
                "{}. [{}] {}",
                index + 1,
                change_kind_label(&file.status),
                file_display_path(file)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn file_display_path(file: &RepositoryDiffFileDto) -> String {
    match (&file.old_path, &file.new_path) {
        (Some(old_path), Some(new_path)) if old_path != new_path => {
            format!("{old_path} -> {new_path}")
        }
        _ => file.display_path.clone(),
    }
}

fn change_kind_label(kind: &crate::commands::ChangeKind) -> &'static str {
    match kind {
        crate::commands::ChangeKind::Added => "added",
        crate::commands::ChangeKind::Modified => "modified",
        crate::commands::ChangeKind::Deleted => "deleted",
        crate::commands::ChangeKind::Renamed => "renamed",
        crate::commands::ChangeKind::Copied => "copied",
        crate::commands::ChangeKind::TypeChange => "type changed",
        crate::commands::ChangeKind::Conflicted => "conflicted",
    }
}

fn sanitize_provider_commit_message(message: &str) -> CommandResult<String> {
    let mut text = strip_markdown_fence(message);
    text = strip_label_prefix(&text);
    text = strip_wrapping_quotes(&text);
    text = collapse_excess_blank_lines(&text);
    let text = text.trim().to_owned();

    if text.is_empty() {
        return Err(CommandError::retryable(
            "git_commit_message_empty",
            "The selected model returned an empty commit message.",
        ));
    }

    Ok(text)
}

fn strip_markdown_fence(message: &str) -> String {
    let trimmed = message.trim();
    if !trimmed.starts_with("```") {
        return trimmed.to_owned();
    }

    let mut lines: Vec<&str> = trimmed.lines().collect();
    if lines
        .first()
        .is_some_and(|line| line.trim_start().starts_with("```"))
    {
        lines.remove(0);
    }
    if lines.last().is_some_and(|line| line.trim_end() == "```") {
        lines.pop();
    }
    lines.join("\n").trim().to_owned()
}

fn strip_label_prefix(message: &str) -> String {
    let trimmed = message.trim_start();
    let lower = trimmed.to_ascii_lowercase();
    for prefix in ["commit message:", "commit:", "message:"] {
        if lower.starts_with(prefix) {
            return trimmed[prefix.len()..].trim_start().to_owned();
        }
    }
    trimmed.to_owned()
}

fn strip_wrapping_quotes(message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.len() < 2 || trimmed.contains('\n') {
        return trimmed.to_owned();
    }

    let pairs = [('"', '"'), ('\'', '\''), ('`', '`')];
    for (left, right) in pairs {
        if trimmed.starts_with(left) && trimmed.ends_with(right) {
            return trimmed[1..trimmed.len() - 1].trim().to_owned();
        }
    }

    trimmed.to_owned()
}

fn collapse_excess_blank_lines(message: &str) -> String {
    let mut output = Vec::new();
    let mut blank_count = 0usize;
    for line in message.lines() {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                output.push(String::new());
            }
            continue;
        }
        blank_count = 0;
        output.push(line.trim_end().to_owned());
    }
    output.join("\n")
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{build_commit_message_prompt, sanitize_provider_commit_message};
    use crate::commands::{ChangeKind, RepositoryDiffFileDto};

    #[test]
    fn preserves_body_while_collapsing_extra_blank_lines() {
        let message = "fix: generate commit messages\n\n\n\nUse the staged diff only.";
        assert_eq!(
            sanitize_provider_commit_message(message).expect("message is valid"),
            "fix: generate commit messages\n\nUse the staged diff only."
        );
    }

    #[test]
    fn commit_message_prompt_lists_files_and_exposes_git_tool_workflow() {
        let prompt = build_commit_message_prompt(
            "project-1",
            &[
                RepositoryDiffFileDto {
                    old_path: Some("included.rs".into()),
                    new_path: Some("included.rs".into()),
                    display_path: "included.rs".into(),
                    status: ChangeKind::Modified,
                    hunks: Vec::new(),
                    patch: "diff --git a/included.rs b/included.rs\n+visible\n".into(),
                    truncated: false,
                    cache_key: "included".into(),
                },
                RepositoryDiffFileDto {
                    old_path: Some("omitted.rs".into()),
                    new_path: Some("omitted.rs".into()),
                    display_path: "omitted.rs".into(),
                    status: ChangeKind::Modified,
                    hunks: Vec::new(),
                    patch: String::new(),
                    truncated: true,
                    cache_key: "omitted".into(),
                },
            ],
        );

        assert!(prompt.contains("Staged files (2):"));
        assert!(prompt.contains("[modified] included.rs"));
        assert!(prompt.contains("[modified] omitted.rs"));
        assert!(prompt.contains("The staged file list below is complete"));
        assert!(prompt.contains("Xero has not provided a capped all-file diff"));
        assert!(prompt.contains("read_staged_diff"));
        assert!(!prompt.contains("Staged patch excerpt:"));
    }
}
