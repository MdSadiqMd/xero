# Multi-Pane Agent Workspace — Build Plan

## Goal

Let the user spawn up to 6 independent agent sessions side-by-side inside the Agent tab. Each pane is a fully independent agent harness (own session, own composer, own runtime stream, own settings). The grid responds to viewport changes, sidebar state, and minimum pane sizes by reflowing into the best-fit arrangement.

---

## Resolved decisions

| Decision | Value |
| --- | --- |
| Spawn control | Separate icon button right of `+ New Session` (`SplitSquareHorizontal`), disabled at paneCount = 6 |
| Spawn behavior | Always a fresh new session — no prompt cloning |
| Layout persistence | Per project, across reloads (key: `agentWorkspaceLayout`) |
| Composer | One per pane, with independent settings |
| Composer style | Compact density attaches the composer to the bottom of the pane (non-floating) |
| Compact-density threshold | `paneCount ≥ 3`, regardless of viewport width |
| Sidebar coupling | Auto-collapse to peek when `paneCount > 1`; restore prior pinned state when returning to 1 |
| Sidebar layout response | Workspace reflows when sidebar peek/pin changes width |
| Closing | Cannot close all — minimum 1 pane. Closing the 2nd pane returns to full single-pane (today's UX) |
| Sidebar focus highlight | Focused pane's session highlighted; other loaded sessions show a small pane-number chip |
| Min pane size | 320 × 260 (provisional, retuned in Phase 3) |
| Advanced-controls UX | Both: per-pane gear popover **and** ⌘K palette |
| Reflow animation | 200ms ease; 120ms debounce on sidebar peek to avoid jitter |
| Cap | 6 panes max |

---

## Terminology

- **Pane** — one rendered agent harness inside the Agent tab.
- **Workspace** — the multi-pane container. One per project.
- **Pane slot** — `{ id, agentSessionId }` record stored in workspace state.
- **Spawn** — user-facing verb for adding a new pane.
- **Density** — `comfortable` (1–2 panes) | `compact` (3–6 panes).
- **Arrangement** — a grid shape, e.g., `2x3-cols`, `3x1`. Used as a key for persisted splitter ratios.

---

## Layout engine

The layout is constraint-solved on every layout pass.

**Inputs**

- `paneCount` (1–6)
- `availableWidth`, `availableHeight` of the workspace box
- `minPaneWidth = 320`, `minPaneHeight = 260` (provisional)
- `userLayout` — last manually-resized splitter ratios for this `(projectId, paneCount, arrangementKey)`

**Preference table (try in order; first viable wins)**

| paneCount | Arrangements |
| --- | --- |
| 1 | `1×1` |
| 2 | `1×2`, `2×1` |
| 3 | `1×3`, `3×1` |
| 4 | `2×2`, `4×1`, `1×4` |
| 5 | `2×3` (one cell empty), `3×2` (one cell empty) |
| 6 | `2×3`, `3×2` |

**Viability** — every cell ≥ `minPaneWidth × minPaneHeight` after applying `userLayout` ratios. If none fit, fall back to a vertical scrolling stack (graceful, ugly-but-usable).

**Reflow triggers**

- Window resize
- Sidebar peek / pin / collapse
- Project rail toggle
- Workspace pane add / remove

**Animation** — 200ms ease for arrangement changes. 120ms debounce on sidebar peek before re-running the solver.

---

## State model

```ts
interface AgentWorkspaceState {
  paneSlots: Array<{ id: string; agentSessionId: string }>
  focusedPaneId: string
  splitterRatios: Record<string, number[]> // key: arrangementKey
  prePawnSidebarMode: 'pinned' | 'collapsed' | null // snapshot for restore
}
```

Stored per project under `agentWorkspaceLayout`. Hydrated on project switch; debounced write on changes.

**Hydration rules**

- If a saved `agentSessionId` no longer exists (archived/deleted), drop that slot.
- If hydration yields zero panes, create one fresh slot.
- `focusedPaneId` clamps to the first slot if invalid.

---

## Per-pane callback fan-out

Today every callback in `App.tsx` (e.g. `handleStartRuntimeRun`, `handleAgentResolveOperatorAction`) implicitly targets the project's single active session. Multi-pane requires every callback to accept a `paneId` and resolve to that pane's session.

This is the biggest invasive change in the codebase. Done as a pure refactor in **Phase 1** so single-pane behavior is byte-identical before any UI lands.

---

## Phase 1 — Foundation (pure refactor + layout engine)

**Goal:** land the state model, layout solver, persistence, and per-pane callback fan-out without changing the visible UI. The workspace renders exactly one pane through the new pipeline; behavior is indistinguishable from today.

### Tasks

1. **Layout solver** — `client/lib/agent-workspace-layout.ts`
   - Pure function `solveLayout(input) → { arrangement, ratios, fallback?: 'stack' }`.
   - Unit tests covering every paneCount, viewport edge cases, user-ratio fallbacks, and the impossible-viewport stack fallback.
2. **Workspace state** — extend `useXeroDesktopState` (or the appropriate state hook) to include `agentWorkspaceLayout` per project; expose actions: `spawnPane`, `closePane`, `focusPane`, `setSplitterRatios`.
3. **Persistence layer** — load on project switch, debounced write on changes, drop-stale-slots rule, focus clamp.
4. **Callback fan-out refactor** — every `handleAgent*` in `App.tsx` takes a `paneId`. Internally resolves the pane's `agentSessionId`. Single-pane behavior preserved exactly.
5. **Workspace shell** — `client/components/xero/agent-workspace.tsx` that renders the existing `LiveAgentRuntime` for the (only) pane in the workspace state. Pure plumbing; no splitters, no spawn UI.
6. **Tests**
   - Unit: layout solver (all branches).
   - Integration: `App.test.tsx` smoke — single-pane behavior unchanged after refactor.
   - State: persistence + hydration round-trip, stale-session reconciliation.

### Done when

- All existing agent tests pass without modification of their assertions.
- Single-pane UX is visibly identical.
- A second pane can be added programmatically (devtools / test) and persisted, even though no UI exposes spawn yet.

---

## Phase 2 — Multi-pane UI (single pass)

**Goal:** land every visible multi-pane change in one phase: pane grid, splitters, spawn/close, compact density, sidebar coupling, gear popover + ⌘K palette, keyboard shortcuts, animated reflow, focus highlighting.

### 2a. Pane grid + splitters

- `client/components/xero/agent-runtime/pane-grid.tsx` — renders the solver-chosen arrangement, draggable splitters, animated transitions.
- Splitter ratios persisted per arrangement key.
- Reflow on viewport / sidebar / paneCount change with 200ms ease, 120ms peek debounce.
- Vertical scrolling-stack fallback for impossible viewports.
- Use `react-resizable-panels` if not already present; otherwise reuse the repo's splitter primitive.

### 2b. Spawn / close

- `+ Spawn pane` icon button (`SplitSquareHorizontal`) right of `+ New Session`. Disabled at 6 with tooltip.
- Per-pane close `×` in pane header. Hidden when `paneCount === 1`.
- 2 → 1 close: restore `prePawnSidebarMode`.

### 2c. Compact density

- `AgentRuntime` gains `density: 'comfortable' | 'compact'` and `paneId` props.
- `ComposerDock` gains a compact variant:
  - Attached flush to the pane bottom (no floating ring / shadow).
  - Single-line input by default; auto-grows with content.
  - Primary controls inline: model picker, send.
  - Secondary controls (approval, thinking, dictation, context meter, attachments) move into the per-pane gear popover.
- Compact density auto-selected when `paneCount ≥ 3`.

### 2d. Advanced controls — both surfaces

- **Per-pane gear popover** — opens a popover anchored to a gear icon in the compact composer. Houses every secondary control. Each pane keeps its own selections.
- **⌘K palette** — global command palette scoped to the focused pane. Same controls, surfaced via fuzzy commands ("set approval to auto", "switch to gpt-5.4", etc.). Reuse the repo's palette primitive if one exists; otherwise scope a minimal one.

### 2e. Sidebar coupling

- On first spawn, snapshot the current sidebar `mode` (pinned / collapsed) into `prePawnSidebarMode`.
- While `paneCount > 1`: force sidebar to `collapsed` with peek enabled.
- Manual pin while multi-pane is allowed; the workspace shrinks and the layout engine reflows.
- Returning to `paneCount === 1`: restore `prePawnSidebarMode`.
- Focused pane's session row uses the existing "selected" treatment.
- Other panes' sessions show a small chip ("P2", "P3", …) on the right of their row.
- Click a session row:
  - If loaded in another pane → focus that pane.
  - Else → reassign the focused pane to that session.

### 2f. Keyboard

- `⌘1`–`⌘6` — focus pane N.
- `⌘⇧N` — spawn pane (in Agent tab; verify no collision with existing shortcuts).
- `⌘W` — close focused pane (no-op at paneCount = 1).
- `⌥←/→/↑/↓` — cycle focus around the grid.

### 2g. First-run discoverability

- One-time tooltip the first time a user enters compact mode: "Advanced controls moved to the gear icon and ⌘K." Stored per user.

### Done when

- User can spawn up to 6 panes and close down to 1.
- All viewports reflow correctly (window resize, sidebar peek/pin, project rail toggle).
- Each pane has its own composer with independent settings.
- Focused pane's session is highlighted; other panes' sessions show a chip.
- Both gear popover and ⌘K palette expose advanced controls.
- Layout persists across reload per project.
- All Phase 1 tests still pass; new tests cover spawn/close, sidebar coupling, density flip at 3 panes, focus highlighting, layout reflow on min-size violation.

---

## Phase 3 — Hardening

**Goal:** edge cases, concurrency, accessibility, and validation that doesn't fit cleanly into the UI pass.

### Tasks

1. **Confirm-on-close** — modal when closing a pane with a `running` run or unsaved composer text.
2. **Stream concurrency smoke test** — verify 6 simultaneous SSE streams under realistic load. Validate the runtime stream subscription manager isolates per-session state correctly.
3. **Project-switch restore** — switching projects mid-session correctly hydrates the target project's workspace state without leaking streams from the previous project.
4. **Stale-session reconciliation** — archiving or deleting a session that's loaded in another pane: that pane shows the empty-session state, and the slot reassigns to a fresh session on next spawn.
5. **Min-pane-size retune** — measure the real condensed harness, retune `minPaneWidth` / `minPaneHeight` based on usability.
6. **Accessibility**
   - Each pane is its own ARIA `region` with a meaningful name (e.g., "Agent pane 2 — Session 'Inspect build'").
   - Splitter handles follow the ARIA Resize Handle pattern and are keyboard-operable.
   - ⌘K palette announces results.
7. **Performance** — verify React tree doesn't re-render all panes on focus change; memoize per-pane subtrees keyed by `paneId`. Profile with 6 panes streaming.

### Done when

- All edge cases pass manual and automated coverage.
- 6-pane stress smoke test passes.
- A11y checklist is green.
- Profile shows no full-tree re-render on focus change.

---

## Risks

- **Phase 1 callback fan-out** is large and invasive in `App.tsx`. Land it as its own PR before any UI work; review focuses on "single-pane behavior is byte-identical."
- **6 simultaneous SSE streams** — currently assumed safe; validated in Phase 3.
- **Compact composer feature loss** — mitigated by gear popover + ⌘K + first-run tooltip, but a real risk if neither surface is discoverable enough.
- **Layout thrash from sidebar peek** — debounce mitigates; revisit if it still feels jittery.
- **Min pane sizes are a guess** — Phase 3 retunes after seeing real condensed harness.

---

## Out of scope (v1)

- Drag-to-rearrange panes.
- "Link" panes (broadcast prompt to multiple agents).
- Tear a pane out into its own window.
- Mobile / narrow-viewport behavior beyond the scrolling-stack fallback.

---

## Component map

| File | Status | Purpose |
| --- | --- | --- |
| `client/lib/agent-workspace-layout.ts` | new | Pure layout solver |
| `client/components/xero/agent-workspace.tsx` | new | Workspace shell, owns spawn/close/focus, persistence |
| `client/components/xero/agent-runtime/pane-grid.tsx` | new | Renders solver arrangement, splitters, animation |
| `client/components/xero/agent-runtime.tsx` | changed | Adds `density`, `paneId`, `onClose`, `onFocus` props |
| `client/components/xero/agent-runtime/composer-dock.tsx` | changed | Compact variant, gear popover anchor |
| `client/components/xero/agent-sessions-sidebar.tsx` | changed | Pane-loaded chip, click-loaded-session-focuses-pane |
| `client/src/App.tsx` | changed | Replaces single `LiveAgentRuntime` with `<AgentWorkspace />`, callbacks fan out per pane |
| `client/src/features/xero/use-xero-desktop-state.ts` (or equivalent) | changed | `agentWorkspaceLayout` shape + actions |

---

## Open questions still pending

- **Min pane size** — provisional 320×260, retune in Phase 3.
- Any new questions surfaced during build are recorded inline as TODOs and resolved in Phase 3 or as follow-ups.
