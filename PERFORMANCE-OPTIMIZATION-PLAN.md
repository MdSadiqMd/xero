# Xero Performance Hot Points and Optimization Plan

Generated: 2026-06-03

Scope: source-level audit of the Tauri desktop app. This catalog focuses on places where Xero can consume the most user CPU, memory, disk I/O, process slots, IPC bandwidth, or UI render time. It does not rely on browser inspection because this is a Tauri app.

## Executive Summary

The highest-risk performance areas are:

1. Runtime stream replay, event projection, transcript rendering, and IPC payloads.
2. Git status, Git diff, and project-load bundle work during project switching or repository churn.
3. File tree listing, file reading/previews, project search/replace, and workspace indexing/query.
4. Provider streaming, autonomous tool execution, PTY/process output, and terminal sessions.
5. Image/frame/media paths in browser automation, emulator, and desktop-control tooling.
6. SQLite event-log growth, code rollback snapshots, LanceDB agent memory, and storage maintenance.
7. Heavy frontend surfaces such as transcript markdown, diffs, CodeMirror, Mermaid, Shiki, Solana, browser tooling, and emulator views.

The app already has several important guardrails: latest-job cancellation, per-project lanes, frontend request dedupe, IPC payload budgets, runtime stream batching, skipped heavy directories, tree/search result caps, Shiki/markdown caches, and frame coalescing. The optimization work should build on those instead of adding compatibility glue or debug UI.

## Hot Point Catalog

