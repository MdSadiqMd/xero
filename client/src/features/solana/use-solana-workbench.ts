import { useCallback, useEffect, useRef, useState } from "react"
import { invoke, isTauri } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

export type ClusterKind = "localnet" | "mainnet_fork" | "devnet" | "mainnet"

export interface ClusterDescriptor {
  kind: ClusterKind
  label: string
  startable: boolean
  defaultRpcUrl: string
}

export type ValidatorPhase =
  | "idle"
  | "booting"
  | "ready"
  | "stopping"
  | "stopped"
  | "error"

export interface ValidatorStatusPayload {
  phase: ValidatorPhase
  kind?: string | null
  rpcUrl?: string | null
  wsUrl?: string | null
  message?: string | null
}

export interface ClusterStatus {
  running: boolean
  kind?: ClusterKind | null
  rpcUrl?: string | null
  wsUrl?: string | null
  ledgerDir?: string | null
  startedAtMs?: number | null
  uptimeS?: number | null
}

export interface ClusterHandle {
  kind: ClusterKind
  rpcUrl: string
  wsUrl: string
  pid?: number | null
  ledgerDir: string
  startedAtMs: number
}

export interface ToolProbe {
  present: boolean
  path?: string | null
  version?: string | null
}

export interface ToolchainStatus {
  solanaCli: ToolProbe
  anchor: ToolProbe
  cargoBuildSbf: ToolProbe
  rust: ToolProbe
  node: ToolProbe
  pnpm: ToolProbe
  surfpool: ToolProbe
  trident: ToolProbe
  codama: ToolProbe
  solanaVerify: ToolProbe
  wsl2?: ToolProbe | null
}

export interface EndpointHealth {
  cluster: ClusterKind
  id: string
  url: string
  label?: string | null
  healthy: boolean
  latencyMs?: number | null
  lastError?: string | null
  lastCheckedMs?: number | null
  consecutiveFailures: number
}

export interface SnapshotMeta {
  id: string
  label: string
  cluster: string
  createdAtMs: number
  accountCount: number
  path: string
}

export type PersonaRole =
  | "whale"
  | "lp"
  | "voter"
  | "liquidator"
  | "new_user"
  | "custom"

export interface TokenAllocation {
  symbol?: string | null
  mint?: string | null
  amount: number
}

export interface NftAllocation {
  collection: string
  count: number
}

export interface FundingDelta {
  solLamports?: number
  tokens?: TokenAllocation[]
  nfts?: NftAllocation[]
}

export interface RolePreset {
  displayLabel: string
  description: string
  lamports: number
  tokens: TokenAllocation[]
  nfts: NftAllocation[]
}

export interface RoleDescriptor {
  id: PersonaRole
  preset: RolePreset
}

export interface Persona {
  name: string
  role: PersonaRole
  cluster: ClusterKind
  pubkey: string
  keypairPath: string
  createdAtMs: number
  seed: FundingDelta
  note?: string | null
}

export type FundingStep =
  | {
      kind: "airdrop"
      signature?: string | null
      lamports: number
      ok: boolean
      error?: string | null
    }
  | {
      kind: "tokenMint"
      mint: string
      amount: number
      signature?: string | null
      ok: boolean
      error?: string | null
    }
  | {
      kind: "tokenTransfer"
      mint: string
      amount: number
      signature?: string | null
      ok: boolean
      error?: string | null
    }
  | {
      kind: "nftFixture"
      collection: string
      mint?: string | null
      signature?: string | null
      ok: boolean
      error?: string | null
    }

export interface FundingReceipt {
  persona: string
  cluster: string
  steps: FundingStep[]
  succeeded: boolean
  startedAtMs: number
  finishedAtMs: number
}

export interface PersonaCreateResponse {
  persona: Persona
  receipt: FundingReceipt
}

export interface PersonaSpec {
  name: string
  cluster: ClusterKind
  role?: PersonaRole
  seedOverride?: FundingDelta | null
  note?: string | null
}

export type ScenarioKind = "self_contained" | "pipeline_required"

export interface ScenarioDescriptor {
  id: string
  label: string
  description: string
  supportedClusters: ClusterKind[]
  requiredClonePrograms: string[]
  requiredRoles: PersonaRole[]
  kind: ScenarioKind
}

export type ScenarioStatus = "succeeded" | "failed" | "pendingPipeline"

