//! Phase 9 — cross-cluster program version drift detection.
//!
//! The shipping-breakage shape this catches: you pin Metaplex Token
//! Metadata v1.13 in your devnet integration tests because "that's
//! what's deployed there", then mainnet has v1.14 and a new
//! instruction's discriminant byte flips your handler. Same for
//! Jupiter, Squads v4, SPL Governance.
//!
//! We detect drift by fetching each program's on-chain ProgramData
//! SHA-256 from every healthy cluster in the RPC router and comparing
//! the hashes. Same hash across clusters = same deployed bytes =
//! same version. Different hashes = drift; the agent / UI can then
//! decide whether the drift is expected or a blocker.
//!
//! This keeps us off a versioned manifest — the on-chain bytes are
//! authoritative and never lie.

pub mod registry;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::commands::solana::cluster::ClusterKind;
use crate::commands::solana::program::upgrade_safety::BPF_UPGRADEABLE_LOADER;
use crate::commands::solana::rpc_router::RpcRouter;
use crate::commands::solana::tx::RpcTransport;
use crate::commands::{CommandError, CommandResult};

pub use registry::{builtin_tracked_programs, TrackedProgram};

const PROGRAMDATA_DATA_OFFSET: usize = 45;

/// Request shape for `solana_cluster_drift_check`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DriftCheckRequest {
    /// Extra program IDs to include beyond the built-in tracked
    /// registry. Pass a label + id pair so the report groups cleanly
    /// with the built-ins.
    #[serde(default)]
    pub additional: Vec<TrackedProgram>,
    /// Restrict the check to these clusters only. Empty = all clusters
    /// in the router with a healthy endpoint.
    #[serde(default)]
    pub clusters: Vec<ClusterKind>,
    /// Per-cluster RPC URL override. Useful in CI tests where we pin a
    /// scripted transport to specific URLs.
    #[serde(default)]
    pub rpc_urls: BTreeMap<ClusterKind, String>,
    /// Skip the default tracked registry. Usually false — but the
    /// autonomous-runtime wrapper exposes the toggle so callers can
    /// run a focused drift check on a single program.
    #[serde(default)]
    pub skip_builtins: bool,
    /// Timeout per RPC call in ms. Defaults to 4s.
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DriftStatus {
    /// Every cluster has the same program bytes.
    InSync,
    /// At least two clusters have different bytes.
    Drift,
    /// Program missing on at least one cluster (could be intentional).
    PartiallyDeployed,
    /// The lookup couldn't complete for a cluster.
    Inconclusive,
}

