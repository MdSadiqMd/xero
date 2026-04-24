//! Phase 9 — secrets scanning and scope/hygiene checks.
//!
//! Two independent capabilities sharing a small data model:
//!
//! * `scan` walks a project tree looking for Solana-specific secrets:
//!   committed `id.json` keypair files, Helius/Triton/QuickNode/Alchemy
//!   RPC API keys, Privy app secrets, and Jito tip-account constants.
//!   Every finding carries a `severity` and `ruleId` so the frontend
//!   and the deploy gate can make policy decisions off the same shape.
//!
//! * `scope_check` walks the `PersonaStore` and flags mismatches —
//!   the classic case is a keypair marked "mainnet authority" that has
//!   been loaded into a devnet persona slot (or vice versa). This is
//!   the companion to the scanner: we scanned the *tree* for on-disk
//!   leaks, this scans the *running state* for cross-cluster reuse.
//!
//! The deploy gate in `program::deploy::deploy` calls `scan` against
//! the project that contains the `.so` and blocks when a `Critical`
//! finding comes back — that's the acceptance criteria bullet.

pub mod patterns;
pub mod scan;
pub mod scope;

pub use patterns::{builtin_patterns, SecretPattern, SecretPatternKind};
pub use scan::{scan as scan_project, ScanRequest, SecretFinding, SecretScanReport};
pub use scope::{check_scope, ScopeCheckReport, ScopeWarning, ScopeWarningKind};

use serde::{Deserialize, Serialize};

/// Severity ladder for every Phase 9 secret / scope / drift finding.
/// Identical shape to `audit::FindingSeverity` so frontend components
/// can share chip colours.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SecretSeverity {
    /// Blocks deploy. A committed mainnet keypair lives here.
    Critical,
    /// Strongly discouraged. A paid-provider API key is in the tree.
    High,
    /// Warning only — e.g. a Jito tip-account constant used without an
    /// accompanying policy note.
    Medium,
    /// Informational — patterns we surface so the user can confirm
    /// they're intentional.
    Low,
}

impl SecretSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            SecretSeverity::Critical => "critical",
            SecretSeverity::High => "high",
            SecretSeverity::Medium => "medium",
            SecretSeverity::Low => "low",
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            SecretSeverity::Critical => 0,
            SecretSeverity::High => 1,
            SecretSeverity::Medium => 2,
            SecretSeverity::Low => 3,
        }
    }
}
