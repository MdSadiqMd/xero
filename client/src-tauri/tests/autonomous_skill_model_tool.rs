use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    sync::{Arc, Mutex},
};

use cadence_desktop_lib::{
    commands::{
        RuntimeRunActiveControlSnapshotDto, RuntimeRunApprovalModeDto, RuntimeRunControlStateDto,
    },
    db::project_store,
    runtime::{
        AutonomousBundledSkillRoot, AutonomousLocalSkillRoot, AutonomousSkillCacheStore,
        AutonomousSkillRuntime, AutonomousSkillRuntimeConfig, AutonomousSkillSource,
        AutonomousSkillSourceEntryKind, AutonomousSkillSourceError,
        AutonomousSkillSourceFileRequest, AutonomousSkillSourceFileResponse,
        AutonomousSkillSourceMetadata, AutonomousSkillSourceTreeEntry,
        AutonomousSkillSourceTreeRequest, AutonomousSkillSourceTreeResponse,
        AutonomousSkillToolStatus, AutonomousToolAccessAction, AutonomousToolAccessRequest,
        AutonomousToolOutput, AutonomousToolRequest, AutonomousToolRuntime,
        AutonomousToolSearchRequest, CadenceSkillSourceKind, CadenceSkillSourceState,
        CadenceSkillToolDynamicAssetInput, CadenceSkillToolInput, CadenceSkillTrustState,
        FilesystemAutonomousSkillCacheStore, ToolRegistry, ToolRegistryOptions,
    },
};
use rusqlite::Connection;
use tempfile::TempDir;

#[derive(Default)]
struct FixtureSkillSource {
    state: Mutex<FixtureSkillSourceState>,
}

#[derive(Default)]
struct FixtureSkillSourceState {
    tree_response: Option<Result<AutonomousSkillSourceTreeResponse, AutonomousSkillSourceError>>,
    files: Vec<(
        String,
        String,
        Result<AutonomousSkillSourceFileResponse, AutonomousSkillSourceError>,
    )>,
}

impl FixtureSkillSource {
    fn set_tree_response(
        &self,
        response: Result<AutonomousSkillSourceTreeResponse, AutonomousSkillSourceError>,
    ) {
        self.state.lock().expect("fixture lock").tree_response = Some(response);
    }

    fn set_file(&self, repo: &str, path: &str, content: &str) {
        self.state.lock().expect("fixture lock").files.push((
            repo.into(),
            path.into(),
            Ok(AutonomousSkillSourceFileResponse {
                bytes: content.as_bytes().to_vec(),
            }),
        ));
    }
}

impl AutonomousSkillSource for FixtureSkillSource {
    fn list_tree(
        &self,
        _request: &AutonomousSkillSourceTreeRequest,
    ) -> Result<AutonomousSkillSourceTreeResponse, AutonomousSkillSourceError> {
        self.state
            .lock()
            .expect("fixture lock")
            .tree_response
            .clone()
            .expect("fixture tree response")
    }

    fn fetch_file(
        &self,
        request: &AutonomousSkillSourceFileRequest,
    ) -> Result<AutonomousSkillSourceFileResponse, AutonomousSkillSourceError> {
        self.state
            .lock()
            .expect("fixture lock")
            .files
            .iter()
            .find(|(repo, path, _)| repo == &request.repo && path == &request.path)
            .map(|(_, _, response)| response.clone())
            .unwrap_or_else(|| {
                Err(AutonomousSkillSourceError::Status {
                    status: 404,
                    message: format!("{}:{}", request.repo, request.path),
                })
            })
    }
}

fn runtime_controls() -> RuntimeRunControlStateDto {
    RuntimeRunControlStateDto {
        active: RuntimeRunActiveControlSnapshotDto {
            model_id: "test-model".into(),
            thinking_effort: None,
            approval_mode: RuntimeRunApprovalModeDto::Yolo,
            plan_mode_required: false,
            revision: 1,
            applied_at: "2026-04-25T00:00:00Z".into(),
        },
        pending: None,
    }
}

