# Platform Support Audit And Plan

Date: 2026-04-30

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

## Immediate Fixes Applied

- Changed Tauri bundle targets from macOS-only `["app"]` to `"all"` so Windows and Linux package targets are not excluded at config level.
- Made autonomous command sanitized `PATH` fallback platform-aware instead of injecting a Unix path on Windows.
- Made Solana fallback binary search paths platform-aware:
  - Windows: `USERPROFILE`-based Solana/Cargo/AVM paths and `APPDATA/npm`.
  - macOS: Homebrew and system binary locations.
  - Linux: standard Unix binary locations.
- Made `scripts/dev-preflight.mjs` use Windows shell spawning for `.cmd`/`.bat` tools and attempt to launch Docker Desktop on Windows.
- Replaced macOS-looking UI placeholders such as `/Users/you/...` and `~/.config/...` with neutral, cross-platform path prompts.

## Current Platform Matrix

| Area | macOS | Windows | Linux | Notes |
| --- | --- | --- | --- | --- |
| Tauri app boot | Expected | Expected | Expected | Config now targets all platforms. Needs real build verification on Windows/Linux. |
| App state and global DB | Expected | Expected | Expected | Main registry uses `app.path().app_data_dir()` and avoids repo-local `.xero`. |
| Per-project state DB | Expected | Expected | Expected | Stored under app-data derived project directories. Unix chmod hardening is no-op on Windows by design. |
| Frontend shell chrome | Expected | Expected | Expected | UI has macOS, Windows, and Linux variants. Detection still depends on user agent. |
| iOS Simulator | Expected | Unsupported by design | Unsupported by design | Correctly cfg-gated and hidden outside macOS. |
| Native dictation | Expected | Unsupported by design today | Unsupported by design today | Current command returns typed unsupported diagnostics outside macOS. |
| Android emulator | Expected | Expected with SDK/JDK prerequisites | Expected with SDK/JDK/KVM prerequisites | Provisioning supports all three, but archive extraction still shells out to host tools. |
| Browser cookie import | Expected | Expected | Expected | Sidecar builds per host. Safari is macOS-only by cfg. |
| Solana toolchain probe/install | Expected | Partial | Partial | Managed Agave/Anchor installers cover macOS and Windows x64 plus Linux x64. Linux arm64 is not covered. |
| Autonomous command/process sessions | Expected | Expected | Expected | Owned process spawning is platform-aware. |
| Autonomous system process/port inspection | Expected | Missing | Expected | Windows currently returns unsupported for system process/port inspection. |
| macOS automation tool | Expected | Unsupported by design | Unsupported by design | Correctly returns typed unavailable result outside macOS. |
| Dev preflight | Expected | Improved, needs verification | Expected | Windows Docker Desktop launch added; Mix and Docker command spawning now uses shell on Windows. |

## Remaining Gaps

### P0: Verify Real Builds On All Desktop Targets

The repository has a platform matrix document, but it references stale test names (`runtime_session_bridge`, `runtime_event_stream`) that are not present in the current tree. Replace it with a current, runnable matrix and run it on:

- macOS arm64 and x64 where possible.
- Windows x64.
- Linux x64.

Commands to validate per host:

- `pnpm --dir client test`
- `cargo test --manifest-path client/src-tauri/Cargo.toml`
- `cargo check --manifest-path client/src-tauri/Cargo.toml`
- `pnpm --dir client exec tauri build --debug`

Keep the user instruction in mind: only one Cargo command at a time.

### P0: Add Windows System Process And Port Inspection

`client/src-tauri/src/runtime/autonomous_tool_runtime/process_manager.rs` currently returns unsupported on non-Unix hosts for:

- `list_system_processes()`
- `list_system_ports()`
- external process signaling

Implement Windows support using stable OS tools/APIs:

- Process list: prefer PowerShell `Get-CimInstance Win32_Process` or `tasklist /fo csv /v` with robust parsing.
- Ports: prefer PowerShell `Get-NetTCPConnection` plus process lookup, with `netstat -ano` fallback.
- Signals: support terminate/kill through `taskkill /PID <pid> /T` and `/F` for force.

