//! AVD (Android Virtual Device) enumeration.
//!
//! `emulator -list-avds` gives us names. `avdmanager list avd -c` also gives
//! names but more reliably when the user has a non-default AVD home. When
//! both are available we merge, preferring `avdmanager` output because it
//! includes hidden AVDs in the user's `~/.android/avd` directory even if
//! they haven't been registered with `emulator`.

use std::path::Path;
use std::process::{Command, Stdio};

use serde::Serialize;

use super::sdk::AndroidSdk;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidAvd {
    /// AVD name (e.g. "Pixel_8_API_34"). Used as the `@<name>` argument to
    /// the emulator binary.
    pub name: String,
    pub display_name: String,
    /// Best-effort device kind derived from the AVD's `hw.lcd` metadata.
    /// We default to `phone` if we can't tell.
    pub kind: AvdKind,
    /// Best-effort viewport resolution. May be `None` when avdmanager isn't
    /// available — in that case the emulator will report it over scrcpy's
    /// video-metadata header once it connects.
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub density: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AvdKind {
    Phone,
    Tablet,
}

/// Enumerate AVDs using whichever tools are available.
pub fn list(sdk: &AndroidSdk) -> std::io::Result<Vec<AndroidAvd>> {
    let names = collect_names(sdk)?;
    if names.is_empty() {
        return Ok(Vec::new());
    }

    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let meta = probe_avd_ini(&name);
        let (display_name, kind, width, height, density) = match meta {
            Some(m) => (
                m.display_name.unwrap_or_else(|| name.clone()),
                m.kind,
                m.width,
                m.height,
                m.density,
            ),
            None => (humanize_name(&name), AvdKind::Phone, None, None, None),
        };
        out.push(AndroidAvd {
            name,
            display_name,
            kind,
            width,
            height,
            density,
        });
    }
    Ok(out)
}

fn collect_names(sdk: &AndroidSdk) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();

    if let Some(emulator) = sdk.emulator_path() {
        let output = Command::new(emulator)
            .arg("-list-avds")
            .stderr(Stdio::null())
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                for line in String::from_utf8_lossy(&out.stdout).lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        names.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    if let Some(avdmanager) = sdk.avdmanager.as_deref() {
        let output = Command::new(avdmanager)
            .args(["list", "avd", "-c"])
            .stderr(Stdio::null())
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                for line in String::from_utf8_lossy(&out.stdout).lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if !names.contains(&trimmed.to_string()) {
                        names.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    Ok(names)
}

struct IniMeta {
    display_name: Option<String>,
    kind: AvdKind,
    width: Option<u32>,
    height: Option<u32>,
    density: Option<u32>,
}

/// Parse the AVD's `config.ini` out of `~/.android/avd/<name>.avd/config.ini`
/// for metadata we can't get from CLI flags. Failing to find it is not an
/// error — it just means the sidebar will show the AVD with defaults.
fn probe_avd_ini(name: &str) -> Option<IniMeta> {
    let home = dirs::home_dir()?;
    let ini_path = home
        .join(".android/avd")
        .join(format!("{name}.avd"))
        .join("config.ini");
    let bytes = std::fs::read(&ini_path).ok()?;
    parse_ini_meta(&bytes, &ini_path)
}

fn parse_ini_meta(bytes: &[u8], _ini_path: &Path) -> Option<IniMeta> {
    let text = String::from_utf8_lossy(bytes);
    let mut display_name = None;
    let mut width = None;
    let mut height = None;
    let mut density = None;
    let mut tag = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "avd.ini.displayname" | "hw.device.name" => {
                if display_name.is_none() && !value.is_empty() {
                    display_name = Some(value.to_string());
                }
            }
            "hw.lcd.width" => width = value.parse().ok(),
            "hw.lcd.height" => height = value.parse().ok(),
            "hw.lcd.density" => density = value.parse().ok(),
            "tag.id" | "hw.device.manufacturer" => {
                if tag.is_none() {
                    tag = Some(value.to_string());
                }
            }
            _ => {}
        }
    }

    let kind = classify_kind(width, height, tag.as_deref());
    Some(IniMeta {
        display_name,
        kind,
        width,
        height,
        density,
    })
}

fn classify_kind(width: Option<u32>, height: Option<u32>, _tag: Option<&str>) -> AvdKind {
    match (width, height) {
        (Some(w), Some(h)) if w.min(h) >= 1200 => AvdKind::Tablet,
        _ => AvdKind::Phone,
    }
}

fn humanize_name(name: &str) -> String {
    name.replace(['_', '-'], " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parses_standard_config_ini() {
        let ini = b"\
avd.ini.displayname = Pixel 8\n\
hw.lcd.width = 1080\n\
hw.lcd.height = 2400\n\
hw.lcd.density = 420\n\
";
        let meta = parse_ini_meta(ini, Path::new("x")).expect("parsed");
        assert_eq!(meta.display_name.as_deref(), Some("Pixel 8"));
        assert_eq!(meta.width, Some(1080));
        assert_eq!(meta.height, Some(2400));
        assert_eq!(meta.density, Some(420));
        assert_eq!(meta.kind, AvdKind::Phone);
    }

    #[test]
    fn tablet_detected_from_dimensions() {
        let ini = b"\
hw.lcd.width = 1600\n\
hw.lcd.height = 2560\n\
";
        let meta = parse_ini_meta(ini, Path::new("x")).expect("parsed");
        assert_eq!(meta.kind, AvdKind::Tablet);
    }

    #[test]
    fn humanize_name_replaces_separators() {
        assert_eq!(humanize_name("Pixel_8_API_34"), "Pixel 8 API 34");
    }
}