fn skill_runtime(root: &TempDir, source: Arc<FixtureSkillSource>) -> AutonomousSkillRuntime {
    AutonomousSkillRuntime::with_source_and_cache(
        AutonomousSkillRuntimeConfig::default(),
        source,
        Arc::new(FilesystemAutonomousSkillCacheStore::new(
            root.path().join("skill-cache"),
        )) as Arc<dyn AutonomousSkillCacheStore>,
    )
}

fn runtime_with_skills(
    root: &TempDir,
    source: Arc<FixtureSkillSource>,
    local_root: &Path,
    bundled_root: &Path,
) -> AutonomousToolRuntime {
    AutonomousToolRuntime::new(root.path())
        .expect("tool runtime")
        .with_skill_tool(
            "project-1",
            skill_runtime(root, source),
            vec![AutonomousBundledSkillRoot {
                bundle_id: "cadence".into(),
                version: "2026.04.25".into(),
                root_path: bundled_root.to_path_buf(),
            }],
            vec![AutonomousLocalSkillRoot {
                root_id: "personal".into(),
                root_path: local_root.to_path_buf(),
            }],
        )
}

fn runtime_with_bundled_version(
    root: &TempDir,
    source: Arc<FixtureSkillSource>,
    bundled_root: &Path,
    version: &str,
) -> AutonomousToolRuntime {
    AutonomousToolRuntime::new(root.path())
        .expect("tool runtime")
        .with_skill_tool(
            "project-1",
            skill_runtime(root, source),
            vec![AutonomousBundledSkillRoot {
                bundle_id: "cadence".into(),
                version: version.into(),
                root_path: bundled_root.to_path_buf(),
            }],
            Vec::new(),
        )
}

fn write_skill(root: &Path, directory: &str, name: &str, description: &str) {
    let skill_dir = root.join(directory);
    fs::create_dir_all(&skill_dir).expect("create skill dir");
    fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n"),
    )
    .expect("write skill");
    fs::write(skill_dir.join("guide.md"), "# Guide\n").expect("write guide");
}

fn init_project_state(repo_root: &Path) {
    let cadence_dir = repo_root.join(".cadence");
    fs::create_dir_all(&cadence_dir).expect("create .cadence");
    let mut connection =
        Connection::open(cadence_dir.join("state.db")).expect("open project state db");
    cadence_desktop_lib::db::migrations::migrations()
        .to_latest(&mut connection)
        .expect("migrate project state db");
    connection
        .execute(
            "INSERT OR IGNORE INTO projects (id, name, description) VALUES (?1, ?2, ?3)",
            ("project-1", "Project", "SkillTool test project"),
        )
        .expect("seed project row");
}

fn github_source_metadata(skill_id: &str, tree_hash: &str) -> AutonomousSkillSourceMetadata {
    AutonomousSkillSourceMetadata {
        repo: "vercel-labs/skills".into(),
        path: format!("skills/{skill_id}"),
        reference: "main".into(),
        tree_hash: tree_hash.into(),
    }
}

fn standard_skill_tree(skill_id: &str, tree_hash: &str) -> AutonomousSkillSourceTreeResponse {
    AutonomousSkillSourceTreeResponse {
        entries: vec![
            AutonomousSkillSourceTreeEntry {
                path: format!("skills/{skill_id}"),
                kind: AutonomousSkillSourceEntryKind::Tree,
                hash: tree_hash.into(),
                bytes: None,
            },
            AutonomousSkillSourceTreeEntry {
                path: format!("skills/{skill_id}/SKILL.md"),
                kind: AutonomousSkillSourceEntryKind::Blob,
                hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
                bytes: Some(80),
            },
            AutonomousSkillSourceTreeEntry {
                path: format!("skills/{skill_id}/guide.md"),
                kind: AutonomousSkillSourceEntryKind::Blob,
                hash: "cccccccccccccccccccccccccccccccccccccccc".into(),
                bytes: Some(20),
            },
        ],
    }
}

