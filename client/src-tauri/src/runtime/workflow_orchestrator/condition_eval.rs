use std::collections::BTreeMap;

use serde_json::{json, Value as JsonValue};

use crate::commands::contracts::workflows::{
    WorkflowConditionDto, WorkflowNodeRunStatusDto, WorkflowNumberCompareOperatorDto,
};

#[derive(Debug, Clone, Default)]
pub struct WorkflowConditionContext {
    pub node_statuses: BTreeMap<String, WorkflowNodeRunStatusDto>,
    pub artifacts: BTreeMap<String, JsonValue>,
    pub failure_classes: BTreeMap<String, String>,
    pub latest_failure_class: Option<String>,
    pub loop_attempts: BTreeMap<String, u32>,
    pub human_decisions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowConditionEvaluation {
    pub matched: bool,
    pub evidence: JsonValue,
}

pub fn evaluate_workflow_condition(
    condition: &WorkflowConditionDto,
    context: &WorkflowConditionContext,
) -> WorkflowConditionEvaluation {
    match condition {
        WorkflowConditionDto::Always => WorkflowConditionEvaluation {
            matched: true,
            evidence: json!({ "kind": "always", "matched": true }),
        },
        WorkflowConditionDto::All { conditions } => {
            let evaluations = conditions
                .iter()
                .map(|condition| evaluate_workflow_condition(condition, context))
                .collect::<Vec<_>>();
            let matched = evaluations.iter().all(|evaluation| evaluation.matched);
            WorkflowConditionEvaluation {
                matched,
                evidence: json!({
                    "kind": "all",
                    "matched": matched,
                    "children": evaluations.into_iter().map(|evaluation| evaluation.evidence).collect::<Vec<_>>()
                }),
            }
        }
        WorkflowConditionDto::Any { conditions } => {
            let evaluations = conditions
                .iter()
                .map(|condition| evaluate_workflow_condition(condition, context))
                .collect::<Vec<_>>();
            let matched = evaluations.iter().any(|evaluation| evaluation.matched);
            WorkflowConditionEvaluation {
                matched,
                evidence: json!({
                    "kind": "any",
                    "matched": matched,
                    "children": evaluations.into_iter().map(|evaluation| evaluation.evidence).collect::<Vec<_>>()
                }),
            }
        }
        WorkflowConditionDto::Not { condition } => {
            let evaluation = evaluate_workflow_condition(condition, context);
            WorkflowConditionEvaluation {
                matched: !evaluation.matched,
                evidence: json!({
                    "kind": "not",
                    "matched": !evaluation.matched,
                    "child": evaluation.evidence
                }),
            }
        }
        WorkflowConditionDto::NodeStatus { node_id, status } => {
            let actual = context.node_statuses.get(node_id).copied();
            let matched = actual == Some(*status);
            WorkflowConditionEvaluation {
                matched,
                evidence: json!({
                    "kind": "node_status",
                    "nodeId": node_id,
                    "expected": status.as_str(),
                    "actual": actual.map(WorkflowNodeRunStatusDto::as_str),
                    "matched": matched
                }),
            }
        }
        WorkflowConditionDto::ArtifactExists { artifact_ref } => {
            let matched = context.artifacts.contains_key(artifact_ref);
            WorkflowConditionEvaluation {
                matched,
                evidence: json!({
                    "kind": "artifact_exists",
                    "artifactRef": artifact_ref,
                    "matched": matched
                }),
            }
        }
        WorkflowConditionDto::ArtifactFieldEquals {
            artifact_ref,
            path,
            value,
        } => {
            let actual = context
                .artifacts
                .get(artifact_ref)
                .and_then(|artifact| json_path_lookup(artifact, path));
            let matched = actual == Some(value);
            WorkflowConditionEvaluation {
                matched,
                evidence: json!({
                    "kind": "artifact_field_equals",
                    "artifactRef": artifact_ref,
                    "path": path,
                    "expected": value,
                    "actual": actual.cloned(),
                    "matched": matched
                }),
            }
        }
        WorkflowConditionDto::ArtifactFieldIn {
            artifact_ref,
            path,
            values,
        } => {
            let actual = context
                .artifacts
                .get(artifact_ref)
                .and_then(|artifact| json_path_lookup(artifact, path));
            let matched = actual
                .map(|actual| values.iter().any(|value| value == actual))
                .unwrap_or(false);
            WorkflowConditionEvaluation {
                matched,
                evidence: json!({
                    "kind": "artifact_field_in",
                    "artifactRef": artifact_ref,
                    "path": path,
                    "values": values,
                    "actual": actual.cloned(),
                    "matched": matched
                }),
            }
        }
        WorkflowConditionDto::ArtifactFieldNumberCompare {
            artifact_ref,
            path,
            operator,
            value,
        } => {
            let actual = context
                .artifacts
                .get(artifact_ref)
                .and_then(|artifact| json_path_lookup(artifact, path))
                .and_then(JsonValue::as_f64);
            let matched = actual
                .map(|actual| compare_numbers(actual, *operator, *value))
                .unwrap_or(false);
            WorkflowConditionEvaluation {
                matched,
                evidence: json!({
                    "kind": "artifact_field_number_compare",
                    "artifactRef": artifact_ref,
                    "path": path,
                    "operator": format!("{operator:?}"),
                    "expected": value,
                    "actual": actual,
                    "matched": matched
                }),
            }
        }
        WorkflowConditionDto::FailureClassIs {
            node_id,
            failure_class,
        } => {
            let actual = node_id
                .as_ref()
                .and_then(|node_id| context.failure_classes.get(node_id))
                .cloned()
                .or_else(|| context.latest_failure_class.clone());
            let matched = actual.as_deref() == Some(failure_class.as_str());
            WorkflowConditionEvaluation {
                matched,
                evidence: json!({
                    "kind": "failure_class_is",
                    "nodeId": node_id,
                    "expected": failure_class,
                    "actual": actual,
                    "matched": matched
                }),
            }
        }
        WorkflowConditionDto::LoopAttemptLt { loop_key, value } => {
            let actual = context.loop_attempts.get(loop_key).copied().unwrap_or(0);
            let matched = actual < *value;
            WorkflowConditionEvaluation {
                matched,
                evidence: json!({
                    "kind": "loop_attempt_lt",
                    "loopKey": loop_key,
                    "expected": value,
                    "actual": actual,
                    "matched": matched
                }),
            }
        }
        WorkflowConditionDto::LoopAttemptGte { loop_key, value } => {
            let actual = context.loop_attempts.get(loop_key).copied().unwrap_or(0);
            let matched = actual >= *value;
            WorkflowConditionEvaluation {
                matched,
                evidence: json!({
                    "kind": "loop_attempt_gte",
                    "loopKey": loop_key,
                    "expected": value,
                    "actual": actual,
                    "matched": matched
                }),
            }
        }
        WorkflowConditionDto::HumanDecisionIs {
            checkpoint_node_id,
            decision,
        } => {
            let actual = context.human_decisions.get(checkpoint_node_id);
            let matched = actual.map(String::as_str) == Some(decision.as_str());
            WorkflowConditionEvaluation {
                matched,
                evidence: json!({
                    "kind": "human_decision_is",
                    "checkpointNodeId": checkpoint_node_id,
                    "expected": decision,
                    "actual": actual,
                    "matched": matched
                }),
            }
        }
    }
}

fn compare_numbers(actual: f64, operator: WorkflowNumberCompareOperatorDto, expected: f64) -> bool {
    match operator {
        WorkflowNumberCompareOperatorDto::Eq => (actual - expected).abs() < f64::EPSILON,
        WorkflowNumberCompareOperatorDto::Neq => (actual - expected).abs() >= f64::EPSILON,
        WorkflowNumberCompareOperatorDto::Gt => actual > expected,
        WorkflowNumberCompareOperatorDto::Gte => actual >= expected,
        WorkflowNumberCompareOperatorDto::Lt => actual < expected,
        WorkflowNumberCompareOperatorDto::Lte => actual <= expected,
    }
}

pub fn json_path_lookup<'a>(value: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
    let mut cursor = value;
    let trimmed = path.trim();
    if trimmed == "$" {
        return Some(cursor);
    }
    let remainder = trimmed.strip_prefix("$.")?;
    for segment in remainder.split('.') {
        if segment.is_empty() {
            return None;
        }
        let (field, indexes) = parse_path_segment(segment)?;
        cursor = cursor.get(field)?;
        for index in indexes {
            cursor = cursor.get(index)?;
        }
    }
    Some(cursor)
}

