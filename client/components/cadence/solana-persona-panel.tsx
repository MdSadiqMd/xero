"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import {
  Loader2,
  Plus,
  Sparkles,
  Trash2,
  Users,
  Wallet,
} from "lucide-react"
import { cn } from "@/lib/utils"
import type {
  ClusterKind,
  FundingDelta,
  FundingReceipt,
  Persona,
  PersonaRole,
  RoleDescriptor,
} from "@/src/features/solana/use-solana-workbench"

interface SolanaPersonaPanelProps {
  cluster: ClusterKind
  personas: Persona[]
  roles: RoleDescriptor[]
  busy: boolean
  onRefresh: () => void
  onCreate: (name: string, role: PersonaRole, note: string | null) => Promise<FundingReceipt | null>
  onDelete: (name: string) => Promise<boolean>
  onFund: (name: string, delta: FundingDelta) => Promise<FundingReceipt | null>
  clusterRunning: boolean
}

export function SolanaPersonaPanel({
  cluster,
  personas,
  roles,
  busy,
  onRefresh,
  onCreate,
  onDelete,
  onFund,
  clusterRunning,
}: SolanaPersonaPanelProps) {
  const [newName, setNewName] = useState("")
  const [newRole, setNewRole] = useState<PersonaRole>("whale")
  const [newNote, setNewNote] = useState("")
  const [expandedName, setExpandedName] = useState<string | null>(null)
  const [lastReceipt, setLastReceipt] = useState<FundingReceipt | null>(null)
  const [statusMessage, setStatusMessage] = useState<string | null>(null)

  useEffect(() => {
    onRefresh()
  }, [cluster, onRefresh])

  const rolePresets = useMemo(() => {
    const map = new Map<PersonaRole, RoleDescriptor>()
    for (const descriptor of roles) {
      map.set(descriptor.id, descriptor)
    }
    return map
  }, [roles])

  const handleCreate = useCallback(async () => {
    const trimmed = newName.trim()
    if (!trimmed) {
      setStatusMessage("Provide a persona name")
      return
    }
    setStatusMessage(null)
    const receipt = await onCreate(trimmed, newRole, newNote.trim() || null)
    if (receipt) {
      setLastReceipt(receipt)
      setStatusMessage(
        receipt.succeeded
          ? `Created ${trimmed} — ${receipt.steps.length} funding step(s)`
          : `Created ${trimmed} with ${countFailures(receipt)} failing step(s)`,
      )
      setNewName("")
      setNewNote("")
    }
  }, [newName, newNote, newRole, onCreate])

  const handleRefund = useCallback(
    async (persona: Persona) => {
      const preset = rolePresets.get(persona.role)
      const delta: FundingDelta = preset
        ? {
            solLamports: preset.preset.lamports,
            tokens: preset.preset.tokens,
            nfts: preset.preset.nfts,
          }
        : persona.seed
      const receipt = await onFund(persona.name, delta)
      if (receipt) {
        setLastReceipt(receipt)
        setStatusMessage(
          receipt.succeeded
            ? `Re-funded ${persona.name}`
            : `Re-fund for ${persona.name} had ${countFailures(receipt)} failure(s)`,
        )
      }
    },
    [onFund, rolePresets],
  )

  return (
    <section className="border-b border-border/70 px-3 py-3">
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Users className="h-3.5 w-3.5 text-muted-foreground" />
          <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
            Personas ({personas.length})
          </span>
        </div>
        <button
          type="button"
          onClick={onRefresh}
          disabled={busy}
          className="rounded-md px-1.5 py-0.5 text-[10px] uppercase tracking-[0.1em] text-muted-foreground hover:bg-secondary/50 hover:text-foreground disabled:opacity-40"
        >
          Refresh
        </button>
      </div>

      <div className="mb-3 rounded-md border border-border/60 bg-background/40 p-2">
        <div className="mb-1.5 text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
          New persona on {clusterLabel(cluster)}
        </div>
        <div className="flex flex-col gap-1.5">
          <input
            aria-label="Persona name"
            className="rounded-md border border-border/60 bg-background/80 px-2 py-1 text-[12px] outline-none focus:border-primary/60"
            onChange={(event) => setNewName(event.target.value)}
            placeholder="e.g. whale-1"
            value={newName}
          />
          <select
            aria-label="Persona role"
            className="rounded-md border border-border/60 bg-background/80 px-2 py-1 text-[12px] outline-none focus:border-primary/60"
            onChange={(event) => setNewRole(event.target.value as PersonaRole)}
            value={newRole}
          >
            {roles.map((role) => (
              <option key={role.id} value={role.id}>
                {role.preset.displayLabel} · {formatLamports(role.preset.lamports)} SOL
              </option>
            ))}
          </select>
          <input
            aria-label="Persona note"
            className="rounded-md border border-border/60 bg-background/80 px-2 py-1 text-[11px] outline-none focus:border-primary/60"
            onChange={(event) => setNewNote(event.target.value)}
            placeholder="note (optional)"
            value={newNote}
          />
          <button
            type="button"
            onClick={handleCreate}
            disabled={busy}
            className={cn(
              "inline-flex items-center justify-center gap-1.5 rounded-md border border-primary/50 bg-primary/15 px-2.5 py-1 text-[11px] font-medium text-primary transition-colors",
              "hover:bg-primary/25 disabled:opacity-50",
            )}
          >
            {busy ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              <Plus className="h-3 w-3" />
            )}
            Create + fund
          </button>
          {!clusterRunning ? (
            <p className="text-[10.5px] text-muted-foreground">
              Cluster is stopped — persona will be created locally without funding until you start a
              validator.
            </p>
          ) : null}
          {statusMessage ? (
            <p className="text-[10.5px] text-muted-foreground">{statusMessage}</p>
          ) : null}
        </div>
      </div>

      {personas.length === 0 ? (
        <p className="text-[11px] text-muted-foreground">
          No personas yet. Create one to seed a named wallet on {clusterLabel(cluster)}.
        </p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {personas.map((persona) => {
            const expanded = expandedName === persona.name
            return (
              <li
                key={`${persona.cluster}-${persona.name}`}
                className="rounded-md border border-border/60 bg-background/40"
              >
                <button
                  type="button"
                  onClick={() =>
                    setExpandedName((prev) => (prev === persona.name ? null : persona.name))
                  }
                  className="flex w-full items-center justify-between gap-2 px-2 py-1.5 text-left"
                >
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2">
                      <Wallet className="h-3 w-3 text-primary" />
                      <span className="truncate text-[12px] font-medium text-foreground">
                        {persona.name}
                      </span>
                      <span className="shrink-0 rounded bg-secondary/40 px-1.5 py-0.5 text-[9px] uppercase tracking-[0.12em] text-muted-foreground">
                        {persona.role}
                      </span>
                    </div>
                    <div className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground">
                      {persona.pubkey}
                    </div>
                  </div>
                </button>

                {expanded ? (
                  <div className="border-t border-border/50 px-2 py-1.5">
                    <div className="grid grid-cols-[auto,1fr] gap-x-3 gap-y-1 text-[10.5px]">
                      <span className="text-muted-foreground">Lamports</span>
                      <span className="font-mono text-foreground/85">
                        {persona.seed.solLamports ?? 0}
                      </span>
                      <span className="text-muted-foreground">Tokens</span>
                      <span className="text-foreground/85">
                        {(persona.seed.tokens ?? [])
                          .map((t) => `${t.symbol ?? t.mint ?? "?"}·${t.amount}`)
                          .join(", ") || "—"}
                      </span>
                      <span className="text-muted-foreground">NFTs</span>
                      <span className="text-foreground/85">
                        {(persona.seed.nfts ?? [])
                          .map((n) => `${n.collection}×${n.count}`)
                          .join(", ") || "—"}
                      </span>
                      {persona.note ? (
                        <>
                          <span className="text-muted-foreground">Note</span>
                          <span className="text-foreground/85">{persona.note}</span>
                        </>
                      ) : null}
                    </div>
                    <div className="mt-2 flex items-center gap-1.5">
                      <button
                        type="button"
                        onClick={() => void handleRefund(persona)}
                        disabled={busy || !clusterRunning}
                        className="inline-flex items-center gap-1 rounded-md border border-primary/40 bg-primary/10 px-2 py-0.5 text-[10.5px] text-primary hover:bg-primary/20 disabled:opacity-50"
                      >
                        <Sparkles className="h-3 w-3" />
                        Re-fund
                      </button>
                      <button
                        type="button"
                        onClick={() => void onDelete(persona.name)}
                        disabled={busy}
                        className="inline-flex items-center gap-1 rounded-md border border-border/60 bg-background/40 px-2 py-0.5 text-[10.5px] text-muted-foreground hover:border-destructive/60 hover:text-destructive disabled:opacity-50"
                      >
                        <Trash2 className="h-3 w-3" />
                        Delete
                      </button>
                    </div>
                  </div>
                ) : null}
              </li>
            )
          })}
        </ul>
      )}

      {lastReceipt ? (
        <div className="mt-2 rounded-md border border-border/50 bg-background/30 p-2">
          <div className="text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
            Last funding receipt · {lastReceipt.persona}
          </div>
          <ul className="mt-1 flex flex-col gap-0.5 text-[10.5px]">
            {lastReceipt.steps.map((step, idx) => (
              <li key={idx} className={cn(step.ok ? "text-foreground/85" : "text-destructive")}>
                {describeStep(step)}
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </section>
  )
}

function describeStep(step: FundingReceipt["steps"][number]): string {
  switch (step.kind) {
    case "airdrop":
      return `airdrop ${step.lamports} lamports${step.ok ? " ✓" : ` ✗ ${step.error ?? ""}`}`
    case "tokenMint":
      return `mint ${step.amount} of ${short(step.mint)}${step.ok ? " ✓" : ` ✗ ${step.error ?? ""}`}`
    case "tokenTransfer":
      return `transfer ${step.amount} of ${short(step.mint)}${step.ok ? " ✓" : ` ✗ ${step.error ?? ""}`}`
    case "nftFixture":
      return `nft ${step.collection}${step.ok ? " ✓" : ` ✗ ${step.error ?? ""}`}`
    default:
      return "unknown step"
  }
}

function short(value: string): string {
  return value.length > 10 ? `${value.slice(0, 4)}…${value.slice(-4)}` : value
}

function countFailures(receipt: FundingReceipt): number {
  return receipt.steps.filter((s) => !s.ok).length
}

function formatLamports(lamports: number): string {
  const sol = lamports / 1_000_000_000
  if (sol >= 1) return `${sol.toLocaleString("en-US", { maximumFractionDigits: 2 })}`
  return sol.toFixed(3)
}

function clusterLabel(cluster: ClusterKind): string {
  switch (cluster) {
    case "localnet":
      return "localnet"
    case "mainnet_fork":
      return "forked mainnet"
    case "devnet":
      return "devnet"
    case "mainnet":
      return "mainnet"
  }
}
