//! Emulator sidebar backend — iOS Simulator and Android Emulator bring-up.
//!
//! Phase 2 scaffolded the frame pipeline (FrameBus + `emulator://` URI
//! scheme) and a synthetic frame driver. Phase 3 wires in the real Android
//! pipeline (emulator process + scrcpy). Phase 4 adds the iOS pipeline.

pub mod android;
pub mod automation;
pub mod codec;
pub mod decoder;
pub mod events;
pub mod frame_bus;
pub mod ios;
pub mod process;
pub mod sdk;
#[cfg(feature = "emulator-synthetic")]
pub mod synthetic;
pub mod uri_scheme;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use base64::Engine;
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

use automation::{
    AppDescriptor, BundleIdRequest, HardwareKeyRequest, InstallAppRequest, LaunchAppRequest,
    LocationRequest, LogSubscribeRequest, PushNotificationRequest, ScreenshotResponse, Selector,
    SubscriptionToken, SwipeRequest, TapTarget, TypeRequest, UiTree,
};

/// Process-wide emulator state. Holds the FrameBus (shared with the URI
/// scheme handler) and the single active device session, if any.
pub struct EmulatorState {
    frame_bus: Arc<FrameBus>,
    active: Mutex<Option<ActiveDevice>>,
    log_collector: automation::logs::LogCollector,
    log_stream: Mutex<Option<LogStreamHandle>>,
}

enum LogStreamHandle {
    Android(automation::logs::AndroidLogStream),
}