| Rank | Hot point | User-system pressure | Common triggers | Existing guardrails | Primary optimization direction |
| --- | --- | --- | --- | --- | --- |
| 1 | Runtime stream replay and live agent event projection | CPU, memory, SQLite reads, IPC serialization, React render time | Active agent runs, reopening a run, project switch, long transcripts, media-rich tool output | Incremental replay limit, compact IPC patches, runtime stream item budgets, frontend batching, cheap validation outside test/opt-in paths | Make projection indexed and incremental, aggregate provider deltas, snapshot less often, virtualize transcript history |
| 2 | Git repository status and diff | Disk I/O, CPU in libgit2, large IPC payloads | Project load, 5s visible polling, VCS sidebar, large untracked sets, diff view | Latest-job cancellation, request dedupe, payload budgets, patch byte caps, skip untracked contents for status line counts | Event/watch invalidation instead of fixed polling, cache by repository signature, split summaries from full entries, lazy per-file diff |
| 3 | Project load bundle | CPU, disk I/O, SQLite reads, Git status work | Selecting or opening projects, switching between active sessions | Single blocking bundle, latest-job cancellation | Split expensive optional sections from first-paint state, reuse cached repository/runtime summaries, parallelize only non-conflicting reads where safe |
| 4 | File tree, file index, file read, previews, and markdown assets | Disk walking, file reads, hashing, memory from large files/previews | File explorer open, search panel, markdown preview, image assets, large repos | Skipped directories, node/file caps, 2 MB text read limit, latest visible read job, payload budgets | Persistent metadata cache, avoid repeat full-file hashing, lazy asset expansion, reuse tree snapshots by directory |
| 5 | Project search and replace | Disk walking, regex CPU, file reads/writes, memory for result sets | User search in large repo, replace across project | Skipped directories, max file size, max result/file caps, cancellation checks, project file lane for replace | Streaming/paged results, line streaming instead of full `read_to_string`, stronger replace progress/cancel checkpoints |
| 6 | Workspace index, status, and query | Disk walking, SQLite scans, JSON embedding decode, sort CPU | Workspace tab/status refresh, semantic query, indexing changed files | File/byte caps, changed-file detection, SQLite transactions, query limits | Cheap cached status, incremental stale detection, binary embeddings, top-k heap, FTS5 lexical index |
| 7 | Provider streams, autonomous runtime, process manager, and PTY output | CPU, subprocesses, channel traffic, DB writes, terminal memory | Agent runs, shell tools, project runners, long command output | Process manager abstractions, stream event types, terminal history surfaces | Coalesce token/output deltas, byte-rate limit outputs, bounded ring buffers, batch DB writes |
| 8 | Browser automation and native CDP | Subprocesses, screenshots, image decode/compare, event telemetry | Browser sessions, diagnostics, screenshots, accessibility snapshots | Payload caps, diagnostic caps, resize/event coalescing | Lazy diagnostics, downsample screenshots, keep large image bytes off IPC, cap event rings by active view |
| 9 | Emulator sessions and frame delivery | CPU/GPU, encode/decode, memory, IPC/event pressure | Android/iOS emulator use, screenshots, active device stream | Frame bus metadata events, frontend frame coalescer, visibility checks | Adaptive FPS/quality, pause or lower rate when hidden, thumbnail-first image delivery, bounded frame cache |
| 10 | Desktop-control screenshots/OCR/clipboard media | CPU, image encoding, OCR, base64 memory | Autonomous desktop tools, screenshot checks, clipboard image reads | Base64 size caps and latency fields in tool output | Queue and dedupe screenshots, defer OCR until needed, store media by reference, downscale for checks |
| 11 | SQLite project store and agent event logs | DB size, read latency, write amplification, memory during replay | Long agent runs, many tool calls, project history views | Indexes for runtime events/messages/tool calls, read-latest APIs | Event compaction, retention policy, query-plan tests, WAL/checkpoint tuning, batched writes |
| 12 | Code rollback and code history snapshots | Disk I/O, hashing, blob storage, SQLite writes | Agent edits, rollback capture, large file changes | Scoped capture logic, pruning/validation paths | Hash-before-copy, blob dedup, incremental snapshots, background maintenance |
| 13 | LanceDB-backed agent memory and retrieval | CPU, disk I/O, vector search latency, background storage maintenance | Memory insert/search, long-running agent memory growth | Dedicated small Tokio runtime, connection cache, schema quarantine/recreate | Batch optimize/compact, bound fallback scans, retrieval cache by scope/fingerprint, vector latency metrics |
| 14 | Rich frontend render surfaces | JS CPU, layout, memory, bundle load | Long conversations, diffs, markdown/code blocks, Mermaid, CodeMirror tabs | Memoized transcript components, Shiki/token caches, markdown segment caches, visible-range smoke tests | Virtualize long transcript/diff surfaces, stable selectors, lazy-load heavy modules, enforce render budgets |
| 15 | Solana workbench | External processes, RPC/log polling, feed rendering | Local validator, audit/fuzz, log stream, account/program views | Feed caps, log poll interval, dedupe windows | Pause pollers when hidden, backpressure log streams, process lanes, incremental feed rendering |

## Detailed Findings

### 1. Runtime stream and transcript pipeline

Key files:

- `client/src-tauri/src/commands/subscribe_runtime_stream.rs`
- `client/src/lib/xero-desktop.ts`
- `client/src/features/xero/use-xero-desktop-state/runtime-stream.ts`
- `packages/ui/src/model/runtime-stream.ts`
- `packages/ui/src/components/transcript/conversation-section.tsx`
- `packages/ui/src/components/transcript/conversation-markdown.tsx`
- `packages/ui/src/lib/shiki.ts`

Why it is hot:

- Replaying a run loads persisted agent events from SQLite, projects those events, extracts media from tool results, estimates/compacts JSON payloads, and sends IPC items.
- Live runs can generate high-frequency provider deltas and tool events.
- The shared runtime stream model repeatedly filters, maps, sorts, and rebuilds timeline arrays while merging events.
- Transcript rendering includes markdown segmentation, code highlighting, Mermaid, diff blocks, tool cards, and animated turn sections.

Existing strengths:

