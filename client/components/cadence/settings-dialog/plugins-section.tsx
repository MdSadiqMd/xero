import { useMemo, useState } from 'react'
import {
  AlertCircle,
  FolderPlus,
  LoaderCircle,
  Plug,
  RefreshCcw,
  Search,
  ShieldAlert,
  Trash2,
} from 'lucide-react'
import type {
  AgentPaneView,
  OperatorActionErrorView,
  SkillRegistryLoadStatus,
  SkillRegistryMutationStatus,
} from '@/src/features/cadence/use-cadence-desktop-state'
import {
  getPluginCommandAvailabilityLabel,
  getSkillSourceStateLabel,
  getSkillTrustStateLabel,
  type PluginCommandContributionDto,
  type PluginRegistryEntryDto,
  type RemovePluginRequestDto,
  type RemovePluginRootRequestDto,
  type SetPluginEnabledRequestDto,
  type SkillRegistryDto,
  type UpsertPluginRootRequestDto,
} from '@/src/lib/cadence-model'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from '@/components/ui/alert-dialog'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'
import { cn } from '@/lib/utils'
import { SectionHeader } from './section-header'

interface PluginsSectionProps {
  agent: AgentPaneView | null
  skillRegistry: SkillRegistryDto | null
  skillRegistryLoadStatus: SkillRegistryLoadStatus
  skillRegistryLoadError: OperatorActionErrorView | null
  skillRegistryMutationStatus: SkillRegistryMutationStatus
  pendingSkillSourceId: string | null
  skillRegistryMutationError: OperatorActionErrorView | null
  onRefreshSkillRegistry?: (options?: { force?: boolean }) => Promise<SkillRegistryDto>
  onReloadSkillRegistry?: (options?: { projectId?: string | null; includeUnavailable?: boolean }) => Promise<SkillRegistryDto>
  onUpsertPluginRoot?: (request: UpsertPluginRootRequestDto) => Promise<SkillRegistryDto>
  onRemovePluginRoot?: (request: RemovePluginRootRequestDto) => Promise<SkillRegistryDto>
  onSetPluginEnabled?: (request: SetPluginEnabledRequestDto) => Promise<SkillRegistryDto>
  onRemovePlugin?: (request: RemovePluginRequestDto) => Promise<SkillRegistryDto>
}

type PluginRootForm = {
  rootId: string
  path: string
  enabled: boolean
}

type PluginRootErrors = Partial<Record<keyof PluginRootForm, string>>

function defaultPluginRootForm(): PluginRootForm {
  return {
    rootId: '',
    path: '',
    enabled: true,
  }
}

function formatTimestamp(value: string | null | undefined): string {
  if (!value) {
    return 'Never'
  }
  const parsed = Date.parse(value)
  if (!Number.isFinite(parsed)) {
    return value
  }
  return new Date(parsed).toLocaleString()
}

function formatHash(value: string | null | undefined): string {
  if (!value) {
    return 'None'
  }
  return value.length > 12 ? value.slice(0, 12) : value
}

function isAbsolutePath(path: string): boolean {
  return path.startsWith('/') || /^[A-Za-z]:[\\/]/.test(path)
}

function validatePluginRootForm(form: PluginRootForm): PluginRootErrors {
  const errors: PluginRootErrors = {}
  const path = form.path.trim()
  const rootId = form.rootId.trim()

  if (!path) {
    errors.path = 'Path is required.'
  } else if (!isAbsolutePath(path)) {
    errors.path = 'Use an absolute directory path.'
  }

  if (rootId && !/^[a-z0-9-]+$/.test(rootId)) {
    errors.rootId = 'Root id must be lowercase kebab-case.'
  }

  return errors
}

function hasErrors(errors: PluginRootErrors): boolean {
  return Object.values(errors).some(Boolean)
}

