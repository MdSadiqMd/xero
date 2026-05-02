# UI/UX Latency Optimization Plan

Reader: an internal Xero engineer or agent responsible for making the desktop UI feel immediate under normal editor, agent, repository, and sidebar workflows.

Post-read action: implement the optimization work in safe phases, with measurable before/after evidence, without adding temporary debug UI or changing product behavior for its own sake.

Status: draft.

## Goal

Xero currently feels like many actions carry small but noticeable latency. The audit indicates this is not a single obvious blocking operation. It is the compound effect of broad React re-renders, high-frequency runtime events, eager editor code, hidden-but-mounted UI surfaces, layout polling, and unstable callback identities.

The goal is to reduce sub-frame and low-millisecond friction across the UI layer while preserving the current product shape:

- Agent streaming should remain smooth during tool-heavy runs.
- Editor typing should stay local to the editor hot path.
- Opening and closing sidebars should not disturb heavy panes.
- Repository status updates should not trigger unnecessary diff reloads.
- Inactive views should not participate in active-view updates.
- Startup and first interaction should not pay for editor/language functionality before it is needed.

## Product Constraints

- This is a Tauri desktop app. Do not validate the app by opening it in a normal browser.
- Use ShadCN/Radix UI components where possible for any user-facing UI changes.
- Do not add temporary debug or test UI. Profiling and measurement must live in tests, scripts, traces, or developer tooling, not in the product surface.
- Run scoped tests and format checks. Avoid repo-wide Rust commands unless a phase explicitly needs them.
- Run only one Cargo command at a time.
- Do not create branches or stashes unless explicitly asked.
- New durable state belongs under OS app-data, not the legacy repo-local `.xero/` state.
- Backwards compatibility is prohibited unless explicitly requested; prefer clean new contracts over compatibility shims.

## Research Principles To Apply

VS Code keeps user-facing operations responsive by isolating extension work from the renderer and lazily activating extension behavior. Xero should apply the same principle to runtime streams, repository work, and feature sidebars: data production and validation should not force the entire UI shell to render.

VS Code's editor performance work also shows that real hot paths can be surprising. Measure the actual slow calls before and after each phase; do not assume a data-structure or native boundary change is an automatic win.

Zed optimizes around frame budgets and foreground/background separation. Treat the UI thread as a scarce foreground executor. Anything that is not immediately visible or required for the current input should be deferred, coalesced, or moved out of the hot path.

CodeMirror already virtualizes the editor viewport and separates DOM write/measure phases. Xero should avoid wrapping it in a React-controlled pattern that forces whole-document strings and parent state updates on every keystroke.

## Baseline Hypotheses

The audit found these likely contributors:

1. Centralized desktop state causes broad App-level re-renders.
2. Runtime and repository events update several top-level state slices per event.
3. Runtime stream item validation and delivery happen on the hot path before React updates.
4. Agent, workflow, execution, and sidebar surfaces remain mounted while hidden.
5. CodeMirror language packages are eagerly imported.
6. CodeMirror changes are mirrored through React by calling full-document `toString()` on every edit.
7. Browser sidebar resize handling reads layout every animation frame while open.
8. VCS diff loading depends on unstable array and callback identities.
9. Some rail/sidebar animations still use layout-affecting properties in frequently changing surfaces.
10. The shell/titlebar re-renders for state changes it does not visually depend on.

## Success Metrics

Use these as targets, not as dogma. If a target is unrealistic after profiling, update the plan with evidence.

- Agent stream burst: one visible React commit per frame or fewer during high-frequency stream events.
- Runtime event handling: no repeated full-shell renders for stream-only updates.
- Editor typing: keystrokes should not cause App shell, project rail, or inactive sidebars to re-render.
- Sidebar open/close: no continuous layout polling after the sidebar reaches steady state.
- VCS panel: selected-file diff is not refetched unless project, selected path, selected scope, or repository revision changes.
- Bundle shape: editor language code is not part of the initial app chunk unless the first view actually needs an editor.
- Long tasks: normal agent streaming, editor typing, and sidebar toggles should avoid main-thread tasks over one frame budget in production profiling.

## Phase 0: Baseline And Measurement

Purpose: establish proof before changing architecture.

