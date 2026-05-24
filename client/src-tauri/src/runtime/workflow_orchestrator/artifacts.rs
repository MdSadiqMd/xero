use std::collections::BTreeMap;

use serde_json::{json, Value as JsonValue};

use crate::{
    commands::{
        contracts::workflows::{
            WorkflowArtifactRecordDto, WorkflowInputBindingDto, WorkflowOutputContractDto,
            WorkflowOutputExtractionDto,
        },
        CommandError,
    },
    db::project_store::{AgentMessageRole, AgentRunSnapshotRecord},
};

use super::condition_eval::json_path_lookup;

pub fn final_assistant_text(snapshot: &AgentRunSnapshotRecord) -> Option<String> {
    snapshot
        .messages
        .iter()
        .rev()
        .find(|message| {
            message.role == AgentMessageRole::Assistant && !message.content.trim().is_empty()
        })
        .map(|message| message.content.trim().to_string())
}

pub fn extract_workflow_artifact_payload(
    contract: &WorkflowOutputContractDto,
    final_text: &str,
) -> Result<(JsonValue, Option<String>), CommandError> {
    match contract.extraction {
        WorkflowOutputExtractionDto::GenericText => {
            Ok((json!({ "text": final_text }), Some(final_text.to_string())))
        }
        WorkflowOutputExtractionDto::JsonObject => {
            let value = parse_json_output(final_text)?;
            if !value.is_object() {
                return Err(CommandError::user_fixable(
                    "workflow_artifact_extraction_failed",
                    "Xero expected the agent output to be a JSON object for this typed artifact.",
                ));
            }
            let render_text = render_text_for_payload(&value, contract.render_text_path.as_deref());
            Ok((value, render_text))
        }
        WorkflowOutputExtractionDto::JsonArray => {
            let value = parse_json_output(final_text)?;
            if !value.is_array() {
                return Err(CommandError::user_fixable(
                    "workflow_artifact_extraction_failed",
                    "Xero expected the agent output to be a JSON array for this typed artifact.",
                ));
            }
            let render_text = render_text_for_payload(&value, contract.render_text_path.as_deref());
            Ok((value, render_text))
        }
    }
}

pub fn build_agent_node_prompt(
    workflow_name: &str,
    node_title: &str,
    prompt_preface: Option<&str>,
    initial_input: Option<&JsonValue>,
    input_bindings: &[WorkflowInputBindingDto],
    artifacts: &[WorkflowArtifactRecordDto],
) -> Result<String, CommandError> {
    let mut lines = Vec::new();
    if let Some(preface) = prompt_preface
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        lines.push(preface.to_string());
        lines.push(String::new());
    }
    lines.push(format!("Workflow: {workflow_name}"));
    lines.push(format!("Current node: {node_title}"));
    lines.push(String::new());
    lines.push("Use the Workflow inputs below as the contract for this handoff.".into());

    let artifact_index = artifact_index(artifacts);
    for binding in input_bindings {
        let (name, required, label, value) = match binding {
            WorkflowInputBindingDto::RunInput {
                name,
                required,
                path,
                prompt_label,
            } => {
                let value = match (initial_input, path.as_deref()) {
                    (Some(value), Some(path)) => json_path_lookup(value, path).cloned(),
                    (Some(value), None) => Some(value.clone()),
                    (None, _) => None,
                };
                (
                    name,
                    *required,
                    prompt_label.as_deref().unwrap_or(name),
                    value,
                )
            }
            WorkflowInputBindingDto::Artifact {
                name,
                required,
                artifact_ref,
                path,
                prompt_label,
            } => {
                let value = artifact_index.get(artifact_ref).and_then(|artifact| {
                    path.as_deref()
                        .and_then(|path| json_path_lookup(&artifact.payload, path).cloned())
                        .or_else(|| Some(artifact.payload.clone()))
                });
                (
                    name,
                    *required,
                    prompt_label.as_deref().unwrap_or(name),
                    value,
                )
            }
        };
        let Some(value) = value else {
            if required {
                return Err(CommandError::user_fixable(
                    "workflow_required_input_missing",
                    format!("Workflow node `{node_title}` cannot start because input `{name}` is missing."),
                ));
            }
            continue;
        };
        lines.push(String::new());
        lines.push(format!("## {label}"));
        lines.push(render_binding_value(&value));
    }

    if input_bindings.is_empty() {
        if let Some(input) = initial_input {
            lines.push(String::new());
            lines.push("## Workflow input".into());
            lines.push(render_binding_value(input));
        }
    }

    Ok(lines.join("\n"))
}

