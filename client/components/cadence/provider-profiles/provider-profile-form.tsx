import { useEffect, useState, type ElementType } from "react"
import { openUrl } from "@tauri-apps/plugin-opener"
import {
  AlertCircle,
  Check,
  KeyRound,
  LoaderCircle,
  LogIn,
  LogOut,
  Lock,
} from "lucide-react"
import {
  AnthropicIcon,
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
  ProviderProfilesLoadStatus,
  ProviderProfilesSaveStatus,
} from "@/src/features/cadence/use-cadence-desktop-state"
import {
  getProviderMismatchCopy,
  resolveSelectedRuntimeProvider,
} from "@/src/features/cadence/use-cadence-desktop-state/runtime-provider"
import {
  getActiveProviderProfile,
  type ProviderProfileDto,
  type ProviderProfilesDto,
  type RuntimeSessionView,
  type UpsertProviderProfileRequestDto,
} from "@/src/lib/cadence-model"

type SupportedProviderId = ProviderProfileDto["providerId"]
type ProviderCatalogId = SupportedProviderId | "anthropic" | "google"
type AuthPending = "login" | "logout" | null

type ProviderDraft = {
  label: string
  modelId: string
  openrouterApiKey: string
  clearOpenrouterApiKey: boolean
}

interface ProviderCatalogEntry {
  id: ProviderCatalogId
  label: string
  description: string
  Icon: ElementType
  supported: boolean
  defaultProfileId?: string
  defaultModelId?: string
}

interface ProviderProfileCard {
  key: string
  catalog: ProviderCatalogEntry
  profile: ProviderProfileDto | null
}

const PROVIDER_CATALOG: ProviderCatalogEntry[] = [
  {
    id: "openai_codex",
    label: "OpenAI Codex",
    description: "App-global provider profile. Browser sign-in happens when you bind a runtime session.",
    Icon: OpenAIIcon,
    supported: true,
    defaultProfileId: "openai_codex-default",
    defaultModelId: "openai_codex",
  },
  {
    id: "openrouter",
    label: "OpenRouter",
    description: "App-global provider profile backed by a saved API key.",
    Icon: KeyRound,
    supported: true,
    defaultProfileId: "openrouter-default",
    defaultModelId: "openai/gpt-4.1-mini",
  },
  {
    id: "anthropic",
    label: "Anthropic",
    description: "Coming soon.",
    Icon: AnthropicIcon,
    supported: false,
  },
  {
    id: "google",
    label: "Google",
    description: "Coming soon.",
    Icon: GoogleIcon,
    supported: false,
  },
]

function errMsg(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.trim().length > 0) return error.message
  if (typeof error === "string" && error.trim().length > 0) return error
  return fallback
}

function errorViewMessage(error: OperatorActionErrorView | null, fallback: string): string {
  if (error?.message?.trim()) return error.message
  return fallback
}

function createDraft(card: ProviderProfileCard): ProviderDraft {
  return {
    label: card.profile?.label ?? card.catalog.label,
    modelId: card.profile?.modelId ?? card.catalog.defaultModelId ?? "",
    openrouterApiKey: "",
    clearOpenrouterApiKey: false,
  }
}

function getProfileCards(providerProfiles: ProviderProfilesDto | null): ProviderProfileCard[] {
  const cards: ProviderProfileCard[] = []
  const activeProfileId = providerProfiles?.activeProfileId ?? null

  for (const catalogEntry of PROVIDER_CATALOG) {
    if (!catalogEntry.supported) continue

    const matches = (providerProfiles?.profiles ?? [])
      .filter((profile) => profile.providerId === catalogEntry.id)
      .sort((left, right) => {
        const leftActive = left.profileId === activeProfileId
        const rightActive = right.profileId === activeProfileId

        if (leftActive !== rightActive) return leftActive ? -1 : 1
        return left.label.localeCompare(right.label)
      })

    if (matches.length === 0) {
      cards.push({
        key: `${catalogEntry.id}-placeholder`,
        catalog: catalogEntry,
        profile: null,
      })
      continue
    }

    cards.push(
      ...matches.map((profile) => ({
        key: profile.profileId,
        catalog: catalogEntry,
        profile,
      })),
    )
  }

  return cards
}

