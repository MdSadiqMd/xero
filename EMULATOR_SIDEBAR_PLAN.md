# Mobile Emulator Sidebar — Implementation Plan

Production-grade integration of the iOS Simulator and Android Emulator as sidebars, each triggered from its own titlebar button (`Apple` icon and `Android`/`Smartphone` icon). Mirrors the existing `BrowserSidebar` architecture for UX consistency but uses a native-sidecar + frame-streaming pipeline (no WKWebView WebRTC dependency, no external-window reparenting).

---

## 1. Goal & Constraints

### Goal
Let a developer, from within Cadence, pop open an iOS or Android device running a real OS image, interact with it (tap, type, scroll, hardware buttons, rotate), and close it — without leaving the app and without launching `Simulator.app` / the Android Emulator window separately.

### Hard constraints (user-supplied)
1. **Production grade only.** No screenshot-polling MVP. H.264 streaming pipeline from day one.
2. **Bundled sidecars.** Ship `scrcpy-server.jar` and `idb_companion` inside the app; do not require users to `brew install` / manually download anything beyond their platform SDK.
3. **One active device at a time** across the whole app. Switching device or sidebar shuts the previous one down.
4. **Two titlebar buttons.** Separate iOS and Android triggers — not a single "emulator" button with a tab.
5. **Platform matrix:**
   - macOS host: both buttons visible.
   - Windows / Linux host: iOS button hidden (Simulator requires Xcode / macOS).
6. **User-provided SDKs:** we bundle sidecar binaries but cannot bundle Xcode or the Android SDK. Missing-SDK state is a first-class UX surface.
7. **LLM-agent driveable.** Every capability the sidebar UI exposes to a human — tap, type, scroll, hardware buttons, install/launch apps, inspect UI — must be callable programmatically via the same Tauri command surface, so that an agent (future MCP server, `gsd-*` automation, test harness) can drive the emulator without visual observation. The UI is one client of the automation layer; the agent is another.

### Inherited constraints (from `AGENTS.md` and codebase)
- ShadCN for all UI where possible.
- Never add debug/test UI — only user-facing.
- Tauri app only (no browser execution path needed).
- Mirror the `BrowserSidebar` (`client/components/cadence/browser-sidebar.tsx`) lifecycle conventions: resize handle, width persistence, mutex-open pattern in `App.tsx:100–114`, titlebar wiring in `shell.tsx`.

---

## 2. Architecture Overview

```
┌───────────────────────── Cadence Tauri App ─────────────────────────┐
│                                                                     │
│  Titlebar                                                           │
│   └── [Apple btn]  [Android btn]  [Globe btn]  [Gamepad btn]  ...   │
│                                                                     │
│  Main webview                                                       │
│   ├── ProjectRail                                                   │
│   ├── active view (phases / agent / execution)                      │
│   ├── BrowserSidebar                                                │
│   ├── GamesSidebar                                                  │
│   ├── IosEmulatorSidebar  ◄──┐                                      │
│   └── AndroidEmulatorSidebar ├── <img src="emulator://frame?t=seq"/>│
│                              │   (JPEG blit; rAF-driven)            │
│   pointer/key events ────────┘                                      │
│         │                                                           │
│         ▼  invoke("emulator_input", …)                              │
│                                                                     │
│  Rust backend  (src-tauri/src/commands/emulator/)                   │
│   ├── EmulatorState       ← single-active-device registry           │
│   ├── FrameBus            ← latest frame + seq counter              │
│   ├── URI scheme handler  ← emulator:// → JPEG bytes                │
│   ├── android/            ← emulator process + scrcpy TCP client    │
│   └── ios/                ← simctl + tonic gRPC to idb_companion    │
│         │                                                           │
│         ▼                                                           │
└─────────┼───────────────────────────────────────────────────────────┘
          │
          ▼  child processes (bundled sidecars)
  ┌───────────────┐              ┌────────────────────┐
  │  emulator     │              │  idb_companion     │
  │  (Android)    │              │  (macOS-only)      │
  │               │              │                    │
  │   scrcpy JAR  │              │   CoreSimulator    │
  │   (pushed to  │              │   + SimulatorKit   │
  │    device via │              │   + IndigoHID      │
  │    ADB)       │              │                    │
  └───────┬───────┘              └─────────┬──────────┘
          │ H.264/TCP                      │ H.264/gRPC
          ▼                                ▼
    Android OS image                  iOS Simulator
```

### Data flow per frame (identical on both platforms after decode)
1. Device-side encoder produces an H.264 NAL unit.
2. Sidecar (scrcpy/idb) delivers it to Rust (TCP for scrcpy, gRPC stream for idb).
3. Rust decodes with `openh264` → RGBA → `image` crate re-encodes as JPEG Q80.
4. `FrameBus` atomically stores `Arc<Vec<u8>>` + increments `seq: AtomicU64`.
5. Rust emits `emulator:frame` Tauri event with `{ seq }` (payload is just the counter).
6. Frontend listener swaps `<img src>` to `emulator://frame?t={seq}`.
7. Webview fetches the URI → Tauri async URI scheme handler returns the bytes from `FrameBus`.
8. Webview decodes JPEG and paints (hardware-accelerated on all three webview engines).

