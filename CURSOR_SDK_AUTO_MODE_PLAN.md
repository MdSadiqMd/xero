# Cursor SDK Auto Mode Plan

## Reader And Outcome

This plan is for an engineer implementing Cursor SDK model routing in Xero.
After reading it, they should be able to add an Auto/default-style Cursor model option without accidentally charging users against the wrong Cursor usage pool or mislabeling a normal Composer run as Auto.

## Background

Cursor now distinguishes between at least two usage pools for agent work:

- Auto + Composer: higher included usage when Auto or Composer-family models are selected.
- API: usage charged at the explicit provider-model API price.

Cursor's TypeScript SDK docs say SDK runs follow the same pricing, request pools, and privacy rules as IDE and Cloud Agents, and usage appears in the Cursor dashboard with an SDK tag.

The Cloud Agents API documents the default model behavior as omitting the `model` field entirely. When omitted, Cursor resolves the user default model, then the team default model, then a system default. The docs do not currently guarantee that `model: { id: "auto" }` is a stable public API.

Xero's current Cursor adapter is local-only and always sends an explicit model. Its default is `composer-latest`, so it can benefit from Composer-style limits, but it does not express true Cursor Auto routing.

Relevant upstream docs:

- https://cursor.com/blog/increased-agent-usage
- https://cursor.com/docs/sdk/typescript.md
- https://cursor.com/docs/cloud-agent/api/endpoints.md

## Goals

- Add a first-class Cursor Auto/default model choice in Xero.
- Preserve the existing `composer-latest` behavior as an explicit Composer choice.
- Avoid sending undocumented `auto` as a model id unless Cursor's model catalog confirms it is valid for the authenticated account.
- Make billing/limit implications visible enough that users understand whether they are selecting Auto/default routing, Composer, or an explicit API-priced model.
- Keep the implementation compatible with the current local Cursor SDK bridge.
- Leave room for future Cursor Cloud Agent support, where omitting `model` is officially documented.

## Non-Goals

- Do not enable Cursor Cloud Agents as part of this change.
- Do not add backwards-compatible handling for old Cursor state unless explicitly requested.
- Do not introduce a fake OpenAI-compatible Cursor provider.
- Do not promise unlimited Auto usage. Cursor's Auto/Composer pool is limited and account-dependent.

## Proposed Semantics

Xero should represent Cursor model routing with three distinct choices:

| User-facing choice | Internal route | SDK request behavior |
| --- | --- | --- |
| Auto | `cursor_auto` sentinel | Prefer omitted `model` when the runtime supports it; otherwise use a catalog-confirmed Auto/default alias if available. |
| Composer Latest | `composer-latest` | Send `model: { id: "composer-latest" }`. |
| Specific Cursor model | concrete model id | Send `model: { id: "<model-id>", params? }`. |

The sentinel must never be blindly forwarded as `model.id`. It is an instruction to the adapter.

For the current local bridge, support should be conservative:

1. Discover models with `Cursor.models.list()` when credentials allow it.
2. If the catalog exposes an `auto`, `default`, or equivalent alias, allow Auto to map to that catalog-backed selection.
3. If no Auto/default alias exists, local Auto should either fall back to `composer-latest` with clear labeling or be disabled with a clear explanation.
4. Once Cursor documents omitted local `model` support, update the bridge to omit `model` for local Auto too.

For future cloud support:

1. Auto should omit `model` entirely.
2. The run result's resolved model should be recorded from `run.model` or `result.model`.
3. Xero traces should show both the user's requested route (`cursor_auto`) and Cursor's resolved model when available.

## Implementation Slices

### Slice 1: Model Route Types

Add an internal Cursor model route type that can distinguish:

- `auto`
- `composer_latest`
- `explicit`

Keep provider profile storage non-empty by storing an internal sentinel such as `cursor-auto`, but translate it before calling the SDK. The sentinel should be rejected anywhere generic providers expect a real model id.

Update Cursor provider presets and provider capability projection so the Cursor catalog includes both:

- Auto
- Composer Latest

The Auto option should have copy indicating that it uses Cursor's default/Auto-style routing when supported by the SDK and account.

### Slice 2: Bridge Request Shape

Change the Node bridge so `--model` can be omitted or set to a sentinel.

The bridge should build `Agent.create()` options in one place:

- For explicit models: include `model`.
- For Auto/default with runtime support: omit `model`.
- For Auto/default without omitted-model support: use a validated catalog alias.
- For unsupported Auto/default: fail with a structured `cursor_auto_unavailable` error.

Do the same for per-run `agent.send()` options if follow-up routing is added later.

