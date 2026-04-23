//! Emulator sidebar backend — iOS Simulator and Android Emulator bring-up.
//!
//! Phase 2 scaffolds the frame pipeline (FrameBus + `emulator://` URI scheme)
//! and a synthetic frame driver that exercises it end-to-end without a real
//! device. Phases 3 and 4 replace the synthetic driver with scrcpy and
//! `idb_companion` sidecars respectively.

pub mod codec;
pub mod events;
pub mod frame_bus;
pub mod sdk;
#[cfg(feature = "emulator-synthetic")]
pub mod synthetic;
pub mod uri_scheme;

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime, State};

use crate::commands::{CommandError, CommandResult};

pub use events::{
    FramePayload, StatusPayload, StatusPhase, EMULATOR_FRAME_EVENT,
    EMULATOR_SDK_STATUS_CHANGED_EVENT, EMULATOR_STATUS_EVENT,
};
pub use frame_bus::{Frame, FrameBus};
pub use sdk::{probe_sdks, AndroidSdkStatus, IosSdkStatus, SdkStatus};
pub use uri_scheme::{handle as handle_uri_scheme, URI_SCHEME};

/// Process-wide emulator state. Holds the FrameBus (shared with the URI
/// scheme handler) and the single active device session, if any.
pub struct EmulatorState {
    frame_bus: Arc<FrameBus>,
    active: Mutex<Option<ActiveDevice>>,
}

impl Default for EmulatorState {
    fn default() -> Self {
        Self {
            frame_bus: Arc::new(FrameBus::new()),
            active: Mutex::new(None),
        }
    }
}

impl EmulatorState {
    pub fn frame_bus(&self) -> Arc<FrameBus> {
        Arc::clone(&self.frame_bus)
    }
}

/// Platform tag shared between the frontend and backend.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EmulatorPlatform {
    Ios,
    Android,
}

impl EmulatorPlatform {
    pub fn as_str(self) -> &'static str {
        match self {
            EmulatorPlatform::Ios => "ios",
            EmulatorPlatform::Android => "android",
        }
    }
}

/// The backing session for the currently-running device. Phase 2 only
/// implements the synthetic variant; Phases 3 and 4 add the real variants.
enum ActiveDevice {
    #[cfg(feature = "emulator-synthetic")]
    Synthetic {
        platform: EmulatorPlatform,
        device_id: String,
        // Field is only held for its Drop impl (joins the producer thread);
        // direct access isn't needed beyond that.
        #[allow(dead_code)]
        session: synthetic::SyntheticSession,
    },
    // Android { session: android::AndroidSession }  // phase 3
    // Ios     { session: ios::IosSession }          // phase 4
    #[cfg(not(feature = "emulator-synthetic"))]
    #[allow(dead_code)]
    Placeholder,
}