#[test]
fn owned_agent_skill_descriptor_and_tool_search_are_gated_by_skill_support() {
    let root = tempfile::tempdir().expect("temp dir");
    init_project_state(root.path());
    let controls = runtime_controls();
    let disabled = ToolRegistry::for_prompt(root.path(), "Use skills for this task.", &controls);
    assert!(!disabled.descriptor_names().contains("skill"));

    let enabled = ToolRegistry::for_prompt_with_options(
        root.path(),
        "Use skills for this task.",
        &controls,
        ToolRegistryOptions {
            skill_tool_enabled: true,
        },
    );
    assert!(enabled.descriptor_names().contains("skill"));

    let disabled_runtime = AutonomousToolRuntime::new(root.path()).expect("runtime");
    let disabled_search = disabled_runtime
        .tool_search(AutonomousToolSearchRequest {
            query: "skill".into(),
            limit: None,
        })
        .expect("search tools");
    match disabled_search.output {
        AutonomousToolOutput::ToolSearch(output) => {
            assert!(!output.matches.iter().any(|item| item.tool_name == "skill"));
        }
        other => panic!("unexpected output: {other:?}"),
    }

    let source = Arc::new(FixtureSkillSource::default());
    source.set_tree_response(Ok(standard_skill_tree(
        "find-skills",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )));
    let enabled_runtime = AutonomousToolRuntime::new(root.path())
        .expect("runtime")
        .with_skill_tool(
            "project-1",
            skill_runtime(&root, source),
            Vec::new(),
            Vec::new(),
        );
    let enabled_search = enabled_runtime
        .tool_search(AutonomousToolSearchRequest {
            query: "skill".into(),
            limit: None,
        })
        .expect("search tools");
    match enabled_search.output {
        AutonomousToolOutput::ToolSearch(output) => {
            assert!(output
                .matches
                .iter()
                .any(|item| item.tool_name == "skill" && item.group == "skills"));
        }
        other => panic!("unexpected output: {other:?}"),
    }

    let access = enabled_runtime
        .tool_access(AutonomousToolAccessRequest {
            action: AutonomousToolAccessAction::List,
            groups: Vec::new(),
            tools: Vec::new(),
            reason: None,
        })
        .expect("tool access list");
    match access.output {
        AutonomousToolOutput::ToolAccess(output) => {
            assert!(output
                .available_groups
                .iter()
                .any(|group| group.name == "skills"));
        }
        other => panic!("unexpected output: {other:?}"),
    }
}