function getOpenRouterReadinessBadge(profile: ProviderProfileDto | null) {
  if (!profile || profile.providerId !== "openrouter") return null

  if (profile.readiness.status === "ready") {
    return {
      label: "Ready",
      className: "border border-emerald-500/30 bg-emerald-500/10 text-emerald-500 dark:text-emerald-400",
    }
  }

  if (profile.readiness.status === "malformed") {
    return {
      label: "Needs repair",
      className: "border border-amber-500/30 bg-amber-500/10 text-amber-600 dark:text-amber-300",
    }
  }

  return {
    label: "Needs key",
    className: "border border-border bg-secondary text-muted-foreground",
  }
}

function getProfileId(card: ProviderProfileCard): string {
  return card.profile?.profileId ?? card.catalog.defaultProfileId ?? `${card.catalog.id}-default`
}

function getProfileReference(profile: Pick<ProviderProfileDto, "profileId" | "label"> | null): string {
  if (!profile) return "the selected profile"

  const profileId = profile.profileId.trim()
  const label = profile.label.trim()

  if (label.length === 0) return profileId || "the selected profile"
  if (profileId.length === 0 || profileId === label) return label
  return `${label} (${profileId})`
}

function isCardSelected(providerProfiles: ProviderProfilesDto | null, card: ProviderProfileCard): boolean {
  const activeProfileId = providerProfiles?.activeProfileId?.trim() ?? ""
  if (activeProfileId.length === 0) return false
  return activeProfileId === getProfileId(card)
}

function buildUpsertRequest(
  card: ProviderProfileCard,
  draft: ProviderDraft,
  activate: boolean,
): UpsertProviderProfileRequestDto {
  return {
    profileId: getProfileId(card),
    providerId: card.catalog.id as SupportedProviderId,
    label: draft.label.trim(),
    modelId:
      card.catalog.id === "openai_codex"
        ? card.catalog.defaultModelId ?? "openai_codex"
        : draft.modelId.trim(),
    ...(card.catalog.id === "openrouter"
      ? draft.clearOpenrouterApiKey
        ? { openrouterApiKey: "" }
        : draft.openrouterApiKey.trim().length > 0
          ? { openrouterApiKey: draft.openrouterApiKey.trim() }
          : {}
      : {}),
    activate,
  }
}

export interface ProviderProfileFormProps {
  providerProfiles: ProviderProfilesDto | null
  providerProfilesLoadStatus: ProviderProfilesLoadStatus
  providerProfilesLoadError: OperatorActionErrorView | null
  providerProfilesSaveStatus: ProviderProfilesSaveStatus
  providerProfilesSaveError: OperatorActionErrorView | null
  onRefreshProviderProfiles?: (options?: { force?: boolean }) => Promise<ProviderProfilesDto>
  onUpsertProviderProfile?: (request: UpsertProviderProfileRequestDto) => Promise<ProviderProfilesDto>
  onSetActiveProviderProfile?: (profileId: string) => Promise<ProviderProfilesDto>
  runtimeSession?: RuntimeSessionView | null
  hasSelectedProject?: boolean
  onStartLogin?: () => Promise<RuntimeSessionView | null>
  onLogout?: () => Promise<RuntimeSessionView | null>
  openAiMissingProjectLabel?: string
  openAiMissingProjectDescription?: string
  showUnavailableProviders?: boolean
}

