# Legacy Compatibility Removal Audit

Date: 2026-04-28

This document inventories code that appears to exist primarily for backwards compatibility, legacy data migration, old runtime shapes, or deprecated project-local state. The app is new and backwards compatibility is prohibited unless explicitly requested, so the default recommendation is to remove these paths after confirming the current fresh-install flows are covered by tests.

## Summary

The largest compatibility surface is in the Tauri backend. A fresh app startup still imports old JSON files, global SQLite still creates and backfills old provider-profile tables, and several runtime commands synthesize old runtime/provider-profile snapshots for consumers that have not fully moved to flat provider credentials.

The next largest bucket is project-store migration history: deprecated workflow tables, old autonomous-unit schema, and a one-shot SQLite-to-Lance agent memory migration still run or remain in tests. There are also product-level compatibility choices, especially the legacy dictation engine, the Solana wallet-adapter scaffold, and the browser first-tab legacy label.

## High-Confidence Removal Targets

### 1. Legacy JSON Import Pipeline

These files import old repo/app JSON state into the current global database. In a new app with no backwards compatibility requirement, this entire startup migration path can be removed.

Primary files:

- `client/src-tauri/src/lib.rs`
  - Startup setup resolves `LegacyJsonImportPaths` and runs `run_legacy_json_imports`.
- `client/src-tauri/src/global_db/mod.rs`
  - Exports importer helpers.
  - Defines legacy JSON filenames such as `provider-profiles.json`, `provider-profile-credentials.json`, `runtime-settings.json`, `openrouter-credentials.json`, `openai-auth.json`, `notification-credentials.json`, `dictation-settings.json`, `skill-sources.json`, `mcp-registry.json`, `provider-model-catalogs.json`, and `project-registry.json`.
  - Defines `LegacyJsonImportPaths`.
  - Orchestrates `run_legacy_json_imports`.
- `client/src-tauri/src/global_db/importer.rs`
  - One-shot import of dictation settings, skill sources, MCP registry, provider model catalog cache, plus helper cleanup/removal of imported JSON files.
- `client/src-tauri/src/global_db/legacy_runtime_settings.rs`
  - Decodes and validates old `runtime-settings.json` and OpenRouter credential shapes.
- `client/src-tauri/src/provider_profiles/importer.rs`
  - Imports current-schema provider profile JSON, pre-profile runtime settings, OpenRouter credentials, and OpenAI auth into provider profiles.
- `client/src-tauri/src/auth/importer.rs`
  - Imports `openai-auth.json` into `openai_codex_sessions`.
- `client/src-tauri/src/notifications/credential_store/importer.rs`
  - Imports `notification-credentials.json`.
- `client/src-tauri/src/registry.rs`
  - Imports `project-registry.json`.
- `client/src-tauri/src/auth/mod.rs`
  - Re-exports `import_legacy_openai_codex_sessions`.
- `client/src-tauri/src/notifications/mod.rs`
  - Re-exports `import_legacy_notification_credentials`.

Removal shape:

- Delete the importer modules and legacy runtime settings decoder.
- Remove startup import calls from `lib.rs`.
- Remove legacy filename/path types from `global_db/mod.rs`.
- Remove importer-specific tests and fixtures.
- Keep only current database initialization and current settings persistence.

Risk:

- Low for new installs.
- Medium if existing developer machines still depend on old JSON files.

Confidence: High.

### 2. Provider Profile Compatibility Layer

Flat provider credentials appear to be the current direction, but old provider-profile tables and DTOs are still used to keep older callers compiling.

Primary files:

- `client/src-tauri/src/provider_credentials/mod.rs`
  - Notes that the flat provider credential store coexists with legacy `provider_profiles` until a later phase.
- `client/src-tauri/src/provider_credentials/readiness.rs`
  - Readiness proof mirrors the older provider-profile readiness proof.
- `client/src-tauri/src/commands/provider_profiles.rs`
  - Comments say old provider-profile commands were deleted, but an internal snapshot loader remains for legacy consumers.
- `client/src-tauri/src/provider_profiles/mod.rs`
  - Loads a synthesized provider profile snapshot from flat credentials.
