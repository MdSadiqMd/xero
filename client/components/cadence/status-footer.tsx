"use client"

import {
  Bell,
  CircleDot,
  Coins,
  Cpu,
  DollarSign,
  GitBranch,
  GitCommit,
} from "lucide-react"
import { cn } from "@/lib/utils"

export type FooterRuntimeState = "idle" | "running" | "paused"

export interface StatusFooterProps {
  git?: {
    branch?: string | null
    hasChanges?: boolean
    changedFiles?: number
    headSha?: string | null
  } | null
  runtime?: {
    provider?: string | null
    state?: FooterRuntimeState
  } | null
}

// ---------------------------------------------------------------------------
// Mock data — live surfaces override only the parts already backed by state.
// ---------------------------------------------------------------------------

const MOCK_FOOTER = {
  branch: "main",
  upstream: { ahead: 2, behind: 0 },
  workingTree: { dirty: true, changedFiles: 1 },
  lastCommit: {
    shortSha: "b114b71",
    message: "feat: Added durable-denial guardrails and shipped-path YOLO safety proof",
    relativeTime: "12m ago",
  },
  project: {
    name: "joe",
    phase: "Phase 7 · Status footer",
  },
  runtime: {
    provider: "OpenAI Codex",
    state: "idle" as FooterRuntimeState,
  },
  spend: {
    totalTokens: 1_284_530,
    totalUsd: 18.42,
  },
  notifications: 3,
}

// ---------------------------------------------------------------------------
// Footer
// ---------------------------------------------------------------------------

export function StatusFooter({ git = null, runtime = null }: StatusFooterProps) {
  const footer = {
    ...MOCK_FOOTER,
    branch: normalizeFooterText(git?.branch, MOCK_FOOTER.branch),
    workingTree: {
      dirty: git?.hasChanges ?? MOCK_FOOTER.workingTree.dirty,
      changedFiles: git?.changedFiles ?? MOCK_FOOTER.workingTree.changedFiles,
    },
    lastCommit: {
      ...MOCK_FOOTER.lastCommit,
      shortSha: formatShortSha(git?.headSha) ?? MOCK_FOOTER.lastCommit.shortSha,
    },
    runtime: {
      provider: normalizeFooterText(runtime?.provider, MOCK_FOOTER.runtime.provider),
      state: runtime?.state ?? MOCK_FOOTER.runtime.state,
    },
  }

  const { branch, upstream, workingTree, lastCommit, runtime: runtimeStatus, spend, notifications } = footer

  const truncatedCommit =
    lastCommit.message.length > 46 ? `${lastCommit.message.slice(0, 46)}…` : lastCommit.message

  return (
    <footer
      aria-label="Status bar"
      className="flex h-8 shrink-0 items-center justify-between gap-3 border-t border-border bg-sidebar px-3 text-[11px] leading-none text-muted-foreground"
    >
      {/* Left: git branch + working tree + last commit ----------------------- */}
      <div className="flex min-w-0 items-center gap-3">
        <span className="flex items-center gap-1.5">
          <GitBranch className="h-3 w-3" />
          <span className="font-medium text-foreground/80">{branch}</span>
          <span className="text-muted-foreground/70">
            ↑{upstream.ahead} ↓{upstream.behind}
          </span>
        </span>

        <Divider />

        <span className="flex items-center gap-1.5">
          <CircleDot
            className={cn(
              "h-3 w-3",
              workingTree.dirty ? "text-amber-500" : "text-emerald-500",
            )}
          />
          <span>
            {workingTree.dirty
              ? `${workingTree.changedFiles} change${workingTree.changedFiles === 1 ? "" : "s"}`
              : "clean"}
          </span>
        </span>

        <Divider />

        <span className="flex min-w-0 items-center gap-1.5">
          <GitCommit className="h-3 w-3 shrink-0" />
          <span className="font-mono text-foreground/70">{lastCommit.shortSha}</span>
          <span className="truncate">{truncatedCommit}</span>
          <span className="shrink-0 text-muted-foreground/70">· {lastCommit.relativeTime}</span>
        </span>
      </div>

      {/* Right: project · runtime · network · notifications · version ------- */}
      <div className="flex shrink-0 items-center gap-3">
        <span className="flex items-center gap-1.5">
          <Cpu className="h-3 w-3" />
          <span>{runtimeStatus.provider}</span>
          <RuntimeStateDot state={runtimeStatus.state} />
          <span className="capitalize">{runtimeStatus.state}</span>
        </span>

        <Divider />

        <span
          className="flex items-center gap-1.5"
          aria-label={`Project spend: ${formatTokens(spend.totalTokens)} tokens, ${formatUsd(spend.totalUsd)}`}
          title="Total project spend"
        >
          <Coins className="h-3 w-3" />
          <span>{formatTokens(spend.totalTokens)} tok</span>
          <span className="text-muted-foreground/60">·</span>
          <DollarSign className="h-3 w-3" />
          <span className="font-medium text-foreground/80">{formatUsd(spend.totalUsd)}</span>
        </span>

        <Divider />

        <span className="flex items-center gap-1.5" aria-label={`${notifications} unread notifications`}>
          <Bell className="h-3 w-3" />
          <span>{notifications}</span>
        </span>
      </div>
    </footer>
  )
}

function formatTokens(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(2)}M`
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`
  return value.toString()
}

function formatUsd(value: number): string {
  return value.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}

function normalizeFooterText(value: string | null | undefined, fallback: string): string {
  const trimmed = value?.trim()
  return trimmed && trimmed.length > 0 ? trimmed : fallback
}

function formatShortSha(value: string | null | undefined): string | null {
  const trimmed = value?.trim()
  if (!trimmed || trimmed === "No HEAD") {
    return null
  }

  return trimmed.slice(0, 7)
}

// ---------------------------------------------------------------------------
// Internal pieces
// ---------------------------------------------------------------------------

function Divider({ className }: { className?: string }) {
  return <span aria-hidden="true" className={cn("h-3 w-px bg-border", className)} />
}

function RuntimeStateDot({ state }: { state: FooterRuntimeState }) {
  const color =
    state === "running"
      ? "bg-emerald-500 animate-pulse"
      : state === "paused"
        ? "bg-amber-500"
        : "bg-muted-foreground/50"
  return <span aria-hidden="true" className={cn("h-1.5 w-1.5 rounded-full", color)} />
}