#[test]
fn skill_tool_merges_sources_filters_trust_and_invokes_validated_context() {
    let root = tempfile::tempdir().expect("temp dir");
    init_project_state(root.path());
    let local_root = root.path().join("local-skills");
    let bundled_root = root.path().join("bundled-skills");
    write_skill(&local_root, "local-skill", "local-skill", "Local skill.");
    write_skill(
        &bundled_root,
        "bundled-skill",
        "bundled-skill",
        "Bundled skill.",
    );
    write_skill(
        &root.path().join(".cadence").join("skills"),
        "project-skill",
        "project-skill",
        "Project skill.",
    );

    let source = Arc::new(FixtureSkillSource::default());
    let tree_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    source.set_tree_response(Ok(standard_skill_tree("find-skills", tree_hash)));
    source.set_file(
        "vercel-labs/skills",
        "skills/find-skills/SKILL.md",
        "---\nname: find-skills\ndescription: Find skills.\n---\n\n# Find Skills\n",
    );
    source.set_file(
        "vercel-labs/skills",
        "skills/find-skills/guide.md",
        "# GitHub guide\n",
    );
    let runtime = runtime_with_skills(&root, source, &local_root, &bundled_root);

    let list = runtime
        .execute(AutonomousToolRequest::Skill(CadenceSkillToolInput::List {
            query: Some("skill".into()),
            include_unavailable: false,
            limit: Some(10),
        }))
        .expect("list skills");
    let candidates = match list.output {
        AutonomousToolOutput::Skill(output) => {
            assert_eq!(output.status, AutonomousSkillToolStatus::Succeeded);
            output.candidates
        }
        other => panic!("unexpected output: {other:?}"),
    };
    let ids = candidates
        .iter()
        .map(|candidate| candidate.skill_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        ids,
        BTreeSet::from([
            "bundled-skill",
            "find-skills",
            "local-skill",
            "project-skill"
        ])
    );
    assert!(candidates
        .iter()
        .any(|candidate| candidate.source_kind == CadenceSkillSourceKind::Github));

    let bundled_source_id = candidates
        .iter()
        .find(|candidate| candidate.skill_id == "bundled-skill")
        .expect("bundled candidate")
        .source_id
        .clone();
    let local_source_id = candidates
        .iter()
        .find(|candidate| candidate.skill_id == "local-skill")
        .expect("local candidate")
        .source_id
        .clone();

    let approval_required = runtime
        .execute(AutonomousToolRequest::Skill(
            CadenceSkillToolInput::Invoke {
                source_id: local_source_id.clone(),
                approval_grant_id: None,
                include_supporting_assets: true,
            },
        ))
        .expect("invoke local skill without approval returns a typed boundary");
    match approval_required.output {
        AutonomousToolOutput::Skill(output) => {
            assert_eq!(output.status, AutonomousSkillToolStatus::ApprovalRequired);
            assert!(output.context.is_none());
        }
        other => panic!("unexpected output: {other:?}"),
    }

    let bundled = runtime
        .execute(AutonomousToolRequest::Skill(
            CadenceSkillToolInput::Invoke {
                source_id: bundled_source_id,
                approval_grant_id: None,
                include_supporting_assets: true,
            },
        ))
        .expect("invoke bundled skill");
    match bundled.output {
        AutonomousToolOutput::Skill(output) => {
            assert_eq!(output.status, AutonomousSkillToolStatus::Succeeded);
            let context = output.context.expect("bundled context");
            assert_eq!(context.skill_id, "bundled-skill");
            assert!(context.markdown.content.contains("# bundled-skill"));
            assert_eq!(context.supporting_assets.len(), 1);
        }
        other => panic!("unexpected output: {other:?}"),
    }

    let local = runtime
        .execute(AutonomousToolRequest::Skill(
            CadenceSkillToolInput::Invoke {
                source_id: local_source_id.clone(),
                approval_grant_id: Some("approval-1".into()),
                include_supporting_assets: true,
            },
        ))
        .expect("invoke approved local skill");
    match local.output {
        AutonomousToolOutput::Skill(output) => {
            assert_eq!(output.status, AutonomousSkillToolStatus::Succeeded);
            assert_eq!(
                output.selected.as_ref().map(|candidate| candidate.trust),
                Some(CadenceSkillTrustState::UserApproved)
            );
            assert_eq!(
                output.selected.as_ref().map(|candidate| candidate.state),
                Some(CadenceSkillSourceState::Enabled)
            );
            assert!(output
                .context
                .expect("local context")
                .markdown
                .content
                .contains("# local-skill"));
        }
        other => panic!("unexpected output: {other:?}"),
    }
    project_store::set_installed_skill_enabled(
        root.path(),
        &local_source_id,
        false,
        "2026-04-25T01:00:00Z",
    )
    .expect("disable local skill");
    let hidden = runtime
        .execute(AutonomousToolRequest::Skill(CadenceSkillToolInput::List {
            query: Some("local".into()),
            include_unavailable: false,
            limit: Some(10),
        }))
        .expect("list visible skills");
    match hidden.output {
        AutonomousToolOutput::Skill(output) => {
            assert!(output
                .candidates
                .iter()
                .all(|candidate| candidate.skill_id != "local-skill"));
        }
        other => panic!("unexpected output: {other:?}"),
    }
    let visible_for_diagnostics = runtime
        .execute(AutonomousToolRequest::Skill(CadenceSkillToolInput::List {
            query: Some("local".into()),
            include_unavailable: true,
            limit: Some(10),
        }))
        .expect("list unavailable skills");
    match visible_for_diagnostics.output {
        AutonomousToolOutput::Skill(output) => {
            let local = output
                .candidates
                .iter()
                .find(|candidate| candidate.skill_id == "local-skill")
                .expect("disabled local skill appears with diagnostics requested");
            assert_eq!(local.state, CadenceSkillSourceState::Disabled);
        }
        other => panic!("unexpected output: {other:?}"),
    }

    let github_source = cadence_desktop_lib::runtime::CadenceSkillSourceRecord::github_autonomous(
        cadence_desktop_lib::runtime::CadenceSkillSourceScope::project("project-1").unwrap(),
        &github_source_metadata("find-skills", tree_hash),
        CadenceSkillSourceState::Discoverable,
        CadenceSkillTrustState::Trusted,
    )
    .expect("github source")
    .source_id;
    let github = runtime
        .execute(AutonomousToolRequest::Skill(
            CadenceSkillToolInput::Invoke {
                source_id: github_source,
                approval_grant_id: None,
                include_supporting_assets: true,
            },
        ))
        .expect("invoke github skill from discovery cache");
    match github.output {
        AutonomousToolOutput::Skill(output) => {
            assert_eq!(output.status, AutonomousSkillToolStatus::Succeeded);
            assert_eq!(
                output.context.expect("github context").skill_id,
                "find-skills"
            );
        }
        other => panic!("unexpected output: {other:?}"),
    }
}