export interface ScenarioRun {
  id: string
  cluster: ClusterKind
  persona: string
  status: ScenarioStatus
  signatures: string[]
  steps: string[]
  fundingReceipts: FundingReceipt[]
  pipelineHint?: string | null
  startedAtMs: number
  finishedAtMs: number
}

export interface ScenarioSpec {
  id: string
  cluster: ClusterKind
  persona: string
  params?: unknown
}

export interface PersonaEventPayload {
  kind: "created" | "updated" | "funded" | "deleted" | "imported"
  cluster: string
  name: string
  pubkey?: string | null
  tsMs: number
  message?: string | null
}

export interface ScenarioEventPayload {
  kind: "started" | "progress" | "completed" | "failed" | "pending_pipeline"
  id: string
  cluster: string
  persona: string
  tsMs: number
  message?: string | null
  signatureCount: number
}

export interface StartOpts {
  clonePrograms?: string[]
  cloneAccounts?: string[]
  reset?: boolean
  rpcPort?: number
  wsPort?: number
  bootTimeoutSecs?: number
  seedPersonas?: boolean
  limitLedger?: number
}

export interface UseSolanaWorkbench {
  clusters: ClusterDescriptor[]
  toolchain: ToolchainStatus | null
  toolchainLoading: boolean
  status: ClusterStatus
  lastEvent: ValidatorStatusPayload | null
  rpcHealth: EndpointHealth[]
  snapshots: SnapshotMeta[]
  isStarting: boolean
  isStopping: boolean
  error: string | null
  refreshToolchain: () => Promise<void>
  refreshRpcHealth: () => Promise<void>
  refreshSnapshots: () => Promise<void>
  start: (kind: ClusterKind, opts?: StartOpts) => Promise<ClusterHandle | null>
  stop: () => Promise<void>
  // Phase 2 — personas
  personas: Persona[]
  personaRoles: RoleDescriptor[]
  personaBusy: boolean
  lastPersonaEvent: PersonaEventPayload | null
  refreshPersonas: (cluster: ClusterKind) => Promise<void>
  createPersona: (
    spec: PersonaSpec,
    rpcUrl?: string | null,
  ) => Promise<PersonaCreateResponse | null>
  fundPersona: (
    cluster: ClusterKind,
    name: string,
    delta: FundingDelta,
    rpcUrl?: string | null,
  ) => Promise<FundingReceipt | null>
  deletePersona: (cluster: ClusterKind, name: string) => Promise<boolean>
  // Phase 2 — scenarios
  scenarios: ScenarioDescriptor[]
  lastScenarioRun: ScenarioRun | null
  lastScenarioEvent: ScenarioEventPayload | null
  scenarioBusy: boolean
  refreshScenarios: () => Promise<void>
  runScenario: (spec: ScenarioSpec) => Promise<ScenarioRun | null>
}

const SOLANA_VALIDATOR_STATUS_EVENT = "solana:validator:status"
const SOLANA_PERSONA_EVENT = "solana:persona"
const SOLANA_SCENARIO_EVENT = "solana:scenario"

interface Options {
  /** When false, the hook releases listeners and doesn't probe. */
  active: boolean
}

function tauriInvoke<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T | null> {
  if (!isTauri()) return Promise.resolve(null)
  return invoke<T>(command, args).catch(() => null)
}

function errorMessage(error: unknown): string {
  if (error && typeof error === "object" && "message" in error) {
    const message = (error as { message?: unknown }).message
    if (typeof message === "string" && message.length > 0) return message
  }
  if (typeof error === "string" && error.length > 0) return error
  return "Solana workbench command failed"
}