- `client/src-tauri/src/provider_profiles/store.rs`
  - Keeps provider-profile metadata, migration state, `migrated_from_legacy`, `migrated_at`, legacy credential file shapes, serde aliases, synthesized snapshots, and old credential merging.
- `client/src-tauri/src/provider_profiles/sql.rs`
  - Reads and writes old provider-profile tables and migration metadata.
- `client/src-tauri/src/auth/store.rs`
  - Mirrors OpenAI OAuth state into provider credentials when legacy provider-profile snapshot flows change.
- `client/src-tauri/src/commands/contracts/runtime.rs`
  - Keeps old `RuntimeSettingsDto`, `ProviderProfileDto`, provider-profile migration DTOs, and old provider-profile command request DTOs.
- `client/src-tauri/src/commands/get_runtime_settings.rs`
  - Builds old runtime-settings snapshots from provider-profile snapshots.
- `client/src-tauri/src/commands/get_runtime_session.rs`
  - Loads provider-profile snapshots and derives runtime settings from them.
- `client/src-tauri/src/commands/runtime_support/run.rs`
  - Builds runtime-settings snapshots and falls back to legacy `openrouter_api_key` and `anthropic_api_key` fields.
- `client/src/lib/cadence-model/runtime.ts`
  - Still writes runtime-settings compatibility fields for older providers.

Known current consumers to migrate before deletion:

- `client/src-tauri/src/provider_models/mod.rs`
- `client/src-tauri/src/runtime/provider.rs`
- `client/src-tauri/src/runtime/diagnostics.rs`
- `client/src-tauri/src/commands/doctor_report.rs`
- `client/src-tauri/src/commands/provider_diagnostics.rs`
- `client/src-tauri/src/commands/get_runtime_session.rs`
- `client/src-tauri/src/commands/runtime_support/run.rs`

Tests and fixtures:

- `client/src/test/legacy-provider-profiles.ts`
- `client/components/cadence/onboarding/steps/providers-step.test.tsx`
- `client/components/cadence/settings-dialog.test.tsx`
- `client/src/features/cadence/use-cadence-desktop-state.runtime-run.test.tsx`
- `client/src/lib/cadence-model.test.ts`

Removal shape:

- Make provider credentials the only backend source of provider auth and readiness.
- Delete old provider-profile stores, SQL helpers, migration metadata, compatibility DTOs, and skipped legacy fixtures.
- Update runtime commands to request only flat provider credentials.
- Remove legacy fallback key fields from frontend models once backend no longer emits them.

Risk:

- Medium. The code comments already name several legacy consumers, so this should be done as a coordinated runtime/provider refactor rather than by deleting files first.

Confidence: High.

### 3. Global Database Legacy Provider Schema

The global database migration still creates old provider-profile tables and immediately creates the newer provider-credential table next to them.

Primary file:

- `client/src-tauri/src/global_db/migrations.rs`
  - Initial schema creates `provider_profiles`, `provider_profiles_metadata`, `provider_profile_credentials`, `openai_codex_sessions`, and singleton JSON-like tables.
  - Later schema creates `provider_credentials`.
  - Comments say the old provider-profile triplet remains only during transition.
  - Backfill code migrates old provider-profile rows into `provider_credentials`.

Removal shape:

- Collapse global DB migrations into a new-app baseline schema.
- Create only the current tables needed by the app.
- Drop provider-profile triplet tables and their backfill.
- Remove `migrated_from_legacy` and migration-state columns unless they have a current product purpose.

Risk:

- Low for new installs.
- Medium if tests assume historical migration behavior.

Confidence: High.

### 4. Deprecated Workflow And Autonomous-Unit Migration History

The per-project SQLite migration chain still contains old workflow tables, old autonomous-unit shapes, transitional session-scoped copies, and tests asserting that deprecated schema was eventually dropped.

Primary file:

