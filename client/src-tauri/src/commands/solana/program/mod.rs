//! Program build / upgrade-safety / deploy / Squads / verified-build.
//!
//! Phase 5 surface. Each submodule is independent and trait-mockable so
//! integration tests can compose end-to-end deploys against scripted
//! sub-processes and scripted RPC transports.

pub mod build;
pub mod deploy;
pub mod squads;
pub mod upgrade_safety;
pub mod verified_build;

pub use build::{
    build, BuildKind, BuildProfile, BuildReport, BuildRequest, BuildRunner, BuiltArtifact,
    SystemBuildRunner,
};
pub use deploy::{
    deploy, rollback, ArchiveRecord, BufferWriteOutcome, DeployAuthority, DeployResult,
    DeployRunner, DeployServices, DeploySpec, DirectDeployOutcome, PostDeployOptions,
    RollbackRequest, RollbackResult, SystemDeployRunner,
};
pub use squads::{
    synthesize as squads_synthesize, SquadsProposalDescriptor, SquadsProposalRequest,
    UpgradeInstruction, UpgradeInstructionAccount, DEFAULT_VAULT_INDEX, SQUADS_V4_PROGRAM_ID,
};
pub use upgrade_safety::{
    check as upgrade_safety_check, AuthorityCheck, AuthorityCheckOutcome, LayoutCheck, SizeCheck,
    SizeCheckOutcome, UpgradeSafetyReport, UpgradeSafetyRequest, UpgradeSafetyVerdict,
    BPF_UPGRADEABLE_LOADER, PROGRAM_DATA_MAX_BYTES,
};
pub use verified_build::{
    submit as verified_build_submit, SystemVerifiedBuildRunner, VerifiedBuildRequest,
    VerifiedBuildResult, VerifiedBuildRunner,
};
