import { useEffect, useState, type ElementType } from "react"
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
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { cn } from "@/lib/utils"
import type {
  OperatorActionErrorView,
  ProviderModelCatalogLoadStatus,
  ProviderProfilesLoadStatus,
  ProviderProfilesSaveStatus,
} from "@/src/features/cadence/use-cadence-desktop-state"
import {
  getProviderMismatchCopy,
  resolveSelectedRuntimeProvider,
} from "@/src/features/cadence/use-cadence-desktop-state/runtime-provider"
import type { CloudProviderPreset } from "@/src/lib/cadence-model/provider-presets"
import {
  getActiveProviderProfile,
  getProviderModelCatalogFetchedAt,
  type ProviderModelCatalogDto,
  type ProviderModelDto,
  type ProviderProfilesDto,
  type ProviderProfileDto,
  type RuntimeSessionView,
  type UpsertProviderProfileRequestDto,
  upsertProviderProfileRequestSchema,
} from "@/src/lib/cadence-model"
import {
  formatProviderConnectionLabel,
  isApiKeyCloudProvider,
  isLocalCloudProvider,
  listCloudProviderPresets,
  usesAmbientCloudProvider,
} from "@/src/lib/cadence-model/provider-presets"

type SupportedProviderId = ProviderProfileDto["providerId"]
type AuthPending = "login" | "logout" | null

type ProviderDraft = {
  label: string
  modelId: string
  apiKey: string
  clearApiKey: boolean
  baseUrl: string
  apiVersion: string
  region: string
  projectId: string
}

interface ProviderProfileCard {
  key: string
  preset: CloudProviderPreset
  profile: ProviderProfileDto | null
}

interface ProviderModelChoice {
  modelId: string
  label: string
  groupId: string
  groupLabel: string
  availability: "available" | "orphaned"
  availabilityLabel: string
}

interface ProviderModelChoiceGroup {
  id: string
  label: string
  items: ProviderModelChoice[]
}

