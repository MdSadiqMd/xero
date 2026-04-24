//! Autonomous-runtime wrapper for the Solana workbench.
//!
//! Phase 1 only exposes two tools: `solana_cluster` (start/stop/status
//! plus snapshot lifecycle) and `solana_logs` (fetch the tail of the
//! supervisor's log ring). Later phases slot in alongside.
//!
//! Mirrors `browser.rs`: the request enum is JSON-in/JSON-out, the
//! executor trait is trivially mockable, and the production wiring
//! bridges to the Tauri `SolanaState` the UI uses.

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::commands::solana::{
    idl::{self, codama::CodamaGenerationRequest},
    pda, AltCandidate, AltCreateResult, AltExtendResult, AltResolveReport, BumpAnalysis,
    ClusterHandle, ClusterKind, ClusterPda, ClusterStatus, CodamaGenerationReport, CodamaTarget,
    DerivedAddress, DriftReport, EndpointHealth, ExplainRequest, FeeEstimate, Idl, IdlPublishMode,
    IdlPublishReport, IdlPublishRequest, IdlRegistry, IdlSubscriptionToken, KnownProgramLookup,
    PdaSite, ResolveArgs, SamplePercentile, SeedPart, SendRequest, SimulateRequest,
    SimulationResult, SnapshotMeta, SolanaState, StartOpts, TxPipeline, TxPlan, TxResult, TxSpec,
};
use crate::commands::{CommandError, CommandResult};

