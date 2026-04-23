"use client"

import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { CircleCheckBig, CircleSlash, Loader2, Play, RefreshCw, Square, Waves } from "lucide-react"
import { cn } from "@/lib/utils"
import { SolanaMissingToolchain } from "./solana-missing-toolchain"
import {
  useSolanaWorkbench,
  type ClusterKind,
} from "@/src/features/solana/use-solana-workbench"

const MIN_WIDTH = 320
const DEFAULT_WIDTH = 420
const MAX_WIDTH = 900
const STORAGE_KEY = "cadence.solana.workbench.width"

interface SolanaWorkbenchSidebarProps {
  open: boolean
}

function readPersistedWidth(): number | null {
  if (typeof window === "undefined") return null
  try {
    const raw = window.localStorage?.getItem?.(STORAGE_KEY)
    if (!raw) return null
    const parsed = Number.parseInt(raw, 10)
    if (!Number.isFinite(parsed) || parsed < MIN_WIDTH) return null
    return parsed
  } catch {
    return null
  }
}

function writePersistedWidth(value: number) {
  if (typeof window === "undefined") return
  try {
    window.localStorage?.setItem?.(STORAGE_KEY, String(Math.round(value)))
  } catch {
    /* storage unavailable — default next session */
  }
}

export function SolanaWorkbenchSidebar({ open }: SolanaWorkbenchSidebarProps) {
  const [width, setWidth] = useState<number>(() => readPersistedWidth() ?? DEFAULT_WIDTH)
  const [isResizing, setIsResizing] = useState(false)
  const widthRef = useRef(width)
  widthRef.current = width

  const workbench = useSolanaWorkbench({ active: open })
  const [selectedKind, setSelectedKind] = useState<ClusterKind>("localnet")

  useEffect(() => {
    if (!workbench.clusters.length) return
    setSelectedKind((current) => {
      if (workbench.clusters.some((c) => c.kind === current)) return current
      const firstStartable = workbench.clusters.find((c) => c.startable)
      return firstStartable?.kind ?? current
    })
  }, [workbench.clusters])

  useEffect(() => {
    writePersistedWidth(width)
  }, [width])

  const handleResizeStart = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      if (event.button !== 0) return
      event.preventDefault()
      const startX = event.clientX
      const startWidth = widthRef.current
      setIsResizing(true)

      const previousCursor = document.body.style.cursor
      const previousSelect = document.body.style.userSelect
      document.body.style.cursor = "col-resize"
      document.body.style.userSelect = "none"

      const handleMove = (ev: PointerEvent) => {
        const delta = startX - ev.clientX
        const next = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, startWidth + delta))
        setWidth(next)
      }
      const handleUp = () => {
        window.removeEventListener("pointermove", handleMove)
        window.removeEventListener("pointerup", handleUp)
        window.removeEventListener("pointercancel", handleUp)
        document.body.style.cursor = previousCursor
        document.body.style.userSelect = previousSelect
        setIsResizing(false)
      }

      window.addEventListener("pointermove", handleMove)
      window.addEventListener("pointerup", handleUp)
      window.addEventListener("pointercancel", handleUp)
    },
    [],
  )

  const handleStart = useCallback(() => {
    void workbench.start(selectedKind)
  }, [workbench, selectedKind])

  const handleStop = useCallback(() => {
    void workbench.stop()
  }, [workbench])

  const selectedCluster = useMemo(
    () => workbench.clusters.find((c) => c.kind === selectedKind) ?? null,
    [workbench.clusters, selectedKind],
  )

  return (
    <aside
      aria-hidden={!open}
      className={cn(
        "relative flex shrink-0 flex-col overflow-hidden border-l border-border/80 bg-sidebar",
        !isResizing && "transition-[width] duration-200 ease-out",
        !open && "border-l-0",
      )}
      inert={!open ? true : undefined}
      style={{ width: open ? width : 0 }}
    >
      <div
        aria-label="Resize Solana workbench sidebar"
        aria-orientation="vertical"
        aria-valuemax={MAX_WIDTH}
        aria-valuemin={MIN_WIDTH}
        aria-valuenow={width}
        className={cn(
          "absolute inset-y-0 -left-[3px] z-10 w-[6px] cursor-col-resize bg-transparent transition-colors",
          "hover:bg-primary/30",
          isResizing && "bg-primary/40",
        )}
        onPointerDown={handleResizeStart}
        role="separator"
        tabIndex={open ? 0 : -1}
      />

      <div className="flex h-10 items-center justify-between border-b border-border/70 pl-3 pr-2">
        <div className="flex items-center gap-2">
          <Waves aria-hidden className="h-3.5 w-3.5 text-primary" />
          <span className="text-[10.5px] font-semibold uppercase tracking-[0.1em] text-muted-foreground">
            Solana Workbench
          </span>
        </div>
        <button
          aria-label="Refresh toolchain"
          className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-secondary/50 hover:text-foreground disabled:opacity-60"
          disabled={workbench.toolchainLoading}
          onClick={() => void workbench.refreshToolchain()}
          type="button"
        >
          <RefreshCw
            className={cn(
              "h-3.5 w-3.5",
              workbench.toolchainLoading && "animate-spin",
            )}
          />
        </button>
      </div>

      <SolanaMissingToolchain
        loading={workbench.toolchainLoading}
        onRefresh={() => void workbench.refreshToolchain()}
        status={workbench.toolchain}
      />

      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto scrollbar-thin">
        <section className="border-b border-border/70 px-3 py-3">
          <div className="mb-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
            Cluster
          </div>
          <div className="flex flex-wrap gap-1.5">
            {workbench.clusters.map((cluster) => (
              <button
                key={cluster.kind}
                type="button"
                disabled={!cluster.startable && !workbench.status.running}
                onClick={() => setSelectedKind(cluster.kind)}
                className={cn(
                  "rounded-md border px-2 py-1 text-[11px] transition-colors",
                  selectedKind === cluster.kind
                    ? "border-primary/50 bg-primary/10 text-primary"
                    : "border-border/70 bg-background/40 text-foreground/80 hover:border-primary/40 hover:text-foreground",
                  !cluster.startable && "opacity-60",
                )}
              >
                {cluster.label}
              </button>
            ))}
          </div>
          {selectedCluster ? (
            <p className="mt-2 text-[11px] text-muted-foreground">
              {selectedCluster.startable
                ? "Local cluster — Cadence can spin it up on your machine."
                : "Remote cluster — read-only from here."}
            </p>
          ) : null}
        </section>

        <section className="border-b border-border/70 px-3 py-3">
          <div className="mb-2 flex items-center justify-between">
            <div className="text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
              Validator
            </div>
            <StatusDot status={workbench.status} />
          </div>
          <div className="space-y-2 text-[11px] text-foreground/85">
            <KV label="State" value={workbench.status.running ? "Running" : "Stopped"} />
            <KV label="RPC" value={workbench.status.rpcUrl ?? "—"} />
            <KV label="WS" value={workbench.status.wsUrl ?? "—"} />
            {workbench.status.uptimeS != null ? (
              <KV label="Uptime" value={`${workbench.status.uptimeS}s`} />
            ) : null}
            {workbench.lastEvent?.message ? (
              <div className="text-[11px] text-muted-foreground italic">
                {workbench.lastEvent.message}
              </div>
            ) : null}
          </div>
          <div className="mt-3 flex items-center gap-2">
            <button
              type="button"
              onClick={handleStart}
              disabled={
                !selectedCluster?.startable ||
                workbench.isStarting ||
                workbench.isStopping
              }
              className={cn(
                "inline-flex items-center gap-1.5 rounded-md border border-primary/50 bg-primary/15 px-2.5 py-1 text-[11px] font-medium text-primary transition-colors",
                "hover:bg-primary/25 disabled:opacity-50",
              )}
            >
              {workbench.isStarting ? (
                <Loader2 className="h-3 w-3 animate-spin" />
              ) : (
                <Play className="h-3 w-3 fill-current" />
              )}
              Start
            </button>
            <button
              type="button"
              onClick={handleStop}
              disabled={!workbench.status.running || workbench.isStopping}
              className={cn(
                "inline-flex items-center gap-1.5 rounded-md border border-border/70 bg-background/40 px-2.5 py-1 text-[11px] text-foreground/85 transition-colors",
                "hover:border-destructive/50 hover:text-destructive disabled:opacity-50",
              )}
            >
              {workbench.isStopping ? (
                <Loader2 className="h-3 w-3 animate-spin" />
              ) : (
                <Square className="h-3 w-3" />
              )}
              Stop
            </button>
          </div>
          {workbench.error ? (
            <p className="mt-2 text-[11px] text-destructive">{workbench.error}</p>
          ) : null}
        </section>

        <section className="border-b border-border/70 px-3 py-3">
          <div className="mb-2 flex items-center justify-between">
            <div className="text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
              RPC endpoints
            </div>
            <button
              type="button"
              aria-label="Refresh RPC health"
              className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-secondary/50 hover:text-foreground"
              onClick={() => void workbench.refreshRpcHealth()}
            >
              <RefreshCw className="h-3 w-3" />
            </button>
          </div>
          {workbench.rpcHealth.length === 0 ? (
            <p className="text-[11px] text-muted-foreground">
              Click refresh to probe the free-tier endpoint pool.
            </p>
          ) : (
            <ul className="flex flex-col gap-1.5">
              {workbench.rpcHealth.map((health) => (
                <li
                  key={`${health.cluster}-${health.id}`}
                  className="flex items-center justify-between gap-2 rounded-md border border-border/60 bg-background/40 px-2 py-1.5"
                >
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[11.5px] text-foreground">
                      {health.label ?? health.id}
                    </div>
                    <div className="truncate text-[10.5px] text-muted-foreground">
                      {health.cluster} · {health.url}
                    </div>
                  </div>
                  <div className="flex items-center gap-1 text-[10.5px]">
                    {health.healthy ? (
                      <CircleCheckBig className="h-3 w-3 text-emerald-400" />
                    ) : (
                      <CircleSlash className="h-3 w-3 text-destructive" />
                    )}
                    {health.latencyMs != null ? (
                      <span className="font-mono tabular-nums text-muted-foreground">
                        {health.latencyMs}ms
                      </span>
                    ) : null}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </aside>
  )
}

function StatusDot({ status }: { status: { running: boolean } }) {
  return (
    <span
      aria-label={status.running ? "Running" : "Stopped"}
      className={cn(
        "inline-block h-2 w-2 rounded-full",
        status.running ? "bg-emerald-400" : "bg-muted-foreground/40",
      )}
    />
  )
}

function KV({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-3">
      <span className="text-[10.5px] uppercase tracking-[0.14em] text-muted-foreground">
        {label}
      </span>
      <span className="min-w-0 truncate text-right font-mono text-[11px] tabular-nums text-foreground/85">
        {value}
      </span>
    </div>
  )
}
