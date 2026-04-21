import { useEffect, useState } from "react"
import { AlertCircle, Check, KeyRound, Lock, LoaderCircle } from "lucide-react"
import { AnthropicIcon, GoogleIcon, OpenAIIcon } from "@/components/cadence/brand-icons"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { cn } from "@/lib/utils"
import type {
  OperatorActionErrorView,
  RuntimeSettingsSaveStatus,
} from "@/src/features/cadence/use-cadence-desktop-state"
import type {
  RuntimeSessionView,
  RuntimeSettingsDto,
  UpsertRuntimeSettingsRequestDto,
} from "@/src/lib/cadence-model"
import type { ProviderId } from "../types"

interface ProvidersStepProps {
  runtimeSettings: RuntimeSettingsDto | null
  runtimeSession: RuntimeSessionView | null
  runtimeSettingsSaveStatus: RuntimeSettingsSaveStatus
  runtimeSettingsSaveError: OperatorActionErrorView | null
  onUpsertRuntimeSettings: (request: UpsertRuntimeSettingsRequestDto) => Promise<RuntimeSettingsDto>
}

interface ProviderMeta {
  id: ProviderId
  label: string
  description: string
  Icon: React.ElementType
  disabled?: boolean
}

const PROVIDER_META: ProviderMeta[] = [
  {
    id: "openai_codex",
    label: "OpenAI Codex",
    description: "Sign in with your OpenAI account.",
    Icon: OpenAIIcon,
  },
  {
    id: "openrouter",
    label: "OpenRouter",
    description: "Bring your own API key.",
    Icon: KeyRound,
  },
  {
    id: "anthropic",
    label: "Anthropic",
    description: "Coming soon.",
    Icon: AnthropicIcon,
    disabled: true,
  },
  {
    id: "google",
    label: "Google",
    description: "Coming soon.",
    Icon: GoogleIcon,
    disabled: true,
  },
]

function fallbackErrorMessage(error: OperatorActionErrorView | null, fallback: string): string {
  if (error?.message?.trim()) return error.message
  return fallback
}