pub const AUTONOMOUS_TOOL_SOLANA_CLUSTER: &str = "solana_cluster";
pub const AUTONOMOUS_TOOL_SOLANA_LOGS: &str = "solana_logs";
pub const AUTONOMOUS_TOOL_SOLANA_TX: &str = "solana_tx";
pub const AUTONOMOUS_TOOL_SOLANA_SIMULATE: &str = "solana_simulate";
pub const AUTONOMOUS_TOOL_SOLANA_EXPLAIN: &str = "solana_explain";
pub const AUTONOMOUS_TOOL_SOLANA_ALT: &str = "solana_alt";
pub const AUTONOMOUS_TOOL_SOLANA_IDL: &str = "solana_idl";
pub const AUTONOMOUS_TOOL_SOLANA_CODAMA: &str = "solana_codama";
pub const AUTONOMOUS_TOOL_SOLANA_PDA: &str = "solana_pda";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum AutonomousSolanaClusterAction {
    List,
    Start {
        kind: ClusterKind,
        #[serde(default)]
        opts: StartOpts,
    },
    Stop,
    Status,
    SnapshotList,
    SnapshotCreate {
        label: String,
        accounts: Vec<String>,
        #[serde(default)]
        cluster: Option<ClusterKind>,
        #[serde(default)]
        rpc_url: Option<String>,
    },
    SnapshotDelete {
        id: String,
    },
    RpcHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutonomousSolanaClusterRequest {
    #[serde(flatten)]
    pub action: AutonomousSolanaClusterAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousSolanaLogsRequest {
    /// Reserved for future decoded-log filter support; Phase 1 returns the
    /// process stderr tail from the validator supervisor.
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousSolanaOutput {
    pub action: String,
    /// JSON-serialized response held as a string so the overall tool
    /// output can stay `Eq`-derivable.
    pub value_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum AutonomousSolanaTxAction {
    Build {
        spec: TxSpec,
    },
    Send {
        request: SendRequest,
    },
    PriorityFee {
        cluster: ClusterKind,
        #[serde(default)]
        program_ids: Vec<String>,
        #[serde(default = "default_percentile")]
        target: SamplePercentile,
        #[serde(default)]
        rpc_url: Option<String>,
    },
    Cpi {
        program_id: String,
        instruction: String,
        #[serde(default)]
        args: ResolveArgs,
    },
}

fn default_percentile() -> SamplePercentile {
    SamplePercentile::Median
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutonomousSolanaTxRequest {
    #[serde(flatten)]
    pub action: AutonomousSolanaTxAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousSolanaSimulateRequest {
    pub request: SimulateRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousSolanaExplainRequest {
    pub request: ExplainRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum AutonomousSolanaAltAction {
    Create {
        cluster: ClusterKind,
        authority_persona: String,
        #[serde(default)]
        rpc_url: Option<String>,
    },
    Extend {
        cluster: ClusterKind,
        alt: String,
        addresses: Vec<String>,
        authority_persona: String,
        #[serde(default)]
        rpc_url: Option<String>,
    },
    Resolve {
        addresses: Vec<String>,
        #[serde(default)]
        candidates: Vec<AltCandidate>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutonomousSolanaAltRequest {
    #[serde(flatten)]
    pub action: AutonomousSolanaAltAction,
}

// ---------- IDL / Codama / PDA requests (Phase 4) --------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum AutonomousSolanaIdlAction {
    /// Read a local IDL file into the registry.
    Load { path: String },
    /// Fetch from chain via the injected RPC transport.
    Fetch {
        program_id: String,
        cluster: ClusterKind,
        #[serde(default)]
        rpc_url: Option<String>,
    },
    /// Return the most-recently-cached IDL for a program.
    Get {
        program_id: String,
        #[serde(default)]
        cluster: Option<ClusterKind>,
    },
    /// Start watching a local IDL file. Returns a subscription token.
    Watch { path: String },
    /// Stop a previously-started watch.
    Unwatch { token: IdlSubscriptionToken },
    /// Classify local-vs-on-chain drift.
    Drift {
        program_id: String,
        cluster: ClusterKind,
        local_path: String,
        #[serde(default)]
        rpc_url: Option<String>,
    },
    /// Run `anchor idl init/upgrade`. Caller provides the authority
    /// keypair path explicitly (the runtime doesn't expand personas to
    /// keypairs to keep the agent surface local-first).
    Publish {
        program_id: String,
        cluster: ClusterKind,
        idl_path: String,
        authority_keypair_path: String,
        rpc_url: String,
        mode: IdlPublishMode,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutonomousSolanaIdlRequest {
    #[serde(flatten)]
    pub action: AutonomousSolanaIdlAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutonomousSolanaCodamaRequest {
    pub idl_path: String,
    pub targets: Vec<CodamaTarget>,
    pub output_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum AutonomousSolanaPdaAction {
    Derive {
        program_id: String,
        seeds: Vec<SeedPart>,
        #[serde(default)]
        bump: Option<u8>,
    },
    Scan {
        project_root: String,
    },
    Predict {
        program_id: String,
        seeds: Vec<SeedPart>,
        clusters: Vec<ClusterKind>,
    },
    AnalyseBump {
        program_id: String,
        seeds: Vec<SeedPart>,
        #[serde(default)]
        bump: Option<u8>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AutonomousSolanaPdaRequest {
    #[serde(flatten)]
    pub action: AutonomousSolanaPdaAction,
}

pub trait SolanaExecutor: Send + Sync + std::fmt::Debug {
    fn cluster(
        &self,
        request: AutonomousSolanaClusterRequest,
    ) -> CommandResult<AutonomousSolanaOutput>;

    fn logs(&self, request: AutonomousSolanaLogsRequest) -> CommandResult<AutonomousSolanaOutput>;

    fn tx(&self, request: AutonomousSolanaTxRequest) -> CommandResult<AutonomousSolanaOutput>;

    fn simulate(
        &self,
        request: AutonomousSolanaSimulateRequest,
    ) -> CommandResult<AutonomousSolanaOutput>;

    fn explain(
        &self,
        request: AutonomousSolanaExplainRequest,
    ) -> CommandResult<AutonomousSolanaOutput>;

    fn alt(&self, request: AutonomousSolanaAltRequest) -> CommandResult<AutonomousSolanaOutput>;

    fn idl(&self, request: AutonomousSolanaIdlRequest) -> CommandResult<AutonomousSolanaOutput>;

    fn codama(
        &self,
        request: AutonomousSolanaCodamaRequest,
    ) -> CommandResult<AutonomousSolanaOutput>;

    fn pda(&self, request: AutonomousSolanaPdaRequest) -> CommandResult<AutonomousSolanaOutput>;
}

/// Executor that dispatches against a live `SolanaState`. Safe to clone
/// because it just holds an `Arc<SolanaState>`-ish bundle of the
/// supervisor and snapshot store.
#[derive(Debug, Clone)]
pub struct StateSolanaExecutor {
    inner: Arc<StateInner>,
}

#[derive(Debug)]
struct StateInner {
    supervisor: Arc<crate::commands::solana::ValidatorSupervisor>,
    router: Arc<crate::commands::solana::RpcRouter>,
    snapshots: Arc<crate::commands::solana::SnapshotStore>,
    tx_pipeline: Arc<TxPipeline>,
    idl_registry: Arc<IdlRegistry>,
}

impl StateSolanaExecutor {
    pub fn from_state(state: &SolanaState) -> Self {
        Self {
            inner: Arc::new(StateInner {
                supervisor: state.supervisor(),
                router: state.rpc_router(),
                snapshots: state.snapshots(),
                tx_pipeline: state.tx_pipeline(),
                idl_registry: state.idl_registry(),
            }),
        }
    }

    fn resolve_rpc_url(&self, cluster: ClusterKind) -> Option<String> {
        let status = self.inner.supervisor.status();
        if status.kind == Some(cluster) {
            if let Some(url) = status.rpc_url.clone() {
                return Some(url);
            }
        }
        self.inner.router.pick_healthy(cluster).map(|e| e.url)
    }
}

impl SolanaExecutor for StateSolanaExecutor {
    fn cluster(
        &self,
        request: AutonomousSolanaClusterRequest,
    ) -> CommandResult<AutonomousSolanaOutput> {
        let (action_name, value) = match request.action {
            AutonomousSolanaClusterAction::List => {
                let descriptors = crate::commands::solana::cluster_descriptors();
                (
                    "list".to_string(),
                    serde_json::to_value(descriptors).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaClusterAction::Start { kind, opts } => {
                let (handle, _events) = self.inner.supervisor.start(kind, opts)?;
                let value =
                    serde_json::to_value::<ClusterHandle>(handle).unwrap_or(JsonValue::Null);
                ("start".to_string(), value)
            }
            AutonomousSolanaClusterAction::Stop => {
                let _events = self.inner.supervisor.stop()?;
                ("stop".to_string(), JsonValue::Null)
            }
            AutonomousSolanaClusterAction::Status => {
                let status = self.inner.supervisor.status();
                let value =
                    serde_json::to_value::<ClusterStatus>(status).unwrap_or(JsonValue::Null);
                ("status".to_string(), value)
            }
            AutonomousSolanaClusterAction::SnapshotList => {
                let metas = self.inner.snapshots.list()?;
                let value =
                    serde_json::to_value::<Vec<SnapshotMeta>>(metas).unwrap_or(JsonValue::Null);
                ("snapshot_list".to_string(), value)
            }
            AutonomousSolanaClusterAction::SnapshotCreate {
                label,
                accounts,
                cluster,
                rpc_url,
            } => {
                if accounts.is_empty() {
                    return Err(CommandError::user_fixable(
                        "solana_snapshot_accounts_empty",
                        "Snapshot requires at least one account pubkey.",
                    ));
                }
                let status = self.inner.supervisor.status();
                let cluster_label = cluster
                    .map(|c| c.as_str().to_string())
                    .or_else(|| status.kind.map(|c| c.as_str().to_string()))
                    .unwrap_or_else(|| "unknown".to_string());
                let rpc_url = rpc_url
                    .or(status.rpc_url.clone())
                    .or_else(|| {
                        cluster.and_then(|c| self.inner.router.pick_healthy(c).map(|s| s.url))
                    })
                    .ok_or_else(|| {
                        CommandError::user_fixable(
                            "solana_snapshot_no_rpc_url",
                            "Provide rpcUrl or start a cluster before creating a snapshot.",
                        )
                    })?;
                let meta =
                    self.inner
                        .snapshots
                        .create(&label, &cluster_label, &rpc_url, &accounts)?;
                let value = serde_json::to_value::<SnapshotMeta>(meta).unwrap_or(JsonValue::Null);
                ("snapshot_create".to_string(), value)
            }
            AutonomousSolanaClusterAction::SnapshotDelete { id } => {
                self.inner.snapshots.delete(&id)?;
                ("snapshot_delete".to_string(), JsonValue::Null)
            }
            AutonomousSolanaClusterAction::RpcHealth => {
                let snap = self.inner.router.refresh_health();
                let value =
                    serde_json::to_value::<Vec<EndpointHealth>>(snap).unwrap_or(JsonValue::Null);
                ("rpc_health".to_string(), value)
            }
        };

        let value_json = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
        Ok(AutonomousSolanaOutput {
            action: action_name,
            value_json,
        })
    }

    fn logs(&self, _request: AutonomousSolanaLogsRequest) -> CommandResult<AutonomousSolanaOutput> {
        // Phase 1 surface: caller gets whatever status the supervisor
        // currently reports, so the agent can at least tell whether a
        // cluster is running. Phase 7 will wire the full validator log
        // bus.
        let status = self.inner.supervisor.status();
        let value = serde_json::to_value(status).unwrap_or(JsonValue::Null);
        let value_json = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
        Ok(AutonomousSolanaOutput {
            action: "status".to_string(),
            value_json,
        })
    }

    fn tx(&self, request: AutonomousSolanaTxRequest) -> CommandResult<AutonomousSolanaOutput> {
        let (action_name, value) = match request.action {
            AutonomousSolanaTxAction::Build { spec } => {
                let plan = self.inner.tx_pipeline.build(spec)?;
                (
                    "build".to_string(),
                    serde_json::to_value::<TxPlan>(plan).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaTxAction::Send { request } => {
                let result = self.inner.tx_pipeline.send(request)?;
                (
                    "send".to_string(),
                    serde_json::to_value::<TxResult>(result).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaTxAction::PriorityFee {
                cluster,
                program_ids,
                target,
                rpc_url,
            } => {
                let estimate = self.inner.tx_pipeline.priority_fee_estimate(
                    cluster,
                    &program_ids,
                    target,
                    rpc_url,
                )?;
                (
                    "priority_fee".to_string(),
                    serde_json::to_value::<FeeEstimate>(estimate).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaTxAction::Cpi {
                program_id,
                instruction,
                args,
            } => {
                let lookup = self
                    .inner
                    .tx_pipeline
                    .resolve_cpi(&program_id, &instruction, &args);
                (
                    "cpi".to_string(),
                    serde_json::to_value::<KnownProgramLookup>(lookup).unwrap_or(JsonValue::Null),
                )
            }
        };
        let value_json = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
        Ok(AutonomousSolanaOutput {
            action: action_name,
            value_json,
        })
    }

    fn simulate(
        &self,
        request: AutonomousSolanaSimulateRequest,
    ) -> CommandResult<AutonomousSolanaOutput> {
        let result = self.inner.tx_pipeline.simulate(request.request)?;
        let value = serde_json::to_value::<SimulationResult>(result).unwrap_or(JsonValue::Null);
        let value_json = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
        Ok(AutonomousSolanaOutput {
            action: "simulate".to_string(),
            value_json,
        })
    }

    fn explain(
        &self,
        request: AutonomousSolanaExplainRequest,
    ) -> CommandResult<AutonomousSolanaOutput> {
        let result = self.inner.tx_pipeline.explain(request.request)?;
        let value = serde_json::to_value::<TxResult>(result).unwrap_or(JsonValue::Null);
        let value_json = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
        Ok(AutonomousSolanaOutput {
            action: "explain".to_string(),
            value_json,
        })
    }

    fn alt(&self, request: AutonomousSolanaAltRequest) -> CommandResult<AutonomousSolanaOutput> {
        let (action_name, value) = match request.action {
            AutonomousSolanaAltAction::Create {
                cluster,
                authority_persona,
                rpc_url,
            } => {
                let result =
                    self.inner
                        .tx_pipeline
                        .alt_create(cluster, &authority_persona, rpc_url)?;
                (
                    "alt_create".to_string(),
                    serde_json::to_value::<AltCreateResult>(result).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaAltAction::Extend {
                cluster,
                alt,
                addresses,
                authority_persona,
                rpc_url,
            } => {
                let result = self.inner.tx_pipeline.alt_extend(
                    cluster,
                    &alt,
                    &addresses,
                    &authority_persona,
                    rpc_url,
                )?;
                (
                    "alt_extend".to_string(),
                    serde_json::to_value::<AltExtendResult>(result).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaAltAction::Resolve {
                addresses,
                candidates,
            } => {
                let report = self.inner.tx_pipeline.alt_suggest(&addresses, &candidates);
                (
                    "alt_resolve".to_string(),
                    serde_json::to_value::<AltResolveReport>(report).unwrap_or(JsonValue::Null),
                )
            }
        };
        let value_json = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
        Ok(AutonomousSolanaOutput {
            action: action_name,
            value_json,
        })
    }

    fn idl(&self, request: AutonomousSolanaIdlRequest) -> CommandResult<AutonomousSolanaOutput> {
        let (action_name, value) = match request.action {
            AutonomousSolanaIdlAction::Load { path } => {
                let idl = self.inner.idl_registry.load_file(Path::new(&path))?;
                (
                    "load".to_string(),
                    serde_json::to_value::<Idl>(idl).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaIdlAction::Fetch {
                program_id,
                cluster,
                rpc_url,
            } => {
                let rpc_url = rpc_url
                    .or_else(|| self.resolve_rpc_url(cluster))
                    .ok_or_else(|| {
                        CommandError::user_fixable(
                            "solana_idl_no_rpc",
                            "No RPC URL available — start a cluster or provide rpcUrl.",
                        )
                    })?;
                let idl = self
                    .inner
                    .idl_registry
                    .fetch_on_chain(cluster, &rpc_url, &program_id)?;
                (
                    "fetch".to_string(),
                    serde_json::to_value::<Option<Idl>>(idl).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaIdlAction::Get {
                program_id,
                cluster,
            } => {
                let idl = self.inner.idl_registry.get_cached(&program_id, cluster);
                (
                    "get".to_string(),
                    serde_json::to_value::<Option<Idl>>(idl).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaIdlAction::Watch { path } => {
                let token = self.inner.idl_registry.watch_path(Path::new(&path))?;
                (
                    "watch".to_string(),
                    serde_json::to_value::<IdlSubscriptionToken>(token).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaIdlAction::Unwatch { token } => {
                let ok = self.inner.idl_registry.unwatch(&token)?;
                ("unwatch".to_string(), JsonValue::Bool(ok))
            }
            AutonomousSolanaIdlAction::Drift {
                program_id,
                cluster,
                local_path,
                rpc_url,
            } => {
                let local = self.inner.idl_registry.load_file(Path::new(&local_path))?;
                let rpc_url = rpc_url
                    .or_else(|| self.resolve_rpc_url(cluster))
                    .ok_or_else(|| {
                        CommandError::user_fixable(
                            "solana_idl_no_rpc",
                            "No RPC URL available — start a cluster or provide rpcUrl.",
                        )
                    })?;
                let chain =
                    self.inner
                        .idl_registry
                        .fetch_on_chain(cluster, &rpc_url, &program_id)?;
                let report = idl::drift::classify(&local, chain.as_ref());
                (
                    "drift".to_string(),
                    serde_json::to_value::<DriftReport>(report).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaIdlAction::Publish {
                program_id,
                cluster,
                idl_path,
                authority_keypair_path,
                rpc_url,
                mode,
            } => {
                let runner = idl::publish::SystemAnchorIdlRunner::new();
                let sink = idl::publish::NullProgressSink;
                let report = idl::publish::publish(
                    &runner,
                    &sink,
                    &IdlPublishRequest {
                        program_id,
                        cluster,
                        idl_path,
                        authority_keypair_path,
                        rpc_url,
                        mode,
                    },
                )?;
                (
                    "publish".to_string(),
                    serde_json::to_value::<IdlPublishReport>(report).unwrap_or(JsonValue::Null),
                )
            }
        };
        let value_json = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
        Ok(AutonomousSolanaOutput {
            action: action_name,
            value_json,
        })
    }

    fn codama(
        &self,
        request: AutonomousSolanaCodamaRequest,
    ) -> CommandResult<AutonomousSolanaOutput> {
        let runner = idl::codama::SystemCodamaRunner::new();
        let report = idl::codama::generate(
            &runner,
            &CodamaGenerationRequest {
                idl_path: request.idl_path,
                targets: request.targets,
                output_dir: request.output_dir,
            },
        )?;
        let value =
            serde_json::to_value::<CodamaGenerationReport>(report).unwrap_or(JsonValue::Null);
        let value_json = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
        Ok(AutonomousSolanaOutput {
            action: "generate".to_string(),
            value_json,
        })
    }

    fn pda(&self, request: AutonomousSolanaPdaRequest) -> CommandResult<AutonomousSolanaOutput> {
        let (action_name, value) = match request.action {
            AutonomousSolanaPdaAction::Derive {
                program_id,
                seeds,
                bump,
            } => {
                let derived = match bump {
                    Some(b) => pda::create_program_address(&program_id, &seeds, b)?,
                    None => pda::find_program_address(&program_id, &seeds)?,
                };
                (
                    "derive".to_string(),
                    serde_json::to_value::<DerivedAddress>(derived).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaPdaAction::Scan { project_root } => {
                let sites = pda::scan(Path::new(&project_root))?;
                (
                    "scan".to_string(),
                    serde_json::to_value::<Vec<PdaSite>>(sites).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaPdaAction::Predict {
                program_id,
                seeds,
                clusters,
            } => {
                let predictions = pda::predict(&program_id, &seeds, &clusters)?;
                (
                    "predict".to_string(),
                    serde_json::to_value::<Vec<ClusterPda>>(predictions).unwrap_or(JsonValue::Null),
                )
            }
            AutonomousSolanaPdaAction::AnalyseBump {
                program_id,
                seeds,
                bump,
            } => {
                let analysis = pda::analyse_bump(&program_id, &seeds, bump)?;
                (
                    "analyse_bump".to_string(),
                    serde_json::to_value::<BumpAnalysis>(analysis).unwrap_or(JsonValue::Null),
                )
            }
        };
        let value_json = serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string());
        Ok(AutonomousSolanaOutput {
            action: action_name,
            value_json,
        })
    }
}

/// No-op executor. Returns `policy_denied` for every action so environments
/// without a registered `SolanaState` (unit tests, autonomous runtime built
/// off a bare repo) still surface a useful error.
#[derive(Debug, Default)]
pub struct UnavailableSolanaExecutor;

impl SolanaExecutor for UnavailableSolanaExecutor {
    fn cluster(
        &self,
        _request: AutonomousSolanaClusterRequest,
    ) -> CommandResult<AutonomousSolanaOutput> {
        Err(CommandError::policy_denied(
            "Solana actions require the desktop runtime; no SolanaState is wired.",
        ))
    }

    fn logs(&self, _request: AutonomousSolanaLogsRequest) -> CommandResult<AutonomousSolanaOutput> {
        Err(CommandError::policy_denied(
            "Solana log streaming requires the desktop runtime; no SolanaState is wired.",
        ))
    }

    fn tx(&self, _request: AutonomousSolanaTxRequest) -> CommandResult<AutonomousSolanaOutput> {
        Err(CommandError::policy_denied(
            "Solana tx pipeline requires the desktop runtime; no SolanaState is wired.",
        ))
    }

    fn simulate(
        &self,
        _request: AutonomousSolanaSimulateRequest,
    ) -> CommandResult<AutonomousSolanaOutput> {
        Err(CommandError::policy_denied(
            "Solana simulate requires the desktop runtime; no SolanaState is wired.",
        ))
    }

    fn explain(
        &self,
        _request: AutonomousSolanaExplainRequest,
    ) -> CommandResult<AutonomousSolanaOutput> {
        Err(CommandError::policy_denied(
            "Solana explain requires the desktop runtime; no SolanaState is wired.",
        ))
    }

    fn alt(&self, _request: AutonomousSolanaAltRequest) -> CommandResult<AutonomousSolanaOutput> {
        Err(CommandError::policy_denied(
            "Solana ALT actions require the desktop runtime; no SolanaState is wired.",
        ))
    }

    fn idl(&self, _request: AutonomousSolanaIdlRequest) -> CommandResult<AutonomousSolanaOutput> {
        Err(CommandError::policy_denied(
            "Solana IDL actions require the desktop runtime; no SolanaState is wired.",
        ))
    }

    fn codama(
        &self,
        _request: AutonomousSolanaCodamaRequest,
    ) -> CommandResult<AutonomousSolanaOutput> {
        Err(CommandError::policy_denied(
            "Solana Codama codegen requires the desktop runtime; no SolanaState is wired.",
        ))
    }

    fn pda(&self, _request: AutonomousSolanaPdaRequest) -> CommandResult<AutonomousSolanaOutput> {
        Err(CommandError::policy_denied(
            "Solana PDA actions require the desktop runtime; no SolanaState is wired.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_executor_denies_every_tool() {
        let exec = UnavailableSolanaExecutor;
        let err = exec
            .cluster(AutonomousSolanaClusterRequest {
                action: AutonomousSolanaClusterAction::Status,
            })
            .unwrap_err();
        assert_eq!(err.class, crate::commands::CommandErrorClass::PolicyDenied);

        let err = exec
            .logs(AutonomousSolanaLogsRequest { limit: Some(10) })
            .unwrap_err();
        assert_eq!(err.class, crate::commands::CommandErrorClass::PolicyDenied);
    }

    #[test]
    fn state_executor_list_returns_cluster_descriptors() {
        let state = SolanaState::default();
        let exec = StateSolanaExecutor::from_state(&state);
        let out = exec
            .cluster(AutonomousSolanaClusterRequest {
                action: AutonomousSolanaClusterAction::List,
            })
            .unwrap();
        assert_eq!(out.action, "list");
        let parsed: serde_json::Value = serde_json::from_str(&out.value_json).unwrap();
        let descriptors = parsed.as_array().unwrap();
        assert_eq!(descriptors.len(), 4);
    }

    #[test]
    fn state_executor_status_when_idle_reports_not_running() {
        let state = SolanaState::default();
        let exec = StateSolanaExecutor::from_state(&state);
        let out = exec
            .cluster(AutonomousSolanaClusterRequest {
                action: AutonomousSolanaClusterAction::Status,
            })
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out.value_json).unwrap();
        assert_eq!(parsed.get("running").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn snapshot_create_requires_accounts() {
        let state = SolanaState::default();
        let exec = StateSolanaExecutor::from_state(&state);
        let err = exec
            .cluster(AutonomousSolanaClusterRequest {
                action: AutonomousSolanaClusterAction::SnapshotCreate {
                    label: "test".to_string(),
                    accounts: vec![],
                    cluster: Some(ClusterKind::Localnet),
                    rpc_url: Some("http://127.0.0.1:8899".to_string()),
                },
            })
            .unwrap_err();
        assert_eq!(err.code, "solana_snapshot_accounts_empty");
    }

    #[test]
    fn action_enum_round_trips_through_serde() {
        let req = AutonomousSolanaClusterRequest {
            action: AutonomousSolanaClusterAction::Start {
                kind: ClusterKind::Localnet,
                opts: StartOpts::default(),
            },
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"action\":\"start\""));
        let decoded: AutonomousSolanaClusterRequest = serde_json::from_str(&json).unwrap();
        match decoded.action {
            AutonomousSolanaClusterAction::Start { kind, .. } => {
                assert_eq!(kind, ClusterKind::Localnet);
            }
            _ => panic!("round trip lost variant"),
        }
    }
}
