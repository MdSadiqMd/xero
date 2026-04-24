"use client"

import { useCallback, useMemo, useState } from "react"
import { AlertCircle, CheckCircle2, Gauge, Loader2, Search } from "lucide-react"
import { cn } from "@/lib/utils"
import type {
  ClusterKind,
  FeeEstimate,
  SimulateRequest,
  SimulationResult,
  TxResult,
} from "@/src/features/solana/use-solana-workbench"

interface SolanaTxInspectorProps {
  cluster: ClusterKind
  clusterRunning: boolean
  txBusy: boolean
  lastSimulation: SimulationResult | null
  lastExplanation: TxResult | null
  onSimulate: (request: SimulateRequest) => Promise<SimulationResult | null>
  onExplain: (signature: string) => Promise<TxResult | null>
  onEstimateFee: (programIds: string[]) => Promise<FeeEstimate | null>
}

type InspectorTab = "simulate" | "explain" | "priority"

export function SolanaTxInspector({
  cluster,
  clusterRunning,
  txBusy,
  lastSimulation,
  lastExplanation,
  onSimulate,
  onExplain,
  onEstimateFee,
}: SolanaTxInspectorProps) {
  const [tab, setTab] = useState<InspectorTab>("simulate")
  const [txBytes, setTxBytes] = useState("")
  const [signature, setSignature] = useState("")
  const [programIds, setProgramIds] = useState("")
  const [feeResult, setFeeResult] = useState<FeeEstimate | null>(null)
  const [localError, setLocalError] = useState<string | null>(null)

  const handleSimulate = useCallback(async () => {
    if (!txBytes.trim()) {
      setLocalError("Paste a base64-encoded v0 transaction to simulate.")
      return
    }
    setLocalError(null)
    await onSimulate({
      cluster,
      transactionBase64: txBytes.trim(),
      skipReplaceBlockhash: false,
    })
  }, [cluster, onSimulate, txBytes])

  const handleExplain = useCallback(async () => {
    if (!signature.trim()) {
      setLocalError("Enter a transaction signature to decode.")
      return
    }
    setLocalError(null)
    await onExplain(signature.trim())
  }, [onExplain, signature])

  const handleEstimateFee = useCallback(async () => {
    setLocalError(null)
    const ids = programIds
      .split(/[\s,]+/)
      .map((s) => s.trim())
      .filter(Boolean)
    const result = await onEstimateFee(ids)
    setFeeResult(result)
  }, [onEstimateFee, programIds])

  const disabled = txBusy || !clusterRunning

  return (
    <section className="border-b border-border/70 px-3 py-3">
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Search aria-hidden className="h-3 w-3 text-muted-foreground" />
          <div className="text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
            Tx inspector
          </div>
        </div>
        {txBusy ? <Loader2 className="h-3 w-3 animate-spin text-primary" /> : null}
      </div>

      <div className="mb-2 flex gap-1">
        <TabButton
          active={tab === "simulate"}
          onClick={() => setTab("simulate")}
          label="Simulate"
        />
        <TabButton
          active={tab === "explain"}
          onClick={() => setTab("explain")}
          label="Explain"
        />
        <TabButton
          active={tab === "priority"}
          onClick={() => setTab("priority")}
          label="Priority fee"
        />
      </div>

      {!clusterRunning ? (
        <p className="mb-2 rounded-md bg-muted/40 px-2 py-1.5 text-[10.5px] text-muted-foreground">
          Start a cluster on{" "}
          <span className="font-mono">{cluster}</span> to use the tx inspector.
        </p>
      ) : null}

      {tab === "simulate" ? (
        <div className="space-y-2">
          <label className="block text-[10.5px] text-muted-foreground">
            Base64 v0 transaction
          </label>
          <textarea
            className="w-full resize-y rounded-md border border-border/70 bg-background/60 p-2 font-mono text-[10.5px] leading-tight"
            placeholder="AQABAv..."
            rows={4}
            value={txBytes}
            onChange={(event) => setTxBytes(event.target.value)}
          />
          <button
            type="button"
            disabled={disabled}
            onClick={handleSimulate}
            className={cn(
              "inline-flex items-center gap-1.5 rounded-md border border-primary/50 bg-primary/15 px-2.5 py-1 text-[11px] font-medium text-primary transition-colors",
              "hover:bg-primary/25 disabled:opacity-50",
            )}
          >
            {txBusy ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              <Gauge className="h-3 w-3" />
            )}
            Simulate
          </button>
          {lastSimulation ? (
            <SimulationSummary result={lastSimulation} />
          ) : null}
        </div>
      ) : null}

      {tab === "explain" ? (
        <div className="space-y-2">
          <label className="block text-[10.5px] text-muted-foreground">
            Signature
          </label>
          <input
            type="text"
            value={signature}
            onChange={(event) => setSignature(event.target.value)}
            placeholder="4Ck7..."
            className="w-full rounded-md border border-border/70 bg-background/60 px-2 py-1 font-mono text-[10.5px]"
          />
          <button
            type="button"
            disabled={disabled}
            onClick={handleExplain}
            className={cn(
              "inline-flex items-center gap-1.5 rounded-md border border-primary/50 bg-primary/15 px-2.5 py-1 text-[11px] font-medium text-primary transition-colors",
              "hover:bg-primary/25 disabled:opacity-50",
            )}
          >
            {txBusy ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              <Search className="h-3 w-3" />
            )}
            Decode
          </button>
          {lastExplanation ? (
            <TxResultSummary result={lastExplanation} />
          ) : null}
        </div>
      ) : null}

      {tab === "priority" ? (
        <div className="space-y-2">
          <label className="block text-[10.5px] text-muted-foreground">
            Program IDs (comma or newline separated, optional)
          </label>
          <textarea
            className="w-full resize-y rounded-md border border-border/70 bg-background/60 p-2 font-mono text-[10.5px] leading-tight"
            placeholder="JUP6Lkb..., whirLb..."
            rows={2}
            value={programIds}
            onChange={(event) => setProgramIds(event.target.value)}
          />
          <button
            type="button"
            disabled={txBusy || !clusterRunning}
            onClick={handleEstimateFee}
            className={cn(
              "inline-flex items-center gap-1.5 rounded-md border border-primary/50 bg-primary/15 px-2.5 py-1 text-[11px] font-medium text-primary transition-colors",
              "hover:bg-primary/25 disabled:opacity-50",
            )}
          >
            {txBusy ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              <Gauge className="h-3 w-3" />
            )}
            Estimate
          </button>
          {feeResult ? <FeeEstimateSummary estimate={feeResult} /> : null}
        </div>
      ) : null}

      {localError ? (
        <p className="mt-2 text-[11px] text-destructive">{localError}</p>
      ) : null}
    </section>
  )
}