interface ProviderModelCatalogState {
  profileId: string | null
  catalog: ProviderModelCatalogDto | null
  loadStatus: ProviderModelCatalogLoadStatus
  refreshError: OperatorActionErrorView | null
  stateLabel: string
  detail: string
  tone: "default" | "warning"
  fetchedAt: string | null
  lastSuccessAt: string | null
  choices: ProviderModelChoice[]
  selectedChoice: ProviderModelChoice | null
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

const MODEL_GROUP_LABELS: Record<string, string> = {
  anthropic: "Anthropic",
  deepseek: "DeepSeek",
  google: "Google",
  meta: "Meta",
  "meta-llama": "Meta Llama",
  mistral: "Mistral",
  moonshot: "Moonshot",
  moonshotai: "Moonshot",
  openai: "OpenAI",
  openrouter: "OpenRouter",
  "x-ai": "xAI",
  xai: "xAI",
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

function normalizeOptionalText(value: string): string | null {
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : null
}

function createDraft(card: ProviderProfileCard): ProviderDraft {
  return {
    label: card.profile?.label ?? card.preset.defaultProfileLabel,
    modelId: card.profile?.modelId ?? card.preset.defaultModelId,
    apiKey: "",
    clearApiKey: false,
    baseUrl: card.profile?.baseUrl ?? "",
    apiVersion: card.profile?.apiVersion ?? "",
    region: card.profile?.region ?? "",
    projectId: card.profile?.projectId ?? "",
  }
}

function getProfileCards(providerProfiles: ProviderProfilesDto | null): ProviderProfileCard[] {
  const cards: ProviderProfileCard[] = []
  const activeProfileId = providerProfiles?.activeProfileId ?? null

  for (const preset of listCloudProviderPresets()) {
    const matches = (providerProfiles?.profiles ?? [])
      .filter((profile) => profile.providerId === preset.providerId)
      .sort((left, right) => {
        const leftActive = left.profileId === activeProfileId
        const rightActive = right.profileId === activeProfileId

        if (leftActive !== rightActive) return leftActive ? -1 : 1
        return left.label.localeCompare(right.label)
      })

    if (matches.length === 0) {
      cards.push({
        key: `${preset.providerId}-placeholder`,
        preset,
        profile: null,
      })
      continue
    }

    cards.push(
      ...matches.map((profile) => ({
        key: profile.profileId,
        preset,
        profile,
      })),
    )
  }

  return cards
}

function getProviderReadinessBadge(profile: ProviderProfileDto | null) {
  if (!profile || profile.providerId === "openai_codex") return null

  if (profile.readiness.status === "ready") {
    if (profile.readiness.proof === "local") {
      return {
        label: "Local",
        className: "border border-sky-500/30 bg-sky-500/10 text-sky-600 dark:text-sky-300",
      }
    }

    if (profile.readiness.proof === "ambient") {
      return {
        label: "Ambient auth",
        className: "border border-cyan-500/30 bg-cyan-500/10 text-cyan-600 dark:text-cyan-300",
      }
    }

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

  if (isLocalCloudProvider(profile.providerId)) {
    return {
      label: "Needs local setup",
      className: "border border-border bg-secondary text-muted-foreground",
    }
  }

  if (usesAmbientCloudProvider(profile.providerId)) {
    return {
      label: "Needs ambient setup",
      className: "border border-border bg-secondary text-muted-foreground",
    }
  }

  return {
    label: "Needs key",
    className: "border border-border bg-secondary text-muted-foreground",
  }
}

function hasSavedApiKeyCredential(card: ProviderProfileCard): boolean {
  return Boolean(
    card.profile &&
      isApiKeyCloudProvider(card.profile.providerId) &&
      card.profile.readiness.status !== "missing",
  )
}

function getProfileId(card: ProviderProfileCard): string {
  return card.profile?.profileId ?? card.preset.defaultProfileId
}

function getMissingConnectionFieldLabels(card: ProviderProfileCard, draft: ProviderDraft): string[] {
  const missing: string[] = []

  if (card.preset.baseUrlMode === "required" && !draft.baseUrl.trim()) {
    missing.push("base URL")
  }

  if (card.preset.apiVersionMode === "required" && !draft.apiVersion.trim()) {
    missing.push("API version")
  }

  if (card.preset.regionMode === "required" && !draft.region.trim()) {
    missing.push("region")
  }

  if (card.preset.projectIdMode === "required" && !draft.projectId.trim()) {
    missing.push("project ID")
  }

  return missing
}

function getConnectionLabel(card: ProviderProfileCard): string {
  return formatProviderConnectionLabel({
    providerId: card.preset.providerId,
    baseUrl: card.profile?.baseUrl ?? null,
    region: card.profile?.region ?? null,
    projectId: card.profile?.projectId ?? null,
  })
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
  const baseUrl = card.preset.baseUrlMode === "none" ? null : normalizeOptionalText(draft.baseUrl)
  const apiVersion =
    card.preset.apiVersionMode === "none"
      ? null
      : baseUrl
        ? normalizeOptionalText(draft.apiVersion)
        : null
  const region = card.preset.regionMode === "none" ? null : normalizeOptionalText(draft.region)
  const projectId =
    card.preset.projectIdMode === "none" ? null : normalizeOptionalText(draft.projectId)

  return {
    profileId: getProfileId(card),
    providerId: card.preset.providerId,
    runtimeKind: card.preset.runtimeKind,
    label: draft.label.trim(),
    modelId:
      card.preset.providerId === "openai_codex"
        ? card.preset.defaultModelId
        : draft.modelId.trim(),
    presetId: card.preset.presetId ?? null,
    baseUrl,
    apiVersion,
    region,
    projectId,
    apiKey:
      card.preset.authMode === "api_key"
        ? draft.clearApiKey
          ? ""
          : normalizeOptionalText(draft.apiKey)
        : null,
    activate,
  }
}

function getCatalogRefreshError(
  catalog: ProviderModelCatalogDto | null,
  loadError: OperatorActionErrorView | null,
): OperatorActionErrorView | null {
  if (catalog?.lastRefreshError) {
    return {
      code: catalog.lastRefreshError.code,
      message: catalog.lastRefreshError.message,
      retryable: catalog.lastRefreshError.retryable,
    }
  }

  return loadError
}

function getModelGroupLabel(modelId: string, providerLabel: string): { groupId: string; groupLabel: string } {
  const trimmedModelId = modelId.trim()
  const namespace = trimmedModelId.includes("/") ? trimmedModelId.split("/")[0]?.trim() ?? "" : ""
  if (namespace.length === 0) {
    return {
      groupId: providerLabel.trim().toLowerCase().replace(/[^a-z0-9]+/g, "_") || "provider_models",
      groupLabel: providerLabel,
    }
  }

  const normalizedNamespace = namespace.toLowerCase()
  const knownLabel = MODEL_GROUP_LABELS[normalizedNamespace]
  if (knownLabel) {
    return {
      groupId: normalizedNamespace.replace(/[^a-z0-9]+/g, "_"),
      groupLabel: knownLabel,
    }
  }

  return {
    groupId: normalizedNamespace.replace(/[^a-z0-9]+/g, "_"),
    groupLabel: namespace,
  }
}

function buildProviderModelChoice(model: ProviderModelDto, providerLabel: string): ProviderModelChoice | null {
  const modelId = model.modelId.trim()
  if (modelId.length === 0) {
    return null
  }

  const displayName = model.displayName.trim() || modelId
  const { groupId, groupLabel } = getModelGroupLabel(modelId, providerLabel)

  return {
    modelId,
    label: displayName === modelId ? modelId : `${displayName} · ${modelId}`,
    groupId,
    groupLabel,
    availability: "available",
    availabilityLabel: "Available",
  }
}

function buildOrphanedProviderModelChoice(modelId: string): ProviderModelChoice | null {
  const trimmedModelId = modelId.trim()
  if (trimmedModelId.length === 0) {
    return null
  }

  return {
    modelId: trimmedModelId,
    label: `${trimmedModelId} · unavailable`,
    groupId: "current_selection",
    groupLabel: "Current selection",
    availability: "orphaned",
    availabilityLabel: "Unavailable",
  }
}

function groupProviderModelChoices(choices: ProviderModelChoice[]): ProviderModelChoiceGroup[] {
  const groups = new Map<string, ProviderModelChoiceGroup>()

  for (const choice of choices) {
    const existingGroup = groups.get(choice.groupId)
    if (existingGroup) {
      existingGroup.items.push(choice)
      continue
    }

    groups.set(choice.groupId, {
      id: choice.groupId,
      label: choice.groupLabel,
      items: [choice],
    })
  }

  return Array.from(groups.values())
}

function getCardCatalogState(options: {
  card: ProviderProfileCard
  providerModelCatalogs: Record<string, ProviderModelCatalogDto>
  providerModelCatalogLoadStatuses: Record<string, ProviderModelCatalogLoadStatus>
  providerModelCatalogLoadErrors: Record<string, OperatorActionErrorView | null>
  selectedModelId: string | null
}): ProviderModelCatalogState {
  const profileId = options.card.profile?.profileId ?? options.card.preset.defaultProfileId ?? null
  const catalog = profileId ? options.providerModelCatalogs[profileId] ?? null : null
  const loadStatus: ProviderModelCatalogLoadStatus = profileId
    ? options.providerModelCatalogLoadStatuses[profileId] ?? "idle"
    : "idle"
  const refreshError = getCatalogRefreshError(
    catalog,
    profileId ? options.providerModelCatalogLoadErrors[profileId] ?? null : null,
  )
  const discoveredChoices: ProviderModelChoice[] = []
  const seenModelIds = new Set<string>()

  for (const model of catalog?.models ?? []) {
    const nextChoice = buildProviderModelChoice(model, options.card.preset.label)
    if (!nextChoice || seenModelIds.has(nextChoice.modelId)) {
      continue
    }

    seenModelIds.add(nextChoice.modelId)
    discoveredChoices.push(nextChoice)
  }

  const selectedModelId = options.selectedModelId?.trim() ?? ""
  const selectedDiscoveredChoice = selectedModelId
    ? discoveredChoices.find((choice) => choice.modelId === selectedModelId) ?? null
    : null
  const selectedChoice =
    selectedDiscoveredChoice ?? (selectedModelId ? buildOrphanedProviderModelChoice(selectedModelId) : null)
  const choices =
    selectedChoice && selectedChoice.availability === "orphaned"
      ? [selectedChoice, ...discoveredChoices]
      : discoveredChoices

  if (catalog?.source === "live" && discoveredChoices.length > 0) {
    return {
      profileId,
      catalog,
      loadStatus,
      refreshError,
      stateLabel: "Live catalog",
      detail:
        loadStatus === "loading"
          ? `Refreshing ${options.card.preset.label} model discovery while keeping ${discoveredChoices.length} discovered model${
              discoveredChoices.length === 1 ? "" : "s"
            } visible.`
          : `Showing ${discoveredChoices.length} discovered model${
              discoveredChoices.length === 1 ? "" : "s"
            } for ${options.card.profile?.label ?? options.card.preset.label}.`,
      tone: "default",
      fetchedAt: getProviderModelCatalogFetchedAt(catalog),
      lastSuccessAt: catalog.lastSuccessAt ?? null,
      choices,
      selectedChoice,
    }
  }

  if (discoveredChoices.length > 0) {
    return {
      profileId,
      catalog,
      loadStatus,
      refreshError,
      stateLabel: catalog?.source === "cache" ? "Cached catalog" : "Stale catalog",
      detail: refreshError?.message?.trim()
        ? `${refreshError.message} Cadence is keeping the last successful model catalog for ${options.card.profile?.label ?? options.card.preset.label} visible.`
        : `Cadence is keeping the last successful model catalog for ${options.card.profile?.label ?? options.card.preset.label} visible.`,
      tone: "warning",
      fetchedAt: getProviderModelCatalogFetchedAt(catalog),
      lastSuccessAt: catalog?.lastSuccessAt ?? null,
      choices,
      selectedChoice,
    }
  }

  if (loadStatus === "loading") {
    return {
      profileId,
      catalog,
      loadStatus,
      refreshError,
      stateLabel: "Catalog unavailable",
      detail: `Loading the ${options.card.preset.label} model catalog. Cadence is keeping configured model truth visible without reopening free-text editing.`,
      tone: "default",
      fetchedAt: getProviderModelCatalogFetchedAt(catalog),
      lastSuccessAt: catalog?.lastSuccessAt ?? null,
      choices,
      selectedChoice,
    }
  }

  const unavailableDetail = selectedChoice
    ? `${selectedChoice.modelId} remains visible as the saved model, but discovery cannot confirm it right now.`
    : `Cadence does not have a discovered model catalog for ${options.card.profile?.label ?? options.card.preset.label} yet.`

  return {
    profileId,
    catalog,
    loadStatus,
    refreshError,
    stateLabel: "Catalog unavailable",
    detail: refreshError?.message?.trim()
      ? `${refreshError.message} ${unavailableDetail}`
      : unavailableDetail,
    tone: "warning",
    fetchedAt: getProviderModelCatalogFetchedAt(catalog),
    lastSuccessAt: catalog?.lastSuccessAt ?? null,
    choices,
    selectedChoice,
  }
}

export interface ProviderProfileFormProps {
  providerProfiles: ProviderProfilesDto | null
  providerProfilesLoadStatus: ProviderProfilesLoadStatus
  providerProfilesLoadError: OperatorActionErrorView | null
  providerProfilesSaveStatus: ProviderProfilesSaveStatus
  providerProfilesSaveError: OperatorActionErrorView | null
  providerModelCatalogs?: Record<string, ProviderModelCatalogDto>
  providerModelCatalogLoadStatuses?: Record<string, ProviderModelCatalogLoadStatus>
  providerModelCatalogLoadErrors?: Record<string, OperatorActionErrorView | null>
  onRefreshProviderProfiles?: (options?: { force?: boolean }) => Promise<ProviderProfilesDto>
  onRefreshProviderModelCatalog?: (
    profileId: string,
    options?: { force?: boolean },
  ) => Promise<ProviderModelCatalogDto>
  onUpsertProviderProfile?: (request: UpsertProviderProfileRequestDto) => Promise<ProviderProfilesDto>
  onSetActiveProviderProfile?: (profileId: string) => Promise<ProviderProfilesDto>
  runtimeSession?: RuntimeSessionView | null
  hasSelectedProject?: boolean
  onStartLogin?: () => Promise<RuntimeSessionView | null>
  onLogout?: () => Promise<RuntimeSessionView | null>
  openAiMissingProjectLabel?: string
}

export function ProviderProfileForm({
  providerProfiles,
  providerProfilesLoadStatus,
  providerProfilesLoadError,
  providerProfilesSaveStatus,
  providerProfilesSaveError,
  providerModelCatalogs = {},
  providerModelCatalogLoadStatuses = {},
  providerModelCatalogLoadErrors = {},
  onRefreshProviderProfiles,
  onRefreshProviderModelCatalog,
  onUpsertProviderProfile,
  onSetActiveProviderProfile,
  runtimeSession,
  hasSelectedProject = false,
  onStartLogin,
  onLogout,
  openAiMissingProjectLabel = "Open a project",
}: ProviderProfileFormProps) {
  const [editingCardKey, setEditingCardKey] = useState<string | null>(null)
  const [drafts, setDrafts] = useState<Record<string, ProviderDraft>>({})
  const [pendingAuth, setPendingAuth] = useState<AuthPending>(null)
  const [formError, setFormError] = useState<string | null>(null)
  const [authError, setAuthError] = useState<string | null>(null)

  const cards = getProfileCards(providerProfiles)
  const isRefreshing = providerProfilesLoadStatus === "loading"
  const isSaving = providerProfilesSaveStatus === "running"
  const selectedProfile = getActiveProviderProfile(providerProfiles)
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

  useEffect(() => {
    if (!onRefreshProviderModelCatalog) {
      return
    }

    for (const card of cards) {
      if (!card.profile) {
        continue
      }

      const profileId = card.profile.profileId
      const loadStatus = providerModelCatalogLoadStatuses[profileId] ?? "idle"
      const hasCatalog = Boolean(providerModelCatalogs[profileId])
      if (loadStatus === "idle" && !hasCatalog) {
        void onRefreshProviderModelCatalog(profileId, { force: false }).catch(() => undefined)
      }
    }
  }, [cards, onRefreshProviderModelCatalog, providerModelCatalogLoadStatuses, providerModelCatalogs])

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

    if (card.preset.providerId !== "openai_codex" && !draft.modelId.trim()) {
      setFormError("Model ID is required.")
      return
    }

    if (card.preset.baseUrlMode === "required" && !draft.baseUrl.trim()) {
      setFormError(`${card.preset.label} requires a base URL.`)
      return
    }

    if (card.preset.apiVersionMode === "required" && !draft.apiVersion.trim()) {
      setFormError(`${card.preset.label} requires an API version.`)
      return
    }

    if (card.preset.regionMode === "required" && !draft.region.trim()) {
      setFormError(`${card.preset.label} requires a region.`)
      return
    }

    if (card.preset.projectIdMode === "required" && !draft.projectId.trim()) {
      setFormError(`${card.preset.label} requires a project ID.`)
      return
    }

    if (card.preset.authMode === "api_key") {
      const hasSavedKey = hasSavedApiKeyCredential(card)
      if (!hasSavedKey && !draft.clearApiKey && !draft.apiKey.trim()) {
        setFormError(`${card.preset.label} requires an API key.`)
        return
      }
    }

    const activate = providerProfiles?.activeProfileId?.trim()
      ? providerProfiles.activeProfileId === getProfileId(card)
      : card.profile?.active ?? false
    const parsedRequest = upsertProviderProfileRequestSchema.safeParse(
      buildUpsertRequest(card, draft, activate),
    )

    if (!parsedRequest.success) {
      setFormError(parsedRequest.error.issues[0]?.message ?? "Cadence rejected the provider profile request.")
      return
    }

    setFormError(null)

    try {
      await onUpsertProviderProfile(parsedRequest.data)
      closeEditor(card.key)
    } catch {
      setDraft(card, {
        ...draft,
        apiKey: "",
      })
    }
  }

  async function handleActivate(card: ProviderProfileCard) {
    if (isCardSelected(providerProfiles, card)) return

    setFormError(null)
    setAuthError(null)

    if (!card.profile) {
      const missingFields = getMissingConnectionFieldLabels(card, createDraft(card))
      if (missingFields.length > 0) {
        openEditor(card)
        setFormError(
          `${card.preset.label} needs ${missingFields.join(" and ")} before it can be activated.`,
        )
        return
      }
    }

    try {
      if (card.profile) {
        await onSetActiveProviderProfile?.(card.profile.profileId)
        return
      }

      const parsedRequest = upsertProviderProfileRequestSchema.safeParse(
        buildUpsertRequest(card, createDraft(card), true),
      )
      if (!parsedRequest.success) {
        openEditor(card)
        setFormError(parsedRequest.error.issues[0]?.message ?? "Cadence rejected the provider profile request.")
        return
      }

      await onUpsertProviderProfile?.(parsedRequest.data)
    } catch {
      // Hook state surfaces the typed save error while the last truthful snapshot remains visible.
    }
  }

  async function handleRefreshCatalog(card: ProviderProfileCard) {
    const profileId = card.profile?.profileId
    if (!profileId || !onRefreshProviderModelCatalog) {
      return
    }

    setFormError(null)
    await onRefreshProviderModelCatalog(profileId, { force: true }).catch(() => undefined)
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
    <div className="flex flex-col gap-5">
      {providerProfilesLoadError ? (
        <Alert variant="destructive" className="border-destructive/30 bg-destructive/5 py-3">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription className="text-[13px]">
            {errorViewMessage(providerProfilesLoadError, "Failed to load app-local provider profiles.")}
            {onRefreshProviderProfiles ? (
              <Button
                variant="outline"
                size="sm"
                className="mt-2.5 h-7 gap-1 text-[11px]"
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
        <Alert variant="destructive" className="border-destructive/30 bg-destructive/5 py-3">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription className="text-[13px]">
            {errorViewMessage(providerProfilesSaveError, "Failed to save the selected provider profile.")}
          </AlertDescription>
        </Alert>
      ) : null}

      {selectedProfileUnavailableMessage ? (
        <Alert variant="destructive" className="border-destructive/30 bg-destructive/5 py-3">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription className="text-[13px]">{selectedProfileUnavailableMessage}</AlertDescription>
        </Alert>
      ) : null}

      {formError ? (
        <Alert variant="destructive" className="border-destructive/30 bg-destructive/5 py-3">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription className="text-[13px]">{formError}</AlertDescription>
        </Alert>
      ) : null}

      <div className="grid gap-3">
        {cards.map((card) => {
          const draft = getDraft(card)
          const Icon = PROVIDER_ICON_BY_ID[card.preset.providerId]
          const isEditing = editingCardKey === card.key
          const isApiKeyProvider = card.preset.authMode === "api_key"
          const isOpenAi = card.preset.providerId === "openai_codex"
          const isSelected = isCardSelected(providerProfiles, card)
          const readinessBadge = getProviderReadinessBadge(card.profile)
          const hasSavedApiKey = hasSavedApiKeyCredential(card)
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
          const selectedModelId = (draft.modelId.trim() || card.profile?.modelId || card.preset.defaultModelId || "").trim() || null
          const cardCatalogState = getCardCatalogState({
            card,
            providerModelCatalogs,
            providerModelCatalogLoadStatuses,
            providerModelCatalogLoadErrors,
            selectedModelId,
          })
          const modelChoiceGroups = groupProviderModelChoices(cardCatalogState.choices)
          const isCatalogRefreshing = cardCatalogState.loadStatus === "loading"
          const canRefreshCatalog = Boolean(onRefreshProviderModelCatalog && card.profile && card.preset.supportsCatalogRefresh)
          const connectionLabel = getConnectionLabel(card)

          return (
            <div
              key={card.key}
              className={cn(
                "rounded-lg border bg-card px-5 py-4 transition-colors",
                isSelected ? "border-primary/30 bg-primary/[0.03]" : "border-border",
              )}
            >
              <div className="flex items-start gap-3.5">
                <div
                  className={cn(
                    "flex h-9 w-9 shrink-0 items-center justify-center rounded-md border bg-secondary/60",
                    isSelected ? "border-primary/40 text-primary" : "border-border",
                  )}
                >
                  <Icon className="h-4 w-4 text-foreground/70" />
                </div>

                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <p className="text-[14px] font-medium text-foreground">
                      {card.profile?.label ?? card.preset.label}
                    </p>
                    {isSelected ? (
                      <Badge variant="secondary" className="px-2 py-0 text-[11px]">
                        Active
                      </Badge>
                    ) : null}
                    {readinessBadge ? (
                      <Badge className={cn("px-2 py-0 text-[11px] font-medium", readinessBadge.className)}>
                        {readinessBadge.label}
                      </Badge>
                    ) : null}
                    {isOpenAi && isOpenAiConnected ? (
                      <Badge
                        variant="secondary"
                        className="gap-1.5 border border-emerald-500/30 bg-emerald-500/10 px-2 py-0 text-[11px] font-medium text-emerald-500 dark:text-emerald-400"
                      >
                        <span className="h-1.5 w-1.5 rounded-full bg-emerald-500 dark:bg-emerald-400" />
                        Connected
                      </Badge>
                    ) : null}
                  </div>

                  <p className="mt-1 text-[12px] leading-relaxed text-muted-foreground">
                    {card.preset.description}
                  </p>

                  <p className="mt-1.5 text-[11.5px] text-muted-foreground">
                    Model:{" "}
                    <span className="font-medium text-foreground/80">
                      {card.profile?.modelId ?? card.preset.defaultModelId ?? "Not configured"}
                    </span>
                  </p>

                  <p className="mt-1 text-[11.5px] text-muted-foreground">Connection: {connectionLabel}</p>

                  {inlineStatus ? (
                    <Alert
                      variant={inlineStatus.tone === "error" ? "destructive" : "default"}
                      className={cn(
                        "mt-2.5 py-3",
                        inlineStatus.tone === "warning"
                          ? "border-amber-500/30 bg-amber-500/5 text-amber-700 dark:text-amber-200"
                          : "border-destructive/30 bg-destructive/5",
                      )}
                    >
                      <AlertCircle className="h-4 w-4" />
                      <AlertDescription className="text-[13px] leading-relaxed">
                        <span>{inlineStatus.message}</span>
                        {inlineStatus.recovery ? <span className="mt-1 block">{inlineStatus.recovery}</span> : null}
                      </AlertDescription>
                    </Alert>
                  ) : null}
                </div>

                <div className="flex shrink-0 flex-wrap items-center justify-end gap-2">
                  {isSelected ? null : (
                    <Button
                      size="sm"
                      variant="outline"
                      className="h-8 text-[12px]"
                      disabled={isSaving || isRefreshing || !onUpsertProviderProfile}
                      onClick={() => void handleActivate(card)}
                    >
                      Use this
                    </Button>
                  )}

                  {isEditing ? null : isOpenAi ? (
                    <Button
                      size="sm"
                      variant="secondary"
                      className="h-8 text-[12px]"
                      disabled={isSaving || isRefreshing}
                      onClick={() => openEditor(card)}
                    >
                      Edit label
                    </Button>
                  ) : (
                    <Button
                      size="sm"
                      variant={hasSavedApiKey ? "secondary" : "outline"}
                      className="h-8 text-[12px]"
                      disabled={isSaving || isRefreshing}
                      onClick={() => openEditor(card)}
                    >
                      {hasSavedApiKey ? "Edit" : "Set up"}
                    </Button>
                  )}

                  {shouldRenderOpenAiAuth ? (
                    isOpenAiConnected ? (
                      <Button
                        variant="outline"
                        size="sm"
                        className="h-8 gap-1.5 text-[12px]"
                        disabled={pendingAuth !== null || isRefreshing || isSaving}
                        onClick={() => void handleOpenAiDisconnect()}
                      >
                        {pendingAuth === "logout" ? (
                          <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
                        ) : (
                          <LogOut className="h-3.5 w-3.5" />
                        )}
                        Sign out
                      </Button>
                    ) : isOpenAiInProgress ? (
                      <Badge variant="secondary" className="gap-1.5 text-[11px]">
                        <LoaderCircle className="h-3 w-3 animate-spin" />
                        Connecting…
                      </Badge>
                    ) : !hasSelectedProject ? (
                      <Badge variant="outline" className="text-[11px]">
                        {openAiMissingProjectLabel}
                      </Badge>
                    ) : (
                      <Button
                        size="sm"
                        className="h-8 gap-1.5 text-[12px]"
                        disabled={pendingAuth !== null || isRefreshing || isSaving}
                        onClick={() => void handleOpenAiConnect()}
                      >
                        {pendingAuth === "login" ? (
                          <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
                        ) : (
                          <LogIn className="h-3.5 w-3.5" />
                        )}
                        Sign in
                      </Button>
                    )
                  ) : null}
                </div>
              </div>

              {isEditing ? (
                <div className="mt-3.5 grid gap-3.5 rounded-md border border-dashed border-border/80 bg-background/80 p-3.5">
                  <div className="space-y-2">
                    <Label htmlFor={`${card.key}-label`} className="text-[12px]">
                      Profile label
                    </Label>
                    <Input
                      id={`${card.key}-label`}
                      className="h-9 text-[13px]"
                      disabled={isSaving || isRefreshing}
                      onChange={(event) =>
                        setDraft(card, {
                          ...draft,
                          label: event.target.value,
                        })
                      }
                      placeholder={card.preset.defaultProfileLabel}
                      value={draft.label}
                    />
                  </div>

                  <div className="space-y-2">
                    <div className="flex items-center justify-between gap-3">
                      <Label htmlFor={`${card.key}-model`} className="text-[12px]">
                        Model
                      </Label>
                      {canRefreshCatalog ? (
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          className="h-7 gap-1.5 px-2.5 text-[11px]"
                          disabled={isSaving || isRefreshing || isCatalogRefreshing}
                          onClick={() => void handleRefreshCatalog(card)}
                        >
                          {isCatalogRefreshing ? <LoaderCircle className="h-3 w-3 animate-spin" /> : null}
                          Refresh models
                        </Button>
                      ) : null}
                    </div>

                    {isOpenAi ? (
                      <div className="rounded-md border border-border/80 bg-muted/25 px-3.5 py-3">
                        <p className="text-[13px] font-medium text-foreground">OpenAI Codex</p>
                        <p className="mt-1 font-mono text-[12px] text-muted-foreground">
                          {card.preset.defaultModelId}
                        </p>
                      </div>
                    ) : cardCatalogState.choices.length > 0 ? (
                      <Select
                        disabled={isSaving || isRefreshing}
                        value={draft.modelId}
                        onValueChange={(value) =>
                          setDraft(card, {
                            ...draft,
                            modelId: value,
                          })
                        }
                      >
                        <SelectTrigger id={`${card.key}-model`} className="h-9 w-full text-[13px]" size="sm">
                          <SelectValue placeholder="No models available" />
                        </SelectTrigger>
                        <SelectContent>
                          {modelChoiceGroups.map((group, index) => (
                            <div key={group.id}>
                              {index > 0 ? <SelectSeparator /> : null}
                              <SelectGroup>
                                <SelectLabel>{group.label}</SelectLabel>
                                {group.items.map((choice) => (
                                  <SelectItem key={choice.modelId} value={choice.modelId}>
                                    {choice.label}
                                  </SelectItem>
                                ))}
                              </SelectGroup>
                            </div>
                          ))}
                        </SelectContent>
                      </Select>
                    ) : card.preset.manualModelAllowed ? (
                      <Input
                        id={`${card.key}-model`}
                        className="h-9 font-mono text-[13px]"
                        disabled={isSaving || isRefreshing}
                        onChange={(event) =>
                          setDraft(card, {
                            ...draft,
                            modelId: event.target.value,
                          })
                        }
                        placeholder={card.preset.defaultModelId || "provider/model-id"}
                        value={draft.modelId}
                      />
                    ) : (
                      <div className="rounded-md border border-border/80 bg-muted/25 px-3.5 py-3 text-[12px] text-muted-foreground">
                        No model configuration is required for this provider.
                      </div>
                    )}

                    <p
                      className={cn(
                        "text-[11px]",
                        cardCatalogState.tone === "warning" ? "text-amber-700 dark:text-amber-200" : "text-muted-foreground",
                      )}
                    >
                      <span className="font-medium">{cardCatalogState.stateLabel}.</span> {cardCatalogState.detail}
                    </p>
                  </div>

                  {card.preset.baseUrlMode !== "none" ||
                  card.preset.apiVersionMode !== "none" ||
                  card.preset.regionMode !== "none" ||
                  card.preset.projectIdMode !== "none" ? (
                    <div className="space-y-3 rounded-md border border-border/80 bg-muted/25 px-3.5 py-3">
                      <div>
                        <p className="text-[12px] font-medium text-foreground">Connection</p>
                        <p className="mt-1 text-[11px] text-muted-foreground">{card.preset.connectionHint}</p>
                      </div>

                      {card.preset.baseUrlMode !== "none" ? (
                        <div className="space-y-2">
                          <Label htmlFor={`${card.key}-base-url`} className="text-[12px]">
                            Base URL
                          </Label>
                          <Input
                            id={`${card.key}-base-url`}
                            className="h-9 font-mono text-[13px]"
                            disabled={isSaving || isRefreshing}
                            onChange={(event) =>
                              setDraft(card, {
                                ...draft,
                                baseUrl: event.target.value,
                              })
                            }
                            placeholder={
                              card.preset.providerId === "ollama"
                                ? "http://127.0.0.1:11434/v1"
                                : card.preset.baseUrlMode === "required"
                                  ? "https://example-resource.openai.azure.com/openai/deployments/work"
                                  : "https://api.openai.com/v1"
                            }
                            value={draft.baseUrl}
                          />
                        </div>
                      ) : null}

                      {card.preset.apiVersionMode !== "none" ? (
                        <div className="space-y-2">
                          <Label htmlFor={`${card.key}-api-version`} className="text-[12px]">
                            API version
                          </Label>
                          <Input
                            id={`${card.key}-api-version`}
                            className="h-9 font-mono text-[13px]"
                            disabled={isSaving || isRefreshing}
                            onChange={(event) =>
                              setDraft(card, {
                                ...draft,
                                apiVersion: event.target.value,
                              })
                            }
                            placeholder="2024-10-21"
                            value={draft.apiVersion}
                          />
                        </div>
                      ) : null}

                      {card.preset.regionMode !== "none" ? (
                        <div className="space-y-2">
                          <Label htmlFor={`${card.key}-region`} className="text-[12px]">
                            Region
                          </Label>
                          <Input
                            id={`${card.key}-region`}
                            className="h-9 font-mono text-[13px]"
                            disabled={isSaving || isRefreshing}
                            onChange={(event) =>
                              setDraft(card, {
                                ...draft,
                                region: event.target.value,
                              })
                            }
                            placeholder={card.preset.providerId === "vertex" ? "us-central1" : "us-east-1"}
                            value={draft.region}
                          />
                        </div>
                      ) : null}

                      {card.preset.projectIdMode !== "none" ? (
                        <div className="space-y-2">
                          <Label htmlFor={`${card.key}-project-id`} className="text-[12px]">
                            Project ID
                          </Label>
                          <Input
                            id={`${card.key}-project-id`}
                            className="h-9 font-mono text-[13px]"
                            disabled={isSaving || isRefreshing}
                            onChange={(event) =>
                              setDraft(card, {
                                ...draft,
                                projectId: event.target.value,
                              })
                            }
                            placeholder="vertex-project"
                            value={draft.projectId}
                          />
                        </div>
                      ) : null}
                    </div>
                  ) : null}

                  {isApiKeyProvider ? (
                    <div className="space-y-2">
                      <div className="flex items-center justify-between gap-3">
                        <Label htmlFor={`${card.key}-api-key`} className="text-[12px]">
                          API Key
                        </Label>
                        {hasSavedApiKey ? (
                          <Badge variant="secondary" className="gap-1.5 text-[11px]">
                            <Check className="h-3 w-3" strokeWidth={3} />
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
                          className="h-9 flex-1 font-mono text-[13px]"
                          disabled={isSaving || isRefreshing}
                          onChange={(event) =>
                            setDraft(card, {
                              ...draft,
                              apiKey: event.target.value,
                              clearApiKey:
                                event.target.value.trim().length > 0 ? false : draft.clearApiKey,
                            })
                          }
                          placeholder={
                            hasSavedApiKey
                              ? "Leave blank to keep current key"
                              : card.preset.providerId === "github_models"
                                ? "Paste GitHub token"
                                : "Paste API key"
                          }
                          value={draft.apiKey}
                        />
                        {hasSavedApiKey ? (
                          <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            className="h-9 px-2.5 text-[12px]"
                            disabled={isSaving || isRefreshing}
                            onClick={() =>
                              setDraft(card, {
                                ...draft,
                                apiKey: "",
                                clearApiKey: !draft.clearApiKey,
                              })
                            }
                          >
                            {draft.clearApiKey ? "Keep" : "Clear"}
                          </Button>
                        ) : null}
                      </div>
                      <p
                        className={cn(
                          "text-[11px]",
                          draft.clearApiKey
                            ? "text-destructive/80"
                            : "text-muted-foreground",
                        )}
                      >
                        {draft.clearApiKey
                          ? "Saved key will be removed"
                          : hasSavedApiKey
                            ? "Blank keeps the current key"
                            : `Required for ${card.preset.label}`}
                      </p>
                    </div>
                  ) : card.preset.authMode === "local" ? (
                    <div className="rounded-md border border-sky-500/20 bg-sky-500/5 px-3.5 py-3 text-[12px] text-sky-700 dark:text-sky-200">
                      Cadence treats {card.preset.label} as a local endpoint. No app-local API key is stored for this provider profile.
                    </div>
                  ) : card.preset.authMode === "ambient" ? (
                    <div className="rounded-md border border-cyan-500/20 bg-cyan-500/5 px-3.5 py-3 text-[12px] text-cyan-700 dark:text-cyan-200">
                      Cadence uses ambient desktop credentials for {card.preset.label}. No app-local API key is stored for this provider profile.
                    </div>
                  ) : null}

                  <div className="flex items-center gap-2.5">
                    <Button
                      size="sm"
                      className="h-8 gap-1.5 text-[12px]"
                      disabled={isSaving || isRefreshing || !onUpsertProviderProfile}
                      onClick={() => void handleSave(card)}
                    >
                      {isSaving ? <LoaderCircle className="h-3.5 w-3.5 animate-spin" /> : <Check className="h-3.5 w-3.5" />}
                      Save
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="h-8 text-[12px]"
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
      </div>
    </div>
  )
}