impl DriftStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            DriftStatus::InSync => "in_sync",
            DriftStatus::Drift => "drift",
            DriftStatus::PartiallyDeployed => "partially_deployed",
            DriftStatus::Inconclusive => "inconclusive",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DriftProbe {
    pub cluster: ClusterKind,
    pub rpc_url: String,
    pub program_data_sha256: Option<String>,
    pub program_data_length: Option<u64>,
    pub upgrade_authority: Option<String>,
    pub last_deployed_slot: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DriftEntry {
    pub program: TrackedProgram,
    pub status: DriftStatus,
    pub probes: Vec<DriftProbe>,
    /// Summary human-readable sentence shown in the UI.
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DriftReport {
    pub entries: Vec<DriftEntry>,
    pub clusters_checked: Vec<ClusterKind>,
    pub duration_ms: u128,
    /// True when any entry has `DriftStatus::Drift`. Deploy gates can
    /// use this to warn (not block — drift isn't inherently a
    /// security issue, just a footgun).
    pub has_drift: bool,
}

/// Fully injectable implementation. Callers in production pass the
/// live `RpcRouter` + `RpcTransport` held inside `SolanaState`; tests
/// pass scripted implementations.
pub fn check(
    transport: &Arc<dyn RpcTransport>,
    router: &Arc<RpcRouter>,
    request: &DriftCheckRequest,
) -> CommandResult<DriftReport> {
    let started = Instant::now();
    let mut tracked: Vec<TrackedProgram> = if request.skip_builtins {
        Vec::new()
    } else {
        builtin_tracked_programs()
    };
    tracked.extend(request.additional.iter().cloned());

    if tracked.is_empty() {
        return Err(CommandError::user_fixable(
            "solana_drift_no_programs",
            "Drift check needs at least one tracked program. Pass `additional` or unset \
             `skip_builtins`.",
        ));
    }

    let clusters: Vec<ClusterKind> = if request.clusters.is_empty() {
        // Every cluster in the router. We filter out clusters with no
        // endpoints further down.
        ClusterKind::ALL.to_vec()
    } else {
        request.clusters.clone()
    };

    // Per-cluster URL resolution. We keep the list around so the
    // report can echo the URLs back.
    let mut per_cluster_url: BTreeMap<ClusterKind, String> = BTreeMap::new();
    let mut clusters_actually_checked: Vec<ClusterKind> = Vec::new();
    for cluster in clusters {
        let maybe_url = request
            .rpc_urls
            .get(&cluster)
            .cloned()
            .or_else(|| router.pick_healthy(cluster).map(|e| e.url));
        if let Some(url) = maybe_url {
            per_cluster_url.insert(cluster, url);
            clusters_actually_checked.push(cluster);
        }
    }

    let mut entries: Vec<DriftEntry> = Vec::with_capacity(tracked.len());
    let mut had_drift = false;
    for program in tracked {
        let mut probes: Vec<DriftProbe> = Vec::with_capacity(per_cluster_url.len());
        for cluster in &clusters_actually_checked {
            let url = per_cluster_url.get(cluster).cloned().unwrap_or_default();
            let probe = probe_program(transport.as_ref(), *cluster, &url, &program.program_id);
            probes.push(probe);
        }

        let status = classify(&probes);
        let summary = summarise(&program, &probes, status);
        if status == DriftStatus::Drift {
            had_drift = true;
        }
        entries.push(DriftEntry {
            program,
            status,
            probes,
            summary,
        });
    }

    entries.sort_by(|a, b| a.program.label.cmp(&b.program.label));

    Ok(DriftReport {
        entries,
        clusters_checked: clusters_actually_checked,
        duration_ms: started.elapsed().as_millis(),
        has_drift: had_drift,
    })
}

fn classify(probes: &[DriftProbe]) -> DriftStatus {
    let mut hashes = std::collections::BTreeSet::new();
    let mut missing = 0;
    let mut errors = 0;
    let mut present = 0;
    for probe in probes {
        if let Some(err) = probe.error.as_deref() {
            if err == "program_not_found" {
                missing += 1;
            } else {
                errors += 1;
            }
            continue;
        }
        if let Some(hash) = probe.program_data_sha256.as_deref() {
            hashes.insert(hash.to_string());
            present += 1;
        }
    }
    if errors > 0 && present == 0 {
        return DriftStatus::Inconclusive;
    }
    if missing > 0 && present > 0 {
        return DriftStatus::PartiallyDeployed;
    }
    if hashes.len() <= 1 {
        return DriftStatus::InSync;
    }
    DriftStatus::Drift
}

fn summarise(program: &TrackedProgram, probes: &[DriftProbe], status: DriftStatus) -> String {
    match status {
        DriftStatus::InSync => format!(
            "{} has identical bytes across {} cluster(s).",
            program.label,
            probes.iter().filter(|p| p.program_data_sha256.is_some()).count(),
        ),
        DriftStatus::Drift => {
            let mut clusters_by_hash: BTreeMap<String, Vec<&str>> = BTreeMap::new();
            for probe in probes {
                if let Some(hash) = probe.program_data_sha256.as_deref() {
                    clusters_by_hash
                        .entry(hash.to_string())
                        .or_default()
                        .push(probe.cluster.as_str());
                }
            }
            let groups: Vec<String> = clusters_by_hash
                .into_iter()
                .map(|(hash, clusters)| {
                    let short = hash.chars().take(8).collect::<String>();
                    format!("{}: {}…", clusters.join("+"), short)
                })
                .collect();
            format!("{} drifts: {}", program.label, groups.join(" | "))
        }
        DriftStatus::PartiallyDeployed => format!(
            "{} is missing on at least one cluster — deploy before cross-cluster work.",
            program.label,
        ),
        DriftStatus::Inconclusive => format!(
            "{} lookups failed. Check RPC health and try again.",
            program.label,
        ),
    }
}

fn probe_program(
    transport: &dyn RpcTransport,
    cluster: ClusterKind,
    url: &str,
    program_id: &str,
) -> DriftProbe {
    let mut probe = DriftProbe {
        cluster,
        rpc_url: url.to_string(),
        program_data_sha256: None,
        program_data_length: None,
        upgrade_authority: None,
        last_deployed_slot: None,
        error: None,
    };
    if url.is_empty() {
        probe.error = Some("rpc_url_missing".into());
        return probe;
    }
    // Step 1: getAccountInfo on the program id.
    let program_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [
            program_id,
            {"encoding": "base64", "commitment": "confirmed"}
        ]
    });
    let program_account = match transport.post(url, program_body) {
        Ok(v) => v,
        Err(err) => {
            probe.error = Some(format!("rpc: {}", err.message));
            return probe;
        }
    };
    let program_value = match extract_account(&program_account) {
        Some(v) => v,
        None => {
            probe.error = Some("program_not_found".into());
            return probe;
        }
    };
    let Some(owner) = program_value
        .get("owner")
        .and_then(|o| o.as_str())
    else {
        probe.error = Some("program_owner_missing".into());
        return probe;
    };
    if owner != BPF_UPGRADEABLE_LOADER {
        // Non-upgradeable program — the program bytes live at the
        // program account itself.
        return finalize_with_account(probe, program_value);
    }
    let data_field = program_value
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let decoded = match base64::engine::general_purpose::STANDARD.decode(data_field) {
        Ok(v) => v,
        Err(err) => {
            probe.error = Some(format!("program_data_decode: {err}"));
            return probe;
        }
    };
    // The upgradeable-loader Program enum stores the ProgramData
    // address in bytes 4..36.
    if decoded.len() < 36 {
        probe.error = Some("program_state_too_short".into());
        return probe;
    }
    let mut programdata_bytes = [0u8; 32];
    programdata_bytes.copy_from_slice(&decoded[4..36]);
    let programdata_address = bs58::encode(programdata_bytes).into_string();

    let pd_body = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "getAccountInfo",
        "params": [
            programdata_address,
            {"encoding": "base64", "commitment": "confirmed"}
        ]
    });
    let pd_response = match transport.post(url, pd_body) {
        Ok(v) => v,
        Err(err) => {
            probe.error = Some(format!("rpc_programdata: {}", err.message));
            return probe;
        }
    };
    let pd_value = match extract_account(&pd_response) {
        Some(v) => v,
        None => {
            probe.error = Some("programdata_not_found".into());
            return probe;
        }
    };
    finalize_with_programdata(probe, pd_value)
}