- `client/src-tauri/src/db/migrations.rs`
  - Creates old workflow tables such as workflow phases, graph nodes, graph edges, gates, transitions, and handoff records.
  - Adds workflow columns to autonomous unit tables.
  - Creates session-scoped autonomous tables while copying old workflow columns.
  - Drops old autonomous/runtime tables.
  - Drops deprecated workflow and autonomous unit tables.
  - Adds and later drops workflow columns on operator approval tables.
  - Contains tests asserting deprecated workflow tables and columns are absent.

Related live note:

- `client/src-tauri/src/db/project_store/operator.rs`
  - Operator tables are live, but comments say they are decoupled from the deprecated workflow system.

Removal shape:

- Replace the long historical project DB migration sequence with a fresh baseline for current schema.
- Remove creation/drop churn for workflow tables and transitional autonomous-unit schemas.
- Keep current operator approval tables only if they are product-current.
- Rewrite migration tests around the new baseline and current invariants.

Risk:

- Medium. This touches the project database bootstrap and should be verified with project creation/opening tests.

Confidence: High.

### 5. SQLite Agent Memory To Lance Migration

Agent memory appears to have moved from SQLite to LanceDB, but the old SQLite table migration and pending-import drain are still present.

Primary files:

- `client/src-tauri/src/db/project_store/agent_memory_migration.rs`
  - One-shot SQLite-to-Lance migration.
  - Reads legacy SQLite `agent_memories`.
  - Writes pending import state.
  - Drops old SQLite memory table, indexes, and triggers.
  - Includes migration tests.
- `client/src-tauri/src/db/project_store/connection.rs`
  - Drains pending Lance imports when project databases open.
- `client/src-tauri/src/db/project_store/mod.rs`
  - Includes and re-exports migration helpers.
- `client/src-tauri/src/db/project_store/agent_memory_lance.rs`
  - Defines `agent_memories.lance-pending.json` for staged legacy import.
- `client/src-tauri/src/db/migrations.rs`
  - Creates the old SQLite `agent_memories` table and triggers before later migration hooks run.

Removal shape:

- Make LanceDB the only agent memory backend.
- Remove the SQLite table creation, migration hook, pending import file, and pending-drain path.
- Keep only LanceDB initialization and current memory APIs.

Risk:

- Medium, because LanceDB setup needs a clean fresh-install test and `protoc` must be on `PATH` for builds.

Confidence: High.

### 6. Repo-Local `.cadence` State

Project-local `.cadence/` is explicitly marked legacy in the repo instructions, but several code paths still read, write, exclude, or document it.

Primary files:

- `client/src-tauri/src/git/repository.rs`
  - Ensures `.cadence/` is ignored in `.git/info/exclude`.
- `client/components/cadence/project-rail.tsx`
  - User-facing copy says local `.cadence` database and state remain untouched when a project is removed.
- `client/src-tauri/src/runtime/autonomous_skill_runtime/discovery.rs`
  - Scans project skills from `.cadence/skills`.
- `client/src-tauri/src/runtime/autonomous_tool_runtime/skills.rs`
  - Writes dynamic skills under `.cadence/dynamic-skills`.
- `client/src-tauri/src/runtime/autonomous_skill_runtime/settings.rs`
  - Defaults local and plugin roots to `~/.cadence/skills` and `~/.cadence/plugins`.
- `client/src-tauri/src/runtime/autonomous_tool_runtime/filesystem.rs`
  - Skips `.cadence` during file search.
- `client/src-tauri/src/runtime/autonomous_tool_runtime/repo_scope.rs`
  - Skips `.cadence` during repo-scoped traversal.

Docs to update:

- `docs/skills-and-plugins.md`
- `README.md`

Removal shape:

- Move project state and generated skill artifacts to OS app-data.
- Delete repo-local `.cadence` discovery and write paths.
- Remove `.cadence/` git-exclude management once the app no longer writes it.
- Update settings, docs, tests, and UI copy to refer to the app-data location.

Risk:

- Medium. The replacement app-data layout should be named before deleting the project-local paths.

Confidence: High.

## Product-Decision Compatibility Targets

### 7. Legacy Dictation Engine And Fallback

The dictation stack supports both a modern engine and a legacy `SFSpeechRecognizer` engine. This may be a deliberate product fallback rather than only backwards compatibility, so it needs a product decision.

