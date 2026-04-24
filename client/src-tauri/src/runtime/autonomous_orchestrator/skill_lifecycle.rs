use std::path::Path;

use sha2::{Digest, Sha256};

use crate::{
    auth::now_timestamp,
    commands::CommandError,
    db::project_store::{
        self, AutonomousArtifactPayloadRecord, AutonomousRunSnapshotRecord,
        AutonomousSkillCacheStatusRecord, AutonomousSkillLifecycleCacheRecord,
        AutonomousSkillLifecycleDiagnosticRecord, AutonomousSkillLifecyclePayloadRecord,
        AutonomousSkillLifecycleResultRecord, AutonomousSkillLifecycleSourceRecord,
        AutonomousSkillLifecycleStageRecord, AutonomousUnitArtifactRecord,
        AutonomousUnitArtifactStatus,
    },
    runtime::{AutonomousSkillCacheStatus, AutonomousSkillSourceMetadata},
};

use super::{
    existing_artifact_timestamp, persist_progressed_autonomous_run, upsert_artifact,
    AutonomousRuntimeReconcileIntent, AutonomousSkillLifecycleEvent,
};
use crate::runtime::autonomous_orchestrator::reconcile::reconcile_runtime_snapshot;

pub fn persist_skill_lifecycle_event(
    repo_root: &Path,
    project_id: &str,
    agent_session_id: &str,
    lifecycle: &AutonomousSkillLifecycleEvent,
) -> Result<Option<AutonomousRunSnapshotRecord>, CommandError> {
    let runtime_snapshot =
        match project_store::load_runtime_run(repo_root, project_id, agent_session_id)? {
            Some(snapshot) => snapshot,
            None => return Ok(None),
        };
    let existing = project_store::load_autonomous_run(repo_root, project_id, agent_session_id)?;
    if let Some(snapshot) = existing.as_ref() {
        if snapshot.run.run_id != runtime_snapshot.run.run_id {
            return Err(CommandError::retryable(
                "autonomous_skill_lifecycle_run_mismatch",
                format!(
                    "Cadence refused to persist autonomous skill lifecycle state because durable autonomous run `{}` does not match active runtime run `{}` for project `{project_id}`.",
                    snapshot.run.run_id, runtime_snapshot.run.run_id,
                ),
            ));
        }
    }

    let mut payload = reconcile_runtime_snapshot(
        existing.as_ref(),
        &runtime_snapshot,
        AutonomousRuntimeReconcileIntent::Observe,
    );
    let Some(attempt) = payload.attempt.as_ref() else {
        return Ok(None);
    };

    let stage_label = autonomous_skill_lifecycle_stage_label(&lifecycle.stage);
    let result_label = autonomous_skill_lifecycle_result_label(&lifecycle.result);
    let artifact_id = format!(
        "{}:skill:{}:{}:{}",
        attempt.attempt_id,
        sanitize_artifact_fragment(&lifecycle.skill_id),
        stage_label,
        result_label,
    );
    let timestamp =
        existing_artifact_timestamp(existing.as_ref(), &artifact_id).unwrap_or_else(now_timestamp);
    let summary = autonomous_skill_lifecycle_summary(lifecycle);

    upsert_artifact(
        &mut payload.artifacts,
        AutonomousUnitArtifactRecord {
            project_id: attempt.project_id.clone(),
            run_id: attempt.run_id.clone(),
            unit_id: attempt.unit_id.clone(),
            attempt_id: attempt.attempt_id.clone(),
            artifact_id: artifact_id.clone(),
            artifact_kind: "skill_lifecycle".into(),
            status: autonomous_skill_lifecycle_artifact_status(&lifecycle.result),
            summary,
            content_hash: None,
            payload: Some(AutonomousArtifactPayloadRecord::SkillLifecycle(
                AutonomousSkillLifecyclePayloadRecord {
                    project_id: attempt.project_id.clone(),
                    run_id: attempt.run_id.clone(),
                    unit_id: attempt.unit_id.clone(),
                    attempt_id: attempt.attempt_id.clone(),
                    artifact_id,
                    stage: lifecycle.stage,
                    result: lifecycle.result,
                    skill_id: lifecycle.skill_id.clone(),
                    source: autonomous_skill_lifecycle_source_record(&lifecycle.source),
                    cache: AutonomousSkillLifecycleCacheRecord {
                        key: lifecycle.cache_key.clone(),
                        status: lifecycle
                            .cache_status
                            .as_ref()
                            .map(autonomous_skill_cache_status_record),
                    },
                    diagnostic: lifecycle
                        .diagnostic
                        .as_ref()
                        .map(autonomous_skill_lifecycle_diagnostic_record),
                },
            )),
            created_at: timestamp,
            updated_at: now_timestamp(),
        },
    );

    persist_progressed_autonomous_run(repo_root, project_id, existing.as_ref(), payload).map(Some)
}

