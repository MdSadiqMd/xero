"use client"

import { lazy, Suspense, useEffect, useRef, useState } from "react"
import type {
  AgentPaneView,
  DoctorReportRunStatus,
  McpRegistryLoadStatus,
  McpRegistryMutationStatus,
  OperatorActionErrorView,
  ProviderCredentialsLoadStatus,
  ProviderCredentialsSaveStatus,
  SkillRegistryLoadStatus,
  SkillRegistryMutationStatus,
} from "@/src/features/xero/use-xero-desktop-state"
import type { DictationSettingsAdapter } from "@/components/xero/settings-dialog/dictation-section"
import type { SoulSettingsAdapter } from "@/components/xero/settings-dialog/soul-section"
import type {
  EnvironmentDiscoveryStatusDto,
  EnvironmentProbeReportDto,
  EnvironmentProfileSummaryDto,
  VerifyUserToolRequestDto,
  VerifyUserToolResponseDto,
  ImportMcpServersResponseDto,
  XeroDoctorReportDto,
  McpImportDiagnosticDto,
  McpRegistryDto,
  ProviderCredentialsSnapshotDto,
  ProviderAuthSessionView,
  RuntimeProviderIdDto,
  RunDoctorReportRequestDto,
  ListSkillRegistryRequestDto,
  RemovePluginRequestDto,
  RemovePluginRootRequestDto,
  RemoveSkillLocalRootRequestDto,
  RemoveSkillRequestDto,
  SetPluginEnabledRequestDto,
  SetSkillEnabledRequestDto,
  SkillRegistryDto,
  UpdateGithubSkillSourceRequestDto,
  UpdateProjectSkillSourceRequestDto,
  UpsertPluginRootRequestDto,
  UpsertSkillLocalRootRequestDto,
  UpsertMcpServerRequestDto,
  UpsertNotificationRouteRequestDto,
  UpsertProviderCredentialRequestDto,
} from "@/src/lib/xero-model"
import type { PlatformVariant } from "@/components/xero/shell"
import type {
  GitHubAuthError,
  GitHubAuthStatus,
  GitHubSessionView,
} from "@/src/lib/github-auth"
import { Activity, ArrowLeft, Bell, Bot, Code2, Globe, Heart, KeyRound, Mic, Palette, Plug, PlugZap, UserRound, WandSparkles } from "lucide-react"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog"
import { cn } from "@/lib/utils"

const LazyAccountSection = lazy(() =>
  import("@/components/xero/settings-dialog/account-section").then((module) => ({
    default: module.AccountSection,
  })),
)
const LazyAgentsSection = lazy(() =>
  import("@/components/xero/settings-dialog/agents-section").then((module) => ({
    default: module.AgentsSection,
  })),
)
const LazyBrowserSection = lazy(() =>
  import("@/components/xero/settings-dialog/browser-section").then((module) => ({
    default: module.BrowserSection,
  })),
)
const LazyDevelopmentSection = lazy(() =>
  import("@/components/xero/settings-dialog/development-section").then((module) => ({
    default: module.DevelopmentSection,
  })),
)
const LazyDictationSection = lazy(() =>
  import("@/components/xero/settings-dialog/dictation-section").then((module) => ({
    default: module.DictationSection,
  })),
)
const LazyDiagnosticsSection = lazy(() =>
  import("@/components/xero/settings-dialog/diagnostics-section").then((module) => ({
    default: module.DiagnosticsSection,
  })),
)
const LazyMcpSection = lazy(() =>
  import("@/components/xero/settings-dialog/mcp-section").then((module) => ({
    default: module.McpSection,
  })),
)
const LazyNotificationsSection = lazy(() =>
  import("@/components/xero/settings-dialog/notifications-section").then((module) => ({
    default: module.NotificationsSection,
  })),
)
const LazyProvidersSection = lazy(() =>
  import("@/components/xero/settings-dialog/providers-section").then((module) => ({
    default: module.ProvidersSection,
  })),
)
const LazyPluginsSection = lazy(() =>
  import("@/components/xero/settings-dialog/plugins-section").then((module) => ({
    default: module.PluginsSection,
  })),
)
const LazySkillsSection = lazy(() =>
  import("@/components/xero/settings-dialog/skills-section").then((module) => ({
    default: module.SkillsSection,
  })),
)
const LazySoulSection = lazy(() =>
  import("@/components/xero/settings-dialog/soul-section").then((module) => ({
    default: module.SoulSection,
  })),
)
const LazyThemesSection = lazy(() =>
  import("@/components/xero/settings-dialog/themes-section").then((module) => ({
    default: module.ThemesSection,
  })),
)

