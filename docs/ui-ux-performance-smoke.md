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

The build pass reports total JS/CSS chunk size, gzip size, the largest chunks, and CodeMirror chunk sizes. Wall-clock timings are included as smoke diagnostics only; regressions should be judged by structural counts first, with generous budgets when a timing threshold is needed.

For a focused check while iterating on the replay fixture:

```sh
pnpm --dir ./client exec vitest run src/performance/performance-smoke.test.tsx
```
