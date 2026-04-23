//! iOS session skeleton. Phase 3 ships a not-yet-implemented stub so the
//! command dispatch compiles on macOS; Phase 4 replaces this with the real
//! `idb_companion` + `simctl` pipeline.

use std::sync::Arc;

use tauri::{AppHandle, Runtime};

use crate::commands::emulator::frame_bus::FrameBus;
use crate::commands::emulator::{EmulatorInputRequest, Orientation};
use crate::commands::CommandError;

/// iOS device metadata the `emulator_list_devices` command unwraps. `udid` is
/// the canonical simulator identifier passed back into `emulator_start`.
#[derive(Debug, Clone)]
pub struct SimulatorDescriptor {
    pub udid: String,
    pub display_name: String,
    pub is_tablet: bool,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub scale: Option<f32>,
}

pub struct SpawnArgs<R: Runtime> {
    pub app: AppHandle<R>,
    pub frame_bus: Arc<FrameBus>,
    pub device_id: String,
}

pub struct IosSession {
    device_id: String,
    width: u32,
    height: u32,
}

impl IosSession {
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn dispatch(&self, _request: &EmulatorInputRequest) -> Result<(), CommandError> {
        Err(CommandError::system_fault(
            "ios_not_implemented",
            "iOS input dispatch lands with the Phase 4 idb_companion pipeline.",
        ))
    }

    pub fn set_orientation(&self, _orientation: Orientation) -> Result<(), CommandError> {
        Err(CommandError::system_fault(
            "ios_not_implemented",
            "iOS rotation lands with the Phase 4 idb_companion pipeline.",
        ))
    }
}

pub fn list_devices() -> Vec<SimulatorDescriptor> {
    // Phase 3 returns an empty list on macOS; Phase 4 queries simctl.
    Vec::new()
}

pub fn spawn<R: Runtime>(_args: SpawnArgs<R>) -> Result<IosSession, CommandError> {
    Err(CommandError::system_fault(
        "ios_not_implemented",
        "iOS Simulator streaming is implemented in Phase 4. Use Android for now, or enable \
         --features emulator-synthetic to exercise the pipeline.",
    ))
}
