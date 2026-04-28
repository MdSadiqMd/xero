import { useEffect, useMemo, useState, type ElementType } from "react"
import { openUrl } from "@tauri-apps/plugin-opener"
import {
  AlertCircle,
  Check,
  Cloud,
  KeyRound,
  LoaderCircle,
  LogIn,
  LogOut,
  Server,
} from "lucide-react"
import {
  AnthropicIcon,
  GitHubIcon,
  GoogleIcon,
  OpenAIIcon,
} from "@/components/cadence/brand-icons"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { cn } from "@/lib/utils"
import type {
  OperatorActionErrorView,
  ProviderCredentialsLoadStatus,
  ProviderCredentialsSaveStatus,
} from "@/src/features/cadence/use-cadence-desktop-state"
import type { CloudProviderPreset } from "@/src/lib/cadence-model/provider-presets"
import {
  type ProviderCredentialDto,
  type ProviderCredentialsSnapshotDto,
  type RuntimeProviderIdDto,
  type RuntimeSessionView,
  type UpsertProviderCredentialRequestDto,
} from "@/src/lib/cadence-model"
import { listCloudProviderPresets } from "@/src/lib/cadence-model/provider-presets"

type SupportedProviderId = RuntimeProviderIdDto

type AuthPending = { providerId: SupportedProviderId } | null
type SaveErrorState = { providerId: SupportedProviderId; message: string } | null

interface CredentialDraft {
  apiKey: string
  baseUrl: string
  apiVersion: string
  region: string
  projectId: string
}

const PROVIDER_ICON_BY_ID: Record<SupportedProviderId, ElementType> = {
  openai_codex: OpenAIIcon,
  openrouter: KeyRound,
  anthropic: AnthropicIcon,
  github_models: GitHubIcon,
  openai_api: OpenAIIcon,
  ollama: Server,
  azure_openai: OpenAIIcon,
  gemini_ai_studio: GoogleIcon,
  bedrock: Cloud,
  vertex: GoogleIcon,
}

function errMsg(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.trim().length > 0) return error.message
  if (typeof error === "string" && error.trim().length > 0) return error
  return fallback
}

function errorViewMessage(error: OperatorActionErrorView | null, fallback: string): string {
  if (error?.message?.trim()) return error.message
  return fallback
}

function isSupportedProviderId(value: string | null | undefined): value is SupportedProviderId {
  return (
    value === "openai_codex" ||
    value === "openrouter" ||
    value === "anthropic" ||
    value === "github_models" ||
    value === "openai_api" ||
    value === "ollama" ||
    value === "azure_openai" ||
    value === "gemini_ai_studio" ||
    value === "bedrock" ||
    value === "vertex"
  )
}

function findCredential(
  snapshot: ProviderCredentialsSnapshotDto | null,
  providerId: SupportedProviderId,
): ProviderCredentialDto | null {
  if (!snapshot) return null
  return snapshot.credentials.find((c) => c.providerId === providerId) ?? null
}

function getReadinessBadge(
  preset: CloudProviderPreset,
  credential: ProviderCredentialDto | null,
): { label: string; className: string } {
  if (!credential) {
    if (preset.authMode === "oauth") {
      return {
        label: "Needs sign-in",
        className: "border border-border bg-secondary text-muted-foreground",
      }
    }
    if (preset.authMode === "local") {
      return {
        label: "Needs endpoint",
        className: "border border-border bg-secondary text-muted-foreground",
      }
    }
    if (preset.authMode === "ambient") {
      return {
        label: "Needs cloud config",
        className: "border border-border bg-secondary text-muted-foreground",
      }
    }
    return {
      label: "Needs key",
      className: "border border-border bg-secondary text-muted-foreground",
    }
  }

  switch (credential.readinessProof) {
    case "oauth_session":
      return {
        label: "Signed in",
        className:
          "border border-emerald-500/30 bg-emerald-500/10 text-emerald-500 dark:text-emerald-400",
      }
    case "stored_secret":
      return {
        label: "Ready",
        className:
          "border border-emerald-500/30 bg-emerald-500/10 text-emerald-500 dark:text-emerald-400",
      }
    case "local":
      return {
        label: "Local",
        className:
          "border border-sky-500/30 bg-sky-500/10 text-sky-600 dark:text-sky-300",
      }
    case "ambient":
      return {
        label: "Ambient auth",
        className:
          "border border-cyan-500/30 bg-cyan-500/10 text-cyan-600 dark:text-cyan-300",
      }
  }
}

