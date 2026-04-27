# Provider Model Flow Plan

## Reader And Outcome

This plan is for an engineer implementing the provider/model flow fix. After reading it, they should be able to change the existing provider setup and Agent runtime path so users authenticate providers in Settings or onboarding, then choose a configured provider's model and thinking level from the existing Agent composer.

## Target Flow

1. User opens the Providers tab in Settings or reaches Providers during onboarding.
2. User configures one or more providers by signing in, saving API keys, or saving local/ambient endpoint metadata.
3. A configured provider becomes available to the Agent tab when its provider profile is ready.
4. User opens the Agent tab and uses the existing composer model selector to choose a model from any ready provider profile.
5. User uses the existing composer thinking selector to choose a valid thinking level for that selected model.
6. Sending a message binds the runtime session to the selected provider profile, starts or updates the run with the selected model and thinking effort, and rejects mismatched provider/model combinations.

## Current Problems

The model and thinking selection UI already exists in the Agent composer and should remain the only user-facing model/thinking selector.

The current state layer only feeds the composer with the active provider profile's model catalog. Users who configure multiple providers cannot choose across all ready providers from the Agent tab.

Runtime controls carry model and thinking information, but not provider profile identity. That makes the selected model ambiguous across providers and lets Settings' active profile influence the run after the user has made a composer selection.

Runtime session binding reads the globally active provider profile instead of accepting the provider profile selected by the Agent composer.

Providers setup still exposes model selection, which makes Settings compete with the Agent composer for model truth.

## Design Direction

Reuse the existing Agent composer controls. Do not add a second model/thinking selector.

Make provider setup responsible for authentication and connection readiness only. Settings and onboarding should collect credentials, endpoint metadata, and profile labels, then surface readiness and diagnostics.

Make the Agent composer own run-time model choice. Its model options should be provider-scoped, even if the visible label stays compact.

Make runtime contracts explicit. A run start must know which provider profile owns the chosen model. Backend validation should load that profile's model catalog and validate model and thinking effort against it.

Preserve safe active-run behavior. Model changes within an active run can stay queued through existing control updates when they are within the same bound provider profile. Switching providers during an active run should either be blocked with clear copy or handled as an explicit stop/rebind/new-run workflow.

## Implementation Plan

### 1. Build A Provider-Scoped Composer Catalog

Create an Agent-view projection that combines model catalogs from all ready provider profiles.

Each composer model option should include:

- Provider profile id
- Provider id
- Provider/profile label for grouping
- Model id
- Display label
- Thinking support and effort options
- Stable selection key, such as a profile/model composite

Keep the existing grouped select UI. Use provider/profile groups so the current selector naturally becomes "choose model from configured provider".

### 2. Refresh Catalogs For Ready Providers

The state layer already has catalog loading per profile. Extend the loading policy so all ready profiles needed by the Agent composer are refreshed or hydrated, not just the active profile.

Keep stale/cache behavior per profile. One provider catalog failure should not hide models from other ready providers.

### 3. Extend Runtime Control Input With Provider Profile Identity

Add provider profile identity to the initial run controls and any control path that may change model selection.

The control payload should distinguish:

- Current run provider profile
- Selected model id
- Selected thinking effort
- Approval mode and plan mode

For active runs, reject provider profile changes unless the run lifecycle explicitly supports rebind.

### 4. Bind Sessions To Composer Selection

Update runtime session start/bind commands so the selected provider profile can be passed from the Agent composer path.

Binding should:

- Load the selected provider profile
- Validate readiness
- Bind or refresh the project runtime session for that provider
- Persist runtime session provider identity
- Return a typed error if the profile is missing, not ready, or mismatched

OpenAI browser sign-in should continue to use the selected OpenAI provider profile, but provider setup should not require choosing a model first.

### 5. Validate Run Launch Against Selected Profile

When starting a run:

- Load the selected provider profile
- Load that profile's model catalog
- Validate selected model id exists in that catalog or is accepted by the provider's manual catalog mode
- Validate selected thinking effort is supported for that model
- Build launch environment from the selected provider profile, not global active profile

Runtime run persistence should record provider id and enough selected-profile identity to reconcile future updates safely.

### 6. Remove Model Editing From Provider Setup

Keep provider setup controls for:

- Profile label
- API key or sign-in
- Base URL
- API version
- Region
- Project id
- Diagnostics and catalog refresh state

Remove or de-emphasize provider setup model fields. If a model id is still required internally for backward compatibility or manual catalog fallback, treat it as an internal/default field and migrate toward Agent-owned selection.

### 7. Update Tests

Add or update tests for:

- Multiple ready provider profiles populate the existing Agent composer model selector.
- Model selection preserves provider profile identity.
- Thinking options update when the selected provider-scoped model changes.
- Sending the first message binds the runtime session to the selected provider profile.
- Backend rejects a model id that does not belong to the selected provider profile catalog.
- Backend rejects unsupported thinking effort for the selected model.
- Settings provider setup no longer acts as the user-facing model selection surface.

## Suggested Work Slices

1. Projection-only slice: combine ready provider catalogs into the existing Agent composer model option shape with a provider-scoped selection key.
2. Contract slice: extend TypeScript and Tauri DTOs to carry selected provider profile identity through start-session and start-run paths.
3. Backend validation slice: launch and bind using selected provider profile rather than active profile.
4. Settings cleanup slice: remove visible model selection from provider setup while preserving migration/default compatibility.
5. Test hardening slice: cover multi-provider selection, duplicate model ids, thinking effort validation, and active-run provider switching behavior.

## Open Decisions

- Whether stopped runs should remember their previous provider profile and model selection as defaults for that project.
- Whether active-run provider switching should be blocked only, or offered as a guided stop/rebind/start-new-run flow.
- How much manual model fallback should remain for OpenAI-compatible providers that cannot list models.
