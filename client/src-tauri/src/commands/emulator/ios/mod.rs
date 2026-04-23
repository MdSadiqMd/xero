//! iOS Simulator pipeline. Fully implemented only on macOS — on other hosts
//! every entry point returns `unsupported_platform` and the shell hides the
//! iOS titlebar button.

pub mod sdk;

#[cfg(target_os = "macos")]
pub mod idb_companion;
#[cfg(target_os = "macos")]
pub mod idb_client;
#[cfg(target_os = "macos")]
pub mod input;
#[cfg(target_os = "macos")]
pub mod session;
#[cfg(target_os = "macos")]
pub mod xcrun;

#[cfg(target_os = "macos")]
pub use session::{list_devices, spawn, IosSession, SpawnArgs};

#[cfg(not(target_os = "macos"))]
mod unsupported {
    use crate::commands::CommandError;

    pub fn list_devices() -> Vec<super::sdk::IosDeviceStub> {
        Vec::new()
    }

    pub fn unsupported() -> CommandError {
        CommandError::user_fixable(
            "ios_unsupported",
            "iOS Simulator is only available on macOS.",
        )
    }
}

#[cfg(not(target_os = "macos"))]
pub use unsupported::{list_devices, unsupported};