The bridge's emitted JSONL should include:

- `requestedModelRoute`: `auto`, `composer_latest`, or `explicit`
- `requestedModelId`: the model id when one was sent
- `resolvedModel`: Cursor's run/result model when available
- `runtime`: currently `local`

### Slice 3: CLI Wiring

Update `xero agent cursor` so it can accept:

- `--model auto`
- `--model default`
- `--model composer-latest`
- any explicit catalog-backed model id

The CLI should pass the route to the bridge without forcing every request into a concrete model id.

Trace records should preserve the user intent. If Auto is selected, store the run model as `cursor-auto` initially and update or annotate the trace with Cursor's resolved model when the bridge reports it.

### Slice 4: Desktop Provider Catalog

Expose the Cursor Auto option in the model picker for the Cursor provider.

Behavior:

- Auto appears as a distinct option above Composer Latest.
- Composer Latest remains selectable.
- Manual model entry remains disabled unless Cursor catalog discovery is added for specific models.
- Help text should avoid promising free/unlimited usage. It should say Auto uses Cursor's Auto/default routing and account limits.

Because Xero currently blocks desktop Cursor runs with a message directing users to the CLI harness, this slice can focus on model catalog consistency and future desktop readiness.

### Slice 5: Catalog Discovery

Add a Cursor catalog refresh path that calls the SDK model listing API through the Node bridge or a small companion script.

The refresh result should classify:

- known Composer aliases
- Auto/default aliases if present
- explicit model ids and parameter variants

If `Cursor.models.list()` is unavailable for the user's plan, keep the static catalog with Auto marked as unavailable for local SDK runs unless a documented fallback exists.

### Slice 6: Error Handling

Add structured errors:

- `cursor_auto_unavailable`: Auto/default route cannot be safely mapped for the current runtime/account.
- `cursor_model_catalog_unavailable`: Cursor model listing failed, so model availability could not be confirmed.
- `cursor_model_resolved_to_api_pool`: optional warning if Cursor reports an explicit resolved model that appears API-priced.

Errors should tell the user what to do:

- Select Composer Latest.
- Select a concrete model.
- Refresh Cursor models.
- Upgrade/connect the Cursor account if the SDK reports plan gating.

### Slice 7: Tests

Use fixture-driven tests first. Do not add temporary UI.

Bridge tests:

- Auto route omits `model` when configured for omitted-model support.
- Auto route maps to a catalog-confirmed alias when omitted model is not supported.
- Auto route fails with `cursor_auto_unavailable` when neither route is available.
- Explicit Composer route still sends `composer-latest`.
- Completed events persist `resolvedModel` when Cursor returns it.

CLI tests:

- `--model auto` is accepted and recorded as the Auto route.
- Default CLI behavior remains Composer Latest unless the product decision changes the default.
- Bridge argv does not force `--model composer-latest` for Auto.

Provider catalog tests:

- Cursor catalog includes Auto and Composer Latest.
- Auto is not treated as a generic provider model id.
- Unsupported manual entry remains disabled for Cursor.

## Product Decision Needed

Decide whether the default Cursor selection should remain Composer Latest or change to Auto.

Recommendation: keep Composer Latest as the default until the local SDK has a documented Auto/default route. Add Auto as an explicit selectable option. This avoids surprising users with an ambiguous billing path.

## Verification Plan

Run scoped tests only:

- Node tests for the Cursor SDK bridge.
- Rust CLI tests covering `xero agent cursor` parsing and bridge argv construction.
- Frontend model catalog tests for Cursor provider options.

Manual verification with a real Cursor API key:

1. Run catalog discovery and record whether Auto/default aliases exist.
2. Run a short Composer Latest fixture or smoke test.
3. Run a short Auto/default smoke test only if the SDK route is catalog-confirmed or documented.
4. Confirm Cursor dashboard usage appears under SDK and the expected pool.

## Risks

- Cursor's SDK is in public beta and the model-selection contract may change.
- Local SDK agents currently require a model in the public types, while cloud API docs support omitted model.
- `Cursor.models.list()` may be plan-gated for some users.
- Cursor may resolve Auto to an explicit model internally, and the dashboard may be the only source of truth for which pool was charged.
- Naming Auto too confidently could create billing surprises.

## Done Criteria

- Xero has a distinct Auto/default Cursor route that is not blindly forwarded as `model.id`.
- Composer Latest remains a clear explicit option.
- The bridge emits requested and resolved model metadata.
- Unsupported Auto attempts fail with actionable errors.
- Tests cover route translation and trace metadata.
- Documentation explains the local-runtime limitation and the cloud-ready omitted-model behavior.
