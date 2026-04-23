"use client"

import { useCallback, useEffect, useRef, useState } from "react"
import { invoke, isTauri } from "@tauri-apps/api/core"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"
import { CheckCircle2, Download, ExternalLink, Loader2, RefreshCw, XCircle } from "lucide-react"
import { cn } from "@/lib/utils"
import type { EmulatorPlatform } from "@/src/features/emulator/use-emulator-session"

interface SdkStatus {
  android: {
    present: boolean
    sdkRoot: string | null
    emulatorPath: string | null
    adbPath: string | null
    avdmanagerPath: string | null
  }
  ios: {
    present: boolean
    xcrunPath: string | null
    simctlPath: string | null
    idbCompanionPresent: boolean
    supported: boolean
  }
}

type ProvisionPhase =
  | "starting"
  | "ensuring_java"
  | "downloading_java"
  | "extracting_java"
  | "downloading_cmdline_tools"
  | "extracting_cmdline_tools"
  | "accepting_licenses"
  | "installing_packages"
  | "creating_avd"
  | "completed"
  | "failed"

interface ProvisionEvent {
  phase: ProvisionPhase
  message: string | null
  progress: number | null
  error: string | null
}

interface ProvisionState {
  phase: ProvisionPhase
  message: string | null
  progress: number | null
  error: string | null
  active: boolean
}

const IDLE_STATE: ProvisionState = {
  phase: "starting",
  message: null,
  progress: null,
  error: null,
  active: false,
}

const PROVISION_EVENT = "emulator:android_provision"
const SDK_STATUS_CHANGED_EVENT = "emulator:sdk_status_changed"

const PHASE_LABELS: Record<ProvisionPhase, string> = {
  starting: "Starting",
  ensuring_java: "Checking Java runtime",
  downloading_java: "Downloading Java runtime",
  extracting_java: "Unpacking Java runtime",
  downloading_cmdline_tools: "Downloading Android tools",
  extracting_cmdline_tools: "Unpacking Android tools",
  accepting_licenses: "Accepting SDK licenses",
  installing_packages: "Installing platform-tools + emulator + system image",
  creating_avd: "Creating default AVD",
  completed: "Finished",
  failed: "Failed",
}

interface Props {
  platform: EmulatorPlatform
  onDismiss?: () => void
}

/// Panel shown above the device picker when the host is missing the
/// necessary SDK. Distinct states:
/// - Android: either or both of `adb` / `emulator` not found. Offers
///   first-run provisioning that downloads cmdline-tools, a Temurin
///   JRE (when needed), platform-tools, the emulator, a system image,
///   and a default AVD into the app's data dir.
/// - iOS (macOS): Xcode / `xcrun` not found.
/// - iOS (non-macOS): hidden entirely — the shell already hides the
///   titlebar button on those hosts.
export function EmulatorMissingSdk({ platform, onDismiss }: Props) {
  const [status, setStatus] = useState<SdkStatus | null>(null)
  const [isProbing, setIsProbing] = useState(false)
  const [provision, setProvision] = useState<ProvisionState>(IDLE_STATE)
  const onDismissRef = useRef(onDismiss)
  onDismissRef.current = onDismiss

  const probe = useCallback(async () => {
    if (!isTauri()) return
    setIsProbing(true)
    try {
      const next = await invoke<SdkStatus>("emulator_sdk_status")
      setStatus(next)
    } catch {
      setStatus(null)
    } finally {
      setIsProbing(false)
    }
  }, [])

  useEffect(() => {
    void probe()
  }, [probe])

  // Subscribe to provisioning events so the panel reflects backend
  // progress even if the user navigates away and back. Also refresh
  // probe status when the backend signals SDK discovery changed.
  useEffect(() => {
    if (!isTauri()) return
    let cancelled = false
    const unlisten: UnlistenFn[] = []

    void listen<ProvisionEvent>(PROVISION_EVENT, (event) => {
      if (cancelled) return
      const payload = event.payload
      setProvision((prev) => ({
        phase: payload.phase,
        message: payload.message ?? prev.message,
        progress: payload.progress ?? null,
        error: payload.error,
        active: payload.phase !== "completed" && payload.phase !== "failed",
      }))
      if (payload.phase === "completed") {
        void probe().then(() => {
          if (!cancelled) {
            onDismissRef.current?.()
          }
        })
      }
    }).then((fn) => unlisten.push(fn))

    void listen(SDK_STATUS_CHANGED_EVENT, () => {
      if (!cancelled) void probe()
    }).then((fn) => unlisten.push(fn))

    return () => {
      cancelled = true
      unlisten.forEach((fn) => fn())
    }
  }, [probe])

  const provisionStart = useCallback(async () => {
    if (!isTauri()) return
    setProvision({ ...IDLE_STATE, active: true, phase: "starting" })
    try {
      await invoke("emulator_android_provision")
    } catch (err) {
      const message = errorMessage(err)
      setProvision({
        phase: "failed",
        message: null,
        progress: null,
        error: message,
        active: false,
      })
    }
  }, [])

  if (!status) return null

  const shouldShowProvisionStream = platform === "android" && provision.active
  if (shouldShowProvisionStream) {
    return <ProvisionProgressCard state={provision} />
  }

  if (platform === "android") {
    const panel = androidPanel(status)
    if (!panel) return null
    return (
      <AndroidMissingCard
        failure={provision.error}
        isProbing={isProbing}
        onDismiss={onDismiss}
        onProbe={probe}
        onProvision={provisionStart}
        panel={panel}
      />
    )
  }

  const panel = iosPanel(status)
  if (!panel) return null
  return (
    <PanelCard
      actions={panel.actions}
      detail={panel.detail}
      isProbing={isProbing}
      onDismiss={onDismiss}
      onProbe={probe}
      title={panel.title}
    />
  )
}