fn parse_path_segment(segment: &str) -> Option<(&str, Vec<usize>)> {
    let field_end = segment.find('[').unwrap_or(segment.len());
    let field = &segment[..field_end];
    if field.is_empty() {
        return None;
    }
    let mut indexes = Vec::new();
    let mut rest = &segment[field_end..];
    while !rest.is_empty() {
        let inner = rest.strip_prefix('[')?;
        let close = inner.find(']')?;
        let index = inner[..close].parse::<usize>().ok()?;
        indexes.push(index);
        rest = &inner[close + 1..];
    }
    Some((field, indexes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::contracts::workflows::{
        WorkflowConditionDto, WorkflowNodeRunStatusDto, WorkflowNumberCompareOperatorDto,
    };

    #[test]
    fn condition_eval_matches_artifact_field() {
        let mut context = WorkflowConditionContext::default();
        context.artifacts.insert(
            "verify.verification_result".into(),
            json!({ "status": "gaps_found", "gaps": [{ "id": "a" }] }),
        );

        let result = evaluate_workflow_condition(
            &WorkflowConditionDto::ArtifactFieldEquals {
                artifact_ref: "verify.verification_result".into(),
                path: "$.status".into(),
                value: json!("gaps_found"),
            },
            &context,
        );

        assert!(result.matched);
    }

    #[test]
    fn condition_eval_compares_loop_attempts() {
        let mut context = WorkflowConditionContext::default();
        context.loop_attempts.insert("gap_closure".into(), 1);

        let result = evaluate_workflow_condition(
            &WorkflowConditionDto::LoopAttemptLt {
                loop_key: "gap_closure".into(),
                value: 2,
            },
            &context,
        );

        assert!(result.matched);
    }

    #[test]
    fn condition_eval_matches_node_status() {
        let mut context = WorkflowConditionContext::default();
        context
            .node_statuses
            .insert("work".into(), WorkflowNodeRunStatusDto::Succeeded);

        let result = evaluate_workflow_condition(
            &WorkflowConditionDto::NodeStatus {
                node_id: "work".into(),
                status: WorkflowNodeRunStatusDto::Succeeded,
            },
            &context,
        );

        assert!(result.matched);
    }

    #[test]
    fn condition_eval_reads_array_json_path() {
        let value = json!({ "findings": [{ "severity": "high" }] });

        let actual = json_path_lookup(&value, "$.findings[0].severity");

        assert_eq!(actual, Some(&json!("high")));
    }

    #[test]
    fn condition_eval_compares_numbers() {
        let mut context = WorkflowConditionContext::default();
        context
            .artifacts
            .insert("review.review_findings".into(), json!({ "high_count": 0 }));

        let result = evaluate_workflow_condition(
            &WorkflowConditionDto::ArtifactFieldNumberCompare {
                artifact_ref: "review.review_findings".into(),
                path: "$.high_count".into(),
                operator: WorkflowNumberCompareOperatorDto::Eq,
                value: 0.0,
            },
            &context,
        );

        assert!(result.matched);
    }
}
