"use client"

import { formatDistanceToNow } from "date-fns"
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
    upstream?: {
      ahead?: number | null
      behind?: number | null
    } | null
    hasChanges?: boolean
    changedFiles?: number
    lastCommit?: {
      sha?: string | null
      message?: string | null
      committedAt?: string | null
    } | null
  } | null
  runtime?: {
    provider?: string | null
    state?: FooterRuntimeState
  } | null
}

type FooterGitUpstream = NonNullable<NonNullable<StatusFooterProps["git"]>["upstream"]>

// ---------------------------------------------------------------------------
// Placeholder data for footer surfaces that do not have backend projections yet.
// ---------------------------------------------------------------------------

const FOOTER_PLACEHOLDERS = {
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
  const liveLastCommit = git?.lastCommit
  const liveLastCommitSha = formatShortSha(liveLastCommit?.sha)
  const liveLastCommitMessage = normalizeOptionalFooterText(liveLastCommit?.message)
  const liveLastCommitRelativeTime = formatRelativeCommitTime(liveLastCommit?.committedAt)
  const upstream = normalizeUpstream(git?.upstream)

  const footer = {
    ...FOOTER_PLACEHOLDERS,
    branch: normalizeFooterText(git?.branch, "No branch"),
    workingTree: {
      dirty: git?.hasChanges ?? false,
      changedFiles: git?.changedFiles ?? 0,
    },
    lastCommit:
      liveLastCommitSha && liveLastCommitMessage
        ? {
            shortSha: liveLastCommitSha,
            message: liveLastCommitMessage,
            relativeTime: liveLastCommitRelativeTime ?? "",
          }
        : {
            shortSha: "—",
            message: "No commits yet",
            relativeTime: "",
          },
    runtime: {
      provider: normalizeFooterText(runtime?.provider, FOOTER_PLACEHOLDERS.runtime.provider),
      state: runtime?.state ?? FOOTER_PLACEHOLDERS.runtime.state,
    },
  }

  const { branch, workingTree, lastCommit, runtime: runtimeStatus, spend, notifications } = footer

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
          {upstream ? (
            <span className="text-muted-foreground/70">
              ↑{upstream.ahead} ↓{upstream.behind}
            </span>
          ) : null}
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
          {lastCommit.relativeTime ? (
            <span className="shrink-0 text-muted-foreground/70">· {lastCommit.relativeTime}</span>
          ) : null}
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

function normalizeOptionalFooterText(value: string | null | undefined): string | null {
  const trimmed = value?.trim()
  return trimmed && trimmed.length > 0 ? trimmed : null
}

function formatShortSha(value: string | null | undefined): string | null {
  const trimmed = value?.trim()
  if (!trimmed || trimmed === "No HEAD") {
    return null
  }

  return trimmed.slice(0, 7)
}

function normalizeUpstream(
  value: FooterGitUpstream | null | undefined,
): { ahead: number; behind: number } | null {
  if (!value) {
    return null
  }

  return {
    ahead: normalizeCount(value.ahead),
    behind: normalizeCount(value.behind),
  }
}

function normalizeCount(value: number | null | undefined): number {
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? Math.floor(value) : 0
}

function formatRelativeCommitTime(value: string | null | undefined): string | null {
  const trimmed = value?.trim()
  if (!trimmed) {
    return null
  }

  const parsed = new Date(trimmed)
  if (Number.isNaN(parsed.getTime())) {
    return null
  }

  return formatDistanceToNow(parsed, { addSuffix: true })
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
