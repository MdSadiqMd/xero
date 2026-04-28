# Provider Layer Refactor — Eliminate Profiles & Active Profile

## Status (2026-04-28)

Phases 3.1–3.7 + Phase 4 (1/n)–(9/n) shipped on `main`. The frontend
runs entirely on the credentials-driven UX:

- Settings → Providers and Onboarding → Configure providers consume
  `ProviderCredentialsList`. The legacy `ProviderProfileForm` (1393 LOC)
  is deleted.
- Agent composer model picker is the single point of provider selection;
  the union-of-catalogs feed comes from `composerModelOptions`. The
  legacy "Rebind X before trusting new live activity" copy is gone.
- The `providerProfiles*` and `runtimeSettings*` state slices are gone
  from the orchestrator hook surface.
- Tauri command surface deleted: `list_provider_profiles`,
  `upsert_provider_profile`, `set_active_provider_profile`,
  `logout_provider_profile`, `get_runtime_settings`,
  `upsert_runtime_settings`. New command surface: `start_oauth_login`,
  `complete_oauth_callback`, `list_provider_credentials`,
  `upsert_provider_credential`, `delete_provider_credential`.
- Zod schemas: `provider-profiles.ts` survives as a 105-LOC type-only
  stub. The schemas now no-op (`z.unknown()`); production code does not
  call them. `provider-setup.ts` is fully deleted.

Net deletions across Phase 4: ~18,800 LOC. Build state at completion:
tsc clean, cargo check warning-free, cargo test --no-run compiles, 266
vitest tests pass with zero skipped.

**Deferred to a future wave** (out of scope for this plan):

- The internal `provider_profiles` Rust module + SQLite tables. Auth
  modules and runtime/provider bindings still consume the snapshot to
  compose runtime sessions. Removal needs a coordinated rewrite of
  `auth/store`, `provider_models`, `runtime/provider`,
  `runtime/diagnostics`, plus the migration that drops the three SQLite
  tables. This is the storage-side counterpart described in
  `STORAGE_REFACTOR_PLAN.md` Phase 1/2.5.
- The `provider-profiles.ts` 105-LOC type stub. Removable once nothing
  imports the legacy DTOs (the runtime-settings + provider-profile JSON
  legacy paths still surface them in fixtures).

## Why this exists

The current "provider profile" abstraction is a wrapper around what is effectively
"one credential set per provider" — the codebase already enforces one profile per
provider (`commit e35fc37`). The only real degree of freedom the abstraction adds
is `active_profile_id` in the metadata row, and that single field is the source
of every "after sign-in the composer is still bound to the wrong provider" bug we
keep patching:

- Sign in to OpenAI Codex → credential lands in DB → active still points at
  OpenRouter → composer shows mismatch → user is stuck.
- Add an Anthropic key → credential is saved → active still on OpenAI Codex →
  user has to "rebind" to start a session that uses their new key.
- Multi-provider workflows are impossible because only one provider can be
  "active" at a time.

The user's mental model is correct and matches every other multi-provider IDE:
**configure as many providers as you want; the model picker is the single point
of selection; whichever model you pick determines which provider's credentials
to use for that turn.** No profile rows, no active flag, no rebind dance.

This plan replaces the `provider_profiles` / `active_profile_id` machinery with
a flat per-provider credential store and a unified model registry. After this
refactor, the bug class "profile says X, runtime session says Y, composer
disabled" becomes structurally impossible.

---

## Goals

1. **Delete the profile concept.** No `provider_profiles` table, no
   `ProviderProfileRecord`, no `profile_id`, no `active_profile_id`, no
   `set_active_provider_profile` command, no `ProviderProfileCard` component
   abstraction. Providers are identified by their `provider_id` directly.
2. **One credential row per provider.** Each provider either has credentials
   stored or it doesn't; that boolean drives the "Ready" badge.
3. **Unified model picker.** The composer's model dropdown lists every model
   from every credentialed provider, grouped by provider. Selecting a model is
   the only mechanism for picking a provider for a given turn.
4. **No mismatch state.** Provider mismatch detection, rebind buttons, and the
   "Configure runtime" empty state all go away. If a credential exists for the
   chosen model's provider, you can run; if not, you can't.
5. **Backwards-compatible migration.** Existing user data (the legacy JSON, the
   in-DB rows from Phase 2.x of the storage refactor) flows through a one-shot
   importer so users don't lose their OpenAI session or OpenRouter key.