function TabButton({
  active,
  onClick,
  label,
}: {
  active: boolean
  onClick: () => void
  label: string
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "rounded-md px-2 py-0.5 text-[10.5px] transition-colors",
        active
          ? "bg-primary/15 text-primary"
          : "text-muted-foreground hover:text-foreground",
      )}
    >
      {label}
    </button>
  )
}

function SimulationSummary({ result }: { result: SimulationResult }) {
  const logs = useMemo(() => result.logs.slice(-12), [result.logs])
  return (
    <div className="rounded-md border border-border/60 bg-background/40 p-2">
      <div className="mb-1 flex items-center gap-1.5 text-[10.5px]">
        {result.success ? (
          <CheckCircle2 className="h-3 w-3 text-emerald-400" />
        ) : (
          <AlertCircle className="h-3 w-3 text-destructive" />
        )}
        <span className="font-medium">{result.explanation.summary}</span>
      </div>
      {result.computeUnitsConsumed != null ? (
        <div className="text-[10.5px] text-muted-foreground">
          CU consumed:{" "}
          <span className="font-mono tabular-nums">
            {result.computeUnitsConsumed}
          </span>
        </div>
      ) : null}
      {result.affectedAccounts.length > 0 ? (
        <div className="mt-1 text-[10.5px] text-muted-foreground">
          Accounts: {result.affectedAccounts.length}
        </div>
      ) : null}
      {logs.length > 0 ? (
        <pre className="mt-2 max-h-28 overflow-auto whitespace-pre-wrap break-all rounded bg-muted/40 p-1.5 font-mono text-[10px] leading-snug">
          {logs.join("\n")}
        </pre>
      ) : null}
    </div>
  )
}

function TxResultSummary({ result }: { result: TxResult }) {
  return (
    <div className="rounded-md border border-border/60 bg-background/40 p-2">
      <div className="mb-1 flex items-center gap-1.5 text-[10.5px]">
        {result.explanation.ok ? (
          <CheckCircle2 className="h-3 w-3 text-emerald-400" />
        ) : (
          <AlertCircle className="h-3 w-3 text-destructive" />
        )}
        <span className="font-medium">{result.explanation.summary}</span>
      </div>
      <div className="text-[10.5px] text-muted-foreground">
        Signature:{" "}
        <span className="font-mono tabular-nums">{result.signature}</span>
      </div>
      {result.slot != null ? (
        <div className="text-[10.5px] text-muted-foreground">
          Slot: <span className="font-mono tabular-nums">{result.slot}</span>
        </div>
      ) : null}
      {result.confirmation ? (
        <div className="text-[10.5px] text-muted-foreground">
          Status: {result.confirmation}
        </div>
      ) : null}
      {result.logs.length > 0 ? (
        <pre className="mt-2 max-h-28 overflow-auto whitespace-pre-wrap break-all rounded bg-muted/40 p-1.5 font-mono text-[10px] leading-snug">
          {result.logs.slice(-12).join("\n")}
        </pre>
      ) : null}
    </div>
  )
}

function FeeEstimateSummary({ estimate }: { estimate: FeeEstimate }) {
  return (
    <div className="rounded-md border border-border/60 bg-background/40 p-2">
      <div className="mb-1 text-[10.5px]">
        Recommended:{" "}
        <span className="font-mono tabular-nums">
          {estimate.recommendedMicroLamports}
        </span>{" "}
        µ-lamports / CU ({estimate.recommendedPercentile})
      </div>
      <div className="text-[10.5px] text-muted-foreground">
        {estimate.samples.length} sample(s) from {estimate.source}
      </div>
      <div className="mt-1 grid grid-cols-5 gap-1 text-[10px]">
        {estimate.percentiles.map((p) => (
          <div
            key={p.percentile}
            className="rounded bg-muted/40 px-1 py-0.5 text-center font-mono tabular-nums"
          >
            <div className="text-[9px] uppercase text-muted-foreground">
              {p.percentile}
            </div>
            <div>{p.microLamports}</div>
          </div>
        ))}
      </div>
    </div>
  )
}
