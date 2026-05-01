use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DesktopPlatform {
    Macos,
    Windows,
    Linux,
}

#[tauri::command]
pub fn desktop_platform() -> DesktopPlatform {
    if cfg!(target_os = "macos") {
        DesktopPlatform::Macos
    } else if cfg!(windows) {
        DesktopPlatform::Windows
    } else {
        DesktopPlatform::Linux
    }
}