pub fn artifact_ref_for_record(
    node_id_by_run_id: &BTreeMap<String, String>,
    artifact: &WorkflowArtifactRecordDto,
) -> Option<String> {
    node_id_by_run_id
        .get(&artifact.producer_node_run_id)
        .map(|node_id| format!("{node_id}.{}", artifact.artifact_type))
}

pub fn render_text_for_payload(
    payload: &JsonValue,
    render_text_path: Option<&str>,
) -> Option<String> {
    render_text_path
        .and_then(|path| json_path_lookup(payload, path))
        .and_then(|value| match value {
            JsonValue::String(text) => Some(text.clone()),
            value if value.is_null() => None,
            value => serde_json::to_string_pretty(value).ok(),
        })
}

fn artifact_index(
    artifacts: &[WorkflowArtifactRecordDto],
) -> BTreeMap<String, &WorkflowArtifactRecordDto> {
    let mut index = BTreeMap::new();
    for artifact in artifacts {
        if let Some(node_id) = node_id_from_node_run_id(&artifact.producer_node_run_id) {
            let artifact_ref = format!("{node_id}.{}", artifact.artifact_type);
            index.insert(artifact_ref, artifact);
        }
    }
    index
}

fn node_id_from_node_run_id(node_run_id: &str) -> Option<&str> {
    let after_node = node_run_id.split(":node:").nth(1)?;
    after_node.split(":attempt:").next()
}

fn render_binding_value(value: &JsonValue) -> String {
    match value {
        JsonValue::String(text) => text.clone(),
        _ => serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
    }
}

fn parse_json_output(text: &str) -> Result<JsonValue, CommandError> {
    let trimmed = text.trim();
    if let Ok(value) = serde_json::from_str::<JsonValue>(trimmed) {
        return Ok(value);
    }
    if let Some(fenced) = extract_fenced_json(trimmed) {
        if let Ok(value) = serde_json::from_str::<JsonValue>(fenced.trim()) {
            return Ok(value);
        }
    }
    Err(CommandError::user_fixable(
        "workflow_artifact_extraction_failed",
        "Xero could not extract valid JSON from the agent output.",
    ))
}

fn extract_fenced_json(text: &str) -> Option<&str> {
    let start = text.find("```")?;
    let after_open = &text[start + 3..];
    let content_start = after_open.find('\n').map(|index| index + 1).unwrap_or(0);
    let content = &after_open[content_start..];
    let end = content.find("```")?;
    Some(&content[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::contracts::workflows::{
        WorkflowOutputContractDto, WorkflowOutputExtractionDto,
    };

    #[test]
    fn artifact_extraction_accepts_generic_text() {
        let (payload, render_text) =
            extract_workflow_artifact_payload(&WorkflowOutputContractDto::default(), "done")
                .expect("extract generic text");

        assert_eq!(payload, json!({ "text": "done" }));
        assert_eq!(render_text.as_deref(), Some("done"));
    }

    #[test]
    fn artifact_extraction_accepts_fenced_json_object() {
        let contract = WorkflowOutputContractDto {
            extraction: WorkflowOutputExtractionDto::JsonObject,
            render_text_path: Some("$.summary".into()),
            ..WorkflowOutputContractDto::default()
        };

        let (payload, render_text) =
            extract_workflow_artifact_payload(&contract, "```json\n{\"summary\":\"ok\"}\n```")
                .expect("extract JSON object");

        assert_eq!(payload, json!({ "summary": "ok" }));
        assert_eq!(render_text.as_deref(), Some("ok"));
    }
}
