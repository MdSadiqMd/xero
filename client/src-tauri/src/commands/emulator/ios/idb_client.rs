//! idb gRPC client.
//!
//! idb_companion exposes a gRPC API defined in
//! https://github.com/facebook/idb/blob/main/proto/idb.proto — the services
//! we care about are:
//!   - `VideoStream`: bidirectional H.264 frame source for live streaming
//!   - `HID`:         bidirectional HID-event sink for touch/key input
//!   - `AccessibilityInfo`:  one-shot UI-tree snapshot (used by Phase 5 automation)
//!   - `Install`, `Launch`, `Terminate`, `Log`: lifecycle and log-stream RPCs
//!
//! The full idb.proto is ~1500 lines and needs `tonic-build` + `prost` code
//! generation. To keep the tree buildable without that toolchain we ship
//! this module as a thin, explicitly-not-implemented shim with a clear
//! contract. When we vendor `idb.proto` and wire up `build.rs`, the stubs
//! below become one-line `tonic::Request<...>`/stream adapters around the
//! generated client types — no session-level code has to change.

#![cfg(target_os = "macos")]

use std::time::Duration;

use crate::commands::CommandError;

/// Stable, frontend-exposed handle to a running idb_companion. Even though
/// the transport layer isn't wired yet, we commit to the method surface here
/// so callers (`IosSession`, automation commands) don't have to rewrite
/// once the gRPC pipeline lands.
pub struct IdbClient {
    grpc_port: u16,
}

impl IdbClient {
    pub fn new(grpc_port: u16) -> Self {
        Self { grpc_port }
    }

    pub fn grpc_port(&self) -> u16 {
        self.grpc_port
    }

    /// Would open a bidirectional `VideoStream` RPC and push raw H.264 NAL
    /// units into a callback. Returns `unimplemented` until the proto is
    /// vendored.
    pub fn start_video_stream(
        &self,
        _fps: u32,
        _on_nal: Box<dyn FnMut(&[u8]) + Send>,
    ) -> Result<VideoStreamHandle, CommandError> {
        Err(grpc_unimplemented(
            "VideoStream",
            "iOS live streaming requires vendoring idb.proto and wiring tonic-build",
        ))
    }

    /// Would send an `HIDEvent` over the bidirectional HID RPC.
    pub fn send_hid(&self, _event: super::input::HidEvent) -> Result<(), CommandError> {
        Err(grpc_unimplemented(
            "HID.inject",
            "iOS input dispatch requires vendoring idb.proto",
        ))
    }

    /// Would pull the current accessibility tree. Consumed by
    /// `emulator_ui_dump` in Phase 5.
    pub fn accessibility_tree(&self) -> Result<serde_json::Value, CommandError> {
        Err(grpc_unimplemented(
            "AccessibilityInfo",
            "iOS UI tree requires vendoring idb.proto",
        ))
    }

    /// Would connect to the log stream. Consumed by `emulator_logs_subscribe`
    /// in Phase 5.
    pub fn start_log_stream(
        &self,
        _on_line: Box<dyn FnMut(&str) + Send>,
    ) -> Result<LogStreamHandle, CommandError> {
        Err(grpc_unimplemented(
            "Log",
            "iOS log streaming requires vendoring idb.proto",
        ))
    }
}

/// Opaque handle returned by `start_video_stream`; dropping it cancels the
/// underlying gRPC stream. Currently an empty placeholder.
pub struct VideoStreamHandle;

impl VideoStreamHandle {
    pub fn shutdown(&self, _grace: Duration) {}
}

/// Same shape as `VideoStreamHandle` but for the log stream.
pub struct LogStreamHandle;

impl LogStreamHandle {
    pub fn shutdown(&self, _grace: Duration) {}
}

fn grpc_unimplemented(method: &str, detail: &str) -> CommandError {
    CommandError::system_fault(
        "ios_idb_proto_missing",
        format!(
            "idb gRPC `{method}` is not yet wired up in this Cadence build. {detail}."
        ),
    )
}