Add Windows-shaped unit tests for parsers using fixture text. Avoid tests that require killing real system processes.

### P1: Remove Host CLI Archive Dependencies

Two code paths still shell out for archive/download work:

- `client/src-tauri/build.rs` uses `curl` and `tar` for sidecar fetch/extract.
- `client/src-tauri/src/commands/emulator/android/provision.rs` uses `tar`/`unzip` for Android SDK and JRE extraction.

Replace these with Rust libraries or already-present dependencies:

- Download with `reqwest` where runtime code already does this.
- Extract `.zip` with a zip crate.
- Extract `.tar.gz` with `flate2` plus a tar crate.

This removes assumptions about `curl`, `tar`, and `unzip` being present on Windows or minimal Linux images.

### P1: Split Platform-Specific Bundled Resources

`tauri.conf.json` currently bundles `resources/idb-companion.universal/**/*` on every desktop platform even though it is macOS-only. That is functionally safe if the files exist, but it bloats Windows/Linux bundles and makes non-macOS packaging depend on a macOS sidecar tree.

Options:

- Move iOS resources into a macOS-specific config overlay if the Tauri config system supports the needed merge cleanly.
- Keep the resource path but add CI checks that non-macOS builds do not fail when iOS support is absent.
- Document why the resource is intentionally bundled everywhere if Tauri cannot express per-platform resources.

### P1: Normalize App-Data Ownership For Solana Stores

Most core app state uses Tauri app data. Some Solana stores still use `dirs::data_dir()` directly with `xero/...` or `xero-solana-*` subdirectories:

- snapshots
- personas/keypairs
- program archives
- metaplex worker cache

These are still OS app-data paths, but they are not consistently rooted under the Tauri app-data directory. Decide whether Solana state should be under `app.path().app_data_dir()` as well, then thread the app-data root into `SolanaState` during Tauri setup.

Because this is a new app, do not add legacy migration from the old locations unless explicitly requested.

### P1: Make Test Fixtures Platform-Neutral

Several tests and command-shape assertions use hardcoded `/tmp/...` strings. On Windows, those can compile but produce different display strings or unrealistic command args.

Plan:

- Use `tempfile::tempdir()` or `std::env::temp_dir()` for path values.
- For tests that intentionally assert rendered command strings, add platform-specific expected values or assert structured argv pieces instead of whole Unix paths.
- Keep user-facing virtual paths as forward-slash paths where they are an app protocol, not OS paths.

### P2: Improve Platform Detection In The Frontend

`client/components/xero/shell.tsx` derives platform from `navigator.userAgent`. That is probably good enough inside Tauri WebView/WebKit today, but a backend-provided platform command would be more reliable.

Plan:

- Add a small Tauri command returning `macos | windows | linux`.
- Use it to initialize the shell platform when running in desktop mode.
- Keep the existing user-agent fallback for tests and non-Tauri rendering.

### P2: Expand Redaction Heuristics For Windows Paths

Diagnostics already redact `/Users`, `/home`, `/tmp`, `\\Users\\`, and `:\\Users\\`. Add explicit coverage for common Windows app-data paths:

- `C:\ProgramData\...`
- `C:\Windows\Temp\...`
- `%APPDATA%`/`%LOCALAPPDATA%` shaped values after expansion

Add shared fixtures to keep Rust and TypeScript redaction behavior aligned.

## Acceptance Criteria

- A fresh clone can build the Tauri app on macOS, Windows, and Linux.
- Core project import, file browsing/editing, provider settings, browser UI, notifications config, and agent runtime work on all three desktop OSes.
- Unsupported features return typed, user-visible unsupported results and are hidden or disabled in UI where appropriate.
- No new app state is written to repo-local `.xero/`.
- No hardcoded user home paths appear in production UI or runtime behavior.
- Platform-specific code is guarded with `cfg` or runtime platform checks, and tests cover both supported and unsupported branches.