export type SettingsSection =
  | "account"
  | "providers"
  | "diagnostics"
  | "soul"
  | "dictation"
  | "notifications"
  | "mcp"
  | "skills"
  | "agents"
  | "plugins"
  | "browser"
  | "themes"
  | "development"

interface NavItem {
  id: SettingsSection
  label: string
  icon: React.ElementType
}

interface NavGroup {
  id: string
  label: string
  items: NavItem[]
}

const ACCOUNT_GROUP: NavGroup = {
  id: "account",
  label: "Account",
  items: [{ id: "account", label: "Account", icon: UserRound }],
}

const WORKSPACE_GROUP: NavGroup = {
  id: "workspace",
  label: "Workspace",
  items: [
    { id: "providers", label: "Providers", icon: KeyRound },
    { id: "diagnostics", label: "Diagnostics", icon: Activity },
    { id: "soul", label: "Soul", icon: Heart },
    { id: "dictation", label: "Dictation", icon: Mic },
    { id: "notifications", label: "Notifications", icon: Bell },
    { id: "mcp", label: "MCP", icon: PlugZap },
    { id: "agents", label: "Agents", icon: Bot },
    { id: "skills", label: "Skills", icon: WandSparkles },
    { id: "plugins", label: "Plugins", icon: Plug },
    { id: "browser", label: "Browser", icon: Globe },
  ],
}

const APPEARANCE_GROUP: NavGroup = {
  id: "appearance",
  label: "Appearance",
  items: [{ id: "themes", label: "Themes", icon: Palette }],
}

const DEVELOPER_GROUP: NavGroup = {
  id: "developer",
  label: "Developer",
  items: [{ id: "development", label: "Development", icon: Code2 }],
}

const NAV_GROUPS: NavGroup[] = import.meta.env.DEV
  ? [ACCOUNT_GROUP, WORKSPACE_GROUP, APPEARANCE_GROUP, DEVELOPER_GROUP]
  : [ACCOUNT_GROUP, WORKSPACE_GROUP, APPEARANCE_GROUP]

