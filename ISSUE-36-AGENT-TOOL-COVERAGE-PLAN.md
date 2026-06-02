# Issue 36 Agent Tool Test Coverage Audit And Plan

## Audit Summary

Issue: https://github.com/hyperpush-org/xero/issues/36

Agent-provided tool surfaces audited:

- Tool Registry V2 core: `client/src-tauri/crates/xero-agent-core/src/tool_registry.rs`
  - Existing coverage: descriptor validation, extension manifests and fixtures, schema input validation, policy denials, sandbox denials, approval waits, rollback hooks, budget limits, doom-loop detection, read-only batching, timeout control, mutating sequencing, and result truncation.
  - Gap status: strong coverage; no immediate implementation needed.
- Domain tool packs: `client/src-tauri/crates/xero-agent-core/src/tool_packs.rs`
  - Existing coverage: manifest presence, policy boundaries, self-consistent scenarios, health reports, disabled-pack behavior, and tool-to-pack reverse lookup.
  - Gap status: pack-internal consistency is covered, but cross-surface consistency with the agent-visible autonomous runtime catalog was weak.
- Headless owned-agent runtime: `client/src-tauri/crates/xero-agent-core/src/headless_runtime.rs`
  - Existing coverage: provider tool parsing, registry snapshots, Tool Registry V2 projection, headless identities, observe-only Ask/Plan tool sets, and OpenAI tool round trips.
  - Gap status: covered for Tool Registry V2 execution paths; no immediate implementation needed.
- Autonomous Tauri tool runtime: `client/src-tauri/src/runtime/autonomous_tool_runtime/mod.rs` and submodules
  - Existing coverage: Crawl/Plan allowlists, repository-recon and planning policies, web schema catalog fields, Solana representative requests and redaction, desktop Computer Use manifest diagnostics, desktop rollout gates, Computer Use-only desktop tools, custom-agent tool-policy expansion, subagent role gates, Stages required-check gates, and risky external/browser-control flags.
  - Gap status: add cross-surface tests so domain pack tools cannot silently drift away from `deferred_tool_catalog`, `tool_access` activation groups, or runtime-agent policy classification.
- Tauri command bridges: `client/src-tauri/src/commands/*.rs`
  - Existing coverage: selected command-contract tests exist for agent extension validation, runtime media extraction, list projects, and frontend adapter schemas.
  - Gap status: command bridge coverage is uneven, but the issue priority is the high-risk surfaces provided directly to agents. No UI changes are needed.
- TypeScript canvas and model surfaces: `client/src/lib/xero-model/*` and `client/components/xero/workflow-canvas/*`
  - Existing coverage: runtime protocol parsing, workflow/stage snapshot serialization, graph construction, stage nodes, properties panel policy controls, and Stages terminology in key canvas areas.
  - Gap status: adequate for this issue after Rust runtime drift tests are added.

## Implementation Plan

1. Add autonomous runtime catalog drift tests.
   - Assert every domain tool-pack tool is present in `deferred_tool_catalog(true)`.
   - Assert every domain tool-pack tool has at least one `tool_access` activation group and metadata whose `toolPackIds` include the declaring pack.
   - Assert declared pack activation groups exist in the autonomous runtime access-group table.
2. Add runtime-agent policy classification coverage for cataloged tools.
   - Assert every catalog entry has a known effect class and at least one eligible runtime agent, or a deliberate policy-only explanation.
   - Assert known agent-facing tool access entries are represented in the prompt-visible catalog when enabled.
3. Verify with scoped Rust tests only.
   - Run one Cargo command at a time from `client/src-tauri`.
   - Prefer filtered test runs for the new autonomous runtime tests and any touched core tests.

## Intentional Remaining Gaps

- Full end-to-end Tauri command bridge coverage remains broad and expensive; this plan keeps the change scoped to cross-surface agent tool exposure drift.
- Browser and emulator executor integration behavior remains best tested with existing runtime fakes and unit-level contracts because this Tauri app should not be opened in a browser.
- No backwards-compatibility glue or legacy `.xero/` state paths are introduced.