Work:

- Add or use a production-mode profiling harness for the Tauri frontend. The harness must not add visible product UI.
- Capture baseline React commit counts for these workflows:
  - Open a project and switch between Workflow, Agent, and Editor.
  - Run or replay a high-volume runtime stream burst.
  - Type in the code editor for several seconds.
  - Open, resize, and close browser/VCS/usage sidebars.
  - Receive repeated repository status events while VCS is closed and while it is open.
- Capture bundle composition from the Vite production build.
- Identify the top render causes in App shell, project rail, agent runtime, execution workspace, and sidebars.
- Add small test fixtures or replay utilities for stream bursts and repository status updates if none exist.

Acceptance criteria:

- A baseline note exists in this plan or a companion section with commit counts, bundle sizes, and the slowest user-visible workflows.
- The measurement path can be re-run after each phase.
- No temporary debug UI was introduced.

Verification:

- Run the scoped frontend build.
- Run only the relevant frontend tests for any harness helpers.
- If a Rust replay/helper is added, run only the scoped Rust test for that helper.

## Phase 1: Split UI State By Ownership

Purpose: stop small state changes from waking the whole application shell.

Work:

- Inventory every value currently returned by the main desktop state hook and assign an owner:
  - App/session shell state.
  - Project list and active project metadata.
  - Repository status and diffs.
  - Runtime sessions, runs, streams, and actions.
  - Provider, MCP, skill, doctor, and account settings.
  - Sidebar open/closed UI state.
  - Editor workspace state.
- Introduce selector-based subscriptions for high-churn stores. Prefer a small `useSyncExternalStore`-based store or an already-established local pattern over a new dependency.
- Keep low-churn setup and settings state in normal React state where that is simpler.
- Move runtime stream data out of the top-level App render path.
- Move repository status into a store that lets the shell subscribe only to counts/branch label while VCS subscribes to entries/diffs.
- Split titlebar/shell into memoized leaves so a stream item cannot re-render menus and static controls.
- Stabilize callbacks passed from App into sidebars and panes with `useCallback` where identity currently causes downstream effects.

Acceptance criteria:

- Stream-only updates do not re-render inactive panes or closed sidebars.
- Repository count updates can update the shell badge without forcing VCS diff effects.
- The shell/titlebar only re-renders when shell-visible props change.
- The implementation has no compatibility layer for old state contracts unless a direct caller still needs it during the phase.

Verification:

- Run scoped TypeScript tests for the changed state/store helpers.
- Re-run Phase 0 profiler workflows for agent stream burst and repository status events.
- Run the scoped frontend build.

## Phase 2: Coalesce Runtime And Repository Events

Purpose: turn high-frequency backend events into predictable UI commits.

Work:

- Add an event buffer for runtime stream items outside React state.
- Flush non-urgent stream updates once per animation frame or in a 4-8ms batch window.
- Preserve immediate delivery for critical state transitions:
  - user approval required
  - failure
  - cancellation
  - run completed
  - authentication/session invalid
- Coalesce repeated repository status events by project and revision.
- Avoid mapping the full project list for every runtime update when the changed field is not visible.
- Keep validation safety, but avoid full Zod parsing on every hot stream item if Rust already owns the event contract. Options:
  - Validate command responses and control events normally.
  - Validate a sampled or batched subset of stream items in development/test.
  - Keep a cheap runtime guard for item kind, sequence, run id, and project id in production.
- Add sequence-gap handling that reports a durable error without spamming React.

Acceptance criteria:

- A burst of stream items produces bounded UI commits.
- Critical events still appear without user-visible delay.
- Stream ordering and deduplication remain correct.
- Repository updates no longer reset diffs or project metadata when the effective status did not change.

Verification:

- Add tests for stream buffering, urgent bypass, deduplication, and sequence handling.
- Add tests for repository status coalescing.
- Re-run the agent stream burst profile.
- Run the scoped frontend build.

## Phase 3: De-Control The Code Editor Hot Path

Purpose: let CodeMirror own typing while React observes only the metadata it needs.

Work:

