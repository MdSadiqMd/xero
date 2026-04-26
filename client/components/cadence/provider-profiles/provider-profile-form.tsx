import { useEffect, useState, type ElementType } from "react"
import { openUrl } from "@tauri-apps/plugin-opener"
import {
  Activity,
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
  type CadenceDiagnosticCheckDto,
  type ProviderModelCatalogDto,
  type ProviderModelDto,
  type ProviderProfileDiagnosticsDto,
  type ProviderProfilesDto,
  type ProviderProfileDto,
  type RuntimeSessionView,
  type UpsertProviderProfileRequestDto,
  upsertProviderProfileRequestSchema,
} from "@/src/lib/cadence-model"
import {
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

type ProviderProfileDiagnosticStatus = "idle" | "loading" | "ready" | "error"

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

function getProviderDiagnosticChecks(
  report: ProviderProfileDiagnosticsDto,
): CadenceDiagnosticCheckDto[] {
  return [...report.validationChecks, ...report.reachabilityChecks]
}

function getActionableProviderDiagnosticChecks(
  report: ProviderProfileDiagnosticsDto,
): CadenceDiagnosticCheckDto[] {
  const checks = getProviderDiagnosticChecks(report)
  const actionable = checks.filter((check) => check.status === "failed" || check.status === "warning")
  if (actionable.length > 0) return actionable
  return checks.filter((check) => check.status === "skipped" && check.code !== "provider_profile_not_active")
}

function getProviderDiagnosticSummary(report: ProviderProfileDiagnosticsDto): string {
  const checks = getProviderDiagnosticChecks(report)
  const failed = checks.filter((check) => check.status === "failed").length
  const warnings = checks.filter((check) => check.status === "warning").length

  if (failed > 0) {
    return `Connection check found ${failed} issue${failed === 1 ? "" : "s"}.`
  }

  if (warnings > 0) {
    return `Connection check found ${warnings} warning${warnings === 1 ? "" : "s"}.`
  }

  return "Connection check passed."
}

function getDiagnosticRowClassName(check: CadenceDiagnosticCheckDto): string {
  if (check.status === "failed") {
    return "border-destructive/30 bg-destructive/5 text-destructive"
  }

  if (check.status === "warning") {
    return "border-amber-500/30 bg-amber-500/5 text-amber-700 dark:text-amber-200"
  }

  if (check.status === "skipped") {
    return "border-border bg-muted/30 text-muted-foreground"
  }

  return "border-emerald-500/30 bg-emerald-500/5 text-emerald-700 dark:text-emerald-200"
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
  onCheckProviderProfile?: (
    profileId: string,
    options?: { includeNetwork?: boolean },
  ) => Promise<ProviderProfileDiagnosticsDto>
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
  onCheckProviderProfile,
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
  const [profileDiagnostics, setProfileDiagnostics] = useState<Record<string, ProviderProfileDiagnosticsDto>>({})
  const [profileDiagnosticStatuses, setProfileDiagnosticStatuses] =
    useState<Record<string, ProviderProfileDiagnosticStatus>>({})
  const [profileDiagnosticErrors, setProfileDiagnosticErrors] = useState<Record<string, string | null>>({})

  const cards = getProfileCards(providerProfiles)
  const isRefreshing = providerProfilesLoadStatus === "loading"
  const isSaving = providerProfilesSaveStatus === "running"
  const isMutationDisabled = isSaving || !onUpsertProviderProfile
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

  async function handleCheckConnection(card: ProviderProfileCard) {
    const profileId = card.profile?.profileId
    if (!profileId || !onCheckProviderProfile) {
      return
    }

    setFormError(null)
    setAuthError(null)
    setProfileDiagnosticStatuses((currentStatuses) => ({
      ...currentStatuses,
      [profileId]: "loading",
    }))
    setProfileDiagnosticErrors((currentErrors) => ({
      ...currentErrors,
      [profileId]: null,
    }))

    try {
      const report = await onCheckProviderProfile(profileId, { includeNetwork: true })
      setProfileDiagnostics((currentReports) => ({
        ...currentReports,
        [profileId]: report,
      }))
      setProfileDiagnosticStatuses((currentStatuses) => ({
        ...currentStatuses,
        [profileId]: "ready",
      }))
    } catch (error) {
      setProfileDiagnosticStatuses((currentStatuses) => ({
        ...currentStatuses,
        [profileId]: "error",
      }))
      setProfileDiagnosticErrors((currentErrors) => ({
        ...currentErrors,
        [profileId]: errMsg(error, `Could not check ${card.preset.label}.`),
      }))
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
          const profileDiagnosticReport = card.profile ? profileDiagnostics[card.profile.profileId] ?? null : null
          const profileDiagnosticStatus: ProviderProfileDiagnosticStatus = card.profile
            ? profileDiagnosticStatuses[card.profile.profileId] ?? "idle"
            : "idle"
          const profileDiagnosticError = card.profile
            ? profileDiagnosticErrors[card.profile.profileId] ?? null
            : null
          const actionableDiagnosticChecks = profileDiagnosticReport
            ? getActionableProviderDiagnosticChecks(profileDiagnosticReport)
            : []
          const isCheckingConnection = profileDiagnosticStatus === "loading"
          const canCheckConnection = Boolean(onCheckProviderProfile && card.profile)

          const statusBadge = isOpenAi && isOpenAiConnected
            ? { label: "Connected", className: "border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-300" }
            : readinessBadge

          return (
            <div
              key={card.key}
              className={cn(
                "rounded-lg border bg-card px-3.5 py-3 transition-colors",
                isSelected ? "border-primary/40 bg-primary/[0.03]" : "border-border/70",
              )}
            >
              <div className="flex items-center gap-3">
                <div
                  className={cn(
                    "flex h-7 w-7 shrink-0 items-center justify-center rounded-md border",
                    isSelected ? "border-primary/40 bg-primary/[0.08] text-primary" : "border-border/70 bg-secondary/40 text-foreground/70",
                  )}
                >
                  <Icon className="h-3.5 w-3.5" />
                </div>

                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <p className="truncate text-[13px] font-medium text-foreground">
                      {card.profile?.label ?? card.preset.label}
                    </p>
                    {isSelected ? (
                      <span className="rounded-sm border border-primary/30 bg-primary/[0.08] px-1.5 py-px text-[10px] font-medium uppercase tracking-[0.06em] text-primary">
                        Active
                      </span>
                    ) : null}
                  </div>
                  {card.profile?.modelId ? (
                    <p className="mt-0.5 truncate font-mono text-[11px] text-muted-foreground/80">
                      {card.profile.modelId}
                    </p>
                  ) : null}
                </div>

                <div className="flex shrink-0 flex-wrap items-center justify-end gap-1.5">
                  {statusBadge ? (
                    <span
                      className={cn(
                        "rounded-sm border px-1.5 py-px text-[10px] font-medium",
                        statusBadge.className,
                      )}
                    >
                      {statusBadge.label}
                    </span>
                  ) : null}

                  {canCheckConnection ? (
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      className="h-7 gap-1.5 px-2.5 text-[11.5px]"
                      disabled={isSaving || isCheckingConnection}
                      onClick={() => void handleCheckConnection(card)}
                    >
                      {isCheckingConnection ? (
                        <LoaderCircle className="h-3 w-3 animate-spin" />
                      ) : (
                        <Activity className="h-3 w-3" />
                      )}
                      Check connection
                    </Button>
                  ) : null}

                  {isSelected ? null : (
                    <Button
                      size="sm"
                      variant="outline"
                      className="h-7 px-2.5 text-[11.5px]"
                      disabled={isMutationDisabled}
                      onClick={() => void handleActivate(card)}
                    >
                      Use this
                    </Button>
                  )}

                  {isEditing ? null : isOpenAi ? (
                    <Button
                      size="sm"
                      variant="ghost"
                      className="h-7 px-2.5 text-[11.5px] text-muted-foreground hover:text-foreground"
                      disabled={isSaving}
                      onClick={() => openEditor(card)}
                    >
                      Rename
                    </Button>
                  ) : (
                    <Button
                      size="sm"
                      variant={hasSavedApiKey ? "ghost" : "default"}
                      className={cn(
                        "h-7 px-2.5 text-[11.5px]",
                        hasSavedApiKey ? "text-muted-foreground hover:text-foreground" : "",
                      )}
                      disabled={isSaving}
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
                        className="h-7 gap-1.5 px-2.5 text-[11.5px]"
                        disabled={pendingAuth !== null || isSaving}
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
                      <span className="inline-flex items-center gap-1.5 rounded-sm border border-border bg-secondary/60 px-1.5 py-px text-[10.5px] text-muted-foreground">
                        <LoaderCircle className="h-3 w-3 animate-spin" />
                        Connecting
                      </span>
                    ) : !hasSelectedProject ? (
                      <span className="rounded-sm border border-border bg-secondary/60 px-1.5 py-px text-[10.5px] text-muted-foreground">
                        {openAiMissingProjectLabel}
                      </span>
                    ) : (
                      <Button
                        size="sm"
                        className="h-7 gap-1.5 px-2.5 text-[11.5px]"
                        disabled={pendingAuth !== null || isSaving}
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

              {inlineStatus ? (
                <Alert
                  variant={inlineStatus.tone === "error" ? "destructive" : "default"}
                  className={cn(
                    "mt-2.5 py-2.5",
                    inlineStatus.tone === "warning"
                      ? "border-amber-500/30 bg-amber-500/5 text-amber-700 dark:text-amber-200"
                      : "border-destructive/30 bg-destructive/5",
                  )}
                >
                  <AlertCircle className="h-3.5 w-3.5" />
                  <AlertDescription className="text-[12px] leading-relaxed">
                    <span>{inlineStatus.message}</span>
                    {inlineStatus.recovery ? <span className="mt-1 block">{inlineStatus.recovery}</span> : null}
                  </AlertDescription>
                </Alert>
              ) : null}

              {profileDiagnosticError ? (
                <Alert variant="destructive" className="mt-2.5 border-destructive/30 bg-destructive/5 py-2.5">
                  <AlertCircle className="h-3.5 w-3.5" />
                  <AlertDescription className="text-[12px] leading-relaxed">
                    {profileDiagnosticError}
                  </AlertDescription>
                </Alert>
              ) : profileDiagnosticReport ? (
                <div className="mt-2.5 rounded-md border border-border/80 bg-muted/20 px-3 py-2.5">
                  <div className="flex items-center gap-2">
                    <Activity className="h-3.5 w-3.5 text-muted-foreground" />
                    <p className="text-[12px] font-medium text-foreground">
                      {getProviderDiagnosticSummary(profileDiagnosticReport)}
                    </p>
                  </div>
                  {actionableDiagnosticChecks.length > 0 ? (
                    <div className="mt-2 grid gap-1.5">
                      {actionableDiagnosticChecks.map((check) => (
                        <div
                          key={check.checkId}
                          className={cn(
                            "rounded-md border px-2.5 py-2 text-[11.5px] leading-relaxed",
                            getDiagnosticRowClassName(check),
                          )}
                        >
                          <p className="font-medium">{check.message}</p>
                          {check.remediation ? (
                            <p className="mt-1 opacity-85">{check.remediation}</p>
                          ) : null}
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p className="mt-1.5 text-[11.5px] text-muted-foreground">
                      Validation and provider reachability checks completed without repair steps.
                    </p>
                  )}
                </div>
              ) : null}

              {isEditing ? (
                <div className="mt-3.5 grid gap-3.5 rounded-md border border-dashed border-border/80 bg-background/80 p-3.5">
                  <div className="space-y-2">
                    <Label htmlFor={`${card.key}-label`} className="text-[12px]">
                      Profile label
                    </Label>
                    <Input
                      id={`${card.key}-label`}
                      className="h-9 text-[13px]"
                      disabled={isSaving}
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
                          disabled={isSaving || isCatalogRefreshing}
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
                        disabled={isSaving}
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
                        disabled={isSaving}
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

                    {cardCatalogState.tone === "warning" && cardCatalogState.refreshError?.message ? (
                      <p className="text-[11px] text-amber-700 dark:text-amber-200">
                        {cardCatalogState.refreshError.message}
                      </p>
                    ) : null}
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
                            disabled={isSaving}
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
                            disabled={isSaving}
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
                            disabled={isSaving}
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
                            disabled={isSaving}
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
                          disabled={isSaving}
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
                            disabled={isSaving}
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
                      disabled={isMutationDisabled}
                      onClick={() => void handleSave(card)}
                    >
                      {isSaving ? <LoaderCircle className="h-3.5 w-3.5 animate-spin" /> : <Check className="h-3.5 w-3.5" />}
                      Save
                    </Button>
                    <Button
                      size="sm"
                      variant="ghost"
                      className="h-8 text-[12px]"
                      disabled={isSaving}
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
