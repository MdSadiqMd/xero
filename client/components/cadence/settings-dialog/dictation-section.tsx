import { useCallback, useEffect, useMemo, useState } from "react"
import type React from "react"
import { openUrl } from "@tauri-apps/plugin-opener"
import { AlertTriangle, CheckCircle2, LoaderCircle, Mic, RotateCcw, Settings } from "lucide-react"

import type { CadenceDesktopAdapter } from "@/src/lib/cadence-desktop"
import type {
  DictationEnginePreferenceDto,
  DictationPermissionStateDto,
  DictationPrivacyModeDto,
  DictationSettingsDto,
  DictationStatusDto,
  UpsertDictationSettingsRequestDto,
} from "@/src/lib/cadence-model"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { cn } from "@/lib/utils"
import { SectionHeader } from "./section-header"

export type DictationSettingsAdapter = Pick<
  CadenceDesktopAdapter,
  | "isDesktopRuntime"
  | "speechDictationStatus"
  | "speechDictationSettings"
  | "speechDictationUpdateSettings"
>

interface DictationSectionProps {
  adapter?: DictationSettingsAdapter
}

const SYSTEM_LOCALE_VALUE = "__system__"

const DEFAULT_SETTINGS: DictationSettingsDto = {
  enginePreference: "automatic",
  privacyMode: "on_device_preferred",
  locale: null,
  updatedAt: null,
}

const ENGINE_OPTIONS: Array<{ value: DictationEnginePreferenceDto; label: string; detail: string }> = [
  { value: "automatic", label: "Automatic", detail: "Use modern dictation when available" },
  { value: "modern", label: "Prefer macOS 26 Dictation", detail: "Require the modern SpeechAnalyzer path first" },
  { value: "legacy", label: "Legacy only", detail: "Use SFSpeechRecognizer" },
]

const PRIVACY_OPTIONS: Array<{ value: DictationPrivacyModeDto; label: string; detail: string }> = [
  { value: "on_device_preferred", label: "On-device preferred", detail: "Try local recognition before asking for another path" },
  { value: "on_device_required", label: "On-device required", detail: "Never use Apple server recognition" },
  { value: "allow_network", label: "Allow Apple server recognition", detail: "Permit Apple recognition when local support is unavailable" },
]

