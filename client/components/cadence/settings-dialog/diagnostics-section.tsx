import { useMemo, useState } from "react"
import {
  AlertTriangle,
  CheckCircle2,
  Clipboard,
  CircleSlash,
  LoaderCircle,
  Play,
  RotateCcw,
  XCircle,
} from "lucide-react"
import type {
  DoctorReportRunStatus,
  OperatorActionErrorView,
} from "@/src/features/cadence/use-cadence-desktop-state"
import {
  cadenceDoctorReportSchema,
  renderCadenceDoctorReport,
  type CadenceDiagnosticCheckDto,
  type CadenceDiagnosticStatusDto,
  type CadenceDoctorReportDto,
  type RunDoctorReportRequestDto,
} from "@/src/lib/cadence-model"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { SectionHeader } from "./section-header"

interface DiagnosticsSectionProps {
  doctorReport: CadenceDoctorReportDto | null
  doctorReportStatus: DoctorReportRunStatus
  doctorReportError: OperatorActionErrorView | null
  onRunDoctorReport?: (request?: Partial<RunDoctorReportRequestDto>) => Promise<CadenceDoctorReportDto>
}

type CheckGroupKey =
  | "dictationChecks"
  | "profileChecks"
  | "modelCatalogChecks"
  | "runtimeSupervisorChecks"
  | "mcpDependencyChecks"
  | "settingsDependencyChecks"

const CHECK_GROUPS: Array<{ key: CheckGroupKey; label: string }> = [
  { key: "dictationChecks", label: "Dictation" },
  { key: "profileChecks", label: "Provider profiles" },
  { key: "modelCatalogChecks", label: "Model catalogs" },
  { key: "runtimeSupervisorChecks", label: "Runtime supervisor" },
  { key: "mcpDependencyChecks", label: "MCP dependencies" },
  { key: "settingsDependencyChecks", label: "Settings dependencies" },
]

const STATUS_LABEL: Record<CadenceDiagnosticStatusDto, string> = {
  passed: "Passed",
  warning: "Warning",
  failed: "Failed",
  skipped: "Skipped",
}

const STATUS_ICON = {
  passed: CheckCircle2,
  warning: AlertTriangle,
  failed: XCircle,
  skipped: CircleSlash,
} satisfies Record<CadenceDiagnosticStatusDto, React.ElementType>

const STATUS_CLASS: Record<CadenceDiagnosticStatusDto, string> = {
  passed: "text-emerald-600 dark:text-emerald-400",
  warning: "text-amber-700 dark:text-amber-300",
  failed: "text-destructive",
  skipped: "text-muted-foreground",
}

export function DiagnosticsSection({
  doctorReport,
  doctorReportStatus,
  doctorReportError,
  onRunDoctorReport,
}: DiagnosticsSectionProps) {
  const [copied, setCopied] = useState(false)
  const parsedReport = useMemo(() => {
    if (!doctorReport) return null
    return cadenceDoctorReportSchema.safeParse(doctorReport)
  }, [doctorReport])
  const report = parsedReport?.success ? parsedReport.data : null
  const isRunning = doctorReportStatus === "running"
  const canRun = Boolean(onRunDoctorReport) && !isRunning

  const runReport = (mode: RunDoctorReportRequestDto["mode"]) => {
    void onRunDoctorReport?.({ mode }).catch(() => undefined)
  }

  const copyReport = () => {
    if (!report || typeof navigator === "undefined" || !navigator.clipboard) return
    void navigator.clipboard.writeText(renderCadenceDoctorReport(report, "json")).then(() => {
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1600)
    })
  }

  return (
    <div className="flex flex-col gap-7">
      <SectionHeader
        title="Diagnostics"
        description="Run local provider, runtime, MCP, and settings checks without exposing secrets or local paths."
        actions={
          <>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-8 gap-1.5 text-[12px]"
              disabled={!canRun}
              onClick={() => runReport("quick_local")}
            >
              {isRunning ? <LoaderCircle className="h-3.5 w-3.5 animate-spin" /> : <RotateCcw className="h-3.5 w-3.5" />}
              Quick
            </Button>
            <Button
              type="button"
              size="sm"
              className="h-8 gap-1.5 text-[12px]"
              disabled={!canRun}
              onClick={() => runReport("extended_network")}
            >
              <Play className="h-3.5 w-3.5" />
              Extended
            </Button>
          </>
        }
      />

      {doctorReportError ? (
        <Alert variant="destructive" className="rounded-md px-3 py-2 text-[12px]">
          <AlertTriangle className="h-3.5 w-3.5" />
          <AlertTitle className="text-[12px]">Doctor report failed</AlertTitle>
          <AlertDescription className="text-[12px]">
            <p>{doctorReportError.message}</p>
            {doctorReportError.code ? <p className="font-mono text-[11px]">code: {doctorReportError.code}</p> : null}
          </AlertDescription>
        </Alert>
      ) : null}

      {parsedReport && !parsedReport.success ? (
        <Alert variant="destructive" className="rounded-md px-3 py-2 text-[12px]">
          <XCircle className="h-3.5 w-3.5" />
          <AlertTitle className="text-[12px]">Malformed report</AlertTitle>
          <AlertDescription className="text-[12px]">
            <p>The desktop backend returned diagnostics that failed the shared contract.</p>
          </AlertDescription>
        </Alert>
      ) : null}

      {!report ? (
        <div className="rounded-md border border-dashed border-border/60 bg-secondary/10 px-4 py-10 text-center">
          {isRunning ? (
            <LoaderCircle className="mx-auto h-4 w-4 animate-spin text-muted-foreground" />
          ) : (
            <AlertTriangle className="mx-auto h-4 w-4 text-muted-foreground" />
          )}
          <p className="mt-2 text-[12.5px] font-medium text-foreground">
            {isRunning ? "Running diagnostics" : "No doctor report yet"}
          </p>
          <p className="mt-0.5 text-[11.5px] text-muted-foreground">
            {isRunning ? "Checks are collecting current desktop state." : "Run a quick or extended report."}
          </p>
        </div>
      ) : (
        <>
          <section className="flex flex-col gap-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <h4 className="text-[12.5px] font-semibold text-foreground">Report summary</h4>
                <p className="mt-0.5 text-[11.5px] text-muted-foreground">
                  {report.generatedAt} - {report.mode === "quick_local" ? "Quick local" : "Extended network"}
                </p>
              </div>
              <Button
                type="button"
                variant="outline"
                size="sm"
                className="h-8 gap-1.5 text-[12px]"
                onClick={copyReport}
              >
                <Clipboard className="h-3.5 w-3.5" />
                {copied ? "Copied" : "Copy JSON"}
              </Button>
            </div>

            <div className="grid grid-cols-2 gap-2 sm:grid-cols-5">
              <SummaryPill label="Passed" value={report.summary.passed} tone="passed" />
              <SummaryPill label="Warnings" value={report.summary.warnings} tone="warning" />
              <SummaryPill label="Failed" value={report.summary.failed} tone="failed" />
              <SummaryPill label="Skipped" value={report.summary.skipped} tone="skipped" />
              <SummaryPill label="Total" value={report.summary.total} tone="total" />
            </div>
          </section>

          <section className="flex flex-col gap-3">
            {CHECK_GROUPS.map(({ key, label }) => (
              <CheckGroup key={key} label={label} checks={report[key]} />
            ))}
          </section>
        </>
      )}
    </div>
  )
}