### Input flow
1. Pointer event fires on the `<img>` viewport.
2. Frontend computes normalized (x, y) against known device resolution.
3. `invoke("emulator_input", { kind, x, y, phase, … })`.
4. Rust → scrcpy control socket (Android) or `HIDEvent` gRPC (iOS).
5. Sidecar injects as a synthetic HID event on the device.

### Why this architecture
- **No WebRTC dependency in the webview.** Sidesteps known WKWebView `RTCPeerConnection` issues on macOS and avoids needing custom WebKitGTK compile flags on Linux.
- **No native window reparenting.** Avoids private-API fragility, DPI pain, and Spaces/Mission-Control breakage.
- **Same render pipeline for both OSes.** One frame bus, one URI scheme, one DOM element — only the sidecar driver differs.
- **Hardware-accelerated JPEG decode in every webview.** We keep the expensive H.264 decode in Rust (where we control perf) and hand the webview something it already knows how to draw fast.

---

## 3. Bundled Binaries

| Binary | Source | License | Approx. size | Bundled for |
|---|---|---|---|---|
| `scrcpy-server-v2.x.jar` | [Genymobile/scrcpy](https://github.com/Genymobile/scrcpy) | Apache 2.0 | ~80 KB | all platforms |
| `idb_companion` | [facebook/idb](https://github.com/facebook/idb) | MIT | ~50 MB (macOS x86_64 + arm64 universal) | macOS only |

Bundling mechanism: Tauri v2 `resources` in `tauri.conf.json`, resolved at runtime via `app.path().resource_dir()`. Each platform gets the correct triple via `externalBin` convention if we decide to treat `idb_companion` as a sidecar (it's a long-running daemon, so `externalBin` fits better than `resources`).

User-provided prerequisites (NOT bundled):
- Android: Android SDK with `emulator` + `adb` on `PATH`, at least one AVD created.
- iOS: Xcode 15+ with Command Line Tools; at least one iOS runtime installed.

Detection: startup probe using `which`/`where` for `emulator`, `adb`, `xcrun`. Results stored in `EmulatorState` and surfaced to frontend.

---

## 4. Rust Module Layout

New tree under `client/src-tauri/src/commands/`:

```
emulator/
├── mod.rs                     // EmulatorState, Tauri commands, URI scheme registration
├── frame_bus.rs               // atomic frame buffer + seq, event emission
├── process.rs                 // ChildGuard wrapper: kill-on-drop, graceful shutdown, stderr capture
├── codec.rs                   // openh264 decoder + image-crate JPEG re-encode
├── automation/                // Phase 5 — agent-driveable surface
│   ├── mod.rs                 // UiTree, Selector, TapTarget types + cross-platform dispatcher
│   ├── selector.rs            // selector matching against UiTree nodes
│   ├── android_ui.rs          // uiautomator dump → UiTree normalization
│   ├── ios_ui.rs              // idb AccessibilityInfo → UiTree normalization
│   ├── apps.rs                // install/launch/terminate/list across platforms
│   └── logs.rs                // logcat / idb log streaming into emulator:log events
├── android/
│   ├── mod.rs                 // platform entry: boot, shutdown, input dispatch
│   ├── sdk.rs                 // SDK discovery (ANDROID_HOME, which(), version parsing)
│   ├── avd.rs                 // list AVDs via `emulator -list-avds`
│   ├── emulator_process.rs    // spawn `emulator @<name> -no-window …` + boot-wait
│   ├── adb.rs                 // thin `adb` shell wrapper (push, reverse, shell)
│   ├── scrcpy.rs              // push JAR, open sockets, parse protocol
│   └── input.rs               // scrcpy control message encoding
└── ios/
    ├── mod.rs                 // platform entry (cfg-gated to macOS)
    ├── xcrun.rs               // simctl list / boot / shutdown wrapper / push / location
    ├── idb_companion.rs       // spawn sidecar, gRPC port handshake
    ├── idb_client.rs          // tonic client, generated from idb.proto
    └── input.rs               // HIDEvent builder
```

### `EmulatorState`

```rust
pub struct EmulatorState {
    active: Mutex<Option<ActiveDevice>>,  // single-device invariant
    frame_bus: Arc<FrameBus>,
    sdk: OnceLock<SdkProbe>,               // cached detection result
}

enum ActiveDevice {
    Android(AndroidSession),   // owns emulator child + scrcpy tasks
    Ios(IosSession),           // owns simctl boot + idb_companion + tonic stream
}
```

Registered in `lib.rs:18` alongside `BrowserState`.

### Tauri command surface

**Lifecycle & streaming** (Phase 2–4):
```rust
emulator_sdk_status() -> SdkStatus
    // { android: { present, sdkRoot, avdCount }, ios: { present, xcodeVersion, runtimeCount } }

emulator_list_devices(platform: "android" | "ios") -> Vec<DeviceDescriptor>
    // AVD name / iOS Simulator UDID + display name + resolution + DPR + type (phone/tablet)

emulator_start(platform, device_id) -> DeviceHandle
    // idempotent: if a different device is active, shut it down first.
    // returns viewport size + DPR + frame URI

emulator_stop() -> ()
    // shut down whichever device is currently active

emulator_input(event: InputEvent) -> ()
    // kind: touch_down | touch_move | touch_up | key | text | scroll | hw_button
    // coords are normalized 0..1 against device resolution

emulator_rotate(orientation: "portrait" | "landscape") -> ()

emulator_subscribe_ready() -> ()
    // called by frontend after mounting to receive the current `emulator:status` snapshot
```

**Automation surface** (Phase 5 — agent-callable):
```rust
emulator_screenshot() -> { png_base64, width, height, device_pixel_ratio }
    // pulls latest from FrameBus, no extra device round-trip

emulator_ui_dump() -> UiTree
    // normalized accessibility tree:
    //   { id, role, label, value, enabled, focused, bounds: {x,y,w,h}, children: [...] }
    // Android: `adb shell uiautomator dump` → XML → normalize
    // iOS: idb AccessibilityInfoRequest (native JSON tree)

emulator_find(selector: Selector) -> Vec<UiNode>
    // Selector = { label?, role?, id?, text?, contains?, visible? } — fields ANDed

emulator_tap(target: TapTarget) -> ()
    // TapTarget = { kind: "point", x, y } | { kind: "element", selector }
    // Element taps dump UI, locate, tap at bounds center atomically

emulator_swipe(from: Point, to: Point, duration_ms) -> ()

emulator_type(text: String, into?: Selector) -> ()
    // With selector: tap it first, then inject text via scrcpy text / idb HID
    // Without: send at current focus

emulator_press_key(key: HardwareKey) -> ()
    // home | back | recents | lock | vol_up | vol_down | enter | escape | tab | backspace

emulator_list_apps() -> Vec<AppDescriptor>
    // { bundle_id, display_name, version, installed_at }

emulator_install_app(source_path: Path) -> AppDescriptor
    // Android: adb install; iOS: idb InstallRequest

emulator_uninstall_app(bundle_id) -> ()

emulator_launch_app(bundle_id, args?: Vec<String>, env?: Map) -> ()

emulator_terminate_app(bundle_id) -> ()

emulator_set_location(lat: f64, lon: f64) -> ()

emulator_push_notification(bundle_id, payload: Json) -> ()   // iOS only (simctl push); returns unsupported on Android

emulator_logs_subscribe(filter?: LogFilter) -> SubscriptionToken
    // Android: adb logcat -v threadtime; iOS: idb LogRequest
    // Streams via `emulator:log` event
```

Events emitted:
- `emulator:status` — `{ phase: "booting" | "connecting" | "streaming" | "stopped" | "error", message? }`
- `emulator:frame` — `{ seq: u64 }` (payload stays tiny; bytes come via URI scheme)
- `emulator:log` — `{ level, tag, message, timestamp_ms }`
- `emulator:sdk_status_changed`

**Design rule:** every automation command takes purely serializable JSON inputs and returns purely serializable JSON outputs — no Tauri-only types leak through. This is what lets the same commands be wrapped by an MCP server in the future without refactoring.

### URI scheme

Registered once in `configure_builder_with_state`:

```rust
.register_asynchronous_uri_scheme_protocol("emulator", move |_app, request, responder| {
    // parse request path: /frame, /status, etc.
    // for /frame: read FrameBus.latest() -> respond with JPEG bytes + ETag header (seq)
})
```

Not HTTP — the webview treats `emulator://…` as a local resource, same way the browser-sidebar cookie-importer already uses Tauri commands.

---

## 5. Frontend Component Layout

New files under `client/components/cadence/`:

```
emulator-sidebar.tsx           // shared shell: resize handle, width persistence, viewport wrapper
ios-emulator-sidebar.tsx       // wraps emulator-sidebar, passes platform="ios"
android-emulator-sidebar.tsx   // wraps emulator-sidebar, passes platform="android"
emulator-device-picker.tsx     // ShadCN select populated from emulator_list_devices
emulator-hardware-strip.tsx    // home / back / recents / lock / volume buttons
emulator-frame.tsx             // <img> viewport + pointer event translation
emulator-missing-sdk.tsx       // first-run panel when SDK/Xcode not detected
```

State hook:

```
client/src/features/emulator/use-emulator-session.ts
```

Mirrors `use-cadence-desktop-state.ts` pattern — owns the `listen` subscriptions for `emulator:status` and `emulator:frame`, exposes `{ status, currentDevice, start, stop, input, rotate }`.

### Integration points in existing code

**`client/components/cadence/shell.tsx`:**
- Add `iosOpen`, `androidOpen`, `onToggleIos`, `onToggleAndroid` to `CadenceShellProps`.
- New `IosBtn` and `AndroidBtn` constants next to `BrowserBtn` (`shell.tsx:167-182`).
- `IosBtn` returns `null` when `platform !== "macos"` (using the existing `detectPlatform()` at `shell.tsx:16-22`).
- Wire both buttons into the macOS and Windows/Linux titlebar layouts (`shell.tsx:300-373`).

**`client/src/App.tsx`:**
- Add `iosOpen`, `androidOpen` to the existing `gamesOpen`/`browserOpen` mutex group (`App.tsx:100-114`).
- Opening any one closes all others *and* calls `emulator_stop()` when closing an emulator sidebar.
- Render `<IosEmulatorSidebar open={iosOpen} />` and `<AndroidEmulatorSidebar open={androidOpen} />` alongside `BrowserSidebar`/`GamesSidebar` (`App.tsx:384-385`).

**`client/src-tauri/capabilities/default.json`:**
- Register the new `emulator` URI scheme.
- No new core permissions needed.

**`client/src-tauri/tauri.conf.json`:**
- Add `scrcpy-server-v2.x.jar` to `resources`.
- Add `idb_companion` to `externalBin` (macOS target only; use `-aarch64-apple-darwin` / `-x86_64-apple-darwin` suffixes).

---

## 6. Phased Execution

Each phase is a shippable commit boundary. No phase depends on work that isn't in an earlier phase.

---

### Phase 1 — Foundation (titlebar, shells, mutex state)

**Goal:** both buttons open empty resizable sidebars that correctly coexist with browser/games; no backend yet.

**Deliverables:**
1. `CadenceShellProps` extended; `IosBtn` + `AndroidBtn` rendered in both macOS and Windows/Linux titlebar branches.
2. iOS button hidden on non-macOS via platform detection.
3. Mutex-open logic in `App.tsx`: opening iOS closes android/browser/games; opening android closes ios/browser/games.
4. `EmulatorSidebar` shared shell with resize handle (reuse MIN_WIDTH/DEFAULT_RATIO/RESIZE_HANDLE_INSET constants from `browser-sidebar.tsx:22-31`).
5. `IosEmulatorSidebar` and `AndroidEmulatorSidebar` thin wrappers with "Not implemented yet" empty state.
6. Tests: `emulator-sidebar.test.tsx` for open/close/resize; update `shell.test.tsx` to cover new buttons and platform gating.

**Does NOT include:** any Rust code, any device enumeration, any streaming.

**Acceptance:** run the app on macOS and Windows; verify iOS button only appears on macOS; click either button, sidebar opens, drag handle resizes, clicking browser closes emulator and vice versa.

---

### Phase 2 — Rust backbone + URI scheme + frame bus

**Goal:** end-to-end frame pipeline proven with a synthetic generator, before any emulator work. This de-risks the rendering path.

**Deliverables:**
1. `commands/emulator/mod.rs` + `EmulatorState` registered in `lib.rs:18`.
2. `frame_bus.rs`: `FrameBus { latest: ArcSwap<Frame>, seq: AtomicU64 }`.
3. `register_asynchronous_uri_scheme_protocol("emulator", …)` serving `/frame` from the bus.
4. `emulator_start`/`emulator_stop`/`emulator_input` command stubs that return `TODO` errors, but `emulator_sdk_status` is real.
5. Dev-only synthetic driver (behind `cfg(feature = "emulator-synthetic")`) that pushes a solid-color frame with an incrementing counter — used for frontend integration testing.
6. `codec.rs` with `openh264` decoder scaffolded + JPEG encode via `image` crate; unit test round-trips a known H.264 NAL.
7. New deps in `Cargo.toml`: `openh264`, `tonic`, `prost`, `tokio-stream`, `arc-swap`, `image` (already present — verify version).
8. Frontend `use-emulator-session.ts` hook: subscribes to `emulator:frame`, swaps `<img src>`; pointer event handlers call `emulator_input` (which currently no-ops).
9. Wire the synthetic driver into both sidebars so you can see frames rendering.

**Acceptance:** with synthetic feature flag on, opening either sidebar shows a color-cycling image updating at 30 FPS, pointer events are observably dispatched (log in Rust), and the URI scheme returns correct bytes for the requested `seq`.

---

### Phase 3 — Android production pipeline

**Goal:** real Android emulator running, streamed via scrcpy, with working input.

**Deliverables:**
1. `android/sdk.rs`: detect `ANDROID_HOME` / `ANDROID_SDK_ROOT`, locate `emulator` and `adb` binaries, version-probe both.
2. `android/avd.rs`: enumerate AVDs via `emulator -list-avds`; hydrate with `avdmanager list avd` for resolution + DPR metadata.
3. `android/emulator_process.rs`:
   - Spawn `emulator @<name> -no-window -no-audio -no-snapshot-save -no-boot-anim -wipe-data=false`.
   - `ChildGuard` kills the process on drop / app close.
   - Boot-wait: poll `adb shell getprop sys.boot_completed` until `1` or timeout (60s).
4. `android/adb.rs`: typed wrappers for `push`, `reverse`, `shell`, `wait-for-device`.
5. `android/scrcpy.rs`:
   - Extract bundled `scrcpy-server.jar` to a temp dir, `adb push` to `/data/local/tmp/scrcpy-server.jar`.
   - `adb reverse localabstract:scrcpy tcp:<port>` so the device can connect back.
   - Spawn the server: `adb shell CLASSPATH=/data/local/tmp/scrcpy-server.jar app_process / com.genymobile.scrcpy.Server 2.7 video=true audio=false control=true tunnel_forward=false log_level=warn`.
   - Accept two inbound TCP connections (video + control).
   - Parse video socket: 12-byte metadata header (codec id, initial width, initial height), then loop { 12-byte frame header (flags + PTS u62 + size u32), N bytes of H.264 }.
   - Feed NALs to `codec::decode`; publish to `FrameBus`.
6. `android/input.rs`: build scrcpy control messages (touch, scroll, key, text, back) and write to the control socket. Use normalized coords scaled to current device resolution.
7. `emulator_start("android", avd_id)` wires it all together. `emulator_stop()` tears down scrcpy → emulator process → frame bus.
8. Frontend: `AndroidEmulatorSidebar` loads real AVD list into `EmulatorDevicePicker`, calls `emulator_start`, renders frames, translates pointer/key events to `emulator_input`.
9. Bundle `scrcpy-server-v2.7.jar` (or whatever the then-current version is) via `tauri.conf.json` resources.

**Acceptance:** on a dev machine with Android SDK + a Pixel 8 AVD, open Android sidebar → device picker shows Pixel 8 → click "Start" → sidebar shows booting state → within ~45s shows live Android home screen → tapping apps opens them → typing on on-screen keyboard works → back gesture works.

---

### Phase 4 — iOS production pipeline

**Goal:** real iOS Simulator running, streamed via idb_companion, with working input. macOS only.

**Deliverables:**
1. `ios/xcrun.rs`:
   - `xcrun simctl list devices available --json` → parsed into `DeviceDescriptor` list.
   - `xcrun simctl boot <UDID>` + poll state via `xcrun simctl list devices` until `Booted`.
   - `xcrun simctl shutdown <UDID>` on teardown.
2. `ios/idb_companion.rs`:
   - Resolve bundled `idb_companion` path via `app.path().resolve("idb_companion", BaseDirectory::Resource)`.
   - Spawn with `--udid <UDID> --grpc-port <port> --log-level warn`; capture stderr for diagnostics.
   - `ChildGuard` tears down on drop.
3. Fetch `idb.proto` from [`facebook/idb/proto/idb.proto`](https://github.com/facebook/idb/blob/main/proto/idb.proto); vendor into `src-tauri/proto/`; build with `tonic-build` in `build.rs`.
4. `ios/idb_client.rs`:
   - Connect tonic client to `http://127.0.0.1:<port>`.
   - Call `video_stream(VideoStreamRequest { format: H264, fps: 30, … })` → receive H.264 frames → feed to `codec::decode` → publish to `FrameBus`.
5. `ios/input.rs`:
   - Build `HIDEvent` messages (touch down/up/move, text, key).
   - Send via `hid` streaming gRPC call.
6. `emulator_start("ios", udid)` wires it together; `emulator_stop()` tears down idb_companion → `simctl shutdown` → frame bus.
7. Add `idb_companion` to `tauri.conf.json` as `externalBin` with `-aarch64-apple-darwin` and `-x86_64-apple-darwin` variants. Verify pre-built binary signatures; re-sign under our Developer ID if notarization requires it.
8. Frontend `IosEmulatorSidebar` parallels Android but disabled on non-macOS (shell already hides the button; sidebar component double-checks via `isTauri()` + platform probe).
9. Auto-disable the Android pipeline from starting while iOS is active (enforced server-side in `EmulatorState::start`).

**Acceptance:** on macOS with Xcode 15+ and iOS 17 runtime, open iOS sidebar → device picker shows "iPhone 15 Pro" → click "Start" → sidebar shows booting, then live iOS home screen → tapping apps works → on-screen keyboard typing works → swiping between home-screen pages works.

---

### Phase 5 — Agent automation surface

**Goal:** every capability the sidebar UI exposes to a human is also callable programmatically via the Tauri command layer, so an LLM agent (or test harness) can drive the emulator without visual observation. This is the layer that lets future work wrap the emulator as an MCP server or expose it to in-app agent runs.

**Deliverables:**
1. `automation/mod.rs` — cross-platform types:
   ```rust
   pub struct UiTree { pub root: UiNode }
   pub struct UiNode {
       pub id: Option<String>,
       pub role: String,          // "button", "textfield", "image", "list", …
       pub label: Option<String>, // accessibility label / content-desc
       pub value: Option<String>, // current text / state
       pub enabled: bool,
       pub focused: bool,
       pub bounds: Bounds,        // device pixels
       pub children: Vec<UiNode>,
   }
   pub struct Selector {
       pub label: Option<String>,
       pub role:  Option<String>,
       pub id:    Option<String>,
       pub text:  Option<String>,
       pub contains: Option<String>,
       pub visible: Option<bool>,
   }
   ```
2. `automation/android_ui.rs`:
   - Call `adb shell uiautomator dump /dev/tty` (dumps XML to stdout, no file round-trip).
   - Parse with `quick-xml`.
   - Normalize AOSP node attrs (`class`, `content-desc`, `text`, `bounds`, `clickable`, `enabled`) into `UiNode`.
   - Cache result with a ~200 ms TTL, invalidated by any input command.
3. `automation/ios_ui.rs`:
   - Call idb gRPC `accessibility_info(AccessibilityInfoRequest { point: None, nested: true })`.
   - Map idb's native JSON tree to our `UiTree` format (idb already returns bounds, role, label).
4. `automation/selector.rs`:
   - Walk `UiTree`, match nodes where all specified selector fields match.
   - Return in depth-first order for deterministic "first match".
5. `automation/apps.rs`:
   - `install(path)`:
     - Android: `adb install -r -d <path>`; parse exit.
     - iOS: idb `install(InstallRequest { payload: tar | ipa | app })`.
   - `launch(bundle_id, args, env)`:
     - Android: `adb shell am start -n <package>/<activity>` (look up main activity from manifest via `aapt` or `adb shell pm dump`).
     - iOS: idb `launch(LaunchRequest { bundle_id, arguments, environment })`.
   - `terminate` / `uninstall` / `list_apps` — symmetric.
6. `automation/logs.rs`:
   - Android: spawn `adb logcat -v threadtime`, parse each line, emit `emulator:log` events.
   - iOS: idb `log(LogRequest)` streaming; map severity.
   - Ring buffer of last 10k entries in Rust for on-demand fetch via `emulator_logs_get_recent()`.
7. Tauri commands (all JSON in / JSON out, per the design rule):
   - `emulator_screenshot` pulls from `FrameBus` — zero extra work, but expose explicitly so agents don't need to know about the URI scheme.
   - `emulator_ui_dump`, `emulator_find`, `emulator_tap`, `emulator_swipe`, `emulator_type`, `emulator_press_key`.
   - `emulator_list_apps`, `emulator_install_app`, `emulator_uninstall_app`, `emulator_launch_app`, `emulator_terminate_app`.
   - `emulator_set_location`, `emulator_push_notification`.
   - `emulator_logs_subscribe`, `emulator_logs_unsubscribe`, `emulator_logs_get_recent`.
8. Atomicity helpers: `emulator_tap({ kind: "element", selector })` does dump + find + tap in one Rust call so agents don't race with their own prior input. If the selector matches zero or more than one node, return a typed error (not an arbitrary pick).
9. Hardware-button mapping table committed once per platform: both scrcpy control messages and idb HID events are opaque integers — expose a stable `HardwareKey` enum so agents never need to learn platform specifics.
10. Test harness: a Rust integration test that (on a real AVD, gated behind `cargo test --features emulator-live`):
    - boots Pixel 8
    - `emulator_launch_app("com.android.settings")`
    - `emulator_ui_dump` — assert tree non-empty
    - `emulator_tap({ element: { label: "Display" } })`
    - dump again — assert "Brightness" node now present
    - `emulator_press_key("back")` — assert back on main settings
    - symmetric test on iOS with `com.apple.Preferences` → "General" → back.

**Does NOT include:** the MCP server itself. That lives in a follow-up milestone as a thin wrapper over these commands — the work here is to keep that wrapper trivial.

**Acceptance:** from the JS console inside the app (or a Rust test), drive the active device through a 5-step interaction purely via semantic commands (no pixel coordinates, no screenshots consulted). The same script runs on both Android and iOS with the only diff being the bundle ID.

---

### Phase 6 — Device polish

**Goal:** make it feel like a device, not a video feed.

**Deliverables:**
1. Device frame SVG bezels per preset (iPhone 15 Pro, iPhone SE, iPad, Pixel 8, Pixel Tablet, Galaxy S24). Rendered as CSS `mask-image` around the `<img>`.
2. Orientation toggle: calls `emulator_rotate`; Android uses `adb shell settings put system user_rotation <n>`; iOS uses `HIDEvent` rotation or `simctl ui <UDID> appearance landscape`.
3. Hardware button strip: per-platform set.
   - Android: back, home, recents, lock, vol up/down.
   - iOS: home (double-tap → app switcher), lock, vol up/down, ringer/silent.
4. Zoom / fit controls — if the device's native resolution exceeds the sidebar width, CSS-scale the `<img>` while keeping pointer coordinate translation accurate.
5. Keyboard input capture: when the viewport has focus, key events route to `emulator_input` (text for printable, key for modifiers/arrows/escape/backspace); also Cmd+V pastes into the device via scrcpy's clipboard sync or `xcrun simctl pbsync`.
6. Width persistence per-platform in `localStorage` (separate keys for iOS and Android since devices have different native widths).

**Acceptance:** the sidebar looks like a phone, not a floating video; rotating works; home/back/recents feel instant; typing a URL into a device browser works via physical keyboard.

---

### Phase 7 — Robustness & first-run UX

**Goal:** cover the failure paths and make missing-SDK humane.

**Deliverables:**
1. First-run missing-SDK panel (`emulator-missing-sdk.tsx`):
   - Android: "Install Android Studio, create an AVD" with a link to the installer and a "Re-detect" button (mirrors cookie-importer banner UX at `browser-sidebar.tsx:676-757`).
   - iOS: "Install Xcode and at least one iOS runtime" with a Mac App Store link.
2. Boot-time loading state with progress phases (`booting kernel` → `waiting for boot-completed` → `starting screen stream`).
3. Crash recovery: if the sidecar exits unexpectedly, emit `emulator:status` with `error` phase + stderr tail; frontend shows a retry button.
4. Graceful shutdown on app close: Tauri `on_window_event(CloseRequested)` calls `emulator_stop()` before window destruction to avoid zombie emulator/idb processes.
5. Zombie process sweep on app startup: if a previous crash left `emulator-*` / `idb_companion` running, offer to kill them.
6. Telemetry-friendly logging: structured events for boot time, stream start time, frame rate, sidecar exit codes — wired into the existing notification/runtime logging path.
7. Integration tests: mock sidecar that speaks the real scrcpy / idb wire protocol but is driven by a test harness, to exercise the Rust backbone without needing a real emulator in CI.

**Acceptance:** uninstall Xcode command-line tools; open iOS sidebar; see a clear setup panel, not an error toast. Kill `emulator` from outside the app; the sidebar shows an error state and offers retry. Quit Cadence mid-stream; no zombie processes linger.

---

### Phase 8 — Optional performance upgrade (defer if Phase 4 is fast enough)

**Goal:** remove the Rust-side H.264→JPEG round-trip; let the webview decode H.264 directly.

**Approach:** fragmented-MP4 muxing + MSE (`MediaSource` + `SourceBuffer`). Rust wraps the H.264 NALs into fMP4 boxes (`moof`/`mdat`) and pushes fragments via a different URI scheme (`emulator://video.mp4`) that the frontend consumes through a `<video>` element.

**Why it's optional:** JPEG-at-30FPS through the existing pipeline will be visually indistinguishable from H.264 for the target use case (UI exploration, not gameplay). This phase is only justified if profiling shows CPU budget is a problem — e.g., an older laptop decoding two 1080p streams.

**Acceptance:** same visual output as Phase 4, CPU usage during streaming drops by ≥50%.

---

## 7. Dependency Changes

### `client/src-tauri/Cargo.toml`
```toml
# Phase 2
arc-swap = "1"
openh264 = "0.6"            # H.264 decode; small C lib, patent-safe freeware
tonic = "0.12"              # gRPC client for idb_companion
prost = "0.13"
tokio-stream = "0.1"

# Phase 5 — automation surface
quick-xml = "0.36"          # uiautomator dump parsing (Android)
# (JSON parsing for idb's accessibility tree reuses existing serde_json)

# build-dependencies
tonic-build = "0.12"
```

### `client/package.json`
No new frontend deps required. We use native `<img>` + pointer events + Tauri's built-in event API. No `android-emulator-webrtc`, no WebRTC libraries, no video player library.

### Bundled sidecar binaries (tracked with Git LFS or fetched by `build.rs`)
- `scrcpy-server-v2.7.jar` — single file, all platforms.
- `idb_companion-universal-apple-darwin` — macOS universal (lipo of x86_64 + arm64).

Build script (`build.rs`) downloads pinned versions on first build and verifies SHA-256 to keep the repo lean.

---

## 8. Tauri Configuration Changes

**`client/src-tauri/tauri.conf.json`:**
```jsonc
{
  "bundle": {
    "resources": [
      "resources/scrcpy-server-v2.7.jar"
    ],
    "externalBin": [
      "binaries/idb_companion"
    ]
  }
}
```

**`client/src-tauri/capabilities/default.json`:** no changes (the custom URI scheme is registered at the Rust level, not gated by a capability permission).

**`client/src-tauri/capabilities/`** new file `emulator.json` only if we find we need to scope-gate shell access for `xcrun`/`adb`/`emulator`; likely not, since we spawn processes through Rust's `std::process::Command` rather than the `shell` plugin.

---

## 9. Testing Strategy

| Layer | Technique |
|---|---|
| Rust — codec | Unit test with a captured H.264 sample → assert decoded frame dimensions + JPEG round-trip. |
| Rust — scrcpy protocol | Fake TCP server that replays a recorded byte stream; assert frames emitted. |
| Rust — idb client | tonic test server implementing `video_stream` + `hid` stubs; assert frames flow and input is received. |
| Rust — lifecycle | `emulator_start` → `emulator_start` on same platform with different device → asserts first is shut down. Single-active-device invariant. |
| Frontend — sidebar | Existing `browser-sidebar.test.tsx` pattern: render with mocked `isTauri`, fire events, assert state. |
| Frontend — hook | `use-emulator-session` under React Testing Library; mock `listen`/`invoke`. |
| Integration | Behind an `emulator-live` cargo feature, a test that boots a real AVD headlessly in CI — only run on self-hosted runner with Android SDK. iOS integration tests require macOS runners. |

We do NOT add UI-level "debug overlays" per `AGENTS.md`. Diagnostic info (FPS, frame size, latency) lives in the structured logs only.

---

## 10. Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| `idb_companion` breaks on new Xcode release | iOS streaming dies | Pin to known-good idb version; add Xcode version check at startup with a compat warning; keep Phase 7 (custom Swift helper using CoreSimulator directly) as a fallback option. |
| AVD cold boot takes 60+ seconds | Feels broken | Honest progress UI in Phase 6; expose a "keep alive on sidebar close" toggle in settings (Phase 6+). |
| H.264 decode CPU cost on old hardware | Fans spin, battery drains | Phase 7 (fMP4+MSE) removes the bottleneck entirely by pushing decode to the webview's hardware path. |
| Pre-built `idb_companion` fails notarization | Cannot ship macOS build | Build `idb_companion` from source in our CI and sign with our Developer ID. Upstream is MIT — allowed. |
| User has emulator.exe / adb.exe conflicting versions on PATH | Boot fails mysteriously | `android/sdk.rs` version-probes and surfaces mismatch in the missing-SDK panel. |
| WKWebView refusing to load custom URI scheme bytes larger than N | Large frames fail | Verify during Phase 2 with synthetic frames up to 8 MB; fall back to chunked multipart response if needed (async URI handler supports streaming). |
| Resource leak if Cadence is force-killed | Zombie emulator process locks AVD | Phase 6 zombie-sweep on startup; optionally write a sentinel file with PIDs for external cleanup. |
| Licensing of bundled scrcpy JAR | Apache 2.0 requires NOTICE | Add Apache 2.0 NOTICE file to Cadence's about/legal section. |
| `uiautomator dump` latency (0.5–2 s on Android) | Slow agent loops | 200 ms dump cache in `automation/android_ui.rs` invalidated by any input; long-term consider hooking `AccessibilityService` for push-based tree updates. |
| Apps with non-native UI (games, Flutter/RN custom-drawn screens) have empty accessibility trees | Semantic selectors find nothing | `emulator_find` returns empty; agent falls back to `emulator_screenshot` + vision. Document this as expected behavior. |
| Android `am start` needs the main activity, not just the package | Launches fail for some apps | `automation/apps.rs` parses `adb shell cmd package resolve-activity --brief <pkg>` to discover launcher activity; caches per-package. |
| idb's accessibility tree uses iOS-native role names ("XCUIElementTypeButton") | Selectors need platform awareness | Normalize to common roles (`button`, `textfield`, etc.) in `ios_ui.rs`; keep raw role in a `platform_role` field for escape hatches. |
| Agent holds a stale UI tree after an animation | Tap lands on wrong element | All tap-by-element commands re-dump immediately before resolving bounds; never reuse a dump across input boundaries. |

---

## 11. Out of Scope

- Real devices over USB/Wi-Fi (only emulators/simulators). scrcpy supports real Android devices — adding it later is a small extension but intentionally deferred.
- Multi-device mosaic. User confirmed single active device.
- Multi-emulator tabs à la the browser. Single active device.
- Android auto / Wear OS / Apple Watch / Apple TV form factors.
- Recording / screen-capture-to-file from within the sidebar (idb supports it; add in a later milestone if demand).
- Drag-and-drop `.app` / `.apk` install via the UI. (Programmatic install via `emulator_install_app` is Phase 5; the UI affordance is deferred.)
- Network conditions simulation (latency, bandwidth throttle). Defer.
- **MCP server wrapper.** Phase 5 makes the command surface MCP-ready but does not ship an MCP server itself — that's a follow-up milestone, likely a small `cadence-emulator-mcp` crate that forwards stdio MCP tool calls to the existing Tauri commands. Keeping it out of scope here prevents scope creep and leaves the design open for alternative agent transports (direct Claude Code tool use, Cadence's own agent runtime, etc.).

---

## 12. Execution Notes

- Each phase is a single PR; no phase should exceed ~1500 LOC of net new code.
- Integration tests require real SDKs; don't gate CI on them. Phase-gate them behind a cargo feature and a self-hosted runner.
- Keep all platform-specific code behind `cfg(target_os = "macos")` or runtime probes; the binary must still build on all three hosts even if iOS is unreachable at runtime.
- Mirror the `browser/` module's file layout religiously — reviewers already understand that shape.
- When in doubt about a UX detail, copy from `BrowserSidebar` rather than inventing. Consistency > cleverness for this feature.
