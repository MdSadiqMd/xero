# Storage Layer Refactor — Phased Plan

## Goals

1. **Global SQLite** for everything non-project-scoped: credentials, API keys, OAuth sessions, provider profiles, notification credentials, runtime/dictation/skill settings, MCP registry, project registry, model catalog cache.
2. **Per-project SQLite** for project state, but **relocated out of the repo** to the user's app-data directory. No more `.cadence/state.db` polluting the working tree.
3. **LanceDB per-project** for `agent_memories` only — the one workload that actually benefits from a vector store. Everything else relational stays in SQLite.
4. **Obfuscation**: no Cadence DB files inside any project repo. All storage lives under `~/Library/Application Support/dev.sn0w.cadence/` (macOS app-data).
5. **Remove the deprecated workflow state machine** — schema and code.

## Scope clarification

The "old workflow state machine" being removed is the six `workflow_*` tables and the `db/project_store/workflow/` module. The **operator tables (`operator_approvals`, `operator_verification_records`, `operator_resume_history`) stay** — verified to be the live human-in-the-loop approval gate for the autonomous tool runtime (`*_with_operator_approval` methods in `runtime/autonomous_tool_runtime/`), not part of the deprecated workflow despite naming overlap. Their schema was refactored to drop workflow coupling (no more `gate_id` / `transition_target` columns).

## Final layout (target state)

```
~/Library/Application Support/dev.sn0w.cadence/
├── cadence.db                                 # global SQLite (credentials, settings, registry)
├── cadence.db-wal                             # WAL sidecar
├── cadence.db-shm
├── projects/
│   └── <project-uuid>/
│       ├── state.db                           # per-project SQLite (sessions, runs, operator, autonomous)
│       ├── state.db-wal
│       ├── state.db-shm
│       └── lance/
│           └── agent_memories.lance/          # vector store for agent memory
├── autonomous-skills/                         # existing cache, unchanged
├── provider-model-catalog-cache.json (deleted after Phase 2)
└── window-state.json                          # remains JSON (UI state, not data)
```

Nothing inside any user repo. `<repo>/.cadence/` is gone.

---

## Phase 0 — Design lock-in (no code)

Decisions to record before coding begins:

- **Project identity**: per-project state is keyed by a UUID stored in the global `projects` table. Lookup by canonical repo path (the existing `CanonicalRepository` resolution).
- **Repo move/rename**: out of scope for this refactor. Tracked separately as a re-link UX ticket. Initial behavior: a moved repo loses its state until the user re-imports.
- **Encryption**: plaintext SQLite + filesystem permissions (0700 dir, 0600 files) for v1. SQLCipher for the global DB (which holds credentials) is a follow-up, not in this batch. Plaintext is no worse than today's plaintext JSON.
- **Migration model**: one-shot importer at startup, gated by a schema-version row. Legacy files deleted only after successful import. Re-running the importer is a no-op.
- **Cross-machine sync**: per-project state becomes machine-local. Users who synced `.cadence/` via dotfiles or cloud drives lose that capability. Documented as a deliberate trade-off.

---

## Phase 1 — Global SQLite skeleton

**New module**: `client/src-tauri/src/global_db/` with `mod.rs`, `migrations.rs`, table modules.

- Path: `<app-data>/cadence.db`, resolved via `DesktopState::global_db_path()` with the existing test-override pattern.
- Open with `Connection::open` + `rusqlite_migration` + `PRAGMA journal_mode=WAL`, `PRAGMA foreign_keys=ON`, `PRAGMA synchronous=NORMAL`.
- One initial migration covering all tables:
  - `provider_profiles` (replaces `provider-profiles.json` metadata)
  - `provider_profile_credentials` (replaces `provider-profile-credentials.json`)
  - `openai_codex_sessions` (replaces `openai-auth.json`)
  - `notification_credentials`, `notification_inbound_cursors` (replaces `notification-credentials.json`)
  - `runtime_settings`, `dictation_settings`, `skill_sources` (one row each, singleton via `id = 1` PK)
  - `mcp_registry` (one row per server)
  - `provider_model_catalog_cache` (replaces JSON cache)
  - `projects`, `repositories` (replaces `project-registry.json` and the duplicated copies in every per-repo DB)
- FK constraints replace the hand-rolled validators in `provider_profiles/store.rs::validate_provider_profiles_contract`.
- Tests pin schema by walking migrations end-to-end on `Connection::open_in_memory()`.

**Single override builder**: `DesktopState::with_global_db_path_override(PathBuf)` replaces all the per-file `with_*_file_override` builders that come up in Phase 6 cleanup.

---

## Phase 2 — Port each global store to SQLite

One commit per store. Each ships independently and deletes its JSON path after a successful import.

### 2.1 — `provider_profiles` + `provider_profile_credentials`
- Replace JSON I/O in `provider_profiles/store.rs`. Keep the public API (`ProviderProfilesSnapshot`, `persist_provider_profiles_snapshot`, `load_provider_profiles_from_paths`) so callers don't change.
- Importer reads `provider-profiles.json` + `provider-profile-credentials.json` once, writes rows, deletes files.
- Drop `OPENROUTER_PROFILE_CREDENTIAL_FILE_NAME` legacy alias handling — superseded by SQL.