#[test]
fn skill_tool_refreshes_stale_bundled_skill_before_invocation() {
    let root = tempfile::tempdir().expect("temp dir");
    init_project_state(root.path());
    let bundled_root = root.path().join("bundled-skills");
    write_skill(
        &bundled_root,
        "bundled-skill",
        "bundled-skill",
        "Bundled skill v1.",
    );

    let source = Arc::new(FixtureSkillSource::default());
    source.set_tree_response(Ok(AutonomousSkillSourceTreeResponse {
        entries: Vec::new(),
    }));
    let runtime_v1 =
        runtime_with_bundled_version(&root, source.clone(), &bundled_root, "2026.04.25");

    let list_v1 = runtime_v1
        .execute(AutonomousToolRequest::Skill(CadenceSkillToolInput::List {
            query: Some("bundled".into()),
            include_unavailable: false,
            limit: Some(10),
        }))
        .expect("list bundled skill");
    let source_id = match list_v1.output {
        AutonomousToolOutput::Skill(output) => output
            .candidates
            .iter()
            .find(|candidate| candidate.skill_id == "bundled-skill")
            .expect("bundled candidate")
            .source_id
            .clone(),
        other => panic!("unexpected output: {other:?}"),
    };

    runtime_v1
        .execute(AutonomousToolRequest::Skill(
            CadenceSkillToolInput::Invoke {
                source_id: source_id.clone(),
                approval_grant_id: None,
                include_supporting_assets: false,
            },
        ))
        .expect("invoke bundled v1");

    write_skill(
        &bundled_root,
        "bundled-skill",
        "bundled-skill",
        "Bundled skill v2.",
    );
    let runtime_v2 = runtime_with_bundled_version(&root, source, &bundled_root, "2026.04.26");
    let stale = runtime_v2
        .execute(AutonomousToolRequest::Skill(CadenceSkillToolInput::List {
            query: Some("bundled".into()),
            include_unavailable: false,
            limit: Some(10),
        }))
        .expect("list stale bundled skill");
    match stale.output {
        AutonomousToolOutput::Skill(output) => {
            let candidate = output
                .candidates
                .iter()
                .find(|candidate| candidate.skill_id == "bundled-skill")
                .expect("stale bundled candidate");
            assert_eq!(candidate.state, CadenceSkillSourceState::Stale);
            assert_eq!(candidate.source_kind, CadenceSkillSourceKind::Bundled);
        }
        other => panic!("unexpected output: {other:?}"),
    }

    let refreshed = runtime_v2
        .execute(AutonomousToolRequest::Skill(
            CadenceSkillToolInput::Invoke {
                source_id,
                approval_grant_id: None,
                include_supporting_assets: false,
            },
        ))
        .expect("refresh stale bundled skill");
    match refreshed.output {
        AutonomousToolOutput::Skill(output) => {
            assert_eq!(output.status, AutonomousSkillToolStatus::Succeeded);
            assert!(output
                .context
                .expect("refreshed context")
                .markdown
                .content
                .contains("Bundled skill v2."));
        }
        other => panic!("unexpected output: {other:?}"),
    }
}