export function DictationSection({ adapter }: DictationSectionProps) {
  const [status, setStatus] = useState<DictationStatusDto | null>(null)
  const [settings, setSettings] = useState<DictationSettingsDto | null>(null)
  const [loadState, setLoadState] = useState<"idle" | "loading" | "ready" | "error">("idle")
  const [saveState, setSaveState] = useState<"idle" | "saving">("idle")
  const [error, setError] = useState<string | null>(null)

  const canUseAdapter = Boolean(
    adapter?.isDesktopRuntime?.() &&
      adapter.speechDictationStatus &&
      adapter.speechDictationSettings &&
      adapter.speechDictationUpdateSettings,
  )

  const load = useCallback(() => {
    if (!canUseAdapter || !adapter?.speechDictationStatus || !adapter.speechDictationSettings) {
      setLoadState("ready")
      setStatus(null)
      setSettings(DEFAULT_SETTINGS)
      return
    }

    setLoadState("loading")
    setError(null)
    Promise.all([adapter.speechDictationStatus(), adapter.speechDictationSettings()])
      .then(([nextStatus, nextSettings]) => {
        setStatus(nextStatus)
        setSettings(nextSettings)
        setLoadState("ready")
      })
      .catch((loadError) => {
        setError(getErrorMessage(loadError, "Cadence could not load dictation settings."))
        setLoadState("error")
      })
  }, [adapter, canUseAdapter])

  useEffect(() => {
    load()
  }, [load])

  const localeOptions = useMemo(() => {
    const values = new Set<string>()
    if (status?.defaultLocale) values.add(status.defaultLocale)
    for (const locale of status?.supportedLocales ?? []) values.add(locale)
    if (settings?.locale) values.add(settings.locale)
    return [...values].sort((left, right) => left.localeCompare(right))
  }, [settings?.locale, status?.defaultLocale, status?.supportedLocales])

  const selectedSettings = settings ?? DEFAULT_SETTINGS
  const selectedLocale = selectedSettings.locale ?? SYSTEM_LOCALE_VALUE
  const selectedLocaleUnsupported = Boolean(
    selectedSettings.locale &&
      localeOptions.length > 0 &&
      !localeOptions.some((locale) => normalizeLocale(locale) === normalizeLocale(selectedSettings.locale)),
  )
  const isMacos = status?.platform === "macos"
  const isBusy = loadState === "loading" || saveState === "saving"

  const updateSettings = (patch: Partial<UpsertDictationSettingsRequestDto>) => {
    if (!adapter?.speechDictationUpdateSettings || !settings) return

    const request: UpsertDictationSettingsRequestDto = {
      enginePreference: patch.enginePreference ?? settings.enginePreference,
      privacyMode: patch.privacyMode ?? settings.privacyMode,
      locale: patch.locale === undefined ? settings.locale : patch.locale,
    }

    setSaveState("saving")
    setError(null)
    adapter
      .speechDictationUpdateSettings(request)
      .then((nextSettings) => {
        setSettings(nextSettings)
      })
      .catch((saveError) => {
        setError(getErrorMessage(saveError, "Cadence could not save dictation settings."))
      })
      .finally(() => setSaveState("idle"))
  }

  return (
    <div className="flex flex-col gap-7">
      <SectionHeader
        title="Dictation"
        description="Configure native macOS speech input for the agent composer."
        actions={
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-8 gap-1.5 text-[12px]"
            disabled={isBusy}
            onClick={load}
          >
            {loadState === "loading" ? (
              <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <RotateCcw className="h-3.5 w-3.5" />
            )}
            Refresh
          </Button>
        }
      />

      {!canUseAdapter ? (
        <UnavailableState title="Dictation settings require the Cadence desktop runtime." />
      ) : loadState === "loading" && !status ? (
        <div className="rounded-md border border-dashed border-border/60 bg-secondary/10 px-4 py-10 text-center">
          <LoaderCircle className="mx-auto h-4 w-4 animate-spin text-muted-foreground" />
          <p className="mt-2 text-[12.5px] font-medium text-foreground">Loading dictation settings</p>
        </div>
      ) : !isMacos ? (
        <UnavailableState title="Native dictation is available only on macOS." />
      ) : (
        <>
          {error ? (
            <Alert variant="destructive" className="rounded-md px-3 py-2 text-[12px]">
              <AlertTriangle className="h-3.5 w-3.5" />
              <AlertTitle className="text-[12px]">Dictation settings need attention</AlertTitle>
              <AlertDescription className="text-[12px]">{error}</AlertDescription>
            </Alert>
          ) : null}

          <section className="grid gap-3 lg:grid-cols-3">
            <SettingSelect
              label="Engine preference"
              value={selectedSettings.enginePreference}
              disabled={isBusy}
              options={ENGINE_OPTIONS}
              onValueChange={(enginePreference) => updateSettings({ enginePreference })}
            />
            <SettingSelect
              label="Privacy mode"
              value={selectedSettings.privacyMode}
              disabled={isBusy}
              options={PRIVACY_OPTIONS}
              onValueChange={(privacyMode) => updateSettings({ privacyMode })}
            />
            <div className="flex flex-col gap-2 rounded-md border border-border/60 px-3 py-3">
              <label className="text-[12px] font-medium text-foreground" htmlFor="dictation-locale">
                Locale
              </label>
              <Select
                value={selectedLocale}
                disabled={isBusy}
                onValueChange={(value) =>
                  updateSettings({ locale: value === SYSTEM_LOCALE_VALUE ? null : value })
                }
              >
                <SelectTrigger id="dictation-locale" aria-label="Dictation locale" className="h-8 w-full text-[12.5px]" size="sm">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={SYSTEM_LOCALE_VALUE}>
                    System default{status?.defaultLocale ? ` (${status.defaultLocale})` : ""}
                  </SelectItem>
                  {localeOptions.map((locale) => (
                    <SelectItem key={locale} value={locale}>
                      {locale}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-[11.5px] leading-[1.45] text-muted-foreground">
                {selectedLocaleUnsupported
                  ? "The selected locale is not in the current backend-supported list."
                  : "Use System default unless a project needs a specific recognition locale."}
              </p>
            </div>
          </section>

          <section className="flex flex-col gap-2.5">
            <h4 className="text-[12.5px] font-semibold text-foreground">Availability</h4>
            <div className="overflow-hidden rounded-md border border-border/60 divide-y divide-border/40">
              <EngineRow label="Modern engine" available={status?.modern.available} reason={status?.modern.reason} />
              <EngineRow label="Legacy engine" available={status?.legacy.available} reason={status?.legacy.reason} />
              <PermissionRow kind="microphone" state={status?.microphonePermission ?? "unknown"} />
              <PermissionRow kind="speech recognition" state={status?.speechPermission ?? "unknown"} />
              <StatusRow
                icon={Mic}
                label="Modern speech assets"
                tone={status?.modernAssets.status === "installed" ? "ok" : status?.modern.available ? "warn" : "muted"}
                value={modernAssetLabel(status)}
              />
            </div>
          </section>
        </>
      )}
    </div>
  )
}

function SettingSelect<T extends string>({
  label,
  value,
  disabled,
  options,
  onValueChange,
}: {
  label: string
  value: T
  disabled: boolean
  options: Array<{ value: T; label: string; detail: string }>
  onValueChange: (value: T) => void
}) {
  const selected = options.find((option) => option.value === value)

  return (
    <div className="flex flex-col gap-2 rounded-md border border-border/60 px-3 py-3">
      <label className="text-[12px] font-medium text-foreground">{label}</label>
      <Select value={value} disabled={disabled} onValueChange={(nextValue) => onValueChange(nextValue as T)}>
        <SelectTrigger aria-label={label} className="h-8 w-full text-[12.5px]" size="sm">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {options.map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <p className="text-[11.5px] leading-[1.45] text-muted-foreground">{selected?.detail}</p>
    </div>
  )
}

function EngineRow({
  label,
  available,
  reason,
}: {
  label: string
  available?: boolean
  reason?: string | null
}) {
  return (
    <StatusRow
      icon={Settings}
      label={label}
      tone={available ? "ok" : "warn"}
      value={available ? "Available" : reason ? humanizeReason(reason) : "Unavailable"}
    />
  )
}

function PermissionRow({
  kind,
  state,
}: {
  kind: "microphone" | "speech recognition"
  state: DictationPermissionStateDto
}) {
  const denied = state === "denied" || state === "restricted"
  const pane = kind === "microphone" ? "Privacy_Microphone" : "Privacy_SpeechRecognition"
  return (
    <StatusRow
      icon={denied ? AlertTriangle : CheckCircle2}
      label={`${capitalize(kind)} permission`}
      tone={state === "authorized" ? "ok" : denied ? "bad" : "warn"}
      value={permissionLabel(state)}
      action={
        denied ? (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="h-7 text-[11.5px]"
            onClick={() => void openMacosPrivacyPane(pane)}
          >
            Open Settings
          </Button>
        ) : null
      }
    />
  )
}

function StatusRow({
  icon: Icon,
  label,
  tone,
  value,
  action,
}: {
  icon: React.ElementType
  label: string
  tone: "ok" | "warn" | "bad" | "muted"
  value: string
  action?: React.ReactNode
}) {
  return (
    <div className="flex items-center justify-between gap-3 px-3.5 py-3">
      <div className="flex min-w-0 items-center gap-2.5">
        <Icon
          className={cn(
            "h-3.5 w-3.5 shrink-0",
            tone === "ok"
              ? "text-emerald-600 dark:text-emerald-400"
              : tone === "bad"
                ? "text-destructive"
                : tone === "warn"
                  ? "text-amber-700 dark:text-amber-300"
                  : "text-muted-foreground",
          )}
        />
        <div className="min-w-0">
          <p className="text-[12.5px] font-medium text-foreground">{label}</p>
          <p className="mt-0.5 text-[11.5px] text-muted-foreground">{value}</p>
        </div>
      </div>
      {action ?? (
        <Badge variant="outline" className="h-5 shrink-0 px-1.5 text-[10.5px]">
          {tone === "ok" ? "Ready" : tone === "bad" ? "Blocked" : "Check"}
        </Badge>
      )}
    </div>
  )
}

function UnavailableState({ title }: { title: string }) {
  return (
    <div className="rounded-md border border-dashed border-border/60 bg-secondary/10 px-4 py-10 text-center">
      <Mic className="mx-auto h-4 w-4 text-muted-foreground" />
      <p className="mt-2 text-[12.5px] font-medium text-foreground">{title}</p>
    </div>
  )
}

function permissionLabel(state: DictationPermissionStateDto): string {
  switch (state) {
    case "authorized":
      return "Allowed"
    case "denied":
    case "restricted":
      return "Open System Settings > Privacy & Security and allow Cadence."
    case "not_determined":
      return "macOS will ask the first time dictation starts."
    case "unsupported":
      return "Unsupported on this system."
    case "unknown":
      return "Current permission state is unknown."
  }
}

function modernAssetLabel(status: DictationStatusDto | null): string {
  if (!status?.modern.available) return "Modern engine unavailable"
  switch (status.modernAssets.status) {
    case "installed":
      return status.modernAssets.locale ? `Installed for ${status.modernAssets.locale}` : "Installed"
    case "not_installed":
      return status.modernAssets.locale ? `Not installed for ${status.modernAssets.locale}` : "Not installed"
    case "unsupported_locale":
      return "Unsupported locale"
    case "unavailable":
    case "unknown":
      return "Asset status unknown"
  }
}

function getErrorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.trim().length > 0) return error.message
  if (typeof error === "string" && error.trim().length > 0) return error
  return fallback
}

function humanizeReason(reason: string): string {
  return reason
    .replace(/_/g, " ")
    .replace(/^\w/, (letter: string) => letter.toUpperCase())
}

function normalizeLocale(locale: string | null | undefined): string {
  return (locale ?? "").trim().replace(/-/g, "_").toLowerCase()
}

function capitalize(value: string): string {
  return value.replace(/^\w/, (letter: string) => letter.toUpperCase())
}

async function openMacosPrivacyPane(pane: string): Promise<void> {
  await openUrl(`x-apple.systempreferences:com.apple.preference.security?${pane}`)
}