### 2.2 — `openai_codex_sessions`
- Replace JSON I/O in `auth/store.rs`. Same pattern. Existing functions (`load_openai_codex_session`, `persist_openai_codex_session`, `clear_openai_codex_sessions`, `sync_openai_profile_link`) keep their signatures.

### 2.3 — `notification_credentials` + `notification_inbound_cursors`
- Replace `notifications/credential_store/file_store.rs::FileNotificationCredentialStore` with a SQLite-backed equivalent. Keep the same trait surface.
- Migrate both `routes` and `inbound_cursors` arrays.

### 2.4 — `runtime_settings`, `dictation_settings`, `skill_sources`, `mcp_registry`, `provider_model_catalog_cache`
- Each is a small store with one or few rows. Straightforward upsert pattern.
- `mcp_registry` keeps `from_env` references (env-var pointers, not values) — schema unchanged in spirit.

### 2.5 — `projects` + `repositories`
- Replaces `project-registry.json`.
- Removes the duplicated `projects` and `repositories` tables from every per-repo DB (those are now globally authoritative).
- Per-repo DB drops these two tables in Phase 3's migration.

### 2.6 — Importer orchestration
- Single `run_legacy_json_imports(state)` called once at app startup.
- Idempotent: each importer skips when the global DB already has the data, runs only when the legacy JSON exists.
- After a successful import, JSON files are removed.

### 2.7 — Override-builder cleanup
- Drop these from `DesktopState`: `with_auth_store_file_override`, `with_notification_credential_store_file_override`, `with_provider_profiles_file_override`, `with_provider_profile_credential_store_file_override`, `with_provider_model_catalog_cache_file_override`, `with_runtime_settings_file_override`, `with_dictation_settings_file_override`, `with_mcp_registry_file_override`, `with_skill_source_settings_file_override`, `with_openrouter_credential_file_override`, `with_registry_file_override`.
- Tests use `with_global_db_path_override(tempdir.path().join("cadence.db"))` instead.

---

## Phase 3 — Relocate per-project SQLite out of the repo

- Replace `db::database_path_for_repo(repo_root)` (`db/mod.rs:30-32`) with `database_path_for_project(project_id)` resolving to `<app-data>/projects/<project_id>/state.db`.
- All `db::project_store::*` functions take `project_id` instead of `repo_root` for path resolution. `repo_root` is preserved only as identity/lookup metadata.
- On startup, for each project in the global `projects` table:
  - If `<repo>/.cadence/state.db` exists, move it (plus `-wal`, `-shm` sidecars) to the new location.
  - `rm -rf <repo>/.cadence` after successful move.
  - Idempotent: if the target already exists, do nothing.
- `import_repository` writes to the new path immediately; never creates `.cadence/` again.
- Per-repo DB migration: **drop `projects` and `repositories` tables** (they live globally now). Replace with a small `meta` table holding the `project_id` that this DB belongs to, for sanity checks if the file is moved.
- Update `AGENTS.md` / `README.md` guidance: `.cadence/` is legacy. Existing `.gitignore` entries can stay; new repos won't need them.

---

## Phase 4 — LanceDB for `agent_memories`

- Add `lancedb` to `client/src-tauri/Cargo.toml`.
- Per-project dataset at `<app-data>/projects/<project_id>/lance/agent_memories.lance/`.
- Move only `agent_memories` out of SQLite. Schema:
  - `id` (string, PK)
  - `agent_id` (string)
  - `created_at` (timestamp)
  - `content` (string)
  - `metadata` (json string)
  - `embedding` (vector\<f32, N\>, nullable)
- Embedding population is opt-in. No model wired yet; writes work without embeddings; queries fall back to scan when no vectors exist.
- Migration: read all rows from per-project SQLite `agent_memories`, write to Lance, drop the SQLite table in the same migration.
- Update `db/project_store/agent_memory.rs` to read/write Lance instead of `rusqlite`. Public API stays the same.
- Everything else (sessions, runs, tool calls, checkpoints, autonomous, operator) stays in SQLite — relational and transactional.

---

## Phase 5 — Workflow removal

