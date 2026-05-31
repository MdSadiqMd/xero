# Web Search Functionality Audit

Issue: <https://github.com/hyperpush-org/xero/issues/37>

Date: 2026-05-31

## Status

Xero has partial autonomous web functionality today.

`web_fetch` is implemented as a direct HTTP/HTTPS text fetch and can work without any search-provider configuration. `web_search` is implemented and exposed to agents, but it only works when a backend search provider is configured with `XERO_AUTONOMOUS_WEB_SEARCH_URL`; otherwise it fails with `autonomous_web_search_provider_unavailable`.

That means agents can fetch a known current URL, but reliable search from a fresh desktop install is incomplete.

## Current Surfaces

Runtime tools live in `client/src-tauri/src/runtime/autonomous_web_runtime/`:

- `mod.rs` defines `web_search`, `web_fetch`, request/output DTOs, limits, and env-backed search-provider config.
- `search.rs` validates queries, calls the configured provider with `q` and `limit`, accepts JSON `{ "results": [{ "title", "url", "snippet" }] }`, normalizes HTTP/HTTPS result URLs, decodes HTML entities, and caps result counts/snippets.
- `fetch.rs` validates absolute HTTP/HTTPS URLs, fetches text/html, application/xhtml+xml, or text/plain, extracts readable HTML text/title, and enforces character/byte limits.
- `transport.rs` uses blocking reqwest with timeouts, redirect limits, optional bearer auth for search providers, and response-size caps.

Agent exposure is wired through these paths:

- Tool descriptors: `client/src-tauri/src/runtime/agent_core/tool_descriptors.rs` exposes `web_search` and `web_fetch` schemas.
- Tool discovery/catalog: `client/src-tauri/src/runtime/autonomous_tool_runtime/mod.rs` exposes web catalog entries and the `web_search_only`, `web_fetch`, and `web` tool-access groups.
- Planner activation: prompts mentioning docs, documentation, internet, latest, current, web search, or web fetch activate search/fetch without browser-control tools.
- Dispatch: `client/src-tauri/src/runtime/autonomous_tool_runtime/mod.rs` maps `AutonomousToolRequest::WebSearch` and `AutonomousToolRequest::WebFetch` to runtime execution.
- Model-visible results: `client/src-tauri/src/runtime/agent_core/provider_loop.rs` compacts web output and marks web content as untrusted lower-priority data.
- Stream summaries: `client/src-tauri/src/commands/subscribe_runtime_stream.rs` maps web calls into `ToolResultSummaryDto::Web`.

Permissions and policy:

- `web_search` and `web_fetch` are `external_service` tools with network risk metadata.
- Computer Use can use external-service tools. Plan and Crawl policies do not expose them.
- Custom agents need an effective policy/base capability that allows `external_service`.
- Project capability revocation can block external integrations via `external_integration:external_service` or the exact tool name.
- Stage gates still apply at runtime through the normal tool enforcement path.

UI and operator affordances:

- Tool categories include a `Web` category for agent authoring and runtime presentation.
- Tool call summaries render as web summaries in transcript/runtime streams.
- The README documents the env vars:
  - `XERO_AUTONOMOUS_WEB_SEARCH_URL`
  - `XERO_AUTONOMOUS_WEB_SEARCH_BEARER_TOKEN`
- There is no first-party settings UI or CLI command for configuring a search provider.

## Verified Behavior

Focused runtime tests now cover:

- Search calls the configured provider, sends `q` and `limit`, includes bearer auth, normalizes returned titles/snippets/URLs, and marks provider-overflow results as truncated.
- Search without a provider returns the expected user-fixable `autonomous_web_search_provider_unavailable` error.
- Fetch works without a search provider, extracts HTML title/text, normalizes content type, and uses the direct HTTP transport.
- Tool-search catalog fields now match the real schemas: `resultCount` and `maxChars`, not stale `limit` and `maxBytes` names.

## Gaps

Search is incomplete for default end-to-end use because there is no built-in provider and no app-data-backed provider setting. A fresh desktop app can expose `web_search` to an agent, but the call fails until the process environment contains a compatible provider endpoint.

Provider setup is operationally fragile because it depends on environment variables instead of user-facing settings, project/global diagnostics, or a test button.

The provider contract is only implicit in Rust code and a short README env-var note. There is no dedicated operator-facing contract explaining query parameters, response shape, auth handling, limits, or failure modes.

There is no Tauri-level health check that verifies the configured search provider from the same runtime path agents will use.

## Implementation Plan

1. Add app-data-backed autonomous web settings.
   - Add a global app-data table/payload such as `autonomous_web_settings`.
   - Store provider endpoint, auth mode metadata, enabled state, and update timestamps.
   - Store bearer secrets through the existing credential/storage pattern used for provider auth where possible; otherwise keep the token out of diagnostics and redact all summaries.
   - Resolve config in `DesktopState::autonomous_web_config` from settings first, then environment as a developer override.

2. Add Tauri commands and model contracts.
   - Add commands such as `autonomous_web_settings`, `autonomous_web_update_settings`, and `autonomous_web_check_provider`.
   - Validate HTTP/HTTPS endpoints, result limits, and bearer-token presence.
   - Keep new state under OS app-data only.

3. Add a settings UI.
   - Add a user-facing Web Search section in the existing agent/tooling settings area using ShadCN components.
   - Show provider enabled/disabled state, endpoint, masked credential status, last check result, and a test action.
   - Do not add temporary debug UI.

4. Tighten diagnostics and docs.
   - Add doctor/support-bundle output that reports whether web search is configured without exposing tokens.
   - Document the provider contract: GET endpoint, `q`, `limit`, optional bearer auth, JSON response, status-code handling, and body limits.
   - Update README from env-var-only setup to settings-first setup with env override notes.

5. Expand tests.
   - Rust unit tests for settings validation, config resolution, provider check, search/fetch success, missing provider, invalid provider response, non-2xx status mapping, truncation, and redaction.
   - Frontend schema and settings UI tests for save/load/test-provider flows.
   - Runtime stream tests for web call summaries and failures.
   - A scoped integration test with a local mock search provider to prove the same Tauri runtime path works end-to-end.