export function ProviderProfileForm({
  providerProfiles,
  providerProfilesLoadStatus,
  providerProfilesLoadError,
  providerProfilesSaveStatus,
  providerProfilesSaveError,
  onRefreshProviderProfiles,
  onUpsertProviderProfile,
  onSetActiveProviderProfile,
  runtimeSession,
  hasSelectedProject = false,
  onStartLogin,
  onLogout,
  openAiMissingProjectLabel = "Open a project",
  openAiMissingProjectDescription = "Select an imported project to sign in the selected OpenAI profile.",
  showUnavailableProviders = false,
}: ProviderProfileFormProps) {
  const [editingCardKey, setEditingCardKey] = useState<string | null>(null)
  const [drafts, setDrafts] = useState<Record<string, ProviderDraft>>({})
  const [pendingAuth, setPendingAuth] = useState<AuthPending>(null)
  const [formError, setFormError] = useState<string | null>(null)
  const [authError, setAuthError] = useState<string | null>(null)

  const cards = getProfileCards(providerProfiles)
  const unavailableProviders = showUnavailableProviders
    ? PROVIDER_CATALOG.filter((entry) => !entry.supported)
    : []

  const isRefreshing = providerProfilesLoadStatus === "loading"
  const isSaving = providerProfilesSaveStatus === "running"
  const selectedProfile = getActiveProviderProfile(providerProfiles)
  const selectedProfileReference = getProfileReference(selectedProfile)
  const selectedProvider = resolveSelectedRuntimeProvider(providerProfiles, null, runtimeSession ?? null)
  const providerMismatchCopy = getProviderMismatchCopy(selectedProvider, runtimeSession ?? null)
  const selectedProfileUnavailableMessage =
    providerProfiles &&
    providerProfilesLoadStatus !== "loading" &&
    selectedProvider.providerId === "openai_codex" &&
    (!selectedProfile || selectedProfile.providerId !== "openai_codex")
      ? "Cadence could not start OpenAI login because the selected provider profile is unavailable. Refresh Settings and retry."
      : null

  useEffect(() => {
    setAuthError(null)
  }, [providerProfiles?.activeProfileId])

  function getDraft(card: ProviderProfileCard): ProviderDraft {
    return drafts[card.key] ?? createDraft(card)
  }

  function setDraft(card: ProviderProfileCard, next: ProviderDraft) {
    setDrafts((current) => ({
      ...current,
      [card.key]: next,
    }))
  }

  function openEditor(card: ProviderProfileCard) {
    setEditingCardKey(card.key)
    setDrafts((current) => ({
      ...current,
      [card.key]: current[card.key] ?? createDraft(card),
    }))
    setFormError(null)
    setAuthError(null)
  }

  function closeEditor(cardKey: string) {
    setEditingCardKey((current) => (current === cardKey ? null : current))
    setFormError(null)
    setDrafts((current) => {
      const next = { ...current }
      delete next[cardKey]
      return next
    })
  }

  async function handleSave(card: ProviderProfileCard) {
    if (!onUpsertProviderProfile) return

    const draft = getDraft(card)

    if (!draft.label.trim()) {
      setFormError("Profile label is required.")
      return
    }

    if (card.catalog.id === "openrouter") {
      const hasSavedKey = card.profile?.readiness.ready ?? false
      const isClearingKey = draft.clearOpenrouterApiKey

      if (!draft.modelId.trim()) {
        setFormError("Model ID is required.")
        return
      }

      if (!hasSavedKey && !isClearingKey && !draft.openrouterApiKey.trim()) {
        setFormError("OpenRouter requires an API key.")
        return
      }
    }

    setFormError(null)

    try {
      const activate = providerProfiles?.activeProfileId?.trim()
        ? providerProfiles.activeProfileId === getProfileId(card)
        : card.profile?.active ?? false
      await onUpsertProviderProfile(buildUpsertRequest(card, draft, activate))
      closeEditor(card.key)
    } catch {
      setDraft(card, {
        ...draft,
        openrouterApiKey: "",
      })
    }
  }

  async function handleActivate(card: ProviderProfileCard) {
    if (isCardSelected(providerProfiles, card)) return

    setFormError(null)
    setAuthError(null)

    try {
      if (card.profile) {
        await onSetActiveProviderProfile?.(card.profile.profileId)
        return
      }

      const draft = createDraft(card)
      await onUpsertProviderProfile?.({
        profileId: getProfileId(card),
        providerId: card.catalog.id as SupportedProviderId,
        label: draft.label,
        modelId: draft.modelId,
        activate: true,
      })
    } catch {
      // Hook state surfaces the typed save error while the last truthful snapshot remains visible.
    }
  }

  async function handleOpenAiConnect() {
    if (!hasSelectedProject || !onStartLogin) return

    if (!selectedProfile || selectedProfile.providerId !== "openai_codex") {
      setAuthError(
        selectedProfileUnavailableMessage ??
          "Cadence could not start OpenAI login because the selected provider profile is unavailable. Refresh Settings and retry.",
      )
      return
    }

    setPendingAuth("login")
    setFormError(null)
    setAuthError(null)

    try {
      const next = await onStartLogin()
      if (next?.authorizationUrl) {
        try {
          await openUrl(next.authorizationUrl)
        } catch {
          // Browser open failed — the runtime flow still started in the desktop backend.
        }
      }
    } catch (error) {
      setAuthError(errMsg(error, "Could not start login."))
    } finally {
      setPendingAuth(null)
    }
  }

  async function handleOpenAiDisconnect() {
    if (!onLogout) return

    setPendingAuth("logout")
    setFormError(null)
    setAuthError(null)

    try {
      await onLogout()
    } catch (error) {
      setAuthError(errMsg(error, "Could not sign out."))
    } finally {
      setPendingAuth(null)
    }
  }

  return (
    <div className="flex flex-col gap-4">
      {providerProfilesLoadError ? (
        <Alert variant="destructive" className="border-destructive/30 bg-destructive/5 py-2.5">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription className="text-[12px]">
            {errorViewMessage(providerProfilesLoadError, "Failed to load app-local provider profiles.")}
            {onRefreshProviderProfiles ? (
              <Button
                variant="outline"
                size="sm"
                className="mt-2 h-6 gap-1 text-[10px]"
                disabled={isRefreshing}
                onClick={() => void onRefreshProviderProfiles({ force: true }).catch(() => undefined)}
              >
                {isRefreshing ? <LoaderCircle className="h-3 w-3 animate-spin" /> : null}
                Retry
              </Button>
            ) : null}
          </AlertDescription>
        </Alert>
      ) : null}

      {providerProfilesSaveError ? (
        <Alert variant="destructive" className="border-destructive/30 bg-destructive/5 py-2.5">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription className="text-[12px]">
            {errorViewMessage(providerProfilesSaveError, "Failed to save the selected provider profile.")}
          </AlertDescription>
        </Alert>
      ) : null}

      {selectedProfileUnavailableMessage ? (
        <Alert variant="destructive" className="border-destructive/30 bg-destructive/5 py-2.5">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription className="text-[12px]">{selectedProfileUnavailableMessage}</AlertDescription>
        </Alert>
      ) : null}

      {formError ? (
        <Alert variant="destructive" className="border-destructive/30 bg-destructive/5 py-2.5">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription className="text-[12px]">{formError}</AlertDescription>
        </Alert>
      ) : null}

      <div className="grid gap-2.5">
        {cards.map((card) => {
          const draft = getDraft(card)
          const isEditing = editingCardKey === card.key
          const isOpenRouter = card.catalog.id === "openrouter"
          const isOpenAi = card.catalog.id === "openai_codex"
          const isSelected = isCardSelected(providerProfiles, card)
          const readinessBadge = getOpenRouterReadinessBadge(card.profile)
          const hasSavedOpenRouterKey = Boolean(card.profile?.providerId === "openrouter" && card.profile.readiness.ready)
          const migratedAt = card.profile?.migratedAt ?? null
          const shouldRenderOpenAiAuth = isOpenAi && isSelected && Boolean(onStartLogin && onLogout)
          const isSelectedRuntimeProvider = runtimeSession?.providerId === selectedProvider.providerId
          const selectedRuntimeErrorMessage = runtimeSession?.lastError?.message?.trim() || null
          const isOpenAiConnected = Boolean(
            shouldRenderOpenAiAuth &&
              selectedProvider.providerId === "openai_codex" &&
              runtimeSession?.providerId === "openai_codex" &&
              runtimeSession.isAuthenticated,
          )
          const isOpenAiInProgress = Boolean(
            shouldRenderOpenAiAuth &&
              selectedProvider.providerId === "openai_codex" &&
              runtimeSession?.providerId === "openai_codex" &&
              runtimeSession.isLoginInProgress,
          )
          const openAiScopeCopy = isOpenAi
            ? !selectedProfile
              ? "Select an OpenAI provider profile before starting sign-in."
              : isSelected
                ? hasSelectedProject
                  ? `OpenAI sign-in applies to the selected profile ${selectedProfileReference}.`
                  : openAiMissingProjectDescription
                : `OpenAI sign-in only runs against the selected profile ${selectedProfileReference}. Select this profile first to manage auth.`
            : null
          const inlineStatus = isSelected
            ? providerMismatchCopy
              ? {
                  tone: "warning" as const,
                  message: providerMismatchCopy.reason,
                  recovery: providerMismatchCopy.sessionRecoveryCopy,
                }
              : authError && isOpenAi
                ? {
                    tone: "error" as const,
                    message: authError,
                    recovery: null,
                  }
                : isSelectedRuntimeProvider && selectedRuntimeErrorMessage
                  ? {
                      tone: "error" as const,
                      message: selectedRuntimeErrorMessage,
                      recovery: null,
                    }
                  : null
            : null

          return (
            <div
              key={card.key}
              className={cn(
                "rounded-lg border bg-card px-4 py-3 transition-colors",
                isSelected ? "border-primary/30 bg-primary/[0.03]" : "border-border",
              )}
            >
              <div className="flex items-start gap-3">
                <div
                  className={cn(
                    "flex h-8 w-8 shrink-0 items-center justify-center rounded-md border bg-secondary/60",
                    isSelected ? "border-primary/40 text-primary" : "border-border",
                  )}
                >
                  <card.catalog.Icon className="h-4 w-4 text-foreground/70" />
                </div>

                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-1.5">
                    <p className="text-[13px] font-medium text-foreground">
                      {card.profile?.label ?? card.catalog.label}
                    </p>
                    {isSelected ? (
                      <Badge variant="secondary" className="px-1.5 py-0 text-[10px]">
                        Active profile
                      </Badge>
                    ) : null}
                    {readinessBadge ? (
                      <Badge className={cn("px-1.5 py-0 text-[10px] font-medium", readinessBadge.className)}>
                        {readinessBadge.label}
                      </Badge>
                    ) : null}
                    {isOpenAi && isOpenAiConnected ? (
                      <Badge
                        variant="secondary"
                        className="gap-1 border border-emerald-500/30 bg-emerald-500/10 px-1.5 py-0 text-[10px] font-medium text-emerald-500 dark:text-emerald-400"
                      >
                        <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 dark:bg-emerald-400" />
                        Connected
                      </Badge>
                    ) : null}
                    {card.profile?.migratedFromLegacy ? (
                      <Badge variant="outline" className="px-1.5 py-0 text-[10px]">
                        Migrated
                      </Badge>
                    ) : null}
                  </div>

                  <p className="mt-0.5 text-[11px] leading-relaxed text-muted-foreground">
                    {card.catalog.description}
                  </p>

                  {openAiScopeCopy ? (
                    <p className="mt-1 text-[10px] leading-relaxed text-muted-foreground">{openAiScopeCopy}</p>
                  ) : null}

                  <div className="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-[10px] text-muted-foreground">
                    <span>Model: {card.profile?.modelId ?? card.catalog.defaultModelId ?? "Not configured"}</span>
                    {card.profile ? <span>Profile ID: {card.profile.profileId}</span> : null}
                    {migratedAt ? <span>Migrated {migratedAt}</span> : null}
                  </div>

                  {inlineStatus ? (
                    <Alert
                      variant={inlineStatus.tone === "error" ? "destructive" : "default"}
                      className={cn(
                        "mt-2 py-2.5",
                        inlineStatus.tone === "warning"
                          ? "border-amber-500/30 bg-amber-500/5 text-amber-700 dark:text-amber-200"
                          : "border-destructive/30 bg-destructive/5",
                      )}
                    >
                      <AlertCircle className="h-4 w-4" />
                      <AlertDescription className="text-[12px] leading-relaxed">
                        <span>{inlineStatus.message}</span>
                        {inlineStatus.recovery ? <span className="mt-1 block">{inlineStatus.recovery}</span> : null}
                      </AlertDescription>
                    </Alert>
                  ) : null}
                </div>

                <div className="flex shrink-0 flex-wrap items-center justify-end gap-2">
                  {isSelected ? (
                    <Badge variant="secondary" className="text-[10px]">
                      Using this
                    </Badge>
                  ) : (
                    <Button
                      size="sm"
                      variant="outline"
                      className="h-7 text-[11px]"
                      disabled={isSaving || isRefreshing || !onUpsertProviderProfile}
                      onClick={() => void handleActivate(card)}
                    >
                      Use this profile
                    </Button>
                  )}

                  {isEditing ? null : isOpenAi ? (
                    <Button
                      size="sm"
                      variant="secondary"
                      className="h-7 text-[11px]"
                      disabled={isSaving || isRefreshing}
                      onClick={() => openEditor(card)}
                    >
                      Edit label
                    </Button>
                  ) : (
                    <Button
                      size="sm"
                      variant={hasSavedOpenRouterKey ? "secondary" : "outline"}
                      className="h-7 text-[11px]"
                      disabled={isSaving || isRefreshing}
                      onClick={() => openEditor(card)}
                    >
                      {hasSavedOpenRouterKey ? "Edit setup" : "Set up"}
                    </Button>
                  )}

                  {shouldRenderOpenAiAuth ? (
                    isOpenAiConnected ? (
                      <Button
                        variant="outline"
                        size="sm"
                        className="h-7 gap-1 text-[11px]"
                        disabled={pendingAuth !== null || isRefreshing || isSaving}
                        onClick={() => void handleOpenAiDisconnect()}
                      >
                        {pendingAuth === "logout" ? (
                          <LoaderCircle className="h-3 w-3 animate-spin" />
                        ) : (
                          <LogOut className="h-3 w-3" />
                        )}
                        Sign out
                      </Button>
                    ) : isOpenAiInProgress ? (
                      <Badge variant="secondary" className="gap-1 text-[10px]">
                        <LoaderCircle className="h-3 w-3 animate-spin" />
                        Connecting…
                      </Badge>
                    ) : !hasSelectedProject ? (
                      <Badge variant="outline" className="text-[10px]">
                        {openAiMissingProjectLabel}
                      </Badge>
                    ) : (
                      <Button
                        size="sm"
                        className="h-7 gap-1 text-[11px]"
                        disabled={pendingAuth !== null || isRefreshing || isSaving}
                        onClick={() => void handleOpenAiConnect()}
                      >
                        {pendingAuth === "login" ? (
                          <LoaderCircle className="h-3 w-3 animate-spin" />
                        ) : (
                          <LogIn className="h-3 w-3" />
                        )}
                        Sign in
                      </Button>
                    )
                  ) : null}
                </div>
              </div>

              {isEditing ? (
                <div className="mt-3 grid gap-3 rounded-md border border-dashed border-border/80 bg-background/80 p-3">
                  <div className="space-y-1.5">
                    <Label htmlFor={`${card.key}-label`} className="text-[11px]">
                      Profile label
                    </Label>
                    <Input
                      id={`${card.key}-label`}
                      className="h-8 text-[12px]"
                      disabled={isSaving || isRefreshing}
                      onChange={(event) =>
                        setDraft(card, {
                          ...draft,
                          label: event.target.value,
                        })
                      }
                      placeholder={card.catalog.label}
                      value={draft.label}
                    />
                  </div>

                  <div className="space-y-1.5">
                    <Label htmlFor={`${card.key}-model`} className="text-[11px]">
                      Model ID
                    </Label>
                    <Input
                      id={`${card.key}-model`}
                      className="h-8 font-mono text-[12px]"
                      disabled={isSaving || isRefreshing || isOpenAi}
                      onChange={(event) =>
                        setDraft(card, {
                          ...draft,
                          modelId: event.target.value,
                        })
                      }
                      placeholder={card.catalog.defaultModelId}
                      value={isOpenAi ? card.catalog.defaultModelId ?? draft.modelId : draft.modelId}
                    />
                    <p className="text-[10px] text-muted-foreground">
                      {isOpenAi ? "OpenAI Codex stays pinned to the desktop runtime model." : "Use the exact OpenRouter model slug."}
                    </p>
                  </div>

                  {isOpenRouter ? (
                    <div className="space-y-1.5">
                      <div className="flex items-center justify-between gap-3">
                        <Label htmlFor={`${card.key}-api-key`} className="text-[11px]">
                          API Key
                        </Label>
                        {hasSavedOpenRouterKey ? (
                          <Badge variant="secondary" className="gap-1 text-[10px]">
                            <Check className="h-2.5 w-2.5" strokeWidth={3} />
                            Key saved
                          </Badge>
                        ) : null}
                      </div>
                      <div className="flex gap-2">
                        <Input
                          id={`${card.key}-api-key`}
                          type="password"
                          autoComplete="off"
                          spellCheck={false}
                          className="h-8 flex-1 font-mono text-[12px]"
                          disabled={isSaving || isRefreshing}
                          onChange={(event) =>
                            setDraft(card, {
                              ...draft,
                              openrouterApiKey: event.target.value,
                              clearOpenrouterApiKey:
                                event.target.value.trim().length > 0 ? false : draft.clearOpenrouterApiKey,
                            })
                          }
                          placeholder={hasSavedOpenRouterKey ? "Leave blank to keep current key" : "Paste API key"}
                          value={draft.openrouterApiKey}
                        />
                        {hasSavedOpenRouterKey ? (
                          <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            className="h-8 px-2 text-[11px]"
                            disabled={isSaving || isRefreshing}
                            onClick={() =>
                              setDraft(card, {
                                ...draft,
                                openrouterApiKey: "",
                                clearOpenrouterApiKey: !draft.clearOpenrouterApiKey,
                              })
                            }
                          >
                            {draft.clearOpenrouterApiKey ? "Keep" : "Clear"}
                          </Button>
                        ) : null}
                      </div>
                      <p
                        className={cn(
                          "text-[10px]",
                          draft.clearOpenrouterApiKey ? "text-destructive/80" : "text-muted-foreground",
                        )}
                      >
                        {draft.clearOpenrouterApiKey
                          ? "Saved key will be removed"
                          : hasSavedOpenRouterKey
                            ? "Blank keeps the current key"
                            : "Required for OpenRouter"}
                      </p>
                    </div>
                  ) : null}

                  <div className="flex items-center gap-2">
                    <Button
                      size="sm"
                      className="h-7 gap-1 text-[11px]"
                      disabled={isSaving || isRefreshing || !onUpsertProviderProfile}
                      onClick={() => void handleSave(card)}
                    >
                      {isSaving ? <LoaderCircle className="h-3 w-3 animate-spin" /> : <Check className="h-3 w-3" />}
                      Save
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="h-7 text-[11px]"
                      disabled={isSaving || isRefreshing}
                      onClick={() => closeEditor(card.key)}
                    >
                      Cancel
                    </Button>
                  </div>
                </div>
              ) : null}
            </div>
          )
        })}

        {unavailableProviders.map((provider) => (
          <div key={provider.id} className="rounded-lg border border-border/70 bg-card/30 px-4 py-3">
            <div className="flex items-center gap-3">
              <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border bg-secondary/40">
                <provider.Icon className="h-4 w-4 text-muted-foreground" />
              </div>
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-1.5">
                  <p className="text-[13px] font-medium text-muted-foreground">{provider.label}</p>
                  <Badge variant="outline" className="gap-1 text-[10px]">
                    <Lock className="h-2.5 w-2.5" />
                    Unavailable
                  </Badge>
                </div>
                <p className="mt-0.5 text-[11px] text-muted-foreground">{provider.description}</p>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
