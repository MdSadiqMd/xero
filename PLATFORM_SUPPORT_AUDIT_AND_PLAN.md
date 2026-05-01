# Platform Support Audit And Plan

Date: 2026-04-30
Implementation status updated: 2026-05-01

Goal: make every feasible Xero desktop feature work on macOS, Windows, and Linux. Platform-only features are allowed when the host really requires them, such as iOS Simulator, macOS dictation/TCC prompts, and macOS window automation.

## Audit Scope

Reviewed:

- Tauri desktop config and capabilities: `client/src-tauri/tauri.conf.json`, `client/src-tauri/capabilities/*.json`
- Rust app-data and registry storage: `client/src-tauri/src/state.rs`, `client/src-tauri/src/global_db`, `client/src-tauri/src/db`
- Process/runtime tools: `client/src-tauri/src/runtime/platform_adapter.rs`, `client/src-tauri/src/runtime/process_tree.rs`, `client/src-tauri/src/runtime/autonomous_tool_runtime`
- Mobile emulator support: `client/src-tauri/src/commands/emulator`
- Solana workbench toolchains and state: `client/src-tauri/src/commands/solana`
- Browser cookie sidecar: `client/src-tauri/src/commands/browser/cookie_import.rs`, `client/src-tauri/crates/cookie-importer`
- Dev scripts: `scripts/dev-preflight.mjs`, `client/scripts/*.mjs`
- Frontend platform gates and user-facing path copy: `client/components/xero`

## Implementation Complete

- Changed Tauri bundle targets from macOS-only `["app"]` to `"all"` so Windows and Linux package targets are not excluded at config level.
- Made autonomous command sanitized `PATH` fallback platform-aware instead of injecting a Unix path on Windows.
- Made Solana fallback binary search paths platform-aware:
  - Windows: `USERPROFILE`-based Solana/Cargo/AVM paths and `APPDATA/npm`.
  - macOS: Homebrew and system binary locations.
  - Linux: standard Unix binary locations.
- Made `scripts/dev-preflight.mjs` use Windows shell spawning for `.cmd`/`.bat` tools and attempt to launch Docker Desktop on Windows.
- Replaced macOS-looking UI placeholders such as `/Users/you/...` and `~/.config/...` with neutral, cross-platform path prompts.
- Replaced the stale desktop matrix with runnable commands in `client/src-tauri/tests/platform-matrix.md`.
- Added Windows system process, listening-port, process-exists, and external signal support in the autonomous process manager, with parser fixtures for PowerShell and legacy command output.
- Removed host CLI archive assumptions from the Tauri build script and Android provisioning by using Rust download/extraction libraries for `.zip` and `.tar.gz` archives.
- Split macOS-only `idb-companion.universal` resources into `tauri.macos.conf.json`; the base config no longer requires the iOS sidecar tree for Windows/Linux packaging.
- Threaded Tauri `app_data_dir()` into `SolanaState` so snapshots, personas, Metaplex worker cache, and program archives share one app-owned Solana state root.
- Added a backend `desktop_platform` Tauri command and taught the shell to prefer it over user-agent detection, with the old user-agent path retained for tests and non-Tauri rendering.
- Expanded Rust and TypeScript path redaction coverage for common Windows app-data and temp locations.
- Replaced Rust `/tmp/...` test fixtures in the audited platform-sensitive paths with `temp_dir()`/`TempDir` derived paths.

## Current Platform Matrix

| Area | macOS | Windows | Linux | Notes |
| --- | --- | --- | --- | --- |
| Tauri app boot | Expected | Expected | Expected | Base config targets all platforms; macOS-only sidecars live in the macOS overlay. |
| App state and global DB | Expected | Expected | Expected | Main registry uses `app.path().app_data_dir()` and avoids repo-local `.xero`. |
| Per-project state DB | Expected | Expected | Expected | Stored under app-data derived project directories. Unix chmod hardening is no-op on Windows by design. |
| Frontend shell chrome | Expected | Expected | Expected | UI has macOS, Windows, and Linux variants, initialized from the backend platform command in desktop mode. |
| iOS Simulator | Expected | Unsupported by design | Unsupported by design | Correctly cfg-gated and hidden outside macOS. |
| Native dictation | Expected | Unsupported by design today | Unsupported by design today | Current command returns typed unsupported diagnostics outside macOS. |
| Android emulator | Expected | Expected with SDK/JDK prerequisites | Expected with SDK/JDK/KVM prerequisites | Provisioning supports all three and uses Rust archive extraction. |
| Browser cookie import | Expected | Expected | Expected | Sidecar builds per host. Safari is macOS-only by cfg. |
| Solana toolchain probe/install | Expected | Expected on x64 | Expected on x64 | Managed Agave/Anchor installers cover macOS plus Windows/Linux x64. Linux arm64 remains unsupported by that upstream artifact set. |
| Autonomous command/process sessions | Expected | Expected | Expected | Owned process spawning is platform-aware. |
| Autonomous system process/port inspection | Expected | Expected | Expected | Windows uses PowerShell first with `tasklist`, `netstat`, and `taskkill` fallbacks. |
| macOS automation tool | Expected | Unsupported by design | Unsupported by design | Correctly returns typed unavailable result outside macOS. |
| Dev preflight | Expected | Expected | Expected | Windows Docker Desktop launch added; Mix and Docker command spawning uses shell on Windows. |

## Native Host Verification Gate

Implementation work is complete in this branch. Release validation still needs the matrix in `client/src-tauri/tests/platform-matrix.md` run on actual desktop hosts:

- macOS arm64 and x64 where available.
- Windows x64.
- Linux x64.

Per-host commands:

- `pnpm --dir client test`
- `cargo test --manifest-path client/src-tauri/Cargo.toml`
- `cargo check --manifest-path client/src-tauri/Cargo.toml`
- `pnpm --dir client exec tauri build --debug`

Keep Cargo commands serialized so the Cargo lock is never contended.

## Acceptance Criteria

- A fresh clone can build the Tauri app on macOS, Windows, and Linux.
- Core project import, file browsing/editing, provider settings, browser UI, notifications config, and agent runtime work on all three desktop OSes.
- Unsupported features return typed, user-visible unsupported results and are hidden or disabled in UI where appropriate.
- No new app state is written to repo-local `.xero/`.
- No hardcoded user home paths appear in production UI or runtime behavior.
- Platform-specific code is guarded with `cfg` or runtime platform checks, and tests cover both supported and unsupported branches.