impl Default for EmulatorState {
    fn default() -> Self {
        Self {
            frame_bus: Arc::new(FrameBus::new()),
            active: Mutex::new(None),
            log_collector: automation::logs::LogCollector::new(),
            log_stream: Mutex::new(None),
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

/// The backing session for the currently-running device.
enum ActiveDevice {
    Android {
        device_id: String,
        session: android::AndroidSession,
    },
    #[cfg(target_os = "macos")]
    Ios {
        device_id: String,
        session: ios::IosSession,
    },
    #[cfg(feature = "emulator-synthetic")]
    Synthetic {
        platform: EmulatorPlatform,
        device_id: String,
        // Dropping this joins the producer thread.
        #[allow(dead_code)]
        session: synthetic::SyntheticSession,
    },
}

impl ActiveDevice {
    fn platform(&self) -> EmulatorPlatform {
        match self {
            ActiveDevice::Android { .. } => EmulatorPlatform::Android,
            #[cfg(target_os = "macos")]
            ActiveDevice::Ios { .. } => EmulatorPlatform::Ios,
            #[cfg(feature = "emulator-synthetic")]
            ActiveDevice::Synthetic { platform, .. } => *platform,
        }
    }

    fn device_id(&self) -> &str {
        match self {
            ActiveDevice::Android { device_id, .. } => device_id,
            #[cfg(target_os = "macos")]
            ActiveDevice::Ios { device_id, .. } => device_id,
            #[cfg(feature = "emulator-synthetic")]
            ActiveDevice::Synthetic { device_id, .. } => device_id,
        }
    }
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
    #[serde(default)]
    pub dx: Option<f32>,
    #[serde(default)]
    pub dy: Option<f32>,
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
    match request.platform {
        EmulatorPlatform::Android => {
            #[allow(unused_mut)]
            let mut out: Vec<DeviceDescriptor> = android::list_devices()
                .into_iter()
                .map(|avd| DeviceDescriptor {
                    id: avd.name,
                    display_name: avd.display_name,
                    kind: match avd.kind {
                        android::avd::AvdKind::Phone => DeviceKind::Phone,
                        android::avd::AvdKind::Tablet => DeviceKind::Tablet,
                    },
                    width: avd.width.unwrap_or(0),
                    height: avd.height.unwrap_or(0),
                    device_pixel_ratio: avd
                        .density
                        .map(|d| d as f32 / 160.0)
                        .unwrap_or(2.0),
                })
                .collect();

            #[cfg(feature = "emulator-synthetic")]
            if out.is_empty() {
                out.push(DeviceDescriptor {
                    id: "synthetic-pixel".to_string(),
                    display_name: "Synthetic Pixel".to_string(),
                    kind: DeviceKind::Phone,
                    width: synthetic::synthetic_width(),
                    height: synthetic::synthetic_height(),
                    device_pixel_ratio: 2.0,
                });
            }

            Ok(out)
        }
        EmulatorPlatform::Ios => {
            #[cfg(target_os = "macos")]
            {
                #[allow(unused_mut)]
                let mut out: Vec<DeviceDescriptor> = ios::list_devices()
                    .into_iter()
                    .map(|sim| DeviceDescriptor {
                        id: sim.udid,
                        display_name: sim.display_name,
                        kind: if sim.is_tablet {
                            DeviceKind::Tablet
                        } else {
                            DeviceKind::Phone
                        },
                        width: sim.width.unwrap_or(0),
                        height: sim.height.unwrap_or(0),
                        device_pixel_ratio: sim.scale.unwrap_or(3.0),
                    })
                    .collect();

                #[cfg(feature = "emulator-synthetic")]
                if out.is_empty() {
                    out.push(DeviceDescriptor {
                        id: "synthetic-iphone".to_string(),
                        display_name: "Synthetic iPhone".to_string(),
                        kind: DeviceKind::Phone,
                        width: synthetic::synthetic_width(),
                        height: synthetic::synthetic_height(),
                        device_pixel_ratio: 3.0,
                    });
                }

                Ok(out)
            }
            #[cfg(not(target_os = "macos"))]
            {
                Ok(Vec::new())
            }
        }
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

    // Single-active-device invariant — shut down any previous session first.
    stop_active(&app, &state)?;

    #[cfg(feature = "emulator-synthetic")]
    if request.device_id.starts_with("synthetic-") {
        return start_synthetic(&app, &state, request);
    }

    match request.platform {
        EmulatorPlatform::Android => start_android(&app, &state, request),
        EmulatorPlatform::Ios => start_ios(&app, &state, request),
    }
}

fn start_android<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, EmulatorState>,
    request: EmulatorStartRequest,
) -> CommandResult<EmulatorStartResponse> {
    let scrcpy_jar = android::scrcpy::bundled_jar_path(app).map_err(|err| {
        CommandError::user_fixable(
            "scrcpy_jar_missing",
            format!(
                "scrcpy-server.jar is not bundled with this Cadence build: {err}. Drop the jar \
                 into client/src-tauri/resources/ and rebuild."
            ),
        )
    })?;

    let session = android::spawn(android::SpawnArgs {
        app: app.clone(),
        frame_bus: state.frame_bus(),
        device_id: request.device_id.clone(),
        scrcpy_jar,
    })?;

    let width = session.width();
    let height = session.height();

    let mut active = state.active.lock().expect("emulator active mutex poisoned");
    *active = Some(ActiveDevice::Android {
        device_id: request.device_id.clone(),
        session,
    });

    Ok(EmulatorStartResponse {
        platform: request.platform,
        device_id: request.device_id,
        width,
        height,
        device_pixel_ratio: 2.0,
        frame_url: "emulator://localhost/frame".to_string(),
    })
}

#[cfg(target_os = "macos")]
fn start_ios<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, EmulatorState>,
    request: EmulatorStartRequest,
) -> CommandResult<EmulatorStartResponse> {
    let session = ios::spawn(ios::SpawnArgs {
        app: app.clone(),
        frame_bus: state.frame_bus(),
        device_id: request.device_id.clone(),
    })?;

    let width = session.width();
    let height = session.height();

    let mut active = state.active.lock().expect("emulator active mutex poisoned");
    *active = Some(ActiveDevice::Ios {
        device_id: request.device_id.clone(),
        session,
    });

    Ok(EmulatorStartResponse {
        platform: request.platform,
        device_id: request.device_id,
        width,
        height,
        device_pixel_ratio: 3.0,
        frame_url: "emulator://localhost/frame".to_string(),
    })
}

#[cfg(not(target_os = "macos"))]
fn start_ios<R: Runtime>(
    _app: &AppHandle<R>,
    _state: &State<'_, EmulatorState>,
    _request: EmulatorStartRequest,
) -> CommandResult<EmulatorStartResponse> {
    Err(CommandError::user_fixable(
        "ios_unsupported",
        "iOS Simulator is only available on macOS.",
    ))
}

#[cfg(feature = "emulator-synthetic")]
fn start_synthetic<R: Runtime>(
    app: &AppHandle<R>,
    state: &State<'_, EmulatorState>,
    request: EmulatorStartRequest,
) -> CommandResult<EmulatorStartResponse> {
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
    let active = active.as_ref().ok_or_else(|| {
        CommandError::user_fixable(
            "emulator_not_running",
            "No emulator device is currently running.",
        )
    })?;

    match active {
        ActiveDevice::Android { session, .. } => dispatch_android_input(session, &request),
        #[cfg(target_os = "macos")]
        ActiveDevice::Ios { session, .. } => dispatch_ios_input(session, &request),
        #[cfg(feature = "emulator-synthetic")]
        ActiveDevice::Synthetic { .. } => Ok(()),
    }
}

fn dispatch_android_input(
    session: &android::AndroidSession,
    request: &EmulatorInputRequest,
) -> CommandResult<()> {
    use android::input::MotionAction;

    match request.kind {
        InputKind::TouchDown => {
            let x = request.x.unwrap_or(0.0);
            let y = request.y.unwrap_or(0.0);
            session.send_touch(MotionAction::Down, x, y)
        }
        InputKind::TouchMove => {
            let x = request.x.unwrap_or(0.0);
            let y = request.y.unwrap_or(0.0);
            session.send_touch(MotionAction::Move, x, y)
        }
        InputKind::TouchUp => {
            let x = request.x.unwrap_or(0.0);
            let y = request.y.unwrap_or(0.0);
            session.send_touch(MotionAction::Up, x, y)
        }
        InputKind::Scroll => {
            let x = request.x.unwrap_or(0.5);
            let y = request.y.unwrap_or(0.5);
            let dx = (request.dx.unwrap_or(0.0) * 32.0) as i16;
            let dy = (request.dy.unwrap_or(0.0) * 32.0) as i16;
            session.send_scroll(x, y, dx, dy)
        }
        InputKind::Key | InputKind::HwButton => {
            let name = request
                .button
                .as_deref()
                .or(request.key.as_deref())
                .unwrap_or("");
            let keycode = map_hardware_key(name).ok_or_else(|| {
                CommandError::user_fixable(
                    "emulator_unknown_key",
                    format!("Unknown hardware key: {name}"),
                )
            })?;
            session.send_key(keycode)
        }
        InputKind::Text => {
            let text = request.text.as_deref().unwrap_or("");
            session.send_text(text)
        }
    }
}

#[cfg(target_os = "macos")]
fn dispatch_ios_input(
    session: &ios::IosSession,
    request: &EmulatorInputRequest,
) -> CommandResult<()> {
    session.dispatch(request)
}

fn map_hardware_key(name: &str) -> Option<android::input::Keycode> {
    use android::input::Keycode;
    match name {
        "home" => Some(Keycode::Home),
        "back" => Some(Keycode::Back),
        "recents" | "app_switch" | "menu" => Some(Keycode::AppSwitch),
        "vol_up" | "volume_up" => Some(Keycode::VolumeUp),
        "vol_down" | "volume_down" => Some(Keycode::VolumeDown),
        "power" | "lock" => Some(Keycode::Power),
        "enter" => Some(Keycode::Enter),
        "backspace" | "delete" | "del" => Some(Keycode::Del),
        "tab" => Some(Keycode::Tab),
        "escape" => Some(Keycode::Escape),
        "search" => Some(Keycode::Search),
        "dpad_left" | "left" => Some(Keycode::DpadLeft),
        "dpad_right" | "right" => Some(Keycode::DpadRight),
        "dpad_up" | "up" => Some(Keycode::DpadUp),
        "dpad_down" | "down" => Some(Keycode::DpadDown),
        _ => None,
    }
}

#[tauri::command]
pub fn emulator_rotate(
    state: State<'_, EmulatorState>,
    request: EmulatorRotateRequest,
) -> CommandResult<()> {
    let active = state.active.lock().expect("emulator active mutex poisoned");
    let active = active.as_ref().ok_or_else(|| {
        CommandError::user_fixable(
            "emulator_not_running",
            "No emulator device is currently running.",
        )
    })?;

    match active {
        ActiveDevice::Android { session, .. } => {
            let rotation = match request.orientation {
                Orientation::Portrait => 0,
                Orientation::Landscape => 1,
            };
            session.send_rotate(rotation)
        }
        #[cfg(target_os = "macos")]
        ActiveDevice::Ios { session, .. } => session.set_orientation(request.orientation),
        #[cfg(feature = "emulator-synthetic")]
        ActiveDevice::Synthetic { .. } => Ok(()),
    }
}

#[tauri::command]
pub fn emulator_subscribe_ready<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, EmulatorState>,
) -> CommandResult<StatusPayload> {
    let active = state.active.lock().expect("emulator active mutex poisoned");
    let payload = match active.as_ref() {
        Some(device) => StatusPayload::new(StatusPhase::Streaming)
            .with_platform(device.platform().as_str())
            .with_device(device.device_id().to_string()),
        None => StatusPayload::new(StatusPhase::Stopped),
    };
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
        Some(device) => (
            Some(device.platform().as_str().to_string()),
            Some(device.device_id().to_string()),
        ),
        None => (None, None),
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

    drop(taken); // Joins the decoder thread + kills child processes.

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
