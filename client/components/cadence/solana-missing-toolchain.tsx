"use client"

import { ExternalLink, RefreshCw } from "lucide-react"
import { cn } from "@/lib/utils"
import type { ToolchainStatus } from "@/src/features/solana/use-solana-workbench"

interface Props {
  status: ToolchainStatus | null
  loading: boolean
  onRefresh: () => void
}

// Panel shown above the cluster picker when the host is missing the
// minimum Solana toolchain (the Solana CLI). Other tools are only flagged
// if they would be actively used by the current cluster flow.
export function SolanaMissingToolchain({ status, loading, onRefresh }: Props) {
  if (!status) return null
  const panel = buildPanel(status)
  if (!panel) return null

  return (
    <div
      aria-live="polite"
      className="flex shrink-0 flex-col gap-2 border-b border-border/60 bg-amber-500/10 px-3 py-2 text-[11px] leading-relaxed"
      role="region"
    >
      <div className="font-medium text-amber-200">{panel.title}</div>
      <div className="text-muted-foreground">{panel.detail}</div>
      <div className="flex flex-wrap items-center gap-2">
        {panel.actions.map((action) => (
          <a
            className={cn(
              "inline-flex items-center gap-1 rounded-md border border-border/70 bg-background/60 px-2 py-0.5",
              "text-[11px] text-foreground transition-colors hover:border-primary/50 hover:text-primary",
            )}
            href={action.href}
            key={action.label}
            rel="noreferrer"
            target="_blank"
          >
            {action.label}
            <ExternalLink className="h-3 w-3" />
          </a>
        ))}
        <button
          aria-label="Re-detect toolchain"
          className="inline-flex items-center gap-1 rounded-md border border-border/70 bg-background/60 px-2 py-0.5 text-[11px] text-foreground transition-colors hover:border-primary/50 hover:text-primary disabled:opacity-60"
          disabled={loading}
          onClick={onRefresh}
          type="button"
        >
          <RefreshCw className={cn("h-3 w-3", loading && "animate-spin")} />
          Re-detect
        </button>
      </div>
    </div>
  )
}

interface Panel {
  title: string
  detail: string
  actions: Array<{ label: string; href: string }>
}

function buildPanel(status: ToolchainStatus): Panel | null {
  if (!status.solanaCli.present) {
    return {
      title: "Solana CLI not found",
      detail:
        "Install the Solana tool suite (v1.18+) so Cadence can spin up a localnet and submit transactions. Cadence searches PATH and the default Solana install directory.",
      actions: [
        {
          label: "Install Solana CLI",
          href: "https://docs.solanalabs.com/cli/install",
        },
      ],
    }
  }

  // Anchor is optional for pure Rust programs, but advise if both rust
  // and anchor are missing so the user understands the full toolchain.
  if (!status.anchor.present && !status.cargoBuildSbf.present) {
    return {
      title: "Program build tooling not found",
      detail:
        "Install Anchor (for framework projects) or cargo-build-sbf (for raw Rust programs) when you're ready to build a deployable .so.",
      actions: [
        { label: "Install Anchor", href: "https://www.anchor-lang.com/docs/installation" },
        { label: "Install cargo-build-sbf", href: "https://docs.solanalabs.com/cli/install" },
      ],
    }
  }

  return null
}
