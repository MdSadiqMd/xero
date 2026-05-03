# UI/UX Performance Smoke

Run the browser-free latency gate from the repo root:

```sh
pnpm run performance:smoke
```

The script runs `client/src/performance/performance-smoke.test.tsx` under Vitest/JSDOM and then runs a production Vite build. It does not open the Tauri app in a browser.

The replay fixture reports structural guardrails for the highest-risk flows:

- high-volume runtime stream buffering and flush count
- repository status churn render commits for shell subscribers
- editor typing-style cursor report coalescing
- sidebar resize scheduler frame and IPC counts
- VCS diff cache reuse and revision invalidation
- cache entry counts and approximate retained bytes for markdown segments,
  diff parsing/patch caches, Shiki tokens, runtime streams, project trees,
  and provider-model catalogs

The build pass reports total JS/CSS chunk size, gzip size, the largest chunks, and CodeMirror chunk sizes. Wall-clock timings are included as smoke diagnostics only; regressions should be judged by structural counts first, with generous budgets when a timing threshold is needed.

For a focused check while iterating on the replay fixture:

```sh
pnpm --dir ./client exec vitest run src/performance/performance-smoke.test.tsx
```

## Heap Snapshot Procedure

For Phase 17 memory checks, use a release Tauri build and capture heap snapshots
from the platform webview inspector, not a normal browser tab:

1. Start from a fresh app launch and capture an idle heap snapshot.
2. Replay the target flow: runtime stream burst, large VCS diff, large project
   tree expansion, or settings open with provider/model catalogs loaded.
3. Let the UI settle for 5 seconds, close or switch away from the surface, then
   capture a second heap snapshot.
4. Record retained sizes for markdown/diff tokens, project tree nodes, runtime
   stream views, provider catalogs, browser logs/events, and emulator frame
   objects alongside the smoke report.

The smoke report's byte counts are approximate structural counters. Treat them
as a repeatable early warning; use the webview heap snapshots to confirm actual
release retained size before tightening budgets.