- Incremental replay defaults to a bounded latest-event window.
- Runtime stream payloads are compacted and capped before IPC.
- Frontend delivery is batched with a small per-tick time budget.
- Markdown and Shiki have cache and byte limits.
- Existing performance smoke tests cover bursty runtime events, cache memory bounds, and large visible lists.

Optimization plan:

1. Convert runtime stream projection to an indexed model with ordered id arrays plus maps for transcript items, tool calls, plans, and actions. Avoid whole-array filter/sort work for each event.
2. Aggregate provider token/message deltas into 20-50 ms chunks before persistence and IPC. Store a compact final message and optional sampled deltas for debugging or replay fidelity.
3. Send live event deltas as deltas. Reserve full snapshots for initial replay, repair, and periodic checkpoints.
4. Add conversation virtualization or windowing for long transcripts. Keep the latest tail mounted, with jump/search affordances for older turns.
5. Extend the runtime smoke test from 1,000 events to a larger stress case and record merge duration, retained bytes, and flush count.

### 2. Git status, diff, and project-load bundle

Key files:

- `client/src-tauri/src/commands/get_project_load_bundle.rs`
- `client/src-tauri/src/commands/get_repository_status.rs`
- `client/src-tauri/src/commands/get_repository_diff.rs`
- `client/src-tauri/src/git/repository.rs`
- `client/src-tauri/src/git/diff.rs`
- `client/src/features/xero/use-xero-desktop-state.ts`
- `client/src/features/xero/use-xero-desktop-state/runtime-stream.ts`
- `client/src/lib/backend-request-coordinator.ts`

Why it is hot:

- Project load currently bundles project metadata, repository status, runtime session, runtime run, and autonomous run lookups.
- Repository status uses libgit2 status traversal, including untracked files and untracked directories.
- Diff rendering can include untracked file content and patch generation, which is expensive in large repos.
- The frontend polls visible repository status every 5 seconds, so a large working tree can repeatedly consume disk and CPU.

Existing strengths:

- Backend jobs cancel stale latest work.
- Repository status and diff have frontend request dedupe and payload budgets.
- Status line-counting intentionally skips untracked file contents.
- Diff patches are byte-capped.

Optimization plan:

1. Add a repository status cache keyed by project id, HEAD oid, index mtime/size, worktree signature, and a short untracked-directory signature.
2. Replace fixed visible polling with file-system watcher invalidation plus a slow fallback poll.
3. Split repository status into cheap summary and full entries. Most UI surfaces should request the summary only.
4. Make untracked file content diff lazy. Show untracked paths first, then read/render content only when the user expands or opens the file.
5. Add temp-repo performance tests for thousands of untracked files and large diffs.

### 3. Files, search, and previews

Key files:

- `client/src-tauri/src/commands/project_files.rs`
- `client/src-tauri/src/commands/search_project.rs`
- `client/src/lib/file-system-tree.ts`

Why it is hot:

- File listing walks the project tree and builds folder/file indexes.
- File reads detect type, hash files, read text previews, inspect CSV/markdown, and inspect image assets.
- Markdown previews can discover image references and hash/read referenced assets.
- Project search walks the tree and reads each candidate file up to configured size limits.
- Replace can walk, read, modify, and write many files.

Existing strengths:

- Heavy directories such as `.git`, `node_modules`, `dist`, `build`, and `target` are skipped.
- File tree, file index, search, and read commands have caps and latest-job cancellation.
- Replace runs in the per-project file lane.
- The frontend stores file trees in normalized maps instead of rebuilding a nested tree every time.

Optimization plan:

1. Introduce a persistent file metadata cache under OS app-data keyed by project id, path, mtime, size, and inode/file id where available.
2. Reuse cached hashes and detected file types for preview reads and markdown asset resolution.
3. Stream search results in pages or chunks instead of building a full result set before returning.
4. Replace full-file `read_to_string` in search with line streaming for large text files.
5. Add replace progress checkpoints and tighter cancellation checks before each write.

### 4. Workspace index and semantic query