export function ProvidersStep({
  runtimeSettings,
  runtimeSession,
  runtimeSettingsSaveStatus,
  runtimeSettingsSaveError,
  onUpsertRuntimeSettings,
}: ProvidersStepProps) {
  const [configuringId, setConfiguringId] = useState<ProviderId | null>(null)
  const [openrouterModelId, setOpenrouterModelId] = useState("")
  const [openrouterApiKey, setOpenrouterApiKey] = useState("")
  const [formError, setFormError] = useState<string | null>(null)

  const currentProviderId = runtimeSettings?.providerId ?? "openai_codex"
  const openrouterConfigured = runtimeSettings?.openrouterApiKeyConfigured ?? false
  const isOpenAiConnected = Boolean(runtimeSession?.providerId === "openai_codex" && runtimeSession.isAuthenticated)
  const isSaving = runtimeSettingsSaveStatus === "running"

  useEffect(() => {
    if (runtimeSettings?.providerId === "openrouter") {
      setOpenrouterModelId(runtimeSettings.modelId)
    } else if (!openrouterModelId.trim()) {
      setOpenrouterModelId("openai/gpt-4.1-mini")
    }
  }, [openrouterModelId, runtimeSettings])

  async function handleSelectOpenAiCodex() {
    setFormError(null)

    try {
      await onUpsertRuntimeSettings({ providerId: "openai_codex", modelId: "openai_codex" })
    } catch {
      // Hook state exposes the typed save error; keep the current form state intact.
    }
  }

  async function handleSaveOpenRouter() {
    if (!openrouterModelId.trim()) {
      setFormError("Model ID is required.")
      return
    }

    if (!openrouterConfigured && !openrouterApiKey.trim()) {
      setFormError("OpenRouter requires an API key.")
      return
    }

    setFormError(null)

    try {
      await onUpsertRuntimeSettings({
        providerId: "openrouter",
        modelId: openrouterModelId.trim(),
        ...(openrouterApiKey.trim() ? { openrouterApiKey: openrouterApiKey.trim() } : {}),
      })

      setConfiguringId(null)
      setOpenrouterApiKey("")
    } catch {
      // Hook state exposes the typed save error; keep the current form state intact.
    }
  }

  return (
    <div>
      <StepHeader
        title="Configure providers"
        description="Provider setup is app-wide. Projects stay separate, and OpenAI sign-in only happens when you start a runtime session."
      />

      <div className="mt-7 flex flex-col gap-2 animate-in fade-in-0 slide-in-from-bottom-1 duration-300 ease-out [animation-delay:60ms] [animation-fill-mode:both]">
        {PROVIDER_META.map((meta) => {
          const isOpenRouter = meta.id === "openrouter"
          const isOpenAi = meta.id === "openai_codex"
          const hasSetup = isOpenRouter ? openrouterConfigured : isOpenAiConnected
          const isCurrent = currentProviderId === meta.id && hasSetup
          const configOpen = configuringId === meta.id

          return (
            <div
              key={meta.id}
              className={cn(
                "group/card relative rounded-lg border bg-card/40 px-3.5 py-3 transition-all",
                meta.disabled
                  ? "border-border/60 bg-card/20"
                  : isCurrent
                    ? "border-primary/40 bg-primary/[0.03]"
                    : "border-border hover:border-border/80 hover:bg-card/60",
              )}
            >
              {isCurrent ? (
                <span
                  aria-hidden
                  className="absolute inset-y-2 left-0 w-0.5 rounded-full bg-primary/70"
                />
              ) : null}

              <div className="flex items-center gap-3">
                <span
                  className={cn(
                    "flex h-9 w-9 shrink-0 items-center justify-center rounded-md border transition-colors duration-200 ease-out",
                    meta.disabled
                      ? "border-border/60 bg-secondary/30 text-muted-foreground/60"
                      : isCurrent
                        ? "border-primary/40 bg-primary/10 text-primary"
                        : "border-border bg-secondary/50 text-foreground/75 group-hover/card:border-border/80 group-hover/card:text-foreground",
                  )}
                >
                  <meta.Icon className="h-4 w-4" />
                </span>

                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-1.5">
                    <p
                      className={cn(
                        "text-[13px] font-medium",
                        meta.disabled ? "text-muted-foreground" : "text-foreground",
                      )}
                    >
                      {meta.label}
                    </p>
                    {isOpenAi && isOpenAiConnected ? (
                      <Badge
                        variant="secondary"
                        className="gap-1 border border-emerald-500/30 bg-emerald-500/10 px-1.5 py-0 text-[10px] font-medium text-emerald-500 dark:text-emerald-400"
                      >
                        <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 dark:bg-emerald-400" />
                        Connected
                      </Badge>
                    ) : null}
                    {isOpenRouter && openrouterConfigured ? (
                      <Badge variant="secondary" className="px-1.5 py-0 text-[10px]">
                        Key saved
                      </Badge>
                    ) : null}
                  </div>
                  <p className="mt-0.5 truncate text-[11px] leading-relaxed text-muted-foreground">
                    {meta.description}
                  </p>
                </div>

                {meta.disabled ? (
                  <Button
                    size="sm"
                    variant="outline"
                    disabled
                    className="h-7 min-w-[96px] text-[11px]"
                  >
                    Unavailable
                  </Button>
                ) : isOpenAi ? (
                  <Button
                    size="sm"
                    variant={isCurrent ? "secondary" : "outline"}
                    disabled={isSaving}
                    onClick={() => void handleSelectOpenAiCodex()}
                    className={cn(
                      "h-7 min-w-[96px] gap-1 text-[11px]",
                      isCurrent && "border-primary/30 bg-primary/10 text-primary hover:bg-primary/15",
                    )}
                  >
                    {isSaving && isCurrent ? (
                      <LoaderCircle className="h-3 w-3 animate-spin" />
                    ) : isCurrent ? (
                      <Check className="h-3 w-3" />
                    ) : null}
                    {isCurrent ? "Using this" : "Use OpenAI"}
                  </Button>
                ) : isOpenRouter && !configOpen ? (
                  <Button
                    size="sm"
                    variant={openrouterConfigured ? "secondary" : "outline"}
                    disabled={isSaving}
                    onClick={() => {
                      setConfiguringId(meta.id)
                      setFormError(null)
                    }}
                    className={cn(
                      "h-7 min-w-[96px] text-[11px]",
                      openrouterConfigured && isCurrent && "border-primary/30 bg-primary/10 text-primary hover:bg-primary/15",
                    )}
                  >
                    {openrouterConfigured ? "Edit setup" : "Set up"}
                  </Button>
                ) : null}
              </div>

              {isOpenRouter ? (
                <div
                  aria-hidden={!configOpen}
                  className={cn(
                    "grid transition-[grid-template-rows] duration-200 ease-out",
                    configOpen ? "grid-rows-[1fr]" : "grid-rows-[0fr]",
                  )}
                >
                  <div
                    className={cn(
                      "overflow-hidden transition-opacity duration-150",
                      configOpen ? "opacity-100 delay-75" : "pointer-events-none opacity-0",
                    )}
                  >
                    <div className="mt-3 grid gap-3">
                    <div className="space-y-1.5">
                      <Label htmlFor="onboarding-openrouter-model" className="text-[11px] font-medium">
                        Model ID
                      </Label>
                      <Input
                        id="onboarding-openrouter-model"
                        className="h-8 font-mono text-[12px]"
                        disabled={isSaving}
                        onChange={(event) => setOpenrouterModelId(event.target.value)}
                        placeholder="openai/gpt-4.1-mini"
                        value={openrouterModelId}
                      />
                      <p className="text-[10px] text-muted-foreground">Use the exact OpenRouter model slug.</p>
                    </div>

                    <div className="space-y-1.5">
                      <div className="flex items-center justify-between gap-3">
                        <Label htmlFor="onboarding-openrouter-key" className="text-[11px] font-medium">
                          API key
                        </Label>
                        {openrouterConfigured ? (
                          <Badge variant="secondary" className="px-1.5 py-0 text-[10px]">
                            Key saved
                          </Badge>
                        ) : null}
                      </div>
                      <Input
                        id="onboarding-openrouter-key"
                        type="password"
                        autoComplete="off"
                        spellCheck={false}
                        className="h-8 font-mono text-[12px]"
                        disabled={isSaving}
                        onChange={(event) => setOpenrouterApiKey(event.target.value)}
                        placeholder={openrouterConfigured ? "Leave blank to keep current key" : "Paste API key"}
                        value={openrouterApiKey}
                      />
                      <p className="text-[10px] text-muted-foreground">
                        {openrouterConfigured ? "Blank keeps the current key." : "Required for OpenRouter."}
                      </p>
                    </div>

                    {runtimeSettingsSaveError || formError ? (
                      <Alert variant="destructive" className="py-2.5">
                        <AlertCircle className="h-4 w-4" />
                        <AlertDescription className="text-[12px]">
                          {formError ?? fallbackErrorMessage(runtimeSettingsSaveError, "Could not save OpenRouter settings.")}
                        </AlertDescription>
                      </Alert>
                    ) : null}

                    <div className="flex items-center gap-2">
                      <Button
                        size="sm"
                        className="h-7 gap-1 text-[11px]"
                        disabled={isSaving}
                        onClick={() => void handleSaveOpenRouter()}
                      >
                        {isSaving ? <LoaderCircle className="h-3 w-3 animate-spin" /> : <Check className="h-3 w-3" />}
                        Save setup
                      </Button>
                      <Button
                        size="sm"
                        variant="ghost"
                        className="h-7 text-[11px]"
                        disabled={isSaving}
                        onClick={() => {
                          setConfiguringId(null)
                          setFormError(null)
                          setOpenrouterApiKey("")
                        }}
                      >
                        Cancel
                      </Button>
                    </div>
                    </div>
                  </div>
                </div>
              ) : null}
            </div>
          )
        })}
      </div>
    </div>
  )
}

interface StepHeaderProps {
  title: string
  description: string
}

export function StepHeader({ title, description }: StepHeaderProps) {
  return (
    <div>
      <h2 className="text-2xl font-semibold tracking-tight text-foreground">{title}</h2>
      <p className="mt-2 text-[13px] leading-relaxed text-muted-foreground">{description}</p>
    </div>
  )
}