pub(super) fn autonomous_skill_cache_key(source: &AutonomousSkillSourceMetadata) -> String {
    let skill_id = source.path.rsplit('/').next().unwrap_or("skill");
    let mut hasher = Sha256::new();
    hasher.update(format!("{}:{}", source.repo, source.path).as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("{}-{}", skill_id, &digest[..12])
}

fn autonomous_skill_lifecycle_source_record(
    source: &AutonomousSkillSourceMetadata,
) -> AutonomousSkillLifecycleSourceRecord {
    AutonomousSkillLifecycleSourceRecord {
        repo: source.repo.clone(),
        path: source.path.clone(),
        reference: source.reference.clone(),
        tree_hash: source.tree_hash.clone(),
    }
}

fn autonomous_skill_cache_status_record(
    status: &AutonomousSkillCacheStatus,
) -> AutonomousSkillCacheStatusRecord {
    match status {
        AutonomousSkillCacheStatus::Miss => AutonomousSkillCacheStatusRecord::Miss,
        AutonomousSkillCacheStatus::Hit => AutonomousSkillCacheStatusRecord::Hit,
        AutonomousSkillCacheStatus::Refreshed => AutonomousSkillCacheStatusRecord::Refreshed,
    }
}

fn autonomous_skill_lifecycle_diagnostic_record(
    diagnostic: &CommandError,
) -> AutonomousSkillLifecycleDiagnosticRecord {
    AutonomousSkillLifecycleDiagnosticRecord {
        code: diagnostic.code.clone(),
        message: diagnostic.message.clone(),
        retryable: diagnostic.retryable,
    }
}

fn autonomous_skill_lifecycle_stage_label(
    stage: &AutonomousSkillLifecycleStageRecord,
) -> &'static str {
    match stage {
        AutonomousSkillLifecycleStageRecord::Discovery => "discovery",
        AutonomousSkillLifecycleStageRecord::Install => "install",
        AutonomousSkillLifecycleStageRecord::Invoke => "invoke",
    }
}

fn autonomous_skill_lifecycle_result_label(
    result: &AutonomousSkillLifecycleResultRecord,
) -> &'static str {
    match result {
        AutonomousSkillLifecycleResultRecord::Succeeded => "succeeded",
        AutonomousSkillLifecycleResultRecord::Failed => "failed",
    }
}

fn autonomous_skill_lifecycle_artifact_status(
    result: &AutonomousSkillLifecycleResultRecord,
) -> AutonomousUnitArtifactStatus {
    match result {
        AutonomousSkillLifecycleResultRecord::Succeeded => AutonomousUnitArtifactStatus::Recorded,
        AutonomousSkillLifecycleResultRecord::Failed => AutonomousUnitArtifactStatus::Rejected,
    }
}

fn autonomous_skill_lifecycle_summary(lifecycle: &AutonomousSkillLifecycleEvent) -> String {
    let stage_label = autonomous_skill_lifecycle_stage_label(&lifecycle.stage);
    match (&lifecycle.result, lifecycle.diagnostic.as_ref()) {
        (AutonomousSkillLifecycleResultRecord::Succeeded, None) => format!(
            "Autonomous skill `{}` recorded a successful {stage_label} stage.",
            lifecycle.skill_id
        ),
        (AutonomousSkillLifecycleResultRecord::Failed, Some(diagnostic)) => format!(
            "Autonomous skill `{}` failed during {stage_label}: {}",
            lifecycle.skill_id, diagnostic.message
        ),
        _ => format!(
            "Autonomous skill `{}` recorded a {stage_label} lifecycle update.",
            lifecycle.skill_id
        ),
    }
}

fn sanitize_artifact_fragment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| match character {
            ':' | '/' | '\\' | ' ' => '-',
            character
                if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') =>
            {
                character
            }
            _ => '-',
        })
        .collect::<String>();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "event".into()
    } else {
        trimmed.into()
    }
}