## Non-goals

- Multiple credentials per provider (e.g., two Azure tenants). Out of scope; if
  this is needed later, it's a separate "named credential set" feature.
- Per-project credential overrides. Credentials remain global; runtime selection
  remains per-conversation.
- Changing the OAuth flow's UX. Sign-in still happens per provider in Settings;
  the difference is that the act of signing in doesn't claim "active" status.

---

## Target architecture

### Storage (global SQLite)

Replace these tables:

```
provider_profiles_metadata   -- gone
provider_profiles            -- gone
provider_profile_credentials -- gone
openai_codex_sessions        -- folded into provider_credentials
```

…with one table:

```sql
CREATE TABLE provider_credentials (
    provider_id              TEXT    PRIMARY KEY,        -- 'openrouter', 'openai_codex', etc.
    kind                     TEXT    NOT NULL,           -- 'api_key' | 'oauth_session' | 'local' | 'ambient'
    api_key                  TEXT,                       -- non-null when kind='api_key'
    oauth_account_id         TEXT,                       -- non-null when kind='oauth_session'
    oauth_session_id         TEXT,
    oauth_access_token       TEXT,
    oauth_refresh_token      TEXT,
    oauth_expires_at         INTEGER,
    base_url                 TEXT,                       -- per-provider connection config
    api_version              TEXT,
    region                   TEXT,
    scope_project_id         TEXT,
    default_model_id         TEXT,                       -- last-picked model for this provider (UX hint only)
    updated_at               TEXT    NOT NULL,
    CHECK (
        (kind = 'api_key' AND api_key IS NOT NULL) OR
        (kind = 'oauth_session' AND oauth_account_id IS NOT NULL AND oauth_session_id IS NOT NULL) OR
        (kind IN ('local', 'ambient'))
    )
);
```

`default_model_id` is purely a UX hint: when the user picks a model, we remember
it per-provider so the composer can default to a sensible last-used choice.
There is no "active provider"; the composer's selected model is owned by the
runtime run / control state, not by a global flag.

### Runtime selection state

Move "what model am I using right now" into the run-control surface that already
exists:

- `RuntimeRunControlInputView.modelId` — already there; selecting a model in
  the composer writes to the pending controls.
- `RuntimeRunControlInputView.providerId` — **new field**, derived from the
  model. Persisted alongside the run so resume / replay knows which provider's
  credentials the run was bound to.
- `RuntimeSession` continues to exist (it's the auth-flow state machine for
  OAuth), but it's no longer the source of truth for "which provider is
  selected." It only answers "is OpenAI Codex currently mid-OAuth-flow?" Each
  provider gets its own session row keyed by `(project_id, provider_id)` rather
  than the current per-project singleton, so signing in to OpenAI doesn't
  invalidate an Anthropic key on the same project.

### Model registry

`provider_model_catalog_cache` already aggregates models per provider. Today the
frontend looks up the catalog for the *active* profile. After the refactor, the
composer's model picker reads the union of catalogs across every credentialed
provider:

```
modelOptions = providers
  .filter(provider => provider.hasCredential)
  .flatMap(provider => catalog[provider.id].models.map(model => ({
    selectionKey: `${provider.id}:${model.modelId}`,
    providerId: provider.id,
    providerLabel: getRuntimeProviderLabel(provider.id),
    modelId: model.modelId,
    displayName: model.displayName,
    thinking: model.thinking,
  })))
```

Grouped in the dropdown by `providerLabel` (Anthropic / OpenAI / OpenRouter /
…). Local-only providers (Ollama) appear when their endpoint is reachable;
ambient providers (Bedrock, Vertex) appear when ambient creds are configured.

### Settings UI

Each card in Settings → Providers shows:

- Provider icon + label.
- "Ready" / "Needs key" / "Needs sign-in" badge driven by `hasCredential`.
- Inline editor for API key / endpoint / sign-in button — same as today.
- **No** "make active" checkbox, **no** active radio group.

Everything that currently reads `providerProfiles.activeProfileId` either
deletes the read or substitutes the per-run `controls.providerId`.

### Agent tab