- Lazy-load the CodeEditor surface and language extensions. Initial app startup should not import every supported CodeMirror language.
- Replace eager first-party language imports with async language resolvers and compartments.
- Keep the editor document in CodeMirror state during typing.
- Replace per-keystroke full-document `toString()` with one of:
  - change-set/delta forwarding,
  - debounced full snapshot persistence,
  - dirty flag plus explicit snapshot on save/tab switch,
  - or a hybrid where tiny files snapshot eagerly and larger files debounce.
- Throttle cursor position updates to animation frames.
- Avoid whole-document replacement when the external value changes because of the local editor's own edit.
- When loading a new file, create a fresh editor state or transaction according to CodeMirror guidance so undo history and scroll state are correct.
- Keep read-only/theme/language reconfiguration through compartments.

Acceptance criteria:

- Typing in the editor does not re-render the App shell, project rail, or unrelated sidebars.
- Large-file typing avoids full-document string creation on every keystroke.
- Language code is loaded on demand.
- Save, dirty-state, cursor display, theme switching, read-only mode, and file switching still work.

Verification:

- Add focused tests for editor change handling, dirty state, save behavior, file switching, and cursor throttling helpers.
- Re-run the editor typing profile.
- Compare build chunks before and after.
- Run the scoped frontend build.

## Phase 4: Freeze Or Unmount Inactive Surfaces

Purpose: keep inactive views from participating in active-view work.

Work:

- Classify each hidden surface:
  - Must preserve live state while hidden.
  - Can unmount and restore from durable/store state.
  - Can lazy mount on first open.
- Convert heavy sidebars to lazy-mounted bodies. Keep only a tiny stable shell mounted if layout requires it.
- Use the existing VCS pattern as the default: closed panels should not render heavy body content.
- For Agent, Workflow, and Execution panes, prefer one mounted active pane plus cached data stores over three always-active React subtrees.
- Preload likely-next heavy panes on idle, hover, or first project load if first-open latency becomes visible.
- Preserve focus and accessibility semantics when surfaces mount/unmount.
- Keep all newly visible controls user-facing; do not add development-only toggles.

Acceptance criteria:

- Closed sidebars do not re-render on unrelated App state changes.
- Inactive main panes do not process stream, editor, or repository updates unless they own the visible result.
- Opening a previously visited surface restores expected user state.
- First-open latency is measured and acceptable, or preloading is added.

Verification:

- Add focused component tests for open/close state preservation where practical.
- Re-run render-count profiles for view switching and sidebar toggles.
- Run the scoped frontend build.

## Phase 5: Replace Layout Polling With Observers

Purpose: remove continuous layout work from steady-state UI.

Work:

- Replace browser sidebar `requestAnimationFrame` resize polling with `ResizeObserver`.
- Trigger native browser resize on:
  - sidebar open
  - active tab change
  - observed viewport size/position change
  - transition end after width animation
  - explicit user resize drag frames
- During active drag, throttle resize IPC to one call per animation frame.
- When not dragging and no size changes occur, do not read layout every frame.
- Audit other components for steady-state `getBoundingClientRect`, layout reads in loops, or repeated measure/write cycles.

Acceptance criteria:

- Browser sidebar has no perpetual rAF loop in steady state.
- Native child webview remains correctly positioned across open, close, tab switch, tool overlay, and resize.
- IPC calls are bounded during active resizing.

Verification:

- Add tests for resize scheduling helpers if extracted.
- Manually validate through Tauri, not a normal browser.
- Re-run sidebar open/resize profile.

## Phase 6: Stabilize VCS And Repository Workflows

Purpose: make source-control updates feel quiet and deterministic.

Work:

- Memoize VCS callbacks from the App layer.
- Add a repository status revision or stable hash to the repository store.
- Make selected diff loading depend on project id, selected path, selected scope, and repository revision.
- Do not depend on a freshly mapped full entries array for diff loading.
- Cache diff results by project, revision, scope, and path.
- Avoid resetting selected diffs when repository status is effectively unchanged.
- Keep explicit refresh behavior user-facing and predictable.

Acceptance criteria:

- Selected diff does not reload during unrelated App renders.
- Repository badge/count updates do not disrupt the selected VCS file.
- VCS remains accurate after staging, unstaging, discard, commit, and refresh.

Verification:

- Add component/helper tests for selected scope derivation and diff cache invalidation.
- Run scoped tests for VCS helpers/components.
- Re-run repository event and VCS open profiles.

## Phase 7: Reduce Layout-Affecting Animation

Purpose: keep polish without paying avoidable layout cost.

Work:

- Keep the existing CSS-driven sidebar width animation and containment model.
- Audit remaining Motion usage for layout-affecting properties such as `width`, `max-width`, `height`, and layout springs in hot areas.
- Prefer transform/opacity for content reveal.
- For rail labels and dense toolbars, use stable grid/flex tracks and clip/fade content instead of animating intrinsic sizes.
- Disable nonessential transitions during tab/view switches using the existing layout-shifting guard pattern.
- Ensure reduced-motion behavior remains correct.

Acceptance criteria:

- Frequently toggled UI surfaces do not animate intrinsic layout properties unless the animation is isolated and measured.
- Tab/view switches do not trigger sidebar width transitions.
- Motion still feels polished but no longer causes repeated reflow through heavy children.

Verification:

- Re-run sidebar and view-switch profiles.
- Run scoped component tests if helper behavior changes.
- Run the scoped frontend build.

## Phase 8: Bundle And Startup Cleanup

Purpose: reduce initial parse/compile and first-interaction cost.

Work:

- Confirm CodeMirror and language chunks are not statically imported by the initial app entry.
- Lazy-load heavy optional surfaces:
  - editor workspace
  - browser tools
  - games sidebar
  - emulator sidebars
  - Solana workbench
  - workflow builder if not first screen
  - settings subpanels with expensive provider/model registries
- Keep common ShadCN primitives shared; avoid splitting tiny primitives into excessive chunks.
- Audit Shiki language/theme chunks and load only when the rendering path actually needs them.
- Add preload hints on user intent where needed, such as hovering the Editor tab or opening a project that last used the Editor view.

Acceptance criteria:

- Initial bundle no longer includes editor languages unless required by initial route/view.
- Heavy optional surfaces load on first use or preload by intent.
- Startup and first project open profiles improve or stay neutral.

Verification:

- Compare Vite build output before and after.
- Run the scoped frontend build.
- Run startup/first-interaction profile in Tauri.

## Phase 9: Regression Tests And Performance Gates

Purpose: keep the latency work from decaying.

Work:

- Add replay tests for:
  - high-volume runtime streams
  - repository status churn
  - editor typing/change handling
  - sidebar resize scheduling
  - VCS diff cache invalidation
- Add a lightweight performance smoke script that can run locally without opening a browser.
- Make the script report:
  - render/commit counts for fixtures where feasible
  - stream flush count
  - bundle chunk sizes
  - slowest measured tasks in the replay
- Do not make flaky wall-clock thresholds block normal development. Prefer structural assertions, counts, and generous budgets.
- Document the profiling procedure in this plan or a short companion doc if it becomes too long.

Acceptance criteria:

- The highest-risk regressions have tests or repeatable profiling checks.
- The performance smoke path is documented and can be run by a future agent.
- No product UI exists only for measurement.

Verification:

- Run the new smoke script.
- Run the scoped frontend test suite for changed helpers/components.
- Run any scoped Rust tests for changed Tauri commands or event projection.
- Run the scoped frontend build.

Implementation note (2026-05-02):

- Browser-free smoke path: `pnpm run performance:smoke`.
- Replay fixture: `client/src/performance/performance-smoke.test.tsx`.
- Companion procedure: `docs/ui-ux-performance-smoke.md`.
- The smoke path reports runtime stream flush counts, repository shell commit counts, editor cursor coalescing counts, sidebar resize scheduling counts, VCS diff cache invalidation counts, slowest replay task timings, and production bundle chunk sizes.

## Suggested Implementation Order

1. Phase 0: baseline measurement.
2. Phase 2: stream coalescing, because agent runs likely produce the most frequent updates.
3. Phase 1: state ownership split for runtime and repository stores.
4. Phase 3: editor hot-path cleanup.
5. Phase 5: browser sidebar observer replacement.
6. Phase 6: VCS identity and cache stabilization.
7. Phase 4: inactive surface freeze/unmount.
8. Phase 7: animation cleanup.
9. Phase 8: bundle/startup cleanup.
10. Phase 9: performance gates.

