//! Cross-cluster persona scope hygiene.
//!
//! The scanner above catches secrets in the project tree. This module
//! catches a different class of bug: the same *public key* loaded
//! under two cluster registries, or a persona whose note marks it as
//! a mainnet authority while it's registered on devnet/localnet. That
//! pattern is the classic devnet-deploys-with-mainnet-keys footgun.
//!
//! `check_scope` is pure over a `PersonaStore` snapshot — we read the
//! existing cluster registries and emit warnings without touching the
//! filesystem. It's cheap enough that the UI can call it on every
//! workbench open without a spinner.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::commands::solana::cluster::ClusterKind;
use crate::commands::solana::persona::{Persona, PersonaStore};
use crate::commands::CommandResult;

use super::SecretSeverity;

/// Discriminates between the different mistake shapes a scope check
/// can catch. The frontend branches on this to pick the icon + copy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScopeWarningKind {
    /// Same pubkey used on two (or more) clusters.
    CrossClusterReuse,
    /// Persona lives on devnet/localnet but the note says
    /// "mainnet", "prod", "authority", etc.
    MainnetLabelOnNonMainnet,
    /// Persona is on mainnet-fork/devnet and was imported (not
    /// generated). Imports bypass the random-keypair path so the user
    /// may have a real mainnet key in the tree.
    SuspectedRealKeyOnForkOrDevnet,
}