function SummaryPill({
  label,
  value,
  tone,
}: {
  label: string
  value: number
  tone: CadenceDiagnosticStatusDto | "total"
}) {
  return (
    <div className="rounded-md border border-border/60 px-3 py-2">
      <p className="text-[10.5px] font-medium text-muted-foreground">{label}</p>
      <p
        className={cn(
          "mt-0.5 text-[17px] font-semibold tabular-nums text-foreground",
          tone !== "total" ? STATUS_CLASS[tone] : null,
        )}
      >
        {value}
      </p>
    </div>
  )
}

function CheckGroup({
  label,
  checks,
}: {
  label: string
  checks: CadenceDiagnosticCheckDto[]
}) {
  return (
    <div className="flex flex-col gap-2.5">
      <div className="flex items-center justify-between gap-3">
        <h4 className="text-[12.5px] font-semibold text-foreground">
          {label}
          <span className="ml-1.5 font-normal text-muted-foreground">{checks.length}</span>
        </h4>
      </div>
      {checks.length === 0 ? (
        <p className="rounded-md border border-dashed border-border/60 bg-secondary/10 px-3 py-3 text-center text-[12px] text-muted-foreground">
          No checks returned.
        </p>
      ) : (
        <div className="overflow-hidden rounded-md border border-border/60 divide-y divide-border/40">
          {checks.map((check) => (
            <CheckRow key={check.checkId} check={check} />
          ))}
        </div>
      )}
    </div>
  )
}

function CheckRow({ check }: { check: CadenceDiagnosticCheckDto }) {
  const Icon = STATUS_ICON[check.status]

  return (
    <div className="px-3.5 py-3">
      <div className="flex items-start gap-2.5">
        <Icon className={cn("mt-0.5 h-3.5 w-3.5 shrink-0", STATUS_CLASS[check.status])} />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-1.5">
            <p className="text-[12.5px] font-medium text-foreground">{check.message}</p>
            <Badge variant="outline" className="h-5 px-1.5 text-[10.5px]">
              {STATUS_LABEL[check.status]}
            </Badge>
          </div>
          <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
            <span className="font-mono">{check.code}</span>
            {check.affectedProviderId ? <span>Provider {check.affectedProviderId}</span> : null}
            {check.affectedProfileId ? <span>Profile {check.affectedProfileId}</span> : null}
            {check.retryable ? <span>Retryable</span> : null}
            {check.redacted ? <span>Redacted</span> : null}
          </div>
          {check.remediation ? (
            <p className="mt-1.5 rounded-md border border-border/50 bg-secondary/20 px-2.5 py-1.5 text-[11.5px] leading-[1.45] text-muted-foreground">
              {check.remediation}
            </p>
          ) : null}
        </div>
      </div>
    </div>
  )
}