Primary files:

- `client/src-tauri/native/dictation/LegacyEngine.swift`
- `client/src-tauri/native/dictation/SessionLifecycle.swift`
- `client/src-tauri/native/dictation/CapabilityStatus.swift`
- `client/src-tauri/src/commands/contracts/dictation.rs`
- `client/src-tauri/src/commands/dictation.rs`
- `client/src-tauri/src/commands/doctor_report.rs`
- `client/src/lib/cadence-model/dictation.ts`
- `client/components/cadence/settings-dialog/dictation-section.tsx`
- `client/components/cadence/agent-runtime/use-speech-dictation.ts`
- `client/src-tauri/tests/dev_runner_contract.rs`

Removal shape:

- Remove `DictationEnginePreference::Legacy` and all legacy status/remediation fields.
- Delete `LegacyEngine.swift`.
- Remove modern-to-legacy fallback.
- Update settings UI to expose only current engine choices.
- Rewrite dictation tests around the modern engine only.

Risk:

- Medium to high, depending on whether legacy dictation is still needed for unsupported OS/hardware combinations.

Confidence: Medium.

### 8. Browser First-Tab Legacy Label

The browser command layer keeps a special `cadence-browser` webview label as an alias for the first tab.

Primary files:

- `client/src-tauri/src/commands/browser/tabs.rs`
  - Defines `BROWSER_LEGACY_LABEL = "cadence-browser"`.
- `client/src-tauri/src/commands/browser/mod.rs`
  - Exports the legacy label as `BROWSER_WEBVIEW_LABEL`.
  - Assigns the first tab the legacy label.
  - Keeps first-tab fallback metadata.

Removal shape:

- Use only tab-specific labels such as `cadence-browser-tab-*`.
- Update any screenshot, IPC, or metadata callers to address explicit tab IDs.
- Delete the legacy first-tab alias and fallback metadata path.

Risk:

- Medium. Browser automation and screenshots often depend on stable webview labels.

Confidence: Medium.

### 9. Solana Legacy Wallet Adapter Scaffold

The Solana wallet generator exposes both a legacy Wallet Adapter scaffold and the newer Wallet Standard scaffold.

Primary files:

- `client/src-tauri/src/commands/solana/wallet/mod.rs`
  - Documents Wallet Adapter as legacy and Wallet Standard as the recommended path.
- `client/src-tauri/src/commands/solana/wallet/wallet_adapter.rs`
  - Generates a full scaffold using `@solana/wallet-adapter-react`.
- `client/src-tauri/src/commands/solana/wallet/wallet_standard.rs`
  - Notes it is smaller than the legacy path.

Removal shape:

- Remove `WalletKind::WalletAdapter`.
- Delete the wallet-adapter scaffold generator.
- Keep Wallet Standard as the only wallet scaffold unless there is a current product requirement for the old adapter.

Risk:

- Low to medium.

Confidence: Medium.

## Smaller Compatibility Shims

These are narrower and can be removed opportunistically once nearby code is being touched.

### 10. State Override Compatibility Helpers

Primary file:

- `client/src-tauri/src/state.rs`
  - Per-file overrides were collapsed to a single global database override.
  - Old `with_*_file_override` helpers remain as compatibility/test builder shims.

Removal shape:

- Update tests and callers to use the current global DB override directly.
- Delete old per-file override helpers and path resolver compatibility methods.

Confidence: High.

### 11. Status Footer USD Fallback

Primary files:

- `client/components/cadence/status-footer.tsx`
  - Accepts `totalUsd` as a backwards-compatible fallback.
- `client/src/App.tsx`
  - Appears to pass `totalCostMicros`, which is the current field.

Removal shape:

- Remove `totalUsd` prop support and conversion to micros.

Confidence: High.

### 12. Solana Logs Status Compatibility Action

Primary file:

- `client/src-tauri/src/runtime/autonomous_tool_runtime/solana.rs`
  - Keeps a backwards-compatible `Status` action for Solana logs.

Removal shape:

- Remove the compatibility action if the current tool contract no longer documents or uses it.

Confidence: Medium.

### 13. iOS Emulator Manual-Layout Fallbacks

Primary file:

- `client/src-tauri/src/commands/emulator/ios/xcrun.rs`
  - Resolves older/manual resource layouts for `idb_companion`.

Removal shape:

- Keep only the current bundled resource layout if manual/old layouts are no longer supported.

Confidence: Medium.

### 14. Wire Alias Tolerance

Primary files:

- `client/src-tauri/src/provider_profiles/store.rs`
  - Accepts `openrouter` and `anthropic` aliases for `api_key`.
- `client/src-tauri/src/commands/contracts/runtime.rs`
  - Accepts `o_auth_session` as an alias for `oauth_session`.
- `client/src-tauri/src/runtime/autonomous_tool_runtime/browser.rs`
  - Accepts camelCase aliases such as `tabId` and `timeoutMs` alongside snake_case fields.

Removal shape:

- Remove serde aliases that exist only for old client payloads.
- Be careful with tool-runtime aliases: camelCase support may be model ergonomics rather than legacy compatibility.

Confidence: Low to medium.

## Tests And Fixtures To Revisit

These test surfaces are directly tied to compatibility code and should be deleted or rewritten as part of the refactor.

- Backend importer tests embedded in:
  - `client/src-tauri/src/global_db/mod.rs`
  - `client/src-tauri/src/global_db/importer.rs`
  - `client/src-tauri/src/global_db/legacy_runtime_settings.rs`
  - `client/src-tauri/src/provider_profiles/importer.rs`
  - `client/src-tauri/src/auth/importer.rs`
  - `client/src-tauri/src/notifications/credential_store/importer.rs`
  - `client/src-tauri/src/registry.rs`
  - `client/src-tauri/src/db/project_store/agent_memory_migration.rs`
- Frontend compatibility fixtures/tests:
  - `client/src/test/legacy-provider-profiles.ts`
  - `client/components/cadence/onboarding/steps/providers-step.test.tsx`
  - `client/components/cadence/settings-dialog.test.tsx`
  - `client/src/features/cadence/use-cadence-desktop-state.runtime-run.test.tsx`
  - `client/src/lib/cadence-model.test.ts`
- Dictation compatibility tests:
  - `client/src/lib/cadence-model/dictation.test.ts`
  - `client/src/lib/cadence-desktop.dictation.test.ts`
  - `client/src-tauri/tests/dev_runner_contract.rs`

## Suggested Removal Order

1. Add or confirm fresh-install tests for global DB bootstrap, project DB bootstrap, provider credentials, runtime session creation, and dictation if dictation is in scope.
2. Remove the legacy JSON import pipeline and its tests.
3. Collapse global DB migrations to a current baseline and remove provider-profile table backfill.
4. Replace provider-profile snapshot consumers with flat provider credentials, then delete provider-profile compatibility modules, DTOs, and frontend fixtures.
5. Collapse project DB migrations to a current baseline and remove deprecated workflow/autonomous migration churn.
6. Remove SQLite-to-Lance agent memory migration once LanceDB fresh-install behavior is covered.
7. Move `.cadence/` project state to the chosen OS app-data layout and update docs/UI copy.
8. Decide whether to remove product-level compatibility features: legacy dictation, browser first-tab legacy label, and Solana wallet adapter.
9. Sweep smaller serde aliases and prop fallbacks after the main compatibility layers are gone.

## Not Counted As Removal Targets

These looked legacy-adjacent during the scan but do not appear to be backwards-compatibility code by themselves.

- `openai_compatible` provider support. This is a current provider family, not necessarily a compatibility shim.
- Solana token compatibility matrix logic. This appears to be product functionality.
- `@codemirror/legacy-modes` usage. The package name is historical; the language modes may still be current editor functionality.
- Android Java version parsing for `java version "1.8..."`. This is robust version parsing, not app backwards compatibility.
- Server Phoenix migration files under `server/priv/repo/migrations`. These are normal server schema history unless the server schema is also being reset for a new-app baseline.