Recommended to land **first** as a clean precondition (it doesn't touch the storage refactor and shrinks the surface area).

- New per-project SQLite migration that drops:
  - `workflow_phases`
  - `workflow_graph_nodes`
  - `workflow_graph_edges`
  - `workflow_gate_metadata`
  - `workflow_transition_events`
  - `workflow_handoff_packages`
- Drop any surviving workflow-era indexes on `operator_approvals` (verify whether `idx_operator_approvals_project_gate_status_updated` and `idx_operator_approvals_project_transition_target` still exist in the schema after later migrations dropped their backing columns; remove if so).
- Delete `client/src-tauri/src/db/project_store/workflow/` (the four files: `mod.rs`, `queries.rs`, `sql.rs`, `transition.rs`, `types.rs` — ~94 lines total).
- Move the one survivor — `read_project_row` in `workflow/queries.rs` — into `project_snapshot.rs` or inline at its single call site.
- Strip from `db/project_store/mod.rs`:
  - `pub(crate) mod workflow;`
  - `pub use workflow::*;`
  - `pub(crate) use workflow::{ ... };`
- Frontend/contract sweep: search for workflow phase/gate/transition types in `client/src-tauri/src/commands/contracts/` and `client/src/lib/cadence-model/`. Remove dead types and DTOs.
- Operator tables explicitly stay. Add a comment in `operator.rs` noting the table was decoupled from the deprecated workflow.

**Operator approval verification**: confirmed live via:
- `runtime/autonomous_tool_runtime/mod.rs:734-746` — dispatches all tool requests through `*_with_operator_approval` paths.
- `runtime/autonomous_tool_runtime/process.rs:361,608` — `command_with_operator_approval`, `command_session_start_with_operator_approval`.
- `runtime/autonomous_tool_runtime/filesystem.rs:47` — `read_with_operator_approval`.
- `runtime/autonomous_tool_runtime/priority_tools.rs:636` — `powershell_with_operator_approval`.
- `runtime/autonomous_tool_runtime/process_manager.rs:845` — `process_manager_with_operator_approval`.
- `runtime/autonomous_tool_runtime/macos_automation.rs:32` — `macos_automation_with_operator_approval`.
- `runtime/stream/preflight.rs:9,204` — runtime stream filters `OperatorApprovalStatus::Pending` to gate the agent.
- `commands/resolve_operator_action.rs`, `commands/resume_operator_run.rs` — Tauri commands wired in `lib.rs:198-199`.
- Frontend: `client/src/App.tsx`, `client/src/features/cadence/use-cadence-desktop-state.ts`, `mutation-support.ts`, `agent-runtime-projections/checkpoint-control-loops.ts`, `lib/cadence-model.ts`.

---

## Phase 6 — Cleanup

- Delete `provider_profiles/migration.rs` (legacy v1 → v3 JSON migration). Superseded by the SQLite importer; the in-DB migration path handles schema upgrades.
- Delete the JSON read/write helpers in `commands/get_runtime_settings.rs` (`read_openrouter_credentials_file`, `validate_runtime_settings_contract` JSON branch, etc.).
- Delete file-name constants:
  - `OPENROUTER_CREDENTIAL_FILE_NAME`
  - `OPENAI_CODEX_AUTH_STORE_FILE_NAME`
  - `ANTHROPIC_AUTH_STORE_FILE_NAME`
  - `NOTIFICATION_CREDENTIAL_STORE_FILE_NAME`
  - `PROVIDER_PROFILES_FILE_NAME`
  - `PROVIDER_PROFILE_CREDENTIAL_STORE_FILE_NAME`
  - `RUNTIME_SETTINGS_FILE_NAME`
  - `DICTATION_SETTINGS_FILE_NAME`
  - `MCP_REGISTRY_FILE_NAME`
  - `SKILL_SOURCE_SETTINGS_FILE_NAME`
  - `REGISTRY_FILE_NAME`
  - `PROVIDER_MODEL_CATALOG_CACHE_FILE_NAME`
- Test sweep: replace tempdir+JSON fixtures with `Connection::open_in_memory()` for global-DB tests. Per-project tests already use tempdirs and only need new path plumbing.
- File-mode hardening: at app start, set `0700` on the app-data directory and `0600` on `cadence.db` and per-project `state.db`. Currently no permission hardening exists.

---

## Sequencing

| Order | Phase | Notes |
|-------|-------|-------|
| 1 | Phase 5 | Workflow removal — standalone, lands first as a clean precondition. |
| 2 | Phase 0 | Design decisions recorded, no code. |
| 3 | Phase 1 | Global SQLite skeleton + migration scaffolding. |
| 4 | Phase 2 | Port stores one at a time (2.1 → 2.7). Each is a shippable commit. |
| 5 | Phase 3 | Relocate per-project DB out of the repo. |
| 6 | Phase 4 | LanceDB for `agent_memories` (optional — can be deferred). |
| 7 | Phase 6 | Cleanup, dead-code removal, file-mode hardening. |

Each phase is independently shippable. Phase 4 introduces the only new dependency (`lancedb`); if you want to defer that, drop Phase 4 and keep `agent_memories` in SQLite — the rest of the plan stands.

---

## Risks

- **Project identity on repo move**: until the re-link UX ships, a moved repo loses its state. Document and surface a clear error when state is missing.
- **Importer correctness**: each legacy JSON file has years of edge cases (legacy v1 schemas, malformed files, partial migrations). Ship comprehensive importer fixture tests before deleting the JSON paths.
- **Cross-machine sync regression**: users who relied on syncing `.cadence/` via dotfiles will lose project state portability. Deliberate trade-off; needs release-notes call-out.
- **Test surface churn**: hundreds of tests use file-path overrides today. Phase 1 and Phase 2.7 land the new override pattern; expect a wave of test refactors per store.
- **Operator-table workflow indexes**: verify in Phase 5 that no live SQL still references workflow-coupled indexes on `operator_approvals`. If `derive_operator_action_id` still consumes a `gate_link` parameter, decide whether to strip it or leave the parameter as `Option<()>` for future-proofing.