Key files:

- `client/src-tauri/src/commands/workspace_index.rs`
- `client/src-tauri/src/db/migrations.rs`

Why it is hot:

- Workspace status scans candidates and compares indexed fingerprints.
- Workspace query currently loads indexed rows, decodes embedding JSON, scores rows, and sorts candidates.
- Indexing reads changed files, extracts features, computes embeddings, and writes rows in SQLite.

Existing strengths:

- Indexing has hard caps for files and bytes.
- Index updates use transactions.
- Query limits are bounded.

Optimization plan:

1. Split `workspace_status` into a cheap cached status and an explicit refresh/stale-scan path.
2. Use Git status and file watcher events to maintain stale candidate sets incrementally.
3. Store embeddings in a compact binary representation or a dedicated vector side table instead of JSON.
4. Use a top-k heap for semantic scoring rather than sorting all rows.
5. Add FTS5 for lexical scoring so query can avoid scanning every document for simple text relevance.
6. Add query-plan tests for workspace index queries and performance tests for 5k, 20k, and capped datasets.

### 5. Processes, tools, provider streams, and terminal output

Key files:

- `client/src-tauri/src/runtime/agent_core/provider_loop.rs`
- `client/src-tauri/src/runtime/agent_core/provider_adapters.rs`
- `client/src-tauri/src/runtime/autonomous_tool_runtime/process.rs`
- `client/src-tauri/src/runtime/autonomous_tool_runtime/process_manager.rs`
- `client/src-tauri/src/commands/project_runner.rs`

Why it is hot:

- Provider streams can produce many small deltas.
- Tool execution can spawn subprocesses and stream stdout/stderr at high rates.
- Terminal sessions can accumulate large retained output and frequent UI events.
- Runtime persistence can amplify high-frequency output into DB writes and IPC messages.

Existing strengths:

- Process execution is centralized through runtime/process manager code.
- Project runner commands use blocking jobs and terminal event channels.

Optimization plan:

1. Coalesce provider and process output into bounded time/byte chunks before DB writes and IPC.
2. Apply byte-rate limits per run, per tool, and per terminal session.
3. Store terminal output in bounded ring buffers with persisted checkpoints only when needed.
4. Emit summarized output metadata for hidden/inactive views and full chunks only for active views.
5. Add orphan-process audits and shutdown telemetry to prevent leaked subprocesses.

### 6. Browser, emulator, and desktop-control media

Key files:

- `client/src-tauri/src/commands/browser/native_cdp.rs`
- `client/src-tauri/src/commands/browser/mod.rs`
- `client/src-tauri/src/commands/browser/actions.rs`
- `client/src-tauri/src/commands/emulator/android/mod.rs`
- `client/src/features/emulator/use-emulator-session.ts`
- `client/src/lib/frame-governance.ts`
- `client/src-tauri/src/runtime/autonomous_tool_runtime/desktop_control.rs`

Why it is hot:

- Screenshots and frame streams create image encode/decode cost, large buffers, and possible base64 amplification.
- Browser diagnostics, accessibility trees, console/network logs, and screenshots can be large.
- Emulator streams can produce frame updates faster than the UI needs.
- Desktop-control OCR and image checks can be CPU-heavy.

Existing strengths:

- Emulator frame events keep frame bytes out of normal IPC payloads.
- Frontend frame coalescing drops redundant frames.
- Browser and emulator payload budgets already exist.

Optimization plan:

1. Make all media streams visibility-aware: active view gets normal cadence, background view gets reduced cadence or metadata only.
2. Use adaptive quality and resolution for screenshots and emulator frames.
3. Store large screenshots/frames by reference and send only ids plus metadata over IPC.
4. Defer OCR, image diffing, and accessibility tree collection until a tool or visible UI actually needs them.
5. Keep bounded event rings for console/network/download logs.

### 7. SQLite, event logs, code rollback, and LanceDB

Key files:

- `client/src-tauri/src/db/project_store/agent_core.rs`
- `client/src-tauri/src/db/project_store/code_rollback.rs`
- `client/src-tauri/src/db/project_store/code_history.rs`
- `client/src-tauri/src/db/project_store/agent_memory_lance.rs`
- `client/src-tauri/src/db/project_store/agent_retrieval.rs`
- `client/src-tauri/src/db/migrations.rs`

Why it is hot:

- Long runs create many messages, events, tool calls, and payloads.
- Code rollback and code history can hash, snapshot, and store large file changes.
- LanceDB agent memory search and maintenance can consume CPU and disk I/O as memory grows.

Existing strengths:

- Runtime event/message/tool-call tables have indexes for run-oriented reads.
- Agent memory uses a small dedicated Tokio runtime and cached connections.
- LanceDB schema mismatch paths quarantine/recreate app-data state, matching the no-backwards-compatibility policy for this new app.

Optimization plan:

1. Add event-log compaction: periodic run summaries plus retained raw tail events.
2. Batch adjacent event/message writes during high-frequency streams.
3. Add `EXPLAIN QUERY PLAN` tests for hot runtime, history, and workspace queries.
4. Add storage maintenance for old rollback blobs, duplicate blobs, and stale LanceDB data.
5. Schedule LanceDB optimize/compact work when the app is idle, with strict time and project-scoped limits.

### 8. Frontend render and bundle pressure

Key files:

- `client/src/App.tsx`
- `client/src/features/xero/use-xero-desktop-state.ts`
- `packages/ui/src/model/runtime-stream.ts`
- `packages/ui/src/components/transcript/conversation-section.tsx`
- `packages/ui/src/components/transcript/conversation-markdown.tsx`
- `packages/ui/src/lib/shiki.ts`
- `client/src/performance/performance-smoke.test.tsx`

Why it is hot:

- Large app state and runtime stream updates can trigger expensive selectors and derived view rebuilds.
- Long transcripts, diffs, code blocks, Mermaid diagrams, and CodeMirror editors are heavy.
- Optional feature surfaces can add bundle and initialization cost even when unused.

Existing strengths:

- High-churn runtime state is isolated from regular React state in several places.
- Markdown, Shiki, diff, and list-windowing have smoke-test coverage.
- Large lists already have visible-range tests.

Optimization plan:

1. Add React Profiler based performance tests for transcript tail updates, project switching, VCS sidebar open, and file preview open.
2. Lazy-load feature surfaces behind tabs/dialogs: Solana workbench, browser tools, emulator tools, CodeMirror, Mermaid, and Shiki languages/themes.
3. Move expensive derived runtime views to stable memoized selectors with explicit invalidation.
4. Add render budget assertions to the existing browser-free smoke suite.
5. Keep UI changes user-facing only. Do not add temporary debug panels or test-only controls.

## Execution Plan

### Phase 1: Baseline measurements

Goal: make the hottest paths measurable without adding debug UI.

Tasks:

1. Add backend timing spans or structured log samples around:
   - Project load bundle.
   - Repository status and diff.
   - Project file list/index/read.
   - Project search and replace.
   - Workspace status/query/index.
   - Runtime stream replay/projection/media extraction.
   - Provider/process output persistence.
   - Browser/emulator/desktop-control screenshot/frame paths.
2. Extend `client/src/performance/performance-smoke.test.tsx` with budget assertions for:
   - Runtime stream merge duration and retained bytes.
   - Repository status churn.
   - Workspace query over generated datasets.
   - Transcript tail rendering.
   - Diff/file-list visible window rendering.
3. Define starting budgets:
   - Warm project switch: p95 under 250 ms for UI-ready state.
   - Cold project switch on large repo: p95 under 1,000 ms for UI-ready state.
   - Repository summary: p95 under 150 ms small repo, under 750 ms large repo.
   - Runtime stream flush: under 8 ms per animation frame in the smoke test.
   - Workspace query: under 100 ms for 5k indexed files after warm cache.
   - IPC payloads: zero over-max samples in test and development runs.