export function useSolanaWorkbench({ active }: Options): UseSolanaWorkbench {
  const [clusters, setClusters] = useState<ClusterDescriptor[]>([])
  const [toolchain, setToolchain] = useState<ToolchainStatus | null>(null)
  const [toolchainLoading, setToolchainLoading] = useState(false)
  const [status, setStatus] = useState<ClusterStatus>({ running: false })
  const [lastEvent, setLastEvent] = useState<ValidatorStatusPayload | null>(null)
  const [rpcHealth, setRpcHealth] = useState<EndpointHealth[]>([])
  const [snapshots, setSnapshots] = useState<SnapshotMeta[]>([])
  const [isStarting, setIsStarting] = useState(false)
  const [isStopping, setIsStopping] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const [personas, setPersonas] = useState<Persona[]>([])
  const [personaRoles, setPersonaRoles] = useState<RoleDescriptor[]>([])
  const [personaBusy, setPersonaBusy] = useState(false)
  const [lastPersonaEvent, setLastPersonaEvent] = useState<PersonaEventPayload | null>(null)

  const [scenarios, setScenarios] = useState<ScenarioDescriptor[]>([])
  const [lastScenarioRun, setLastScenarioRun] = useState<ScenarioRun | null>(null)
  const [lastScenarioEvent, setLastScenarioEvent] = useState<ScenarioEventPayload | null>(null)
  const [scenarioBusy, setScenarioBusy] = useState(false)

  const activeRef = useRef(active)
  activeRef.current = active

  const refreshToolchain = useCallback(async () => {
    if (!isTauri()) return
    setToolchainLoading(true)
    try {
      const next = await invoke<ToolchainStatus>("solana_toolchain_status")
      setToolchain(next)
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setToolchainLoading(false)
    }
  }, [])

  const refreshClusters = useCallback(async () => {
    if (!isTauri()) return
    const next = await tauriInvoke<ClusterDescriptor[]>("solana_cluster_list")
    if (next) setClusters(next)
  }, [])

  const refreshStatus = useCallback(async () => {
    if (!isTauri()) return
    const next = await tauriInvoke<ClusterStatus>("solana_cluster_status")
    if (next) setStatus(next)
  }, [])

  const refreshRpcHealth = useCallback(async () => {
    if (!isTauri()) return
    const next = await tauriInvoke<EndpointHealth[]>("solana_rpc_health")
    if (next) setRpcHealth(next)
  }, [])

  const refreshSnapshots = useCallback(async () => {
    if (!isTauri()) return
    const next = await tauriInvoke<SnapshotMeta[]>("solana_snapshot_list")
    if (next) setSnapshots(next)
  }, [])

  const refreshPersonaRoles = useCallback(async () => {
    if (!isTauri()) return
    const next = await tauriInvoke<RoleDescriptor[]>("solana_persona_roles")
    if (next) setPersonaRoles(next)
  }, [])

  const refreshPersonas = useCallback(async (cluster: ClusterKind) => {
    if (!isTauri()) return
    const next = await tauriInvoke<Persona[]>("solana_persona_list", {
      request: { cluster },
    })
    if (next) setPersonas(next)
  }, [])

  const refreshScenarios = useCallback(async () => {
    if (!isTauri()) return
    const next = await tauriInvoke<ScenarioDescriptor[]>("solana_scenario_list")
    if (next) setScenarios(next)
  }, [])

  const createPersona = useCallback(
    async (
      spec: PersonaSpec,
      rpcUrl?: string | null,
    ): Promise<PersonaCreateResponse | null> => {
      if (!isTauri()) return null
      setPersonaBusy(true)
      setError(null)
      try {
        const response = await invoke<PersonaCreateResponse>("solana_persona_create", {
          request: {
            spec,
            rpcUrl: rpcUrl ?? null,
          },
        })
        await refreshPersonas(spec.cluster)
        return response
      } catch (err) {
        setError(errorMessage(err))
        return null
      } finally {
        setPersonaBusy(false)
      }
    },
    [refreshPersonas],
  )

  const fundPersona = useCallback(
    async (
      cluster: ClusterKind,
      name: string,
      delta: FundingDelta,
      rpcUrl?: string | null,
    ): Promise<FundingReceipt | null> => {
      if (!isTauri()) return null
      setPersonaBusy(true)
      setError(null)
      try {
        const receipt = await invoke<FundingReceipt>("solana_persona_fund", {
          request: {
            cluster,
            name,
            delta,
            rpcUrl: rpcUrl ?? null,
          },
        })
        await refreshPersonas(cluster)
        return receipt
      } catch (err) {
        setError(errorMessage(err))
        return null
      } finally {
        setPersonaBusy(false)
      }
    },
    [refreshPersonas],
  )

  const deletePersona = useCallback(
    async (cluster: ClusterKind, name: string): Promise<boolean> => {
      if (!isTauri()) return false
      setPersonaBusy(true)
      setError(null)
      try {
        await invoke("solana_persona_delete", {
          request: { cluster, name },
        })
        await refreshPersonas(cluster)
        return true
      } catch (err) {
        setError(errorMessage(err))
        return false
      } finally {
        setPersonaBusy(false)
      }
    },
    [refreshPersonas],
  )

  const runScenario = useCallback(
    async (spec: ScenarioSpec): Promise<ScenarioRun | null> => {
      if (!isTauri()) return null
      setScenarioBusy(true)
      setError(null)
      try {
        const run = await invoke<ScenarioRun>("solana_scenario_run", {
          request: { spec },
        })
        setLastScenarioRun(run)
        await refreshPersonas(spec.cluster)
        return run
      } catch (err) {
        setError(errorMessage(err))
        return null
      } finally {
        setScenarioBusy(false)
      }
    },
    [refreshPersonas],
  )

  // Mount: probe toolchain + cluster catalogue + status + persona catalog.
  useEffect(() => {
    if (!active || !isTauri()) return
    void refreshClusters()
    void refreshToolchain()
    void refreshStatus()
    void refreshSnapshots()
    void refreshPersonaRoles()
    void refreshScenarios()
  }, [
    active,
    refreshClusters,
    refreshToolchain,
    refreshStatus,
    refreshSnapshots,
    refreshPersonaRoles,
    refreshScenarios,
  ])

  // Listen for status events while the sidebar is visible.
  useEffect(() => {
    if (!active || !isTauri()) return
    let cancelled = false
    const unsubs: UnlistenFn[] = []

    void listen<ValidatorStatusPayload>(
      SOLANA_VALIDATOR_STATUS_EVENT,
      (event) => {
        if (cancelled) return
        setLastEvent(event.payload)
        if (event.payload.phase === "ready") {
          setStatus((current) => ({
            running: true,
            kind: (event.payload.kind as ClusterKind | undefined) ?? current.kind ?? null,
            rpcUrl: event.payload.rpcUrl ?? current.rpcUrl ?? null,
            wsUrl: event.payload.wsUrl ?? current.wsUrl ?? null,
            ledgerDir: current.ledgerDir ?? null,
            startedAtMs: current.startedAtMs ?? null,
            uptimeS: current.uptimeS ?? null,
          }))
        }
        if (
          event.payload.phase === "stopped" ||
          event.payload.phase === "idle"
        ) {
          setStatus({ running: false })
        }
        if (event.payload.phase === "error" && event.payload.message) {
          setError(event.payload.message)
        }
      },
    ).then((unsub) => {
      if (cancelled) {
        unsub()
      } else {
        unsubs.push(unsub)
      }
    })

    void listen<PersonaEventPayload>(SOLANA_PERSONA_EVENT, (event) => {
      if (cancelled) return
      setLastPersonaEvent(event.payload)
    }).then((unsub) => {
      if (cancelled) {
        unsub()
      } else {
        unsubs.push(unsub)
      }
    })

    void listen<ScenarioEventPayload>(SOLANA_SCENARIO_EVENT, (event) => {
      if (cancelled) return
      setLastScenarioEvent(event.payload)
    }).then((unsub) => {
      if (cancelled) {
        unsub()
      } else {
        unsubs.push(unsub)
      }
    })

    // Nudge the backend to re-emit the current status so the UI syncs.
    void invoke("solana_subscribe_ready").catch(() => {
      /* idempotent no-op */
    })

    return () => {
      cancelled = true
      for (const unsub of unsubs) unsub()
    }
  }, [active])

  const start = useCallback(
    async (kind: ClusterKind, opts?: StartOpts): Promise<ClusterHandle | null> => {
      if (!isTauri()) return null
      setIsStarting(true)
      setError(null)
      try {
        const handle = await invoke<ClusterHandle>("solana_cluster_start", {
          request: { kind, opts: opts ?? {} },
        })
        await refreshStatus()
        await refreshRpcHealth()
        return handle
      } catch (err) {
        setError(errorMessage(err))
        return null
      } finally {
        setIsStarting(false)
      }
    },
    [refreshRpcHealth, refreshStatus],
  )

  const stop = useCallback(async () => {
    if (!isTauri()) return
    setIsStopping(true)
    setError(null)
    try {
      await invoke("solana_cluster_stop")
      await refreshStatus()
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setIsStopping(false)
    }
  }, [refreshStatus])

  return {
    clusters,
    toolchain,
    toolchainLoading,
    status,
    lastEvent,
    rpcHealth,
    snapshots,
    isStarting,
    isStopping,
    error,
    refreshToolchain,
    refreshRpcHealth,
    refreshSnapshots,
    start,
    stop,
    personas,
    personaRoles,
    personaBusy,
    lastPersonaEvent,
    refreshPersonas,
    createPersona,
    fundPersona,
    deletePersona,
    scenarios,
    lastScenarioRun,
    lastScenarioEvent,
    scenarioBusy,
    refreshScenarios,
    runScenario,
  }
}