export interface SettingsDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  initialSection?: SettingsSection
  agent: AgentPaneView | null
  providerCredentials: ProviderCredentialsSnapshotDto | null
  providerCredentialsLoadStatus: ProviderCredentialsLoadStatus
  providerCredentialsLoadError: OperatorActionErrorView | null
  providerCredentialsSaveStatus: ProviderCredentialsSaveStatus
  providerCredentialsSaveError: OperatorActionErrorView | null
  onRefreshProviderCredentials?: (options?: {
    force?: boolean
  }) => Promise<ProviderCredentialsSnapshotDto>
  onUpsertProviderCredential?: (
    request: UpsertProviderCredentialRequestDto,
  ) => Promise<ProviderCredentialsSnapshotDto>
  onDeleteProviderCredential?: (
    providerId: RuntimeProviderIdDto,
  ) => Promise<ProviderCredentialsSnapshotDto>
  onStartOAuthLogin?: (request: {
    providerId: RuntimeProviderIdDto
    originator?: string | null
  }) => Promise<ProviderAuthSessionView | null>
  doctorReport?: XeroDoctorReportDto | null
  doctorReportStatus?: DoctorReportRunStatus
  doctorReportError?: OperatorActionErrorView | null
  environmentDiscoveryStatus?: EnvironmentDiscoveryStatusDto | null
  environmentProfileSummary?: EnvironmentProfileSummaryDto
  onRefreshEnvironmentDiscovery?: (options?: { force?: boolean }) => Promise<EnvironmentDiscoveryStatusDto | null>
  onVerifyUserEnvironmentTool?: (request: VerifyUserToolRequestDto) => Promise<VerifyUserToolResponseDto | null>
  onSaveUserEnvironmentTool?: (request: VerifyUserToolRequestDto) => Promise<EnvironmentProbeReportDto | null>
  onRemoveUserEnvironmentTool?: (id: string) => Promise<EnvironmentProbeReportDto | null>
  onRunDoctorReport?: (request?: Partial<RunDoctorReportRequestDto>) => Promise<XeroDoctorReportDto>
  dictationAdapter?: DictationSettingsAdapter
  soulAdapter?: SoulSettingsAdapter
  onUpsertNotificationRoute?: (req: Omit<UpsertNotificationRouteRequestDto, "projectId" | "updatedAt">) => Promise<unknown>
  mcpRegistry?: McpRegistryDto | null
  mcpImportDiagnostics?: McpImportDiagnosticDto[]
  mcpRegistryLoadStatus?: McpRegistryLoadStatus
  mcpRegistryLoadError?: OperatorActionErrorView | null
  mcpRegistryMutationStatus?: McpRegistryMutationStatus
  pendingMcpServerId?: string | null
  mcpRegistryMutationError?: OperatorActionErrorView | null
  onRefreshMcpRegistry?: (options?: { force?: boolean }) => Promise<McpRegistryDto>
  onUpsertMcpServer?: (request: UpsertMcpServerRequestDto) => Promise<McpRegistryDto>
  onRemoveMcpServer?: (serverId: string) => Promise<McpRegistryDto>
  onImportMcpServers?: (path: string) => Promise<ImportMcpServersResponseDto>
  onRefreshMcpServerStatuses?: (options?: { serverIds?: string[] }) => Promise<McpRegistryDto>
  skillRegistry?: SkillRegistryDto | null
  skillRegistryLoadStatus?: SkillRegistryLoadStatus
  skillRegistryLoadError?: OperatorActionErrorView | null
  skillRegistryMutationStatus?: SkillRegistryMutationStatus
  pendingSkillSourceId?: string | null
  skillRegistryMutationError?: OperatorActionErrorView | null
  onRefreshSkillRegistry?: (options?: Partial<ListSkillRegistryRequestDto> & { force?: boolean }) => Promise<SkillRegistryDto>
  onReloadSkillRegistry?: (options?: Partial<ListSkillRegistryRequestDto>) => Promise<SkillRegistryDto>
  onSetSkillEnabled?: (request: SetSkillEnabledRequestDto) => Promise<SkillRegistryDto>
  onRemoveSkill?: (request: RemoveSkillRequestDto) => Promise<SkillRegistryDto>
  onUpsertSkillLocalRoot?: (request: UpsertSkillLocalRootRequestDto) => Promise<SkillRegistryDto>
  onRemoveSkillLocalRoot?: (request: RemoveSkillLocalRootRequestDto) => Promise<SkillRegistryDto>
  onUpdateProjectSkillSource?: (request: UpdateProjectSkillSourceRequestDto) => Promise<SkillRegistryDto>
  onUpdateGithubSkillSource?: (request: UpdateGithubSkillSourceRequestDto) => Promise<SkillRegistryDto>
  onUpsertPluginRoot?: (request: UpsertPluginRootRequestDto) => Promise<SkillRegistryDto>
  onRemovePluginRoot?: (request: RemovePluginRootRequestDto) => Promise<SkillRegistryDto>
  onSetPluginEnabled?: (request: SetPluginEnabledRequestDto) => Promise<SkillRegistryDto>
  onRemovePlugin?: (request: RemovePluginRequestDto) => Promise<SkillRegistryDto>
  platformOverride?: PlatformVariant | null
  onPlatformOverrideChange?: (value: PlatformVariant | null) => void
  onStartOnboarding?: () => void
  githubSession?: GitHubSessionView | null
  githubAuthStatus?: GitHubAuthStatus
  githubAuthError?: GitHubAuthError | null
  onGithubLogin?: () => void
  onGithubLogout?: () => void
  onListAgentDefinitions?: (request: {
    projectId: string
    includeArchived: boolean
  }) => Promise<{ definitions: import("@/src/lib/xero-model/agent-definition").AgentDefinitionSummaryDto[] }>
  onArchiveAgentDefinition?: (request: {
    projectId: string
    definitionId: string
  }) => Promise<import("@/src/lib/xero-model/agent-definition").AgentDefinitionSummaryDto>
  onGetAgentDefinitionVersion?: (request: {
    projectId: string
    definitionId: string
    version: number
  }) => Promise<import("@/src/lib/xero-model/agent-definition").AgentDefinitionVersionSummaryDto | null>
  onAgentRegistryChanged?: () => void
}