Phases 1 and 2 may overlap, but keep their commits/slices separate: first make event delivery bounded, then shrink the number of components that observe the delivered state.

## Slice Breakdown

### Slice A: Runtime Stream Buffer

Deliverable: buffered stream store with urgent bypass and tests.

Risk: missed or delayed critical agent events.

Verification focus: ordering, deduplication, urgent events, render commits under replay.

### Slice B: Runtime Store Selectors

Deliverable: App no longer owns high-frequency runtime stream state directly.

Risk: stale active run/session projection.

Verification focus: agent session switching, run start/stop, stream display, approval prompts.

### Slice C: Repository Store Selectors

Deliverable: shell badge, project metadata, VCS entries, and diffs subscribe to separate slices.

Risk: repository status desync after git operations.

Verification focus: branch label, counts, selected file, diff invalidation.

### Slice D: CodeEditor Hot Path

Deliverable: lazy languages plus debounced/delta editor change propagation.

Risk: dirty state or save semantics regress.

Verification focus: edit/save/file switch/read-only/theme/language changes.

### Slice E: Sidebar Lifecycle

Deliverable: heavy sidebar bodies lazy mount or unmount when closed.

Risk: losing user state on close.

Verification focus: close/reopen state, accessibility, first-open profile.

### Slice F: Browser Resize Scheduler

Deliverable: observer-based native webview resize.

Risk: native webview drift or stale position.

Verification focus: Tauri manual check plus resize scheduling tests.

### Slice G: Motion Cleanup

Deliverable: hot interactions avoid layout-affecting Motion patterns.

Risk: visual regressions or abrupt-feeling controls.

Verification focus: view switch, rail collapse, sidebar toggle, reduced motion.

### Slice H: Bundle Cleanup

Deliverable: initial chunks shed editor/language/optional-surface weight.

Risk: first-use loading delay.

Verification focus: build output, first-use preload behavior, Tauri startup.

## Detailed Acceptance Checklist

- [ ] Baseline profile captured before optimization work.
- [ ] Runtime stream updates are buffered outside React.
- [ ] Critical runtime events bypass normal buffering.
- [ ] Runtime store subscriptions are selector-based.
- [ ] Repository status and diff state are selector-based.
- [ ] App shell does not subscribe to full runtime streams.
- [ ] Shell/titlebar is memoized or split enough to avoid irrelevant renders.
- [ ] CodeEditor does not call full-document `toString()` on every normal edit.
- [ ] CodeMirror languages are loaded on demand.
- [ ] Closed heavy sidebars do not render their bodies.
- [ ] Inactive main panes are frozen, unmounted, or subscribed only to stable external stores.
- [ ] Browser sidebar has no steady-state rAF layout polling.
- [ ] VCS diff loading depends on stable keys.
- [ ] Hot animations avoid uncontained intrinsic layout properties.
- [ ] Build output shows improved chunking.
- [x] Performance smoke checks exist for the highest-risk flows.
- [ ] All verification avoids normal-browser app execution.

## Rollout Notes

Do not try to optimize the whole UI in one patch. Latency work is easy to make worse by moving cost around invisibly. Each slice should:

1. Capture a baseline for the specific flow.
2. Make one architectural change.
3. Verify correctness.
4. Re-run the same profile.
5. Record the result in this file or a small companion note.

If a phase does not improve the measured flow, either revert that phase or document why the change is still valuable for a later phase.

## Open Questions

- Should Xero adopt a small store library, or keep selector stores implemented directly with `useSyncExternalStore`?
- What are the target machines for performance budgets: Apple Silicon only, Intel macOS, Windows, Linux, or all supported Tauri platforms?
- Should runtime stream validation be reduced in production once Rust-side contracts are covered by tests?
- Which view should be treated as the startup-critical first view after project load?
- Should editor persistence use deltas, debounced snapshots, or explicit save snapshots as the primary contract?

## Final Definition Of Done

The optimization effort is done when a production Tauri build shows fewer unnecessary renders and smoother interaction profiles for agent streaming, editor typing, sidebar toggles, and VCS updates; all changed behavior is covered by scoped tests or replay checks; and no temporary debug UI remains.
