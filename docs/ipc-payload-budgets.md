# IPC Payload Budgets

These budgets keep normal Tauri command responses and events small enough for the renderer to validate, derive, and render without creating long UI-thread tasks.

Budgets are warning limits, not data-loss permissions. Commands that can legitimately exceed a budget must return explicit truncation or pagination metadata, and UI surfaces that need the missing data must request a narrower or paged follow-up.

| Boundary | Budget | Current contract |
| --- | ---: | --- |
| Runtime stream item | 32 KiB | Stream items use cheap production guards; dev/test records item size and drops items above the 96 KiB hard cap before React state sees them. |
| Repository status | 384 KiB | Status responses record payload diagnostics when they exceed the budget. Status event delivery is coalesced in the desktop state listener. |
| Repository diff | 96 KiB | Git patches are capped in Rust and return `truncated` plus payload diagnostics when capped. |
| Project tree/listing | 512 KiB | The recursive listing has a 5,000-node cap and returns `truncated`, `omittedEntryCount`, and payload diagnostics. |
| Project search results | 1 MiB | Search uses match caps and preview caps; truncated searches return `truncated` plus payload diagnostics. |
| Browser tab/url/load events | 8 KiB | Browser sidebar coalesces repeated tab/url/load events by tab before applying React state. |
| Browser console event | 16 KiB | Adapter instrumentation records event size for smoke metrics. |
| Emulator frame/status event | 1 KiB | Frame events should carry metadata only; image bytes stay behind the frame URI path. |
| Provider/model registry | 512 KiB | Adapter instrumentation records catalog response size. |
| Settings registries | 256 KiB | Adapter instrumentation records MCP, skill, notification, dictation, browser, and soul settings payload sizes. |

Instrumentation lives in `client/src/lib/ipc-payload-budget.ts` and is enabled in development, tests, or when `globalThis.__XERO_IPC_PAYLOAD_METRICS__` is set. The browser-free performance smoke replay reports the largest representative payloads under `ipcPayloadBudgets`.