Composer model picker becomes the single point of provider selection. The
existing "Configure agent runtime" empty-state changes from "no active profile"
to "no providers credentialed yet" — shown only when `provider_credentials` is
empty. The "Rebind X before trusting new live activity" mismatch banner is
deleted entirely; there is no concept that can cause a mismatch anymore.

---

## Phase 0 — Decision lock-in (no code)

Decisions to record before coding starts.

1. **Provider identity is the primary key.** `provider_id` from the existing
   `RuntimeProviderIdDto` enum is the canonical identifier. Drop all
   `profile_id` strings (`openai_codex-default`, `openrouter-default`, etc.)
   from contracts.
2. **OpenAI Codex sessions live in `provider_credentials`.** No separate
   `openai_codex_sessions` table after Phase 2. Refresh tokens and access
   tokens move into the unified row.
3. **Default model = last selected.** When the user picks a model in the
   composer, write `provider_credentials.default_model_id` for that provider.
   Used to seed the dropdown selection on app open. Not persisted in run state.
4. **Runtime session per (project, provider).** Replace the singleton runtime
   session per project with a keyed map. When the agent picks an OpenRouter
   model after running on OpenAI Codex, the OpenRouter session binds without
   touching the OpenAI session.
5. **No "active" anywhere.** The word `active` is allowed only on
   `agent_sessions.is_active` (the user's currently-open chat), never on
   providers.
6. **Migration is one-shot.** Read everything out of the legacy
   `provider_profiles` / `provider_profile_credentials` tables, fold OAuth
   sessions in, write to `provider_credentials`, drop the old tables.

---

## Phase 1 — Schema & importer

**File**: `client/src-tauri/src/global_db/migrations.rs`

Add a new migration:

```sql
CREATE TABLE provider_credentials ( …as defined above… );

CREATE INDEX idx_provider_credentials_kind ON provider_credentials(kind);

INSERT INTO provider_credentials (provider_id, kind, …)
SELECT
    provider_id,
    CASE credential_link_kind
        WHEN 'openai_codex' THEN 'oauth_session'
        WHEN 'api_key' THEN 'api_key'
        WHEN 'local' THEN 'local'
        WHEN 'ambient' THEN 'ambient'
    END,
    (SELECT api_key FROM provider_profile_credentials WHERE profile_id = pp.profile_id),
    pp.credential_link_account_id,
    pp.credential_link_session_id,
    -- token fields filled by importer pass below from openai_codex_sessions
    NULL, NULL, NULL,
    pp.base_url, pp.api_version, pp.region, pp.scope_project_id,
    pp.model_id,
    COALESCE(pp.credential_link_updated_at, pp.updated_at)
FROM provider_profiles pp
WHERE pp.credential_link_kind IS NOT NULL
ON CONFLICT(provider_id) DO NOTHING;

-- Pull OAuth tokens out of openai_codex_sessions into the unified row.
UPDATE provider_credentials
SET    oauth_access_token  = (SELECT access_token  FROM openai_codex_sessions WHERE account_id = oauth_account_id),
       oauth_refresh_token = (SELECT refresh_token FROM openai_codex_sessions WHERE account_id = oauth_account_id),
       oauth_expires_at    = (SELECT expires_at    FROM openai_codex_sessions WHERE account_id = oauth_account_id)
WHERE  kind = 'oauth_session';

DROP TABLE provider_profiles;
DROP TABLE provider_profiles_metadata;
DROP TABLE provider_profile_credentials;
DROP TABLE openai_codex_sessions;
```

Migration tests:
- Round-trip: seed legacy tables, run migration, assert `provider_credentials`
  matches expected rows.
- OAuth: ensure the `account_id` linkage carries the token columns over.
- No-orphan: API-key profile without matching credential row drops to nothing
  (today's `Malformed` state) instead of being imported as Ready.

**Files deleted at the end of this phase:**
None yet — the new table coexists with the old data until Phase 2 rewires
readers.

---

## Phase 2 — Backend Rust rewrite

### 2.1 Replace `provider_profiles` module

**New module**: `client/src-tauri/src/provider_credentials/` with:

- `mod.rs` — public types `ProviderCredentialRecord`, `ProviderCredentialKind`,
  `ProviderCredentialsSnapshot`. The "snapshot" is a `Vec<ProviderCredentialRecord>`
  (no metadata wrapper — there's no metadata anymore).
- `sql.rs` — `load_all`, `load_by_provider`, `upsert`, `delete`. No transactions
  required for batch writes; each provider is independent.
- `readiness.rs` — `is_ready(record) -> bool` and `readiness_proof(record) ->
  Proof`. Same logic as today minus the `Malformed` case (we either have a
  credential row or we don't — there's no mismatched profile/credential
  combination possible).

**Delete**: `client/src-tauri/src/provider_profiles/` (`mod.rs`, `store.rs`,
`sql.rs`, `importer.rs` — about 2,800 lines).

**Delete from**: `client/src-tauri/src/auth/store.rs`:
- `sync_openai_profile_link` — no profile to sync to. The OAuth completion path
  writes directly to `provider_credentials` instead.
- `load_openai_codex_session_for_profile_link` — no profile linkage needed.

The `auth/store.rs::persist_openai_codex_session` becomes "upsert
`provider_credentials` row with `kind = 'oauth_session'`."

### 2.2 Tauri commands

**Delete commands** (and their bindings in `lib.rs`):
- `list_provider_profiles`
- `upsert_provider_profile`
- `set_active_provider_profile`
- `logout_provider_profile`
- `upsert_runtime_settings` (this was the legacy "set active provider" entry
  point that pre-dated profiles; it's been redundant since profiles landed).

**New commands**:
- `list_provider_credentials() -> Vec<ProviderCredentialDto>`
- `upsert_provider_credential(ProviderCredentialUpsertRequest) -> ProviderCredentialDto`
  - Body: `{ providerId, kind, apiKey?, baseUrl?, apiVersion?, region?, projectId?, defaultModelId? }`.
  - For OAuth providers, `kind: 'oauth_session'` plus the apiKey path is rejected.
- `delete_provider_credential(providerId)` — clears a credential. Replaces both
  `logout_provider_profile` (for OpenAI) and "save with empty key" (for API key
  providers).
- `start_oauth_login(providerId, projectId)` — replaces `start_openai_login`
  with a per-provider entry point. Today only OpenAI Codex uses it; the
  signature stays generic so future browser-auth providers slot in.
- `complete_oauth_callback(providerId, projectId, flowId, manualInput?)` —
  replaces `submit_openai_callback`.

**Rewrite commands**:
- `get_runtime_settings` / `upsert_runtime_settings` — these previously
  projected the active profile back as a `RuntimeSettingsDto` for the legacy
  API. Drop both. Frontend reads `provider_credentials` directly.
- `start_runtime_session(projectId, providerId, modelId)` — already takes a
  provider profile id. Change parameter to `providerId`. The session binds
  using the `provider_credentials` row for that provider.
- `update_runtime_run_controls` — drop `provider_profile_id` field; keep
  `model_id` and add `provider_id`.

### 2.3 Runtime binding

**File**: `client/src-tauri/src/runtime/provider.rs`

Today every runtime binding helper takes a `ProviderProfileRecord` and threads
it through `start_runtime_session`. Refactor to take `ProviderCredentialRecord`
instead. Functions become 1:1 with provider IDs (e.g., `bind_openrouter`,
`bind_anthropic`) rather than profile-driven. The `*_with_operator_approval`
runtime helpers in `runtime/autonomous_tool_runtime/` keep their signatures —
they don't read profile state.

### 2.4 Per-provider runtime session storage

**File**: `client/src-tauri/src/db/project_store/runtime.rs`

Today there's one `runtime_sessions` row per project. Add `provider_id` to the
primary key:

```sql
ALTER TABLE runtime_sessions RENAME TO runtime_sessions_legacy;

CREATE TABLE runtime_sessions (
    project_id   TEXT NOT NULL,
    provider_id  TEXT NOT NULL,
    runtime_kind TEXT NOT NULL,
    flow_id      TEXT,
    session_id   TEXT,
    account_id   TEXT,
    phase        TEXT NOT NULL,
    callback_bound INTEGER,
    authorization_url TEXT,
    redirect_uri TEXT,
    last_error_code TEXT,
    last_error_json TEXT,
    updated_at   TEXT NOT NULL,
    PRIMARY KEY (project_id, provider_id)
);

INSERT INTO runtime_sessions SELECT project_id, provider_id, … FROM runtime_sessions_legacy;
DROP TABLE runtime_sessions_legacy;
```

`get_runtime_session(projectId)` becomes
`get_runtime_sessions(projectId) -> Vec<RuntimeSessionDto>`. The frontend
consumes this list and indexes by provider when it needs to know "is OpenAI
mid-OAuth?"

### 2.5 Importer cleanup

After Phase 1's migration plus 2.x's reader rewrites, no live code references
the legacy tables. Delete:

- `provider_profiles/importer.rs::import_legacy_provider_profiles` body
  (replaced by the schema migration).
- `global_db::legacy_runtime_settings` — its only consumer was the legacy
  importer.
- `LEGACY_PROVIDER_PROFILES_FILE_NAME`, `LEGACY_PROVIDER_PROFILE_CREDENTIALS_FILE_NAME`,
  `LEGACY_RUNTIME_SETTINGS_FILE_NAME`, `LEGACY_OPENROUTER_CREDENTIAL_FILE_NAME`,
  `LEGACY_OPENAI_CODEX_AUTH_STORE_FILE_NAME` constants and the JSON-file
  garbage-collection logic that should have run in Phase 6 of the storage
  refactor (the JSON files still sit next to `cadence.db` today).
- Add a startup pass that deletes those JSON files if they exist — they're
  superseded by `cadence.db` and currently leak personal data into a place no
  reader looks.

---

## Phase 3 — Frontend rewrite

### 3.1 Types & schemas

**File**: `client/src/lib/cadence-model/runtime.ts`

Delete:
- `runtimeSettingsSchema`, `RuntimeSettingsDto`,
  `upsertRuntimeSettingsRequestSchema`, `UpsertRuntimeSettingsRequestDto`,
  `writableRuntimeSettingsProviderIdSchema`.

`runtimeProviderIdSchema` stays — provider IDs are still a closed set.

**File**: `client/src/lib/cadence-model/provider-profiles.ts` → rename to
`provider-credentials.ts`. New exports:

```typescript
export const providerCredentialKindSchema = z.enum([
  'api_key', 'oauth_session', 'local', 'ambient',
])

export const providerCredentialSchema = z.object({
  providerId: runtimeProviderIdSchema,
  kind: providerCredentialKindSchema,
  hasApiKey: z.boolean(),                 // bool projection — we never expose the secret
  oauthAccountId: z.string().nullable(),
  oauthSessionId: z.string().nullable(),
  baseUrl: z.string().nullable(),
  apiVersion: z.string().nullable(),
  region: z.string().nullable(),
  projectId: z.string().nullable(),
  defaultModelId: z.string().nullable(),
  updatedAt: z.string(),
})

export const providerCredentialsSnapshotSchema = z.object({
  credentials: z.array(providerCredentialSchema),
})
```

Delete:
- `providerProfileSchema`, `providerProfilesSchema`,
  `upsertProviderProfileRequestSchema`, `setActiveProviderProfileRequestSchema`,
  `logoutProviderProfileRequestSchema`.
- `getActiveProviderProfile`, `projectRuntimeSettingsFromProviderProfiles`,
  `hasAnyReadyProfile`.

### 3.2 Hook surface

**File**: `client/src/features/cadence/use-cadence-desktop-state/`

Replace `providerProfiles*` state slice with `providerCredentials*`:
- `[providerCredentials, setProviderCredentials]` (array, not object with
  `activeProfileId`).
- `refreshProviderCredentials({ force? })` — same caching pattern.
- `upsertProviderCredential(request)` / `deleteProviderCredential(providerId)`.

Delete:
- `runtimeSettings*` state slice and the entire
  `runtime-settings-notification-mutations.ts` `refreshRuntimeSettings` /
  `upsertRuntimeSettings` block. The `notification-*` half of that file moves
  into a dedicated `notification-mutations.ts`.
- `setActiveProviderProfile` mutation.
- `logoutProviderProfile` mutation.

The `previousRuntimeAuthRef` watcher I added last week (force-refreshes
profiles on auth transitions) goes away — there's no "active profile" for the
backend to recompute, so a refresh is unnecessary; the runtime:updated event
handler already updates `provider_credentials[openai_codex].kind` directly.

### 3.3 Runtime provider selection

**File**: `client/src/features/cadence/use-cadence-desktop-state/runtime-provider.ts`

Delete:
- `resolveSelectedRuntimeProvider` (its job was to reconcile active profile vs
  runtime session vs runtime settings — gone).
- `hasProviderMismatch`, `getProviderMismatchCopy` — there's no mismatch state
  anymore.
- `SelectedRuntimeProviderView`, `ProviderMismatchCopyView`.

Replace with:
- `resolveSelectedModel(controls, credentials, catalogs) -> SelectedModelView`
  — pure projection from "what model did the user pick in this run" plus the
  catalog. No reconciliation, no precedence rules, no fallbacks beyond "if the
  selected model's provider has no credential, the run can't start."

### 3.4 View builders

**File**: `client/src/features/cadence/use-cadence-desktop-state/view-builders.ts`

Today the agent view threads `selectedProvider`, `providerMismatch`,
`providerMismatchCopy`. After the refactor these collapse to `selectedModel`
and `agentRuntimeBlocked` (boolean: true when no credentialed provider exists
or when the chosen model's provider has no credential).

Composer model picker rebuilds with the union-of-catalogs logic from the target
architecture section. Models are sorted by provider label, then by display
name, with the user's last-picked model surfaced first per provider.

### 3.5 Components

**File**: `client/components/cadence/provider-profiles/provider-profile-form.tsx`
→ rename to `provider-credentials-list.tsx`, drop the `ProviderProfileCard`
abstraction. Each row in the list is one provider, mapped 1:1 to the preset.

Drop:
- `groupProfileCards` / `PROVIDER_GROUP_ORDER` / `PROVIDER_GROUP_META` —
  grouping by `authMode` is fine UX, keep that part, but stop pretending each
  group is a "set of profiles you can have multiple of".
- "Make active" / `activate: true` request flag. The upsert command no longer
  takes one.
- The OpenAI "Sign out" button still works — it calls
  `deleteProviderCredential('openai_codex')` instead of
  `logoutProviderProfile`.

**File**: `client/components/cadence/agent-runtime/composer-helpers.ts` and
`agent-runtime.tsx`:

- Delete `providerMismatch` / `providerMismatchCopy` rendering and the rebind
  CTA.
- Delete the `Configure agent runtime` empty state's "no active profile" copy;
  rewrite to "Add a provider credential in Settings to start chatting." with
  the same Configure button.
- Delete `selectedProfile` references; the picker only knows about models.
- Delete `composerProfiles` / `getProfileDisplayLabel` — replaced by the
  per-model `providerLabel` field.

**File**: `client/components/cadence/onboarding/steps/providers-step.tsx`:

The onboarding flow stops pretending you have to pick *one* provider. It shows
the same credentials list as Settings; the "Continue" button enables once at
least one provider is credentialed. No active-profile selection step.

### 3.6 Settings dialog wiring

`client/components/cadence/settings-dialog/providers-section.tsx` becomes a
thin shell over `ProviderCredentialsList`. Drop the `agent`,
`runtimeSession`, `onStartLogin`, `onLogout` props that were only there to
power the "Sign in" button on the OpenAI card — pull those props from the
shared hook directly inside `ProviderCredentialsList` since they're scoped to
provider auth, not to a profile concept.

### 3.7 Tests

Every test that reaches into `providerProfiles.activeProfileId` or
`providerMismatch` needs to be rewritten. There are roughly 25 affected test
files; the line counts are small but the fixtures (`makeProviderProfile`,
`makeProviderProfilesFromRuntimeSettings`, `applyOpenAiRuntimeReadinessToProfiles`)
are widely used. Replace with `makeProviderCredential` / `makeCredentials`
fixtures.

The recent helper `applyOpenAiRuntimeReadinessToProfiles` in
`client/src/App.test.tsx` (added in this branch's last commit) goes away — the
test mock for `getProviderCredentials` returns the OpenAI row with
`kind: 'oauth_session'` directly when emitting the runtime-updated event;
there's no readiness projection to keep in sync.

---

## Phase 4 — Cleanup

1. Search-and-destroy `active_profile_id`, `activeProfileId`,
   `providerProfileId`, `provider_profile_id` across the codebase. Audit
   target: zero hits in production code; tests can keep the strings only as
   migration fixtures.
2. Delete `lib/cadence-model/provider-models.ts::projectRuntimeSettingsFromProviderProfiles`
   and the `runtimeSettings` state in `App.tsx`.
3. Delete the `with_global_db_path_override` test override path that used to
   exist for the long-gone per-store JSON paths — it's only referenced by tests
   touching the legacy importer.
4. Update `STORAGE_REFACTOR_PLAN.md` to note that Phase 2.1 (provider profiles
   port) was superseded by this refactor, and that Phase 6's "delete file-name
   constants" item is finally complete.
5. Audit `AGENTS.md` for any prose that still talks about active profiles or
   the legacy runtime-settings schema.
6. Garbage-collect leftover `provider-profiles.json` and
   `provider-profile-credentials.json` from `~/Library/Application
   Support/dev.sn0w.cadence/` on first boot after the migration runs (they're
   sitting there orphaned from the storage refactor; deleting them is now
   safe).

---

## Sequencing & shippability

| Order | Phase | Notes |
|-------|-------|-------|
| 1 | Phase 0 | Decisions recorded. No code. |
| 2 | Phase 1 | Migration ships behind a feature flag. Old code still reads `provider_profiles`. New table populated but unused. |
| 3 | Phase 2.1–2.3 | Backend module + commands. Old commands kept as thin shims that translate to new ones — frontend still calls old API. |
| 4 | Phase 2.4 | Runtime sessions move to per-provider keying. This is the load-bearing change for the bug class — once sessions are keyed by `(project, provider)`, signing in to OpenAI no longer touches OpenRouter session state. |
| 5 | Phase 3 | Frontend rewrite. Lands as one PR because the type changes ripple through ~50 files; trying to split it produces an unreviewable mess. Old shim commands deleted as part of this PR. |
| 6 | Phase 2.5 + Phase 4 | Importer / dead-code cleanup. Ship after frontend is stable. |

Phase 2.4 is the single most important phase for the user's bug — even without
the frontend rewrite, keying runtime sessions per-provider means the
post-OAuth composer block goes away (the OpenRouter session that was claiming
"this project is bound to me" no longer exists; the OpenAI session is its own
row).

---

## Risks

- **OAuth token migration**: Phase 1's migration has to copy access tokens out
  of `openai_codex_sessions` into `provider_credentials` *before* the table is
  dropped. If the migration runs but the copy fails partway, users lose their
  signed-in session and have to re-auth. Mitigation: wrap the OAuth-copy step
  and the `DROP TABLE` in a single transaction in the migration.
- **Per-project runtime sessions in flight**: If a project has an in-flight
  OAuth flow (`phase = 'awaiting_browser_callback'`) when 2.4 ships, the row
  needs to be carried over with the right `provider_id`. Today the flow's
  provider id is in the row already; the migration just needs to pick it up
  rather than dropping it.
- **Test fixture churn**: ~25 test files touch the profile model. A
  search-and-replace handles ~80% of it; the rest needs hand-editing for
  fixtures whose semantics changed (`active: true` flags, etc.). Budget a day
  for this pass.
- **Catalog cache invalidation**: `provider_model_catalog_cache` is keyed by
  `profile_id` today (`ProviderModelCatalogDto.profileId`). Schema change to
  key by `provider_id` instead. Cached catalogs migrate by lookup of the
  profile→provider mapping; orphans get dropped.
- **Frontend single-PR-ness**: Phase 3 is hard to split because removing
  `activeProfileId` rewrites the prop signature of every component that touches
  the agent pane. Reviewer should expect a +500/-1500 diff. The alternative
  (introducing a parallel `Selected` view that coexists with the active
  profile, then removing the old one) doubles the work. Bite the bullet.
- **External tooling**: any scripts or CLIs that called the old Tauri commands
  (`upsert_provider_profile` etc.) break. Search the repo for invocations; the
  only known ones are tests and `client/scripts/`. None ship to end-users.

---

## Definition of done

- `cadence.db` has a `provider_credentials` table and no
  `provider_profiles*` / `openai_codex_sessions` tables.
- The string `active_profile_id` (any case) appears in no source file outside
  of the migration itself and any test fixture that exercises the migration.
- The user flow from the bug report works: with a fresh DB, signing in to
  OpenAI Codex makes the agent composer immediately usable with OpenAI models;
  separately adding an OpenRouter key makes OpenRouter models appear in the
  same picker, no rebind, no settings re-entry; switching between them is one
  click in the composer dropdown.
- The "Rebind X before trusting new live activity" string and the
  `provider_mismatch` test ID are deleted from the codebase.
- `pnpm test` (client) and `cargo test` pass on a clean checkout.