export function SettingsDialog({
  open,
  onOpenChange,
  initialSection = "providers",
  agent,
  providerCredentials,
  providerCredentialsLoadStatus,
  providerCredentialsLoadError,
  providerCredentialsSaveStatus,
  providerCredentialsSaveError,
  onRefreshProviderCredentials,
  onUpsertProviderCredential,
  onDeleteProviderCredential,
  onStartOAuthLogin,
  doctorReport = null,
  doctorReportStatus = "idle",
  doctorReportError = null,
  environmentDiscoveryStatus = null,
  environmentProfileSummary = null,
  onRefreshEnvironmentDiscovery,
  onVerifyUserEnvironmentTool,
  onSaveUserEnvironmentTool,
  onRemoveUserEnvironmentTool,
  onRunDoctorReport,
  dictationAdapter,
  soulAdapter,
  onUpsertNotificationRoute,
  mcpRegistry = null,
  mcpImportDiagnostics = [],
  mcpRegistryLoadStatus = "idle",
  mcpRegistryLoadError = null,
  mcpRegistryMutationStatus = "idle",
  pendingMcpServerId = null,
  mcpRegistryMutationError = null,
  onRefreshMcpRegistry,
  onUpsertMcpServer,
  onRemoveMcpServer,
  onImportMcpServers,
  onRefreshMcpServerStatuses,
  skillRegistry = null,
  skillRegistryLoadStatus = "idle",
  skillRegistryLoadError = null,
  skillRegistryMutationStatus = "idle",
  pendingSkillSourceId = null,
  skillRegistryMutationError = null,
  onRefreshSkillRegistry,
  onReloadSkillRegistry,
  onSetSkillEnabled,
  onRemoveSkill,
  onUpsertSkillLocalRoot,
  onRemoveSkillLocalRoot,
  onUpdateProjectSkillSource,
  onUpdateGithubSkillSource,
  onUpsertPluginRoot,
  onRemovePluginRoot,
  onSetPluginEnabled,
  onRemovePlugin,
  platformOverride,
  onPlatformOverrideChange,
  onStartOnboarding,
  githubSession = null,
  githubAuthStatus = "idle",
  githubAuthError = null,
  onGithubLogin,
  onGithubLogout,
  onListAgentDefinitions,
  onArchiveAgentDefinition,
  onGetAgentDefinitionVersion,
  onAgentRegistryChanged,
}: SettingsDialogProps) {
  const [section, setSection] = useState<SettingsSection>("providers")
  const refreshOnOpenCallbacksRef = useRef({
    providerCredentials: onRefreshProviderCredentials,
    environment: onRefreshEnvironmentDiscovery,
    mcpRegistry: onRefreshMcpRegistry,
    skillRegistry: onRefreshSkillRegistry,
  })

  useEffect(() => {
    if (open) setSection(initialSection)
  }, [initialSection, open])

  useEffect(() => {
    refreshOnOpenCallbacksRef.current = {
      providerCredentials: onRefreshProviderCredentials,
      environment: onRefreshEnvironmentDiscovery,
      mcpRegistry: onRefreshMcpRegistry,
      skillRegistry: onRefreshSkillRegistry,
    }
  }, [onRefreshEnvironmentDiscovery, onRefreshMcpRegistry, onRefreshProviderCredentials, onRefreshSkillRegistry])

  useEffect(() => {
    if (!open) {
      return
    }

    const { providerCredentials, environment, mcpRegistry, skillRegistry } = refreshOnOpenCallbacksRef.current

    void providerCredentials?.({ force: true }).catch(() => undefined)
    void environment?.().catch(() => undefined)
    void mcpRegistry?.({ force: true }).catch(() => undefined)
    void skillRegistry?.({ force: true }).catch(() => undefined)
  }, [open])

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="left-0 top-0 flex h-screen w-screen max-w-none translate-x-0 translate-y-0 flex-col gap-0 overflow-hidden rounded-none border-0 p-0 shadow-none sm:max-w-none"
        showCloseButton={false}
      >
        <DialogTitle className="sr-only">Settings</DialogTitle>
        <DialogDescription className="sr-only">
          Configure providers, skills, notification routes, and development options.
        </DialogDescription>

        <div className="flex min-h-0 flex-1">
          <nav className="flex w-64 shrink-0 flex-col gap-4 border-r border-border/70 bg-sidebar py-4">
            <div className="px-2.5">
              <button
                type="button"
                onClick={() => onOpenChange(false)}
                className="flex w-full items-center gap-2 rounded-md px-2.5 py-2 text-left text-[13px] font-medium text-muted-foreground transition-colors hover:bg-accent/40 hover:text-foreground"
              >
                <ArrowLeft className="h-4 w-4" />
                Back to app
              </button>
            </div>

            <div className="flex flex-col gap-3.5">
              {NAV_GROUPS.map((group) => (
                <div key={group.id} className="flex flex-col">
                  <span className="px-4 pb-1.5 text-[11px] font-semibold uppercase tracking-[0.14em] text-muted-foreground/70">
                    {group.label}
                  </span>
                  <div className="flex flex-col px-2.5">
                    {group.items.map(({ id, label, icon: Icon }) => {
                      const active = section === id
                      return (
                        <button
                          key={id}
                          type="button"
                          aria-label={label}
                          aria-current={active ? "page" : undefined}
                          onClick={() => setSection(id)}
                          className={cn(
                            "group flex items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-[13.5px] leading-tight transition-colors",
                            active
                              ? "bg-accent/60 text-foreground"
                              : "text-muted-foreground hover:bg-accent/30 hover:text-foreground",
                          )}
                        >
                          <Icon
                            className={cn(
                              "h-4 w-4 shrink-0",
                              active ? "text-foreground" : "text-muted-foreground/80",
                            )}
                          />
                          <span className="truncate font-medium">{label}</span>
                        </button>
                      )
                    })}
                  </div>
                </div>
              ))}
            </div>
          </nav>

          <div className="flex flex-1 flex-col overflow-y-auto scrollbar-thin">
            <div
              key={section}
              className="mx-auto flex w-full max-w-3xl flex-1 flex-col gap-5 px-10 py-10 animate-in fade-in-0 motion-enter"
            >
              <Suspense fallback={<SettingsSectionFallback />}>
              {section === "account" ? (
                <LazyAccountSection
                  session={githubSession ?? null}
                  status={githubAuthStatus ?? "idle"}
                  error={githubAuthError ?? null}
                  onLogin={() => onGithubLogin?.()}
                  onLogout={() => onGithubLogout?.()}
                />
              ) : section === "providers" ? (
                <LazyProvidersSection
                  active={open && section === "providers"}
                  agent={agent}
                  providerCredentials={providerCredentials}
                  providerCredentialsLoadStatus={providerCredentialsLoadStatus}
                  providerCredentialsLoadError={providerCredentialsLoadError}
                  providerCredentialsSaveStatus={providerCredentialsSaveStatus}
                  providerCredentialsSaveError={providerCredentialsSaveError}
                  onRefreshProviderCredentials={onRefreshProviderCredentials}
                  onUpsertProviderCredential={onUpsertProviderCredential}
                  onDeleteProviderCredential={onDeleteProviderCredential}
                  onStartOAuthLogin={onStartOAuthLogin}
                />
              ) : section === "diagnostics" ? (
                <LazyDiagnosticsSection
                  doctorReport={doctorReport}
                  doctorReportStatus={doctorReportStatus}
                  doctorReportError={doctorReportError}
                  environmentDiscoveryStatus={environmentDiscoveryStatus}
                  environmentProfileSummary={environmentProfileSummary}
                  onRefreshEnvironmentDiscovery={onRefreshEnvironmentDiscovery}
                  onVerifyUserEnvironmentTool={onVerifyUserEnvironmentTool}
                  onSaveUserEnvironmentTool={onSaveUserEnvironmentTool}
                  onRemoveUserEnvironmentTool={onRemoveUserEnvironmentTool}
                  onRunDoctorReport={onRunDoctorReport}
                />
              ) : section === "soul" ? (
                <LazySoulSection adapter={soulAdapter} />
              ) : section === "dictation" ? (
                <LazyDictationSection adapter={dictationAdapter} />
              ) : section === "notifications" ? (
                agent ? (
                  <LazyNotificationsSection
                    agent={agent}
                    onUpsertNotificationRoute={onUpsertNotificationRoute}
                  />
                ) : (
                  <ProjectBoundEmptyState
                    title="Notifications require a selected project"
                    body="Provider settings are app-global, but notification routes stay project-bound so Xero never writes cross-project delivery state into the wrong repository view."
                  />
                )
              ) : section === "mcp" ? (
                <LazyMcpSection
                  mcpRegistry={mcpRegistry}
                  mcpImportDiagnostics={mcpImportDiagnostics}
                  mcpRegistryLoadStatus={mcpRegistryLoadStatus}
                  mcpRegistryLoadError={mcpRegistryLoadError}
                  mcpRegistryMutationStatus={mcpRegistryMutationStatus}
                  pendingMcpServerId={pendingMcpServerId}
                  mcpRegistryMutationError={mcpRegistryMutationError}
                  onRefreshMcpRegistry={onRefreshMcpRegistry}
                  onUpsertMcpServer={onUpsertMcpServer}
                  onRemoveMcpServer={onRemoveMcpServer}
                  onImportMcpServers={onImportMcpServers}
                  onRefreshMcpServerStatuses={onRefreshMcpServerStatuses}
                />
              ) : section === "agents" ? (
                <LazyAgentsSection
                  projectId={agent?.project.id ?? null}
                  projectLabel={agent?.project.repository?.displayName ?? agent?.project.name ?? null}
                  onListAgentDefinitions={onListAgentDefinitions}
                  onArchiveAgentDefinition={onArchiveAgentDefinition}
                  onGetAgentDefinitionVersion={onGetAgentDefinitionVersion}
                  onRegistryChanged={onAgentRegistryChanged}
                />
              ) : section === "skills" ? (
                <LazySkillsSection
                  agent={agent}
                  skillRegistry={skillRegistry}
                  skillRegistryLoadStatus={skillRegistryLoadStatus}
                  skillRegistryLoadError={skillRegistryLoadError}
                  skillRegistryMutationStatus={skillRegistryMutationStatus}
                  pendingSkillSourceId={pendingSkillSourceId}
                  skillRegistryMutationError={skillRegistryMutationError}
                  onRefreshSkillRegistry={onRefreshSkillRegistry}
                  onReloadSkillRegistry={onReloadSkillRegistry}
                  onSetSkillEnabled={onSetSkillEnabled}
                  onRemoveSkill={onRemoveSkill}
                  onUpsertSkillLocalRoot={onUpsertSkillLocalRoot}
                  onRemoveSkillLocalRoot={onRemoveSkillLocalRoot}
                  onUpdateProjectSkillSource={onUpdateProjectSkillSource}
                  onUpdateGithubSkillSource={onUpdateGithubSkillSource}
                />
              ) : section === "plugins" ? (
                <LazyPluginsSection
                  agent={agent}
                  skillRegistry={skillRegistry}
                  skillRegistryLoadStatus={skillRegistryLoadStatus}
                  skillRegistryLoadError={skillRegistryLoadError}
                  skillRegistryMutationStatus={skillRegistryMutationStatus}
                  pendingSkillSourceId={pendingSkillSourceId}
                  skillRegistryMutationError={skillRegistryMutationError}
                  onRefreshSkillRegistry={onRefreshSkillRegistry}
                  onReloadSkillRegistry={onReloadSkillRegistry}
                  onUpsertPluginRoot={onUpsertPluginRoot}
                  onRemovePluginRoot={onRemovePluginRoot}
                  onSetPluginEnabled={onSetPluginEnabled}
                  onRemovePlugin={onRemovePlugin}
                />
              ) : section === "browser" ? (
                <LazyBrowserSection />
              ) : section === "themes" ? (
                <LazyThemesSection />
              ) : section === "development" ? (
                <LazyDevelopmentSection
                  platformOverride={platformOverride}
                  onPlatformOverrideChange={onPlatformOverrideChange}
                  onStartOnboarding={onStartOnboarding}
                />
              ) : null}
              </Suspense>
            </div>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  )
}

function SettingsSectionFallback() {
  return (
    <div
      aria-busy="true"
      aria-label="Loading settings section"
      className="min-h-[240px]"
      role="status"
    />
  )
}

function ProjectBoundEmptyState({ title, body }: { title: string; body: string }) {
  return (
    <div className="flex flex-1 items-center justify-center py-14 text-center">
      <div className="max-w-md">
        <Bell className="mx-auto h-4 w-4 text-muted-foreground/70" />
        <p className="mt-3 text-[13px] font-medium text-foreground">{title}</p>
        <p className="mt-1.5 text-[12px] leading-[1.55] text-muted-foreground">{body}</p>
      </div>
    </div>
  )
}
