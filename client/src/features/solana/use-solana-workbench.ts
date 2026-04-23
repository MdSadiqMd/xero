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
}

const SOLANA_VALIDATOR_STATUS_EVENT = "solana:validator:status"

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

  // Mount: probe toolchain + cluster catalogue + status.
  useEffect(() => {
    if (!active || !isTauri()) return
    void refreshClusters()
    void refreshToolchain()
    void refreshStatus()
    void refreshSnapshots()
  }, [active, refreshClusters, refreshToolchain, refreshStatus, refreshSnapshots])

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
  }
}