#[test]
fn skill_tool_dynamic_candidates_start_disabled_untrusted_and_non_invocable() {
    let root = tempfile::tempdir().expect("temp dir");
    init_project_state(root.path());
    let source = Arc::new(FixtureSkillSource::default());
    source.set_tree_response(Ok(AutonomousSkillSourceTreeResponse {
        entries: Vec::new(),
    }));
    let runtime = AutonomousToolRuntime::new(root.path())
        .expect("runtime")
        .with_skill_tool(
            "project-1",
            skill_runtime(&root, source),
            Vec::new(),
            Vec::new(),
        );

    let created = runtime
        .execute(AutonomousToolRequest::Skill(
            CadenceSkillToolInput::CreateDynamic {
                skill_id: "dynamic-skill".into(),
                markdown:
                    "---\nname: dynamic-skill\ndescription: Dynamic skill.\n---\n\n# Dynamic\n"
                        .into(),
                supporting_assets: vec![CadenceSkillToolDynamicAssetInput {
                    relative_path: "notes.md".into(),
                    content: "# Notes\n".into(),
                }],
                source_run_id: Some("run-1".into()),
                source_artifact_id: Some("artifact-1".into()),
            },
        ))
        .expect("create dynamic skill");
    let source_id = match created.output {
        AutonomousToolOutput::Skill(output) => {
            assert_eq!(output.status, AutonomousSkillToolStatus::Succeeded);
            let selected = output.selected.expect("dynamic candidate");
            assert_eq!(selected.state, CadenceSkillSourceState::Disabled);
            assert_eq!(selected.trust, CadenceSkillTrustState::Untrusted);
            selected.source_id
        }
        other => panic!("unexpected output: {other:?}"),
    };

    let duplicate = runtime
        .execute(AutonomousToolRequest::Skill(
            CadenceSkillToolInput::CreateDynamic {
                skill_id: "dynamic-skill".into(),
                markdown: "---\nname: dynamic-skill\ndescription: Dynamic skill updated.\n---\n\n# Dynamic\n"
                    .into(),
                supporting_assets: vec![CadenceSkillToolDynamicAssetInput {
                    relative_path: "notes.md".into(),
                    content: "# Notes updated\n".into(),
                }],
                source_run_id: Some("run-1".into()),
                source_artifact_id: Some("artifact-1".into()),
            },
        ))
        .expect("merge duplicate dynamic skill");
    match duplicate.output {
        AutonomousToolOutput::Skill(output) => {
            assert_eq!(output.status, AutonomousSkillToolStatus::Succeeded);
            assert_eq!(
                output
                    .selected
                    .as_ref()
                    .map(|candidate| candidate.source_id.as_str()),
                Some(source_id.as_str())
            );
        }
        other => panic!("unexpected output: {other:?}"),
    }

    let hidden = runtime
        .execute(AutonomousToolRequest::Skill(CadenceSkillToolInput::List {
            query: Some("dynamic".into()),
            include_unavailable: false,
            limit: Some(10),
        }))
        .expect("list visible dynamic skills");
    match hidden.output {
        AutonomousToolOutput::Skill(output) => assert!(output.candidates.is_empty()),
        other => panic!("unexpected output: {other:?}"),
    }

    let diagnostic = runtime
        .execute(AutonomousToolRequest::Skill(CadenceSkillToolInput::List {
            query: Some("dynamic".into()),
            include_unavailable: true,
            limit: Some(10),
        }))
        .expect("list unavailable dynamic skills");
    match diagnostic.output {
        AutonomousToolOutput::Skill(output) => {
            assert_eq!(output.candidates.len(), 1);
            assert_eq!(
                output.candidates[0].source_kind,
                CadenceSkillSourceKind::Dynamic
            );
        }
        other => panic!("unexpected output: {other:?}"),
    }

    let rejected = runtime
        .execute(AutonomousToolRequest::Skill(
            CadenceSkillToolInput::Invoke {
                source_id,
                approval_grant_id: Some("approval-1".into()),
                include_supporting_assets: true,
            },
        ))
        .expect("dynamic invoke returns typed failure");
    match rejected.output {
        AutonomousToolOutput::Skill(output) => {
            assert_eq!(output.status, AutonomousSkillToolStatus::Failed);
            assert!(output.context.is_none());
            assert!(output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "skill_tool_source_not_enabled"));
        }
        other => panic!("unexpected output: {other:?}"),
    }
}