function createDraft(
  preset: CloudProviderPreset,
  credential: ProviderCredentialDto | null,
): CredentialDraft {
  return {
    apiKey: "",
    baseUrl: credential?.baseUrl ?? "",
    apiVersion: credential?.apiVersion ?? "",
    region: credential?.region ?? "",
    projectId: credential?.projectId ?? "",
  }
}

function buildUpsertRequest(
  preset: CloudProviderPreset,
  draft: CredentialDraft,
): UpsertProviderCredentialRequestDto {
  const trimOrNull = (value: string): string | null => {
    const trimmed = value.trim()
    return trimmed.length > 0 ? trimmed : null
  }

  switch (preset.authMode) {
    case "api_key":
      return {
        providerId: preset.providerId,
        kind: "api_key",
        apiKey: draft.apiKey.trim(),
        baseUrl: trimOrNull(draft.baseUrl),
        apiVersion: trimOrNull(draft.apiVersion),
        region: trimOrNull(draft.region),
        projectId: trimOrNull(draft.projectId),
      }
    case "local":
      return {
        providerId: preset.providerId,
        kind: "local",
        baseUrl: trimOrNull(draft.baseUrl),
      }
    case "ambient":
      return {
        providerId: preset.providerId,
        kind: "ambient",
        region: trimOrNull(draft.region),
        projectId: trimOrNull(draft.projectId),
      }
    case "oauth":
      // OAuth providers go through start_oauth_login, not upsert.
      throw new Error(
        `Cadence persists ${preset.providerId} credentials through OAuth, not the upsert command.`,
      )
  }
}

function validateDraft(
  preset: CloudProviderPreset,
  draft: CredentialDraft,
): string | null {
  if (preset.authMode === "api_key") {
    if (draft.apiKey.trim().length === 0) {
      return "API key is required."
    }
    if (preset.baseUrlMode === "required" && draft.baseUrl.trim().length === 0) {
      return "Base URL is required for this provider."
    }
    if (preset.apiVersionMode === "required" && draft.apiVersion.trim().length === 0) {
      return "API version is required for this provider."
    }
  }
  if (preset.authMode === "local" && draft.baseUrl.trim().length === 0) {
    return "Local endpoint URL is required."
  }
  if (preset.authMode === "ambient") {
    if (preset.regionMode === "required" && draft.region.trim().length === 0) {
      return "Region is required for this provider."
    }
    if (preset.projectIdMode === "required" && draft.projectId.trim().length === 0) {
      return "Project ID is required for this provider."
    }
  }
  return null
}

export interface ProviderCredentialsListProps {
  providerCredentials: ProviderCredentialsSnapshotDto | null
  providerCredentialsLoadStatus: ProviderCredentialsLoadStatus
  providerCredentialsLoadError: OperatorActionErrorView | null
  providerCredentialsSaveStatus: ProviderCredentialsSaveStatus
  providerCredentialsSaveError: OperatorActionErrorView | null
  runtimeSession?: RuntimeSessionView | null
  onRefreshProviderCredentials?: (options?: { force?: boolean }) => Promise<ProviderCredentialsSnapshotDto>
  onUpsertProviderCredential?: (
    request: UpsertProviderCredentialRequestDto,
  ) => Promise<ProviderCredentialsSnapshotDto>
  onDeleteProviderCredential?: (
    providerId: SupportedProviderId,
  ) => Promise<ProviderCredentialsSnapshotDto>
  onStartOAuthLogin?: (request: {
    providerId: SupportedProviderId
    originator?: string | null
  }) => Promise<RuntimeSessionView | null>
}