function errorMessage(err: unknown): string {
  if (err && typeof err === "object" && "message" in err) {
    const message = (err as { message?: unknown }).message
    if (typeof message === "string" && message.length > 0) return message
  }
  if (typeof err === "string" && err.length > 0) return err
  return "Android SDK provisioning failed"
}

interface AndroidPanel {
  title: string
  detail: string
  actions: Array<{ label: string; href: string }>
}

function androidPanel(status: SdkStatus): AndroidPanel | null {
  if (status.android.present) return null

  return {
    title: "Android SDK not set up",
    detail:
      "Cadence can install the Android SDK (command-line tools, emulator, platform-tools, and a default API 34 image) into the app's data directory. Expect a one-time ~1.5 GB download and ~5 minutes.",
    actions: [
      {
        label: "About the Android SDK",
        href: "https://developer.android.com/tools",
      },
    ],
  }
}

function iosPanel(status: SdkStatus): AndroidPanel | null {
  if (!status.ios.supported) return null
  if (status.ios.present && status.ios.idbCompanionPresent) return null

  if (!status.ios.present) {
    return {
      title: "Xcode command-line tools not found",
      detail:
        "Cadence needs Xcode installed so the iOS Simulator framework is available. Run `xcode-select --install` after installing Xcode to finish the setup.",
      actions: [
        { label: "Install Xcode", href: "https://apps.apple.com/app/xcode/id497799835" },
      ],
    }
  }

  return {
    title: "idb_companion sidecar missing",
    detail:
      "The bundled idb_companion helper is not present in this build. Packaged installers include it automatically; for a dev build, rebuild without CADENCE_SKIP_SIDECAR_FETCH, or install it via Homebrew so it resolves from PATH.",
    actions: [
      { label: "Install via Homebrew", href: "https://github.com/facebook/idb" },
    ],
  }
}

function AndroidMissingCard({
  failure,
  isProbing,
  onDismiss,
  onProbe,
  onProvision,
  panel,
}: {
  failure: string | null
  isProbing: boolean
  onDismiss?: () => void
  onProbe: () => void
  onProvision: () => void
  panel: AndroidPanel
}) {
  return (
    <div
      aria-live="polite"
      className="flex shrink-0 flex-col gap-2 border-b border-border/60 bg-amber-500/10 px-3 py-2 text-[11px] leading-relaxed"
      role="region"
    >
      <div className="font-medium text-amber-200">{panel.title}</div>
      <div className="text-muted-foreground">{panel.detail}</div>
      {failure ? (
        <div className="flex items-start gap-1.5 rounded-md border border-red-500/40 bg-red-500/10 px-2 py-1 text-red-200">
          <XCircle className="mt-[2px] h-3 w-3 shrink-0" />
          <span className="break-words">{failure}</span>
        </div>
      ) : null}
      <div className="flex flex-wrap items-center gap-2">
        <button
          className={cn(
            "inline-flex items-center gap-1 rounded-md border border-amber-500/60 bg-amber-500/20 px-2 py-0.5",
            "font-medium text-[11px] text-amber-100 transition-colors hover:border-amber-400 hover:bg-amber-500/30",
          )}
          onClick={onProvision}
          type="button"
        >
          <Download className="h-3 w-3" />
          Set up Android (~1.5 GB, ~5 min)
        </button>
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
          aria-label="Re-detect SDK"
          className="inline-flex items-center gap-1 rounded-md border border-border/70 bg-background/60 px-2 py-0.5 text-[11px] text-foreground transition-colors hover:border-primary/50 hover:text-primary disabled:opacity-60"
          disabled={isProbing}
          onClick={onProbe}
          type="button"
        >
          <RefreshCw className={cn("h-3 w-3", isProbing && "animate-spin")} />
          Re-detect
        </button>
        {onDismiss ? (
          <button
            className="ml-auto text-[11px] text-muted-foreground/80 underline-offset-2 hover:text-foreground hover:underline"
            onClick={onDismiss}
            type="button"
          >
            Dismiss
          </button>
        ) : null}
      </div>
    </div>
  )
}

