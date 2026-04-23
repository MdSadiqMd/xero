"use client"

import { useCallback, useEffect, useMemo, useState } from "react"
import { ChevronRight, Loader2, PlayCircle, Zap } from "lucide-react"
import { cn } from "@/lib/utils"
import type {
  ClusterKind,
  Persona,
  ScenarioDescriptor,
  ScenarioRun,
  ScenarioSpec,
} from "@/src/features/solana/use-solana-workbench"

interface SolanaScenarioPanelProps {
  cluster: ClusterKind
  personas: Persona[]
  scenarios: ScenarioDescriptor[]
  busy: boolean
  lastRun: ScenarioRun | null
  clusterRunning: boolean
  onRunScenario: (spec: ScenarioSpec) => Promise<ScenarioRun | null>
}

export function SolanaScenarioPanel({
  cluster,
  personas,
  scenarios,
  busy,
  lastRun,
  clusterRunning,
  onRunScenario,
}: SolanaScenarioPanelProps) {
  const applicableScenarios = useMemo(
    () => scenarios.filter((s) => s.supportedClusters.includes(cluster)),
    [scenarios, cluster],
  )

  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [selectedPersona, setSelectedPersona] = useState<string | null>(null)

  useEffect(() => {
    // Default selection: first applicable scenario.
    if (!selectedId || !applicableScenarios.some((s) => s.id === selectedId)) {
      setSelectedId(applicableScenarios[0]?.id ?? null)
    }
  }, [applicableScenarios, selectedId])

  useEffect(() => {
    // Default persona: first persona whose role matches the scenario's
    // required roles, falling back to the first persona.
    const scenario = applicableScenarios.find((s) => s.id === selectedId)
    if (!scenario) {
      setSelectedPersona(null)
      return
    }
    const matching =
      personas.find((p) => scenario.requiredRoles.includes(p.role)) ?? personas[0]
    setSelectedPersona(matching?.name ?? null)
  }, [applicableScenarios, selectedId, personas])

  const selectedScenario = useMemo(
    () => applicableScenarios.find((s) => s.id === selectedId) ?? null,
    [applicableScenarios, selectedId],
  )

  const handleRun = useCallback(async () => {
    if (!selectedScenario || !selectedPersona) return
    await onRunScenario({
      id: selectedScenario.id,
      cluster,
      persona: selectedPersona,
      params: {},
    })
  }, [selectedScenario, selectedPersona, onRunScenario, cluster])

  return (
    <section className="border-b border-border/70 px-3 py-3">
      <div className="mb-2 flex items-center gap-2">
        <Zap className="h-3.5 w-3.5 text-muted-foreground" />
        <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
          Scenarios
        </span>
      </div>

      {applicableScenarios.length === 0 ? (
        <p className="text-[11px] text-muted-foreground">
          No scenarios available on {cluster}. Switch clusters to see available runbooks.
        </p>
      ) : (
        <div className="flex flex-col gap-1.5">
          {applicableScenarios.map((scenario) => {
            const selected = scenario.id === selectedId
            const kindLabel =
              scenario.kind === "self_contained" ? "runs now" : "needs TxPipeline"
            return (
              <button
                key={scenario.id}
                type="button"
                onClick={() => setSelectedId(scenario.id)}
                className={cn(
                  "group flex w-full flex-col items-start gap-0.5 rounded-md border px-2 py-1.5 text-left transition-colors",
                  selected
                    ? "border-primary/60 bg-primary/10"
                    : "border-border/60 bg-background/40 hover:border-primary/40",
                )}
              >
                <div className="flex w-full items-center gap-2">
                  <ChevronRight
                    className={cn(
                      "h-3 w-3 text-muted-foreground transition-transform",
                      selected && "rotate-90 text-primary",
                    )}
                  />
                  <span className="flex-1 truncate text-[11.5px] font-medium text-foreground">
                    {scenario.label}
                  </span>
                  <span
                    className={cn(
                      "rounded px-1.5 py-0.5 text-[9px] uppercase tracking-[0.12em]",
                      scenario.kind === "self_contained"
                        ? "bg-emerald-500/20 text-emerald-400"
                        : "bg-amber-500/20 text-amber-400",
                    )}
                  >
                    {kindLabel}
                  </span>
                </div>
                <p className="pl-5 pr-1 text-[10.5px] text-muted-foreground">
                  {scenario.description}
                </p>
              </button>
            )
          })}
        </div>
      )}

      {selectedScenario ? (
        <div className="mt-2 rounded-md border border-border/60 bg-background/40 p-2">
          <div className="text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
            Launch {selectedScenario.label}
          </div>
          <div className="mt-1.5 flex flex-col gap-1.5">
            <select
              aria-label="Persona"
              className="rounded-md border border-border/60 bg-background/80 px-2 py-1 text-[12px] outline-none focus:border-primary/60 disabled:opacity-50"
              disabled={personas.length === 0}
              onChange={(event) => setSelectedPersona(event.target.value)}
              value={selectedPersona ?? ""}
            >
              {personas.length === 0 ? (
                <option value="">No personas on this cluster</option>
              ) : null}
              {personas.map((persona) => (
                <option key={persona.name} value={persona.name}>
                  {persona.name} · {persona.role}
                </option>
              ))}
            </select>
            {selectedScenario.requiredClonePrograms.length > 0 ? (
              <div className="text-[10.5px] text-muted-foreground">
                Clone programs:{" "}
                {selectedScenario.requiredClonePrograms
                  .map((p) => p.slice(0, 4) + "…" + p.slice(-4))
                  .join(", ")}
              </div>
            ) : null}
            <button
              type="button"
              onClick={handleRun}
              disabled={!selectedPersona || busy || !clusterRunning}
              className={cn(
                "inline-flex items-center justify-center gap-1.5 rounded-md border border-primary/50 bg-primary/15 px-2.5 py-1 text-[11px] font-medium text-primary transition-colors",
                "hover:bg-primary/25 disabled:opacity-50",
              )}
            >
              {busy ? (
                <Loader2 className="h-3 w-3 animate-spin" />
              ) : (
                <PlayCircle className="h-3 w-3" />
              )}
              Run scenario
            </button>
            {!clusterRunning ? (
              <p className="text-[10.5px] text-muted-foreground">
                Start the {cluster} cluster first — scenarios require an active RPC URL.
              </p>
            ) : null}
          </div>
        </div>
      ) : null}

      {lastRun ? (
        <div className="mt-2 rounded-md border border-border/50 bg-background/30 p-2">
          <div className="flex items-center justify-between">
            <div className="text-[10px] uppercase tracking-[0.14em] text-muted-foreground">
              Last run · {lastRun.id}
            </div>
            <span
              className={cn(
                "rounded px-1.5 py-0.5 text-[9px] uppercase tracking-[0.12em]",
                statusColor(lastRun.status),
              )}
            >
              {lastRun.status}
            </span>
          </div>
          {lastRun.pipelineHint ? (
            <p className="mt-1 text-[10.5px] text-amber-400/90">{lastRun.pipelineHint}</p>
          ) : null}
          <div className="mt-1.5 flex flex-col gap-0.5 text-[10.5px] text-foreground/80">
            {lastRun.steps.map((step, idx) => (
              <span key={idx}>· {step}</span>
            ))}
          </div>
          {lastRun.signatures.length > 0 ? (
            <div className="mt-1 text-[10px] text-muted-foreground">
              {lastRun.signatures.length} signature(s) collected
            </div>
          ) : null}
        </div>
      ) : null}
    </section>
  )
}

function statusColor(status: ScenarioRun["status"]): string {
  switch (status) {
    case "succeeded":
      return "bg-emerald-500/20 text-emerald-400"
    case "failed":
      return "bg-destructive/20 text-destructive"
    case "pendingPipeline":
      return "bg-amber-500/20 text-amber-400"
  }
}
