use serde_json::Value;

#[test]
fn macos_release_config_bundles_and_verifies_desktop_sidecar() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let base_config = read_config(&manifest_dir.join("tauri.conf.json"));
    let macos_config = read_config(&manifest_dir.join("tauri.macos.conf.json"));
    let signing_script =
        std::fs::read_to_string(manifest_dir.join("scripts/sign-macos-target-binaries.sh"))
            .expect("read macOS signing script");
    let verification_script =
        std::fs::read_to_string(manifest_dir.join("scripts/verify-macos-desktop-sidecar.sh"))
            .expect("read macOS sidecar verification script");

    assert!(
        bundle_resources(&base_config).contains(&"resources/xero-desktop-sidecar*".to_owned()),
        "base Tauri resources must include the desktop stream sidecar"
    );
    assert!(
        bundle_resources(&macos_config).contains(&"resources/xero-desktop-sidecar*".to_owned()),
        "macOS Tauri override must not drop the desktop stream sidecar"
    );
    assert!(
        signing_script.contains("resources/xero-desktop-sidecar"),
        "macOS signing must sign the generated sidecar resource before bundling"
    );
    assert!(
        verification_script.contains("Contents/Resources/resources/xero-desktop-sidecar"),
        "macOS release verification must check Tauri's preserved resource path"
    );
}

fn read_config(path: &std::path::Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read Tauri config"))
        .expect("parse Tauri config")
}

fn bundle_resources(config: &Value) -> Vec<String> {
    match &config["bundle"]["resources"] {
        Value::Array(resources) => resources
            .iter()
            .map(|resource| {
                resource
                    .as_str()
                    .expect("bundle resource entries must be strings")
                    .to_owned()
            })
            .collect(),
        Value::Object(resources) => resources.keys().cloned().collect(),
        other => panic!("bundle.resources must be an array or object, got {other:?}"),
    }
}