// ---------- Request/response shapes ----------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmulatorStartRequest {
    pub platform: EmulatorPlatform,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulatorStartResponse {
    pub platform: EmulatorPlatform,
    pub device_id: String,
    pub width: u32,
    pub height: u32,
    pub device_pixel_ratio: f32,
    pub frame_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmulatorInputRequest {
    pub kind: InputKind,
    /// Normalized 0..1 against the device resolution.
    #[serde(default)]
    pub x: Option<f32>,
    #[serde(default)]
    pub y: Option<f32>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub button: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputKind {
    TouchDown,
    TouchMove,
    TouchUp,
    Scroll,
    Key,
    Text,
    HwButton,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmulatorRotateRequest {
    pub orientation: Orientation,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmulatorListDevicesRequest {
    pub platform: EmulatorPlatform,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDescriptor {
    pub id: String,
    pub display_name: String,
    pub kind: DeviceKind,
    pub width: u32,
    pub height: u32,
    pub device_pixel_ratio: f32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceKind {
    Phone,
    Tablet,
}

// ---------- Tauri commands --------------------------------------------------

#[tauri::command]
pub fn emulator_sdk_status() -> CommandResult<SdkStatus> {
    Ok(probe_sdks())
}

#[tauri::command]
pub fn emulator_list_devices(
    request: EmulatorListDevicesRequest,
) -> CommandResult<Vec<DeviceDescriptor>> {
    // Phase 2: no real enumeration yet. Under the synthetic feature we surface
    // a single stub device so the frontend can drive the pipeline.
    #[cfg(feature = "emulator-synthetic")]
    {
        let (id, name, kind) = match request.platform {
            EmulatorPlatform::Android => ("synthetic-pixel", "Synthetic Pixel", DeviceKind::Phone),
            EmulatorPlatform::Ios => ("synthetic-iphone", "Synthetic iPhone", DeviceKind::Phone),
        };
        Ok(vec![DeviceDescriptor {
            id: id.to_string(),
            display_name: name.to_string(),
            kind,
            width: synthetic::synthetic_width(),
            height: synthetic::synthetic_height(),
            device_pixel_ratio: 2.0,
        }])
    }

    #[cfg(not(feature = "emulator-synthetic"))]
    {
        let _ = request;
        Ok(Vec::new())
    }
}

#[tauri::command]
pub fn emulator_start<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, EmulatorState>,
    request: EmulatorStartRequest,
) -> CommandResult<EmulatorStartResponse> {
    if request.device_id.trim().is_empty() {
        return Err(CommandError::invalid_request("deviceId"));
    }

    // Single-active-device invariant — shut down any previous session first
    // so starting a new device is idempotent from the caller's perspective.
    stop_active(&app, &state)?;

    #[cfg(feature = "emulator-synthetic")]
    {
        let _ = app.emit(
            EMULATOR_STATUS_EVENT,
            StatusPayload::new(StatusPhase::Booting)
                .with_platform(request.platform.as_str())
                .with_device(&request.device_id)
                .with_message("starting synthetic frame source"),
        );

        let session = synthetic::SyntheticSession::spawn(
            app.clone(),
            state.frame_bus(),
            request.platform.as_str().to_string(),
            request.device_id.clone(),
        );
        let mut active = state.active.lock().expect("emulator active mutex poisoned");
        *active = Some(ActiveDevice::Synthetic {
            platform: request.platform,
            device_id: request.device_id.clone(),
            session,
        });

        Ok(EmulatorStartResponse {
            platform: request.platform,
            device_id: request.device_id,
            width: synthetic::synthetic_width(),
            height: synthetic::synthetic_height(),
            device_pixel_ratio: 2.0,
            frame_url: "emulator://localhost/frame".to_string(),
        })
    }

    #[cfg(not(feature = "emulator-synthetic"))]
    {
        Err(not_implemented(
            "emulator_start",
            "real device support arrives in Phase 3 (Android) and Phase 4 (iOS). Rebuild with --features emulator-synthetic to exercise the frame pipeline.",
        ))
    }
}

#[tauri::command]
pub fn emulator_stop<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, EmulatorState>,
) -> CommandResult<()> {
    stop_active(&app, &state)
}

#[tauri::command]
pub fn emulator_input(
    state: State<'_, EmulatorState>,
    request: EmulatorInputRequest,
) -> CommandResult<()> {
    let active = state.active.lock().expect("emulator active mutex poisoned");
    if active.is_none() {
        return Err(CommandError::user_fixable(
            "emulator_not_running",
            "No emulator device is currently running.",
        ));
    }

    #[cfg(feature = "emulator-synthetic")]
    {
        // Synthetic driver has no concept of input — acknowledge without
        // doing anything so frontend wiring can be exercised.
        let _ = request;
        Ok(())
    }

    #[cfg(not(feature = "emulator-synthetic"))]
    {
        let _ = request;
        Err(not_implemented(
            "emulator_input",
            "input dispatch lands with the Android/iOS pipelines in Phases 3 and 4.",
        ))
    }
}

#[tauri::command]
pub fn emulator_rotate(
    state: State<'_, EmulatorState>,
    request: EmulatorRotateRequest,
) -> CommandResult<()> {
    let active = state.active.lock().expect("emulator active mutex poisoned");
    if active.is_none() {
        return Err(CommandError::user_fixable(
            "emulator_not_running",
            "No emulator device is currently running.",
        ));
    }

    let _ = request;
    Err(not_implemented(
        "emulator_rotate",
        "rotation lands with the Android/iOS pipelines in Phases 3 and 4.",
    ))
}

#[tauri::command]
pub fn emulator_subscribe_ready<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, EmulatorState>,
) -> CommandResult<StatusPayload> {
    let active = state.active.lock().expect("emulator active mutex poisoned");
    let payload = match active.as_ref() {
        #[cfg(feature = "emulator-synthetic")]
        Some(ActiveDevice::Synthetic {
            platform,
            device_id,
            ..
        }) => StatusPayload::new(StatusPhase::Streaming)
            .with_platform(platform.as_str())
            .with_device(device_id.clone()),
        #[cfg(not(feature = "emulator-synthetic"))]
        Some(ActiveDevice::Placeholder) => StatusPayload::new(StatusPhase::Stopped),
        None => StatusPayload::new(StatusPhase::Stopped),
    };
    // Re-emit so any new listener sees the current phase.
    let _ = app.emit(EMULATOR_STATUS_EVENT, payload.clone());
    Ok(payload)
}

// ---------- Helpers ---------------------------------------------------------

fn stop_active<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, EmulatorState>,
) -> CommandResult<()> {
    let mut active = state.active.lock().expect("emulator active mutex poisoned");
    let taken = active.take();
    drop(active);

    let (platform, device_id) = match &taken {
        #[cfg(feature = "emulator-synthetic")]
        Some(ActiveDevice::Synthetic {
            platform,
            device_id,
            ..
        }) => (Some(platform.as_str().to_string()), Some(device_id.clone())),
        _ => (None, None),
    };

    if taken.is_some() {
        let _ = app.emit(
            EMULATOR_STATUS_EVENT,
            StatusPayload {
                phase: StatusPhase::Stopping,
                platform: platform.clone(),
                device_id: device_id.clone(),
                message: None,
            },
        );
    }

    // Dropping the ActiveDevice joins the synthetic thread.
    drop(taken);

    state.frame_bus().clear();

    let _ = app.emit(
        EMULATOR_STATUS_EVENT,
        StatusPayload {
            phase: StatusPhase::Stopped,
            platform,
            device_id,
            message: None,
        },
    );
    Ok(())
}

fn not_implemented(command: &'static str, detail: &'static str) -> CommandError {
    CommandError::system_fault(
        format!("{command}_not_implemented"),
        format!("{command} is not implemented yet. {detail}"),
    )
}