impl ScopeWarningKind {
    pub fn rule_id(self) -> &'static str {
        match self {
            ScopeWarningKind::CrossClusterReuse => "scope_cross_cluster_reuse",
            ScopeWarningKind::MainnetLabelOnNonMainnet => "scope_mainnet_label_on_devnet",
            ScopeWarningKind::SuspectedRealKeyOnForkOrDevnet => {
                "scope_suspected_real_key_on_fork_or_devnet"
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScopeWarning {
    pub kind: ScopeWarningKind,
    pub severity: SecretSeverity,
    pub persona: String,
    pub cluster: ClusterKind,
    pub related_clusters: Vec<ClusterKind>,
    pub pubkey: String,
    pub message: String,
    pub remediation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScopeCheckReport {
    pub warnings: Vec<ScopeWarning>,
    pub personas_inspected: u32,
    /// Whether at least one warning is high-severity — deploy gates
    /// can consult this to escalate to `user_fixable`.
    pub blocks_deploy: bool,
}

/// Run the scope check over every cluster's persona registry.
pub fn check_scope(store: &Arc<PersonaStore>) -> CommandResult<ScopeCheckReport> {
    let mut by_pubkey: BTreeMap<String, Vec<Persona>> = BTreeMap::new();
    let mut total: u32 = 0;
    for cluster in ClusterKind::ALL {
        let personas = store.list(cluster).unwrap_or_default();
        total = total.saturating_add(personas.len() as u32);
        for p in personas {
            by_pubkey.entry(p.pubkey.clone()).or_default().push(p);
        }
    }

    let mut warnings: Vec<ScopeWarning> = Vec::new();

    // Cross-cluster reuse.
    for (pubkey, group) in &by_pubkey {
        if group.len() < 2 {
            continue;
        }
        let mut names: Vec<&str> = group.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        let mut clusters: Vec<ClusterKind> = group.iter().map(|p| p.cluster).collect();
        clusters.sort();
        clusters.dedup();
        let severity = if clusters.contains(&ClusterKind::Mainnet) {
            SecretSeverity::Critical
        } else {
            SecretSeverity::High
        };
        for persona in group {
            let related: Vec<ClusterKind> = clusters
                .iter()
                .copied()
                .filter(|c| *c != persona.cluster)
                .collect();
            warnings.push(ScopeWarning {
                kind: ScopeWarningKind::CrossClusterReuse,
                severity,
                persona: persona.name.clone(),
                cluster: persona.cluster,
                related_clusters: related,
                pubkey: pubkey.clone(),
                message: format!(
                    "Pubkey {} is registered on {} different cluster(s): {}",
                    pubkey,
                    clusters.len(),
                    clusters
                        .iter()
                        .map(|c| c.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
                remediation:
                    "Generate a fresh keypair per cluster. Never share a mainnet authority with \
                     devnet or localnet personas."
                        .to_string(),
            });
        }
    }

    // Mainnet-label / imported-on-fork patterns.
    for personas in by_pubkey.values() {
        for persona in personas {
            if let Some(warning) = label_warning(persona) {
                warnings.push(warning);
            }
            if let Some(warning) = imported_on_fork_warning(persona) {
                warnings.push(warning);
            }
        }
    }

    warnings.sort_by(|a, b| {
        a.severity
            .rank()
            .cmp(&b.severity.rank())
            .then_with(|| a.persona.cmp(&b.persona))
            .then_with(|| a.kind.rule_id().cmp(b.kind.rule_id()))
    });

    let blocks_deploy = warnings
        .iter()
        .any(|w| matches!(w.severity, SecretSeverity::Critical));

    Ok(ScopeCheckReport {
        warnings,
        personas_inspected: total,
        blocks_deploy,
    })
}

fn label_warning(persona: &Persona) -> Option<ScopeWarning> {
    if matches!(persona.cluster, ClusterKind::Mainnet) {
        return None;
    }
    let haystack = format!(
        "{} {}",
        persona.name.to_ascii_lowercase(),
        persona.note.as_deref().unwrap_or("").to_ascii_lowercase()
    );
    let flagged = ["mainnet", "prod", "production", "authority-mainnet"]
        .iter()
        .any(|needle| haystack.contains(needle));
    if !flagged {
        return None;
    }
    Some(ScopeWarning {
        kind: ScopeWarningKind::MainnetLabelOnNonMainnet,
        severity: SecretSeverity::High,
        persona: persona.name.clone(),
        cluster: persona.cluster,
        related_clusters: vec![ClusterKind::Mainnet],
        pubkey: persona.pubkey.clone(),
        message: format!(
            "Persona '{}' is flagged as mainnet/prod but lives on {}.",
            persona.name,
            persona.cluster.as_str(),
        ),
        remediation: "Rename the persona or generate a dedicated keypair for this cluster.".into(),
    })
}

fn imported_on_fork_warning(persona: &Persona) -> Option<ScopeWarning> {
    if !matches!(
        persona.cluster,
        ClusterKind::MainnetFork | ClusterKind::Devnet
    ) {
        return None;
    }
    // Imported keypairs leave the note field user-set; the PersonaStore
    // only exposes `note` so we use its contents as the heuristic. A
    // future refactor could add an explicit `imported` bool on Persona.
    let note = persona.note.as_deref().unwrap_or("").to_ascii_lowercase();
    if !(note.contains("import") || note.contains("real ")) {
        return None;
    }
    Some(ScopeWarning {
        kind: ScopeWarningKind::SuspectedRealKeyOnForkOrDevnet,
        severity: SecretSeverity::Medium,
        persona: persona.name.clone(),
        cluster: persona.cluster,
        related_clusters: vec![],
        pubkey: persona.pubkey.clone(),
        message: format!(
            "Persona '{}' looks imported and sits on a non-local cluster.",
            persona.name,
        ),
        remediation:
            "Restrict imported keypairs to localnet / mainnet-fork; use freshly-generated keys \
             on devnet."
                .into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::solana::persona::fund::DefaultFundingBackend;
    use crate::commands::solana::persona::keygen::{KeypairStore, OsRngKeypairProvider};
    use tempfile::TempDir;

    fn store_in(dir: &TempDir) -> Arc<PersonaStore> {
        let root = dir.path();
        let keypairs = KeypairStore::new(root.join("kp"), Box::new(OsRngKeypairProvider));
        Arc::new(PersonaStore::new(
            root.to_path_buf(),
            keypairs,
            Box::new(DefaultFundingBackend::new()),
        ))
    }

    fn persona(name: &str, cluster: ClusterKind, pubkey: &str, note: Option<&str>) -> Persona {
        Persona {
            name: name.to_string(),
            role: crate::commands::solana::persona::roles::PersonaRole::Custom,
            cluster,
            pubkey: pubkey.to_string(),
            keypair_path: String::new(),
            created_at_ms: 0,
            seed: Default::default(),
            note: note.map(String::from),
        }
    }

    #[test]
    fn empty_store_emits_no_warnings() {
        let dir = TempDir::new().unwrap();
        let store = store_in(&dir);
        let report = check_scope(&store).unwrap();
        assert!(report.warnings.is_empty());
        assert!(!report.blocks_deploy);
    }

    #[test]
    fn mainnet_label_on_devnet_triggers_warning() {
        let warning = label_warning(&persona(
            "mainnet-deployer",
            ClusterKind::Devnet,
            "PUBKEY",
            None,
        ))
        .expect("should flag");
        assert_eq!(warning.kind, ScopeWarningKind::MainnetLabelOnNonMainnet);
        assert_eq!(warning.severity, SecretSeverity::High);
    }

    #[test]
    fn imported_note_on_fork_raises_medium_warning() {
        let warning = imported_on_fork_warning(&persona(
            "whale",
            ClusterKind::MainnetFork,
            "PUBKEY",
            Some("imported from prod"),
        ))
        .expect("should flag");
        assert_eq!(
            warning.kind,
            ScopeWarningKind::SuspectedRealKeyOnForkOrDevnet
        );
    }

    #[test]
    fn mainnet_persona_is_not_label_flagged() {
        assert!(label_warning(&persona(
            "deploy",
            ClusterKind::Mainnet,
            "PUBKEY",
            Some("mainnet authority"),
        ))
        .is_none());
    }
}