### Phase 2: Runtime stream first

Goal: reduce the most visible high-churn path.

Tasks:

1. Refactor runtime stream merge state to use maps and ordered ids.
2. Aggregate provider deltas before persistence and IPC.
3. Send fewer snapshots during live streaming.
4. Virtualize or window transcript rendering for long conversations.
5. Add regression tests for 10k runtime events and media-rich tool output.

### Phase 3: Git and project switching

Goal: make project switching and repository churn predictable.

Tasks:

1. Add repository status summary cache and invalidation.
2. Move from fixed status polling to watcher-driven invalidation with fallback polling.
3. Split summary and full repository entry APIs.
4. Lazy-load untracked content diffs.
5. Add large temp-repo tests with many untracked files and changed files.

### Phase 4: File, search, and workspace indexing

Goal: reduce repeated full-tree and full-index scans.

Tasks:

1. Add persistent file metadata cache in OS app-data, not `.xero/`.
2. Reuse file hashes/type detection across previews, search, and markdown assets.
3. Stream search results and use line streaming for large text files.
4. Split cheap workspace status from expensive stale scans.
5. Store embeddings compactly and use top-k query selection plus FTS5 lexical scoring.

### Phase 5: Storage and process backpressure

Goal: prevent long-running sessions from growing without bound.

Tasks:

1. Batch high-frequency event/message writes.
2. Add event-log compaction and raw-tail retention.
3. Add rollback blob dedup and maintenance.
4. Bound PTY/process/provider output by bytes and active-view demand.
5. Add query-plan tests for hot SQLite reads.

### Phase 6: Media and optional feature surfaces

Goal: keep expensive image/process features quiet when not visible or needed.

Tasks:

1. Apply visibility-aware frame/screenshot cadence to emulator, browser, and desktop-control media.
2. Store large media by reference and send metadata through IPC.
3. Defer OCR, accessibility snapshots, and image comparison until explicitly needed.
4. Lazy-load Solana, browser, emulator, CodeMirror, Mermaid, and Shiki-heavy paths.
5. Add smoke budgets for feature activation cost.

## Verification Strategy

Use scoped tests and measurements:

- Frontend performance smoke tests in `client/src/performance/performance-smoke.test.tsx`.
- Targeted Rust unit/integration tests for each backend module touched.
- One Cargo command at a time.
- No browser-based verification for this app.
- No temporary debug UI.
- No repo-wide Rust format/test sweeps unless explicitly requested.

For backend performance changes, prefer deterministic generated fixtures:

- Large Git repositories with many untracked files.
- Runtime runs with 1k, 10k, and media-rich events.
- Workspace indexes with 5k and 20k generated rows.
- File trees with skipped heavy directories and many visible source files.
- Terminal/process output streams with high stdout/stderr rates.

## Guardrails and Non-Goals

- Do not add backwards-compatible migrations or glue code for stale app-data state unless explicitly requested. If incompatible app-data state blocks testing, wipe the affected OS app-data state.
- Do not write new project state under `.xero/`; it is legacy repo-local state.
- Do not add temporary debug or test UI.
- Do not rename the runtime per-run Stages concept while optimizing agent-canvas internals.
- Do not create branches or stash work unless requested.
- Do not optimize by removing payload caps, cancellation, or result limits. Tighten them with better UX and measurements instead.

## Recommended First Milestone

Start with runtime stream plus Git status because they combine high user visibility with clear existing boundaries.

Deliverables:

1. Measurement-only patch for runtime replay/merge and repository status/diff.
2. Indexed runtime stream merge model with existing behavior preserved.
3. Repository status summary cache with watcher invalidation.
4. Extended performance smoke tests for runtime bursts and large Git status.
5. A short before/after report from the scoped tests.
