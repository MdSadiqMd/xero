//! Process-local cost ledger.
//!
//! Each tx the workbench sends (or explains with confirmed metadata)
//! adds a `TxCostRecord`. The ledger keeps a bounded in-memory ring —
//! we don't persist across restarts by design so a fresh workbench
//! starts at zero cost. Rolling into disk is a Phase 10 follow-up
//! (the plan file's "indexer storage" note calls it out).

use std::collections::VecDeque;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::commands::solana::cluster::ClusterKind;

const DEFAULT_CAPACITY: usize = 4_096;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TxCostRecord {
    pub cluster: ClusterKind,
    pub signature: String,
    pub lamports_fee: u64,
    pub priority_fee_lamports: u64,
    pub compute_units_consumed: u64,
    pub rent_lamports: u64,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalCostSummary {
    pub tx_count: u64,
    pub lamports_spent: u64,
    pub compute_units_used: u64,
    pub rent_locked_lamports: u64,
    pub by_cluster: Vec<ClusterCostBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClusterCostBreakdown {
    pub cluster: ClusterKind,
    pub tx_count: u64,
    pub lamports_spent: u64,
    pub compute_units_used: u64,
    pub rent_locked_lamports: u64,
}

#[derive(Debug)]
pub struct LocalCostLedger {
    records: Mutex<VecDeque<TxCostRecord>>,
    capacity: usize,
}

impl LocalCostLedger {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            records: Mutex::new(VecDeque::with_capacity(capacity.max(1))),
            capacity: capacity.max(1),
        }
    }

    /// Append a record. Oldest entry is evicted if the capacity cap is
    /// hit.
    pub fn record(&self, record: TxCostRecord) {
        let mut guard = self.records.lock().expect("cost ledger mutex poisoned");
        if guard.len() == self.capacity {
            guard.pop_front();
        }
        guard.push_back(record);
    }

    pub fn len(&self) -> usize {
        self.records
            .lock()
            .map(|g| g.len())
            .unwrap_or_default()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        if let Ok(mut guard) = self.records.lock() {
            guard.clear();
        }
    }

    /// Summarise recorded activity limited to `clusters` and, when
    /// `window_s` is `Some`, only entries newer than that many
    /// seconds. All arithmetic is saturating to keep us on `u64`.
    pub fn summary(&self, clusters: &[ClusterKind], window_s: Option<u64>) -> LocalCostSummary {
        let records = self
            .records
            .lock()
            .map(|g| g.iter().cloned().collect::<Vec<_>>())
            .unwrap_or_default();

        let cutoff = window_s.map(|w| now_ms().saturating_sub(w.saturating_mul(1_000)));

        let mut tx_count = 0u64;
        let mut lamports_spent = 0u64;
        let mut compute_units_used = 0u64;
        let mut rent_locked_lamports = 0u64;
        let mut per_cluster: std::collections::BTreeMap<ClusterKind, ClusterCostBreakdown> =
            std::collections::BTreeMap::new();
        for cluster in clusters {
            per_cluster.insert(
                *cluster,
                ClusterCostBreakdown {
                    cluster: *cluster,
                    tx_count: 0,
                    lamports_spent: 0,
                    compute_units_used: 0,
                    rent_locked_lamports: 0,
                },
            );
        }

        for record in records {
            if !clusters.contains(&record.cluster) {
                continue;
            }
            if let Some(cut) = cutoff {
                if record.timestamp_ms < cut {
                    continue;
                }
            }
            let spend = record
                .lamports_fee
                .saturating_add(record.priority_fee_lamports);
            tx_count = tx_count.saturating_add(1);
            lamports_spent = lamports_spent.saturating_add(spend);
            compute_units_used =
                compute_units_used.saturating_add(record.compute_units_consumed);
            rent_locked_lamports = rent_locked_lamports.saturating_add(record.rent_lamports);
            if let Some(bucket) = per_cluster.get_mut(&record.cluster) {
                bucket.tx_count = bucket.tx_count.saturating_add(1);
                bucket.lamports_spent = bucket.lamports_spent.saturating_add(spend);
                bucket.compute_units_used = bucket
                    .compute_units_used
                    .saturating_add(record.compute_units_consumed);
                bucket.rent_locked_lamports = bucket
                    .rent_locked_lamports
                    .saturating_add(record.rent_lamports);
            }
        }

        LocalCostSummary {
            tx_count,
            lamports_spent,
            compute_units_used,
            rent_locked_lamports,
            by_cluster: per_cluster.into_values().collect(),
        }
    }
}

