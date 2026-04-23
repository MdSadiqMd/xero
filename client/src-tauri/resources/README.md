# Bundled emulator sidecars

This directory holds binaries we embed into the Cadence desktop bundle at
build time. They are resolved at runtime via `app.path().resolve(..., Resource)`.

## scrcpy-server-v2.7.jar

**Required for the Android emulator sidebar to stream frames.**

Download the `scrcpy-server` JAR that matches the scrcpy client version
referenced in `src/commands/emulator/android/scrcpy.rs::SCRCPY_VERSION`
(currently **v2.7**):

```
https://github.com/Genymobile/scrcpy/releases/download/v2.7/scrcpy-server-v2.7
```

Rename the download to `scrcpy-server-v2.7.jar` and drop it here. The Tauri
build will embed it under the app resource directory; the Android pipeline
locates it with `scrcpy::bundled_jar_path` at runtime.

The jar is Apache-2.0 licensed (Genymobile) — a matching `NOTICE` file ships
in Cadence's About dialog as required.

## idb_companion (macOS-only)

**Required for the iOS simulator sidebar to stream frames.**

Either install via Homebrew (`brew install facebook/fb/idb-companion`) — the
SDK probe will pick it up from `/opt/homebrew/bin` or `/usr/local/bin` — or
drop a universal binary here and reference it from `tauri.conf.json` as an
`externalBin`. The iOS pipeline resolves `idb_companion` in this order:

1. Tauri resource directory (this folder).
2. `which idb_companion` on `PATH`.
3. `/opt/homebrew/bin/idb_companion`, `/usr/local/bin/idb_companion`.

The binary is MIT-licensed (Meta / facebook/idb).

---

These binaries are intentionally **not** checked in. Grab them as part of
your build environment — our `build.rs` warns (but does not fail) when they
are missing so local dev builds keep working while CI can gate on presence.
