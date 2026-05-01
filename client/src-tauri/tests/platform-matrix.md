# Desktop Platform Verification Matrix

This matrix is the runnable release gate for Xero desktop platform support.
It intentionally references test targets that exist in the current tree.

## Per-Host Gate

Run these commands on every desktop host. Keep Cargo commands serialized so
the Cargo lock is never contended.

```bash
pnpm --dir client test
cargo test --manifest-path client/src-tauri/Cargo.toml
cargo check --manifest-path client/src-tauri/Cargo.toml
pnpm --dir client exec tauri build --debug
```

## Focused Smoke Set

For fast platform triage before the full gate, run:

```bash
cargo test --manifest-path client/src-tauri/Cargo.toml --test platform_adapters
cargo test --manifest-path client/src-tauri/Cargo.toml --test autonomous_tool_runtime
cargo test --manifest-path client/src-tauri/Cargo.toml --test provider_diagnostics_contract
cargo test --manifest-path client/src-tauri/Cargo.toml --test solana_workbench
pnpm --dir client test components/xero/shell.test.tsx src/lib/xero-model/diagnostics.test.ts src/lib/xero-model/session-context.test.ts
```

## Host Targets

| Host | Required result | Notes |
| --- | --- | --- |
| macOS arm64 | Per-host gate passes | Verifies macOS-only dictation/iOS code paths plus shared desktop support. |
| macOS x64 | Per-host gate passes where hardware/CI is available | Same commands; no test-name substitutions. |
| Windows x64 | Per-host gate passes | Verifies Windows shell selection, process/port parsers, `taskkill` signaling, and `.cmd`/`.bat` behavior. |
| Linux x64 | Per-host gate passes | Verifies `/proc` process/port inspection and Linux packaging. |

## Platform-Only Features

- iOS Simulator and native dictation are macOS-only and must return typed
  unsupported results or remain hidden on Windows and Linux.
- macOS automation remains macOS-only and must return typed unavailable
  results outside macOS.
- Android emulator support is expected on macOS, Windows, and Linux when
  SDK/JDK/hypervisor prerequisites are present.