impl Default for LocalCostLedger {
    fn default() -> Self {
        Self::new()
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_ignores_clusters_not_requested() {
        let ledger = LocalCostLedger::new();
        ledger.record(TxCostRecord {
            cluster: ClusterKind::Mainnet,
            signature: "a".into(),
            lamports_fee: 1_000,
            priority_fee_lamports: 200,
            compute_units_consumed: 50_000,
            rent_lamports: 0,
            timestamp_ms: now_ms(),
        });
        ledger.record(TxCostRecord {
            cluster: ClusterKind::Devnet,
            signature: "b".into(),
            lamports_fee: 500,
            priority_fee_lamports: 0,
            compute_units_consumed: 10_000,
            rent_lamports: 0,
            timestamp_ms: now_ms(),
        });
        let summary = ledger.summary(&[ClusterKind::Mainnet], None);
        assert_eq!(summary.tx_count, 1);
        assert_eq!(summary.lamports_spent, 1_200);
    }

    #[test]
    fn window_cutoff_excludes_old_entries() {
        let ledger = LocalCostLedger::new();
        ledger.record(TxCostRecord {
            cluster: ClusterKind::Mainnet,
            signature: "old".into(),
            lamports_fee: 1_000,
            priority_fee_lamports: 0,
            compute_units_consumed: 0,
            rent_lamports: 0,
            timestamp_ms: 1,
        });
        let summary = ledger.summary(&[ClusterKind::Mainnet], Some(60));
        assert_eq!(summary.tx_count, 0);
    }

    #[test]
    fn capacity_evicts_oldest_records() {
        let ledger = LocalCostLedger::with_capacity(2);
        for i in 0..3 {
            ledger.record(TxCostRecord {
                cluster: ClusterKind::Devnet,
                signature: format!("s-{i}"),
                lamports_fee: 100,
                priority_fee_lamports: 0,
                compute_units_consumed: 0,
                rent_lamports: 0,
                timestamp_ms: now_ms(),
            });
        }
        assert_eq!(ledger.len(), 2);
    }

    #[test]
    fn summary_rolls_per_cluster_breakdowns() {
        let ledger = LocalCostLedger::new();
        ledger.record(TxCostRecord {
            cluster: ClusterKind::Mainnet,
            signature: "a".into(),
            lamports_fee: 100,
            priority_fee_lamports: 0,
            compute_units_consumed: 1,
            rent_lamports: 0,
            timestamp_ms: now_ms(),
        });
        ledger.record(TxCostRecord {
            cluster: ClusterKind::Devnet,
            signature: "b".into(),
            lamports_fee: 200,
            priority_fee_lamports: 50,
            compute_units_consumed: 2,
            rent_lamports: 500,
            timestamp_ms: now_ms(),
        });
        let summary =
            ledger.summary(&[ClusterKind::Mainnet, ClusterKind::Devnet], None);
        let mainnet = summary
            .by_cluster
            .iter()
            .find(|b| b.cluster == ClusterKind::Mainnet)
            .unwrap();
        let devnet = summary
            .by_cluster
            .iter()
            .find(|b| b.cluster == ClusterKind::Devnet)
            .unwrap();
        assert_eq!(mainnet.tx_count, 1);
        assert_eq!(devnet.rent_locked_lamports, 500);
        assert_eq!(devnet.lamports_spent, 250);
    }
}