export function ProviderCredentialsList({
  providerCredentials,
  providerCredentialsLoadStatus,
  providerCredentialsLoadError,
  providerCredentialsSaveStatus,
  providerCredentialsSaveError,
  runtimeSession = null,
  onRefreshProviderCredentials,
  onUpsertProviderCredential,
  onDeleteProviderCredential,
  onStartOAuthLogin,
}: ProviderCredentialsListProps) {
  const presets = useMemo(() => listCloudProviderPresets(), [])
  const [openProviderId, setOpenProviderId] = useState<SupportedProviderId | null>(null)
  const [drafts, setDrafts] = useState<Record<SupportedProviderId, CredentialDraft>>(
    () => ({}) as Record<SupportedProviderId, CredentialDraft>,
  )
  const [authPending, setAuthPending] = useState<AuthPending>(null)
  const [saveError, setSaveError] = useState<SaveErrorState>(null)
  const [openAuthError, setOpenAuthError] = useState<SaveErrorState>(null)

  // Auto-trigger initial load
  useEffect(() => {
    if (providerCredentialsLoadStatus === "idle" && onRefreshProviderCredentials) {
      onRefreshProviderCredentials().catch(() => {
        // Surface error through providerCredentialsLoadError
      })
    }
  }, [providerCredentialsLoadStatus, onRefreshProviderCredentials])

  const updateDraft = (providerId: SupportedProviderId, patch: Partial<CredentialDraft>) => {
    setDrafts((prev) => ({
      ...prev,
      [providerId]: { ...prev[providerId], ...patch },
    }))
  }

  const ensureDraft = (
    providerId: SupportedProviderId,
    preset: CloudProviderPreset,
    credential: ProviderCredentialDto | null,
  ) => {
    setDrafts((prev) =>
      prev[providerId] !== undefined
        ? prev
        : { ...prev, [providerId]: createDraft(preset, credential) },
    )
  }

  const handleToggle = (
    providerId: SupportedProviderId,
    preset: CloudProviderPreset,
    credential: ProviderCredentialDto | null,
  ) => {
    if (openProviderId === providerId) {
      setOpenProviderId(null)
      return
    }
    ensureDraft(providerId, preset, credential)
    setOpenProviderId(providerId)
    setSaveError(null)
  }

  const handleSave = async (preset: CloudProviderPreset) => {
    const providerId = preset.providerId
    if (!isSupportedProviderId(providerId)) return
    if (!onUpsertProviderCredential) return
    const draft = drafts[providerId] ?? createDraft(preset, findCredential(providerCredentials, providerId))

    const validation = validateDraft(preset, draft)
    if (validation) {
      setSaveError({ providerId, message: validation })
      return
    }

    setSaveError(null)
    try {
      await onUpsertProviderCredential(buildUpsertRequest(preset, draft))
      // On success, clear the api key field so the form doesn't keep the secret around.
      updateDraft(providerId, { apiKey: "" })
      setOpenProviderId(null)
    } catch (error) {
      setSaveError({
        providerId,
        message: errMsg(error, "Cadence could not save the provider credential."),
      })
    }
  }

  const handleDelete = async (providerId: SupportedProviderId) => {
    if (!onDeleteProviderCredential) return
    setSaveError(null)
    try {
      await onDeleteProviderCredential(providerId)
      setOpenProviderId(null)
    } catch (error) {
      setSaveError({
        providerId,
        message: errMsg(error, "Cadence could not remove the provider credential."),
      })
    }
  }

  const handleOAuthLogin = async (providerId: SupportedProviderId) => {
    if (!onStartOAuthLogin) return
    setAuthPending({ providerId })
    setOpenAuthError(null)
    try {
      const session = await onStartOAuthLogin({ providerId })
      if (session?.authorizationUrl) {
        try {
          await openUrl(session.authorizationUrl)
        } catch (urlError) {
          setOpenAuthError({
            providerId,
            message: errMsg(urlError, "Cadence could not open the browser for sign-in."),
          })
        }
      }
    } catch (error) {
      setOpenAuthError({
        providerId,
        message: errMsg(error, "Cadence could not start the sign-in flow."),
      })
    } finally {
      setAuthPending(null)
    }
  }

  const showLoadingState =
    providerCredentialsLoadStatus === "loading" && !providerCredentials
  const showLoadError = providerCredentialsLoadStatus === "error"

  return (
    <div className="flex flex-col gap-4">
      {showLoadError && (
        <Alert variant="destructive" className="border-destructive/40">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>
            {errorViewMessage(
              providerCredentialsLoadError,
              "Cadence could not load provider credentials.",
            )}
          </AlertDescription>
        </Alert>
      )}

      {showLoadingState && (
        <div className="flex items-center gap-2 rounded-md border border-border bg-card/50 p-3 text-sm text-muted-foreground">
          <LoaderCircle className="h-4 w-4 animate-spin" />
          Loading provider credentials…
        </div>
      )}

      {presets.map((preset) => {
        const providerId = preset.providerId
        if (!isSupportedProviderId(providerId)) return null
        const credential = findCredential(providerCredentials, providerId)
        const badge = getReadinessBadge(preset, credential)
        const Icon = PROVIDER_ICON_BY_ID[providerId]
        const isOpen = openProviderId === providerId
        const draft = drafts[providerId] ?? createDraft(preset, credential)
        const isSaving =
          providerCredentialsSaveStatus === "running" && openProviderId === providerId
        const localSaveError = saveError?.providerId === providerId ? saveError.message : null
        const localSaveErrorFromAdapter =
          providerCredentialsSaveError && openProviderId === providerId
            ? providerCredentialsSaveError.message
            : null
        const localOpenAuthError =
          openAuthError?.providerId === providerId ? openAuthError.message : null
        const isOAuth = preset.authMode === "oauth"
        const isAuthenticated =
          credential?.kind === "oauth_session" && credential?.hasOauthAccessToken
        const showAuthInProgress =
          isOAuth &&
          !!runtimeSession?.isLoginInProgress &&
          runtimeSession.providerId === providerId

        return (
          <div
            key={providerId}
            className="rounded-lg border border-border bg-card/40 p-4 transition-colors hover:border-border/80"
          >
            <div className="flex items-start justify-between gap-3">
              <div className="flex items-start gap-3">
                <div className="flex h-9 w-9 items-center justify-center rounded-md border border-border bg-background">
                  <Icon className="h-5 w-5" />
                </div>
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-foreground">{preset.label}</span>
                    <Badge className={cn("h-5 px-2 text-[10px] font-medium", badge.className)}>
                      {badge.label}
                    </Badge>
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">{preset.description}</p>
                </div>
              </div>

              <div className="flex shrink-0 items-center gap-2">
                {isOAuth ? (
                  isAuthenticated ? (
                    <>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => handleDelete(providerId)}
                        disabled={!onDeleteProviderCredential}
                      >
                        <LogOut className="mr-1.5 h-3.5 w-3.5" />
                        Sign out
                      </Button>
                    </>
                  ) : (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleOAuthLogin(providerId)}
                      disabled={
                        !onStartOAuthLogin ||
                        authPending?.providerId === providerId ||
                        showAuthInProgress
                      }
                    >
                      {authPending?.providerId === providerId || showAuthInProgress ? (
                        <LoaderCircle className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                      ) : (
                        <LogIn className="mr-1.5 h-3.5 w-3.5" />
                      )}
                      Sign in
                    </Button>
                  )
                ) : (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => handleToggle(providerId, preset, credential)}
                  >
                    {isOpen ? "Close" : credential ? "Edit" : "Configure"}
                  </Button>
                )}
              </div>
            </div>

            {localOpenAuthError && (
              <Alert variant="destructive" className="mt-3 border-destructive/40">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{localOpenAuthError}</AlertDescription>
              </Alert>
            )}

            {!isOAuth && isOpen && (
              <div className="mt-4 space-y-3 border-t border-border pt-4">
                {preset.authMode === "api_key" && (
                  <div className="space-y-2">
                    <Label htmlFor={`${providerId}-api-key`} className="text-xs">
                      API key
                      {credential?.hasApiKey && (
                        <span className="ml-2 text-[11px] text-muted-foreground">
                          (saved — leave empty to keep current)
                        </span>
                      )}
                    </Label>
                    <Input
                      id={`${providerId}-api-key`}
                      type="password"
                      autoComplete="off"
                      value={draft.apiKey}
                      onChange={(e) => updateDraft(providerId, { apiKey: e.target.value })}
                      placeholder={credential?.hasApiKey ? "••••••••" : "Paste your API key"}
                    />
                  </div>
                )}

                {(preset.baseUrlMode !== "none" || preset.authMode === "local") && (
                  <div className="space-y-2">
                    <Label htmlFor={`${providerId}-base-url`} className="text-xs">
                      Base URL
                      {preset.baseUrlMode === "required" || preset.authMode === "local" ? (
                        <span className="ml-1 text-destructive">*</span>
                      ) : null}
                    </Label>
                    <Input
                      id={`${providerId}-base-url`}
                      value={draft.baseUrl}
                      onChange={(e) => updateDraft(providerId, { baseUrl: e.target.value })}
                      placeholder={preset.connectionHint}
                    />
                  </div>
                )}

                {preset.apiVersionMode !== "none" && (
                  <div className="space-y-2">
                    <Label htmlFor={`${providerId}-api-version`} className="text-xs">
                      API version
                      {preset.apiVersionMode === "required" && (
                        <span className="ml-1 text-destructive">*</span>
                      )}
                    </Label>
                    <Input
                      id={`${providerId}-api-version`}
                      value={draft.apiVersion}
                      onChange={(e) => updateDraft(providerId, { apiVersion: e.target.value })}
                    />
                  </div>
                )}

                {preset.regionMode === "required" && (
                  <div className="space-y-2">
                    <Label htmlFor={`${providerId}-region`} className="text-xs">
                      Region <span className="text-destructive">*</span>
                    </Label>
                    <Input
                      id={`${providerId}-region`}
                      value={draft.region}
                      onChange={(e) => updateDraft(providerId, { region: e.target.value })}
                    />
                  </div>
                )}

                {preset.projectIdMode === "required" && (
                  <div className="space-y-2">
                    <Label htmlFor={`${providerId}-project-id`} className="text-xs">
                      Project ID <span className="text-destructive">*</span>
                    </Label>
                    <Input
                      id={`${providerId}-project-id`}
                      value={draft.projectId}
                      onChange={(e) => updateDraft(providerId, { projectId: e.target.value })}
                    />
                  </div>
                )}

                {(localSaveError || localSaveErrorFromAdapter) && (
                  <Alert variant="destructive" className="border-destructive/40">
                    <AlertCircle className="h-4 w-4" />
                    <AlertDescription>
                      {localSaveError ?? localSaveErrorFromAdapter}
                    </AlertDescription>
                  </Alert>
                )}

                <div className="flex items-center justify-between gap-2 pt-2">
                  {credential ? (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => handleDelete(providerId)}
                      disabled={isSaving || !onDeleteProviderCredential}
                      className="text-destructive hover:bg-destructive/10 hover:text-destructive"
                    >
                      Remove
                    </Button>
                  ) : (
                    <span />
                  )}
                  <Button
                    size="sm"
                    onClick={() => handleSave(preset)}
                    disabled={isSaving || !onUpsertProviderCredential}
                  >
                    {isSaving ? (
                      <LoaderCircle className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                    ) : (
                      <Check className="mr-1.5 h-3.5 w-3.5" />
                    )}
                    Save
                  </Button>
                </div>
              </div>
            )}
          </div>
        )
      })}
    </div>
  )
}