fn extract_account(value: &Value) -> Option<Value> {
    value
        .get("result")
        .and_then(|r| r.get("value"))
        .filter(|v| !v.is_null())
        .cloned()
}

fn finalize_with_account(mut probe: DriftProbe, value: Value) -> DriftProbe {
    let data = value
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .unwrap_or("");
    match base64::engine::general_purpose::STANDARD.decode(data) {
        Ok(bytes) => {
            probe.program_data_length = Some(bytes.len() as u64);
            probe.program_data_sha256 = Some(hex_sha256(&bytes));
        }
        Err(err) => {
            probe.error = Some(format!("program_data_decode: {err}"));
        }
    }
    probe
}

fn finalize_with_programdata(mut probe: DriftProbe, value: Value) -> DriftProbe {
    let data = value
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let bytes = match base64::engine::general_purpose::STANDARD.decode(data) {
        Ok(v) => v,
        Err(err) => {
            probe.error = Some(format!("programdata_decode: {err}"));
            return probe;
        }
    };
    if bytes.len() < PROGRAMDATA_DATA_OFFSET {
        probe.error = Some(format!(
            "programdata_header_short: {} bytes, want >= {}",
            bytes.len(),
            PROGRAMDATA_DATA_OFFSET,
        ));
        return probe;
    }
    // Header: 4 bytes enum tag + 8 bytes last_deployed_slot + 1 byte
    // Option<Pubkey> tag + optional 32-byte authority (bytes 13..45).
    let slot = {
        let mut s = [0u8; 8];
        s.copy_from_slice(&bytes[4..12]);
        u64::from_le_bytes(s)
    };
    let has_authority = bytes[12] == 1;
    let authority = if has_authority {
        let mut buf = [0u8; 32];
        buf.copy_from_slice(&bytes[13..45]);
        Some(bs58::encode(buf).into_string())
    } else {
        None
    };
    let program_bytes = &bytes[PROGRAMDATA_DATA_OFFSET..];
    probe.program_data_sha256 = Some(hex_sha256(program_bytes));
    probe.program_data_length = Some(program_bytes.len() as u64);
    probe.last_deployed_slot = Some(slot);
    probe.upgrade_authority = authority;
    probe
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for b in digest {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct ScriptedTransport {
        calls: Mutex<Vec<(String, Value)>>,
        responses: Mutex<std::collections::VecDeque<Value>>,
    }

    impl ScriptedTransport {
        fn push(&self, response: Value) {
            self.responses.lock().unwrap().push_back(response);
        }
    }

    impl RpcTransport for ScriptedTransport {
        fn post(&self, url: &str, body: Value) -> CommandResult<Value> {
            self.calls.lock().unwrap().push((url.into(), body));
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| CommandError::system_fault("scripted", "exhausted"))
        }
    }

    fn make_program_response(owner: &str, data_b64: &str) -> Value {
        json!({
            "result": {
                "value": {
                    "owner": owner,
                    "data": [data_b64, "base64"],
                    "executable": true,
                    "lamports": 1_000_000_u64,
                    "rentEpoch": 0
                }
            }
        })
    }

    fn encode_program_account_pointing_at(programdata_pubkey_bytes: &[u8; 32]) -> String {
        // Upgradeable Program state: 4 bytes tag (2, LE) + 32 bytes programdata pubkey.
        let mut bytes = vec![0u8; 36];
        bytes[0] = 2;
        bytes[4..36].copy_from_slice(programdata_pubkey_bytes);
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    fn encode_programdata(slot: u64, authority: Option<[u8; 32]>, program_bytes: &[u8]) -> String {
        let mut bytes = vec![0u8; PROGRAMDATA_DATA_OFFSET];
        bytes[0] = 3; // ProgramData tag
        bytes[4..12].copy_from_slice(&slot.to_le_bytes());
        if let Some(auth) = authority {
            bytes[12] = 1;
            bytes[13..45].copy_from_slice(&auth);
        }
        bytes.extend_from_slice(program_bytes);
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    #[test]
    fn classify_returns_in_sync_for_identical_hashes() {
        let probes = vec![
            DriftProbe {
                cluster: ClusterKind::Devnet,
                rpc_url: "u".into(),
                program_data_sha256: Some("abc".into()),
                program_data_length: Some(4),
                upgrade_authority: None,
                last_deployed_slot: None,
                error: None,
            },
            DriftProbe {
                cluster: ClusterKind::Mainnet,
                rpc_url: "u".into(),
                program_data_sha256: Some("abc".into()),
                program_data_length: Some(4),
                upgrade_authority: None,
                last_deployed_slot: None,
                error: None,
            },
        ];
        assert_eq!(classify(&probes), DriftStatus::InSync);
    }

    #[test]
    fn classify_detects_drift_across_clusters() {
        let probes = vec![
            DriftProbe {
                cluster: ClusterKind::Devnet,
                rpc_url: "u".into(),
                program_data_sha256: Some("abc".into()),
                program_data_length: Some(4),
                upgrade_authority: None,
                last_deployed_slot: None,
                error: None,
            },
            DriftProbe {
                cluster: ClusterKind::Mainnet,
                rpc_url: "u".into(),
                program_data_sha256: Some("def".into()),
                program_data_length: Some(4),
                upgrade_authority: None,
                last_deployed_slot: None,
                error: None,
            },
        ];
        assert_eq!(classify(&probes), DriftStatus::Drift);
    }

    #[test]
    fn classify_detects_partial_deploy() {
        let probes = vec![
            DriftProbe {
                cluster: ClusterKind::Devnet,
                rpc_url: "u".into(),
                program_data_sha256: Some("abc".into()),
                program_data_length: Some(4),
                upgrade_authority: None,
                last_deployed_slot: None,
                error: None,
            },
            DriftProbe {
                cluster: ClusterKind::Mainnet,
                rpc_url: "u".into(),
                program_data_sha256: None,
                program_data_length: None,
                upgrade_authority: None,
                last_deployed_slot: None,
                error: Some("program_not_found".into()),
            },
        ];
        assert_eq!(classify(&probes), DriftStatus::PartiallyDeployed);
    }

    #[test]
    fn probe_program_follows_upgradeable_loader_pointer() {
        let transport = ScriptedTransport::default();
        let programdata_pubkey = [7u8; 32];
        transport.push(make_program_response(
            BPF_UPGRADEABLE_LOADER,
            &encode_program_account_pointing_at(&programdata_pubkey),
        ));
        transport.push(make_program_response(
            BPF_UPGRADEABLE_LOADER,
            &encode_programdata(42, Some([9u8; 32]), &[1, 2, 3, 4]),
        ));
        let probe = probe_program(
            &transport,
            ClusterKind::Devnet,
            "http://u",
            "ProgramIdBase58",
        );
        assert!(probe.error.is_none(), "unexpected error: {:?}", probe.error);
        assert_eq!(probe.program_data_length, Some(4));
        assert_eq!(probe.last_deployed_slot, Some(42));
        assert!(probe.upgrade_authority.is_some());
        assert!(probe.program_data_sha256.is_some());
    }

    #[test]
    fn probe_program_handles_non_upgradeable() {
        let transport = ScriptedTransport::default();
        transport.push(make_program_response(
            "11111111111111111111111111111111",
            &base64::engine::general_purpose::STANDARD.encode(b"direct"),
        ));
        let probe = probe_program(
            &transport,
            ClusterKind::Localnet,
            "http://u",
            "SomeProgram",
        );
        assert!(probe.error.is_none());
        assert_eq!(probe.program_data_length, Some(6));
    }

    #[test]
    fn probe_program_reports_missing_program() {
        let transport = ScriptedTransport::default();
        transport.push(json!({"result": {"value": null}}));
        let probe = probe_program(
            &transport,
            ClusterKind::Devnet,
            "http://u",
            "Gone",
        );
        assert_eq!(probe.error.as_deref(), Some("program_not_found"));
    }
}