function stateTone(state: PluginRegistryEntryDto['state']): string {
  switch (state) {
    case 'enabled':
      return 'bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-100'
    case 'discoverable':
    case 'installed':
      return 'bg-sky-100 text-sky-800 dark:bg-sky-900/40 dark:text-sky-100'
    case 'disabled':
    case 'stale':
      return 'bg-amber-100 text-amber-800 dark:bg-amber-900/40 dark:text-amber-100'
    case 'failed':
    case 'blocked':
      return 'bg-destructive/15 text-destructive'
  }
}

function trustTone(trust: PluginRegistryEntryDto['trust']): string {
  switch (trust) {
    case 'trusted':
    case 'user_approved':
      return 'bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-100'
    case 'approval_required':
    case 'untrusted':
      return 'bg-amber-100 text-amber-800 dark:bg-amber-900/40 dark:text-amber-100'
    case 'blocked':
      return 'bg-destructive/15 text-destructive'
  }
}

function pluginPendingId(pluginId: string): string {
  return `plugin:${pluginId}`
}

function pluginMatchesQuery(plugin: PluginRegistryEntryDto, query: string): boolean {
  const haystack = [
    plugin.name,
    plugin.pluginId,
    plugin.description,
    plugin.version,
    plugin.rootId,
    plugin.rootPath,
    plugin.pluginRootPath,
    ...plugin.skills.map((skill) => `${skill.contributionId} ${skill.skillId} ${skill.path}`),
    ...plugin.commands.map((command) => `${command.contributionId} ${command.label} ${command.entry}`),
  ]
    .join(' ')
    .toLowerCase()

  return haystack.includes(query)
}

