use std::path::PathBuf;
use std::process::Command;

fn main() {
    tauri_build::build();
    build_cookie_importer();
}

// Build the sibling cookie-importer crate and copy its binary next to the main
// app binary so it can be spawned at runtime. The cookie-importer has to live
// in a separate cargo workspace because it links rookie → rusqlite 0.31 which
// conflicts with the desktop app's rusqlite 0.37 on the `links = "sqlite3"`
// metadata.
fn build_cookie_importer() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let helper_manifest = manifest_dir.join("crates/cookie-importer/Cargo.toml");
    println!("cargo:rerun-if-changed=crates/cookie-importer/Cargo.toml");
    println!("cargo:rerun-if-changed=crates/cookie-importer/src/main.rs");

    // OUT_DIR looks like .../target/<profile>/build/cadence-desktop-<hash>/out.
    // Walk up three levels to reach the parent of target/<profile>/, then the
    // runtime binary lives in target/<profile>/.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR should be inside target/<profile>/build/<pkg>/out")
        .to_path_buf();

    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    let mut cmd = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()));
    cmd.arg("build")
        .arg("--manifest-path")
        .arg(&helper_manifest);
    if profile == "release" {
        cmd.arg("--release");
    }

    let status = cmd.status().expect("failed to spawn cargo for cookie-importer");
    if !status.success() {
        panic!("failed to build cookie-importer (exit {status:?})");
    }

    let helper_target = manifest_dir
        .join("crates/cookie-importer/target")
        .join(&profile)
        .join(binary_name());
    if !helper_target.exists() {
        panic!("cookie-importer build succeeded but binary not found at {helper_target:?}");
    }

    let destination = profile_dir.join(binary_name());
    std::fs::copy(&helper_target, &destination)
        .unwrap_or_else(|error| panic!("failed to copy cookie-importer to {destination:?}: {error}"));
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "cadence-cookie-importer.exe"
    } else {
        "cadence-cookie-importer"
    }
}