function PanelCard({
  actions,
  detail,
  isProbing,
  onDismiss,
  onProbe,
  title,
}: {
  actions: Array<{ label: string; href: string }>
  detail: string
  isProbing: boolean
  onDismiss?: () => void
  onProbe: () => void
  title: string
}) {
  return (
    <div
      aria-live="polite"
      className="flex shrink-0 flex-col gap-2 border-b border-border/60 bg-amber-500/10 px-3 py-2 text-[11px] leading-relaxed"
      role="region"
    >
      <div className="font-medium text-amber-200">{title}</div>
      <div className="text-muted-foreground">{detail}</div>
      <div className="flex flex-wrap items-center gap-2">
        {actions.map((action) => (
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
          aria-label="Re-detect SDK"
          className="inline-flex items-center gap-1 rounded-md border border-border/70 bg-background/60 px-2 py-0.5 text-[11px] text-foreground transition-colors hover:border-primary/50 hover:text-primary disabled:opacity-60"
          disabled={isProbing}
          onClick={onProbe}
          type="button"
        >
          <RefreshCw className={cn("h-3 w-3", isProbing && "animate-spin")} />
          Re-detect
        </button>
        {onDismiss ? (
          <button
            className="ml-auto text-[11px] text-muted-foreground/80 underline-offset-2 hover:text-foreground hover:underline"
            onClick={onDismiss}
            type="button"
          >
            Dismiss
          </button>
        ) : null}
      </div>
    </div>
  )
}

function ProvisionProgressCard({ state }: { state: ProvisionState }) {
  const label = PHASE_LABELS[state.phase] ?? state.phase
  const percent = state.progress != null ? Math.round(state.progress * 100) : null

  const completed = state.phase === "completed"
  const failed = state.phase === "failed"

  return (
    <div
      aria-live="polite"
      className={cn(
        "flex shrink-0 flex-col gap-2 border-b px-3 py-2 text-[11px] leading-relaxed",
        failed
          ? "border-red-500/40 bg-red-500/10"
          : completed
            ? "border-emerald-500/40 bg-emerald-500/10"
            : "border-border/60 bg-amber-500/10",
      )}
      role="region"
    >
      <div className="flex items-center gap-2">
        {failed ? (
          <XCircle className="h-3.5 w-3.5 text-red-300" />
        ) : completed ? (
          <CheckCircle2 className="h-3.5 w-3.5 text-emerald-300" />
        ) : (
          <Loader2 className="h-3.5 w-3.5 animate-spin text-amber-200" />
        )}
        <span className="font-medium text-foreground">
          {failed ? "Provisioning failed" : completed ? "Provisioning complete" : label}
        </span>
        {percent != null && !completed && !failed ? (
          <span className="text-muted-foreground">{percent}%</span>
        ) : null}
      </div>
      {state.message ? (
        <div className="truncate text-muted-foreground" title={state.message}>
          {state.message}
        </div>
      ) : null}
      {state.error ? (
        <div className="break-words text-red-200" title={state.error}>
          {state.error}
        </div>
      ) : null}
      <div className="h-1.5 w-full overflow-hidden rounded-full bg-border/70">
        <div
          className={cn(
            "h-full transition-[width] duration-200",
            failed
              ? "bg-red-400"
              : completed
                ? "bg-emerald-400"
                : percent != null
                  ? "bg-amber-300"
                  : "animate-pulse bg-amber-400/60",
          )}
          style={{ width: percent != null ? `${percent}%` : "100%" }}
        />
      </div>
    </div>
  )
}