export function PluginsSection({
  agent,
  skillRegistry,
  skillRegistryLoadStatus,
  skillRegistryLoadError,
  skillRegistryMutationStatus,
  pendingSkillSourceId,
  skillRegistryMutationError,
  onRefreshSkillRegistry,
  onReloadSkillRegistry,
  onUpsertPluginRoot,
  onRemovePluginRoot,
  onSetPluginEnabled,
  onRemovePlugin,
}: PluginsSectionProps) {
  const projectId = agent?.project.id ?? skillRegistry?.projectId ?? null
  const [query, setQuery] = useState('')
  const [rootForm, setRootForm] = useState<PluginRootForm>(() => defaultPluginRootForm())
  const [rootErrors, setRootErrors] = useState<PluginRootErrors>({})
  const loading = skillRegistryLoadStatus === 'loading'
  const mutating = skillRegistryMutationStatus === 'running'

  const filteredPlugins = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase()
    const plugins = skillRegistry?.plugins ?? []
    if (!normalizedQuery) {
      return plugins
    }
    return plugins.filter((plugin) => pluginMatchesQuery(plugin, normalizedQuery))
  }, [query, skillRegistry?.plugins])

  const handleReload = () => {
    if (onReloadSkillRegistry) {
      void onReloadSkillRegistry({ projectId, includeUnavailable: true }).catch(() => undefined)
      return
    }
    void onRefreshSkillRegistry?.({ force: true }).catch(() => undefined)
  }

  const handleAddRoot = async () => {
    const errors = validatePluginRootForm(rootForm)
    setRootErrors(errors)
    if (hasErrors(errors)) {
      return
    }

    try {
      await onUpsertPluginRoot?.({
        rootId: rootForm.rootId.trim() || null,
        path: rootForm.path.trim(),
        enabled: rootForm.enabled,
        projectId,
      })
      setRootForm(defaultPluginRootForm())
      setRootErrors({})
    } catch {
      // The shared mutation error surface renders the backend diagnostic.
    }
  }

  return (
    <div className="flex flex-col gap-5">
      <SectionHeader
        title="Plugins"
        description="Manage plugin sources that contribute skills and commands into the existing Cadence runtime."
        actions={
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={loading || (!onReloadSkillRegistry && !onRefreshSkillRegistry)}
            onClick={handleReload}
          >
            {loading ? <LoaderCircle className="h-3.5 w-3.5 animate-spin" /> : <RefreshCcw className="h-3.5 w-3.5" />}
            Reload
          </Button>
        }
      />

      {skillRegistryLoadError ? (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Plugin registry unavailable</AlertTitle>
          <AlertDescription>{skillRegistryLoadError.message}</AlertDescription>
        </Alert>
      ) : null}

      {skillRegistryMutationError ? (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Plugin update failed</AlertTitle>
          <AlertDescription>{skillRegistryMutationError.message}</AlertDescription>
        </Alert>
      ) : null}

      <section className="flex flex-col gap-3">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
          <div className="relative flex-1">
            <Search className="pointer-events-none absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
            <Input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              className="h-8 pl-8 text-[12px]"
              placeholder="Search plugins"
              aria-label="Search plugins"
            />
          </div>
          <div className="flex items-center gap-2 text-[11.5px] text-muted-foreground">
            <Badge variant="secondary" className="text-[10px]">
              {skillRegistry?.plugins.length ?? 0} plugins
            </Badge>
            <Badge variant="secondary" className="text-[10px]">
              {skillRegistry?.pluginCommands.length ?? 0} commands
            </Badge>
          </div>
        </div>

        <div className="rounded-md border border-border/70">
          {loading && !skillRegistry ? (
            <div className="flex items-center justify-center gap-2 px-4 py-12 text-[12px] text-muted-foreground">
              <LoaderCircle className="h-3.5 w-3.5 animate-spin" />
              Loading plugins
            </div>
          ) : filteredPlugins.length === 0 ? (
            <div className="px-4 py-12 text-center">
              <Plug className="mx-auto h-4 w-4 text-muted-foreground/70" />
              <p className="mt-3 text-[13px] font-medium text-foreground">No plugins found</p>
              <p className="mt-1 text-[12px] text-muted-foreground">
                {query ? 'Adjust the search query.' : 'Add a plugin root or reload configured roots.'}
              </p>
            </div>
          ) : (
            <div className="divide-y divide-border/70">
              {filteredPlugins.map((plugin) => (
                <PluginRow
                  key={plugin.pluginId}
                  plugin={plugin}
                  projectId={projectId}
                  disabled={mutating}
                  pending={pendingSkillSourceId === pluginPendingId(plugin.pluginId)}
                  onSetPluginEnabled={onSetPluginEnabled}
                  onRemovePlugin={onRemovePlugin}
                />
              ))}
            </div>
          )}
        </div>
      </section>

      <section className="rounded-md border border-border/70 p-3">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h4 className="text-[12px] font-semibold text-foreground">Plugin roots</h4>
            <p className="mt-0.5 text-[11.5px] text-muted-foreground">
              {skillRegistry?.sources.pluginRoots.length ?? 0} configured
            </p>
          </div>
        </div>

        <div className="mt-3 grid gap-2 sm:grid-cols-[0.8fr_1.4fr_auto_auto]">
          <div>
            <Label htmlFor="plugin-root-id" className="sr-only">
              Plugin root id
            </Label>
            <Input
              id="plugin-root-id"
              value={rootForm.rootId}
              onChange={(event) => setRootForm((current) => ({ ...current, rootId: event.target.value }))}
              className="h-8 text-[12px]"
              placeholder="root id"
              aria-invalid={Boolean(rootErrors.rootId)}
            />
            {rootErrors.rootId ? <p className="mt-1 text-[11px] text-destructive">{rootErrors.rootId}</p> : null}
          </div>
          <div>
            <Label htmlFor="plugin-root-path" className="sr-only">
              Plugin root path
            </Label>
            <Input
              id="plugin-root-path"
              value={rootForm.path}
              onChange={(event) => setRootForm((current) => ({ ...current, path: event.target.value }))}
              className="h-8 text-[12px]"
              placeholder="/absolute/path/to/plugins"
              aria-invalid={Boolean(rootErrors.path)}
            />
            {rootErrors.path ? <p className="mt-1 text-[11px] text-destructive">{rootErrors.path}</p> : null}
          </div>
          <label className="flex h-8 items-center gap-2 text-[12px] text-muted-foreground">
            <Switch
              checked={rootForm.enabled}
              onCheckedChange={(enabled) => setRootForm((current) => ({ ...current, enabled }))}
              aria-label="Enable new plugin root"
            />
            Enabled
          </label>
          <Button
            type="button"
            size="sm"
            disabled={mutating || !onUpsertPluginRoot}
            onClick={() => void handleAddRoot()}
          >
            <FolderPlus className="h-3.5 w-3.5" />
            Add
          </Button>
        </div>

        {skillRegistry?.sources.pluginRoots.length ? (
          <div className="mt-3 divide-y divide-border/70 rounded-md border border-border/70">
            {skillRegistry.sources.pluginRoots.map((root) => (
              <div key={root.rootId} className="flex items-center justify-between gap-3 px-3 py-2">
                <div className="min-w-0">
                  <p className="truncate text-[12px] font-medium text-foreground">{root.rootId}</p>
                  <p className="truncate text-[11.5px] text-muted-foreground">{root.path}</p>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  {pendingSkillSourceId === root.rootId ? (
                    <LoaderCircle className="h-3.5 w-3.5 animate-spin text-muted-foreground" />
                  ) : null}
                  <span className="text-[11.5px] text-muted-foreground">
                    {root.enabled ? 'Enabled' : 'Disabled'}
                  </span>
                  <Switch
                    checked={root.enabled}
                    disabled={mutating || !onUpsertPluginRoot}
                    aria-label={`${root.enabled ? 'Disable' : 'Enable'} plugin root ${root.rootId}`}
                    onCheckedChange={(enabled) => {
                      void onUpsertPluginRoot?.({
                        rootId: root.rootId,
                        path: root.path,
                        enabled,
                        projectId,
                      }).catch(() => undefined)
                    }}
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 text-muted-foreground hover:text-destructive"
                    disabled={mutating || !onRemovePluginRoot}
                    aria-label={`Remove plugin root ${root.rootId}`}
                    onClick={() => void onRemovePluginRoot?.({ rootId: root.rootId, projectId }).catch(() => undefined)}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        ) : null}
      </section>

      <section className="rounded-md border border-border/70 p-3">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h4 className="text-[12px] font-semibold text-foreground">Plugin commands</h4>
            <p className="mt-0.5 text-[11.5px] text-muted-foreground">
              {skillRegistry?.pluginCommands.length ?? 0} projected
            </p>
          </div>
        </div>

        {skillRegistry?.pluginCommands.length ? (
          <div className="mt-3 divide-y divide-border/70 rounded-md border border-border/70">
            {skillRegistry.pluginCommands.map((command) => (
              <PluginCommandRow key={command.commandId} command={command} />
            ))}
          </div>
        ) : (
          <div className="mt-3 rounded-md border border-dashed border-border/70 px-4 py-8 text-center">
            <p className="text-[13px] font-medium text-foreground">No plugin commands</p>
            <p className="mt-1 text-[12px] text-muted-foreground">Enabled plugins with command contributions appear here.</p>
          </div>
        )}
      </section>
    </div>
  )
}

interface PluginRowProps {
  plugin: PluginRegistryEntryDto
  projectId: string | null
  disabled: boolean
  pending: boolean
  onSetPluginEnabled?: (request: SetPluginEnabledRequestDto) => Promise<SkillRegistryDto>
  onRemovePlugin?: (request: RemovePluginRequestDto) => Promise<SkillRegistryDto>
}

function PluginRow({
  plugin,
  projectId,
  disabled,
  pending,
  onSetPluginEnabled,
  onRemovePlugin,
}: PluginRowProps) {
  const canMutate = Boolean(projectId)
  const blocked = plugin.trust === 'blocked' || plugin.state === 'blocked'

  return (
    <div className="px-3 py-3">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-1.5">
            <p className="truncate text-[13px] font-semibold text-foreground">{plugin.name}</p>
            <Badge variant="secondary" className="text-[10px]">
              {plugin.version}
            </Badge>
            <Badge className={cn('text-[10px]', stateTone(plugin.state))}>
              {getSkillSourceStateLabel(plugin.state)}
            </Badge>
            <Badge className={cn('text-[10px]', trustTone(plugin.trust))}>
              {getSkillTrustStateLabel(plugin.trust)}
            </Badge>
          </div>
          <p className="mt-1 line-clamp-2 text-[12px] leading-[1.45] text-muted-foreground">
            {plugin.description || plugin.pluginId}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {pending ? <LoaderCircle className="h-3.5 w-3.5 animate-spin text-muted-foreground" /> : null}
          <Switch
            checked={plugin.enabled}
            disabled={disabled || !canMutate || !onSetPluginEnabled || blocked}
            aria-label={`${plugin.enabled ? 'Disable' : 'Enable'} plugin ${plugin.name}`}
            onCheckedChange={(enabled) => {
              if (!projectId) {
                return
              }
              void onSetPluginEnabled?.({ projectId, pluginId: plugin.pluginId, enabled }).catch(() => undefined)
            }}
          />
          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="h-7 w-7 text-muted-foreground hover:text-destructive"
                disabled={disabled || !canMutate || !onRemovePlugin}
                aria-label={`Remove plugin ${plugin.name}`}
              >
                <Trash2 className="h-3.5 w-3.5" />
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>Remove plugin</AlertDialogTitle>
                <AlertDialogDescription>
                  {plugin.name} will be marked unavailable for this project. Its contributed skills and commands will stop loading.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction
                  className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                  onClick={() => {
                    if (!projectId) {
                      return
                    }
                    void onRemovePlugin?.({ projectId, pluginId: plugin.pluginId }).catch(() => undefined)
                  }}
                >
                  Remove
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </div>
      </div>

      <div className="mt-2 grid gap-2 text-[11.5px] text-muted-foreground sm:grid-cols-3">
        <div className="min-w-0">
          <span className="font-medium text-foreground">Plugin </span>
          <span className="font-mono">{plugin.pluginId}</span>
        </div>
        <div className="min-w-0">
          <span className="font-medium text-foreground">Skills </span>
          <span>{plugin.skillCount}</span>
        </div>
        <div className="min-w-0">
          <span className="font-medium text-foreground">Commands </span>
          <span>{plugin.commandCount}</span>
        </div>
      </div>

      {plugin.lastDiagnostic ? (
        <div className="mt-2 flex items-start gap-2 rounded-md bg-destructive/10 px-2 py-1.5 text-[11.5px] text-destructive">
          <ShieldAlert className="mt-0.5 h-3.5 w-3.5 shrink-0" />
          <span>{plugin.lastDiagnostic.message}</span>
        </div>
      ) : null}

      <details className="mt-2 group">
        <summary className="cursor-pointer select-none text-[11.5px] font-medium text-muted-foreground hover:text-foreground">
          Plugin metadata
        </summary>
        <dl className="mt-2 grid gap-x-3 gap-y-1 rounded-md bg-muted/40 p-2 text-[11px] sm:grid-cols-[120px_1fr]">
          <div className="contents">
            <dt className="text-muted-foreground">Root id</dt>
            <dd className="min-w-0 break-words font-mono text-foreground">{plugin.rootId}</dd>
          </div>
          <div className="contents">
            <dt className="text-muted-foreground">Root path</dt>
            <dd className="min-w-0 break-words font-mono text-foreground">{plugin.rootPath}</dd>
          </div>
          <div className="contents">
            <dt className="text-muted-foreground">Plugin path</dt>
            <dd className="min-w-0 break-words font-mono text-foreground">{plugin.pluginRootPath}</dd>
          </div>
          <div className="contents">
            <dt className="text-muted-foreground">Manifest</dt>
            <dd className="min-w-0 break-words font-mono text-foreground">{plugin.manifestPath}</dd>
          </div>
          <div className="contents">
            <dt className="text-muted-foreground">Hash</dt>
            <dd className="min-w-0 break-words font-mono text-foreground">{formatHash(plugin.manifestHash)}</dd>
          </div>
          <div className="contents">
            <dt className="text-muted-foreground">Reloaded</dt>
            <dd className="min-w-0 break-words text-foreground">{formatTimestamp(plugin.lastReloadedAt)}</dd>
          </div>
        </dl>
      </details>

      {plugin.skills.length || plugin.commands.length ? (
        <details className="mt-2 group">
          <summary className="cursor-pointer select-none text-[11.5px] font-medium text-muted-foreground hover:text-foreground">
            Contributions
          </summary>
          <div className="mt-2 grid gap-2 text-[11px] lg:grid-cols-2">
            <ContributionList
              title="Skills"
              emptyLabel="No skill contributions"
              rows={plugin.skills.map((skill) => ({
                id: skill.contributionId,
                label: skill.skillId,
                value: skill.path,
              }))}
            />
            <ContributionList
              title="Commands"
              emptyLabel="No command contributions"
              rows={plugin.commands.map((command) => ({
                id: command.contributionId,
                label: command.label,
                value: command.entry,
              }))}
            />
          </div>
        </details>
      ) : null}
    </div>
  )
}

interface ContributionListProps {
  title: string
  emptyLabel: string
  rows: Array<{ id: string; label: string; value: string }>
}

function ContributionList({ title, emptyLabel, rows }: ContributionListProps) {
  return (
    <div className="rounded-md border border-border/70 p-2">
      <p className="text-[11px] font-semibold text-foreground">{title}</p>
      {rows.length ? (
        <div className="mt-1.5 space-y-1.5">
          {rows.map((row) => (
            <div key={row.id} className="min-w-0">
              <p className="truncate font-medium text-foreground">{row.label}</p>
              <p className="break-words font-mono text-muted-foreground">{row.value}</p>
            </div>
          ))}
        </div>
      ) : (
        <p className="mt-1.5 text-muted-foreground">{emptyLabel}</p>
      )}
    </div>
  )
}

function PluginCommandRow({ command }: { command: PluginCommandContributionDto }) {
  return (
    <div className="px-3 py-2">
      <div className="flex flex-wrap items-center gap-1.5">
        <p className="text-[12px] font-semibold text-foreground">{command.label}</p>
        <Badge variant="secondary" className="text-[10px]">
          {getPluginCommandAvailabilityLabel(command.availability)}
        </Badge>
        <Badge className={cn('text-[10px]', stateTone(command.state))}>
          {getSkillSourceStateLabel(command.state)}
        </Badge>
        <Badge className={cn('text-[10px]', trustTone(command.trust))}>
          {getSkillTrustStateLabel(command.trust)}
        </Badge>
      </div>
      <p className="mt-1 line-clamp-2 text-[11.5px] leading-[1.45] text-muted-foreground">
        {command.description}
      </p>
      <div className="mt-2 grid gap-2 text-[11px] text-muted-foreground sm:grid-cols-3">
        <div className="min-w-0">
          <span className="font-medium text-foreground">Command </span>
          <span className="break-words font-mono">{command.commandId}</span>
        </div>
        <div className="min-w-0">
          <span className="font-medium text-foreground">Plugin </span>
          <span className="break-words font-mono">{command.pluginId}</span>
        </div>
        <div className="min-w-0">
          <span className="font-medium text-foreground">Entry </span>
          <span className="break-words font-mono">{command.entry}</span>
        </div>
      </div>
    </div>
  )
}
