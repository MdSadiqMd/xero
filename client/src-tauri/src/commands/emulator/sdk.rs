use std::env;
use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

/// Result of probing the host machine for each platform's SDK. Surfaced to
/// the frontend so the missing-SDK panel can render without blocking the
/// user on start.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SdkStatus {
    pub android: AndroidSdkStatus,
    pub ios: IosSdkStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AndroidSdkStatus {
    pub present: bool,
    pub sdk_root: Option<String>,
    pub emulator_path: Option<String>,
    pub adb_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IosSdkStatus {
    pub present: bool,
    pub xcrun_path: Option<String>,
    /// Host OS supports iOS Simulator (only macOS does).
    pub supported: bool,
}

pub fn probe_sdks() -> SdkStatus {
    SdkStatus {
        android: probe_android(),
        ios: probe_ios(),
    }
}

fn probe_android() -> AndroidSdkStatus {
    let sdk_root = env::var("ANDROID_HOME")
        .ok()
        .or_else(|| env::var("ANDROID_SDK_ROOT").ok());

    let emulator_path = which_binary("emulator")
        .or_else(|| sdk_root.as_deref().and_then(|root| sdk_bin(root, "emulator/emulator")));
    let adb_path = which_binary("adb")
        .or_else(|| sdk_root.as_deref().and_then(|root| sdk_bin(root, "platform-tools/adb")));

    AndroidSdkStatus {
        present: emulator_path.is_some() && adb_path.is_some(),
        sdk_root,
        emulator_path,
        adb_path,
    }
}

fn probe_ios() -> IosSdkStatus {
    let supported = cfg!(target_os = "macos");
    if !supported {
        return IosSdkStatus {
            present: false,
            xcrun_path: None,
            supported,
        };
    }

    let xcrun_path = which_binary("xcrun");
    IosSdkStatus {
        present: xcrun_path.is_some(),
        xcrun_path,
        supported,
    }
}

fn which_binary(name: &str) -> Option<String> {
    // Use `which` on Unix and `where` on Windows. Fall back to walking PATH
    // manually if neither exits cleanly, since some environments ship with a
    // stripped-down /usr/bin.
    #[cfg(target_family = "unix")]
    let locator = "which";
    #[cfg(target_family = "windows")]
    let locator = "where";

    if let Ok(out) = Command::new(locator).arg(name).output() {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join(exe_name(name));
            if candidate.is_file() {
                Some(candidate.to_string_lossy().into_owned())
            } else {
                None
            }
        })
    })
}

fn exe_name(name: &str) -> PathBuf {
    #[cfg(target_family = "windows")]
    {
        PathBuf::from(format!("{name}.exe"))
    }
    #[cfg(not(target_family = "windows"))]
    {
        PathBuf::from(name)
    }
}

fn sdk_bin(root: &str, relative: &str) -> Option<String> {
    let path = PathBuf::from(root).join(relative);
    let with_ext = if cfg!(target_os = "windows") {
        path.with_extension("exe")
    } else {
        path.clone()
    };
    for candidate in [with_ext, path] {
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}
