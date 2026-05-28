#!/usr/bin/env node
import { createHash } from 'node:crypto'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { createInterface } from 'node:readline/promises'

const APP_DATA_DIRECTORY_NAME = 'com.hyperpush.xero'
const GLOBAL_DATABASE_FILE_NAME = 'xero.db'
const PROJECTS_DIRECTORY = 'projects'
const PROJECT_DATABASE_FILE_NAME = 'state.db'

const TARGET_ALIASES = new Map([
  ['all', 'app-data'],
  ['appdata', 'app-data'],
  ['app-data', 'app-data'],
  ['global', 'global-sqlite'],
  ['global-db', 'global-sqlite'],
  ['globaldb', 'global-sqlite'],
  ['global-sqlite', 'global-sqlite'],
  ['xero-db', 'global-sqlite'],
  ['xero.db', 'global-sqlite'],
  ['sqlite', 'project-sqlite'],
  ['sqlitedb', 'project-sqlite'],
  ['sqlite-db', 'project-sqlite'],
  ['project-sqlite', 'project-sqlite'],
  ['project-sqlitedb', 'project-sqlite'],
  ['state-db', 'project-sqlite'],
  ['state.db', 'project-sqlite'],
  ['lance', 'lancedb'],
  ['lancedb', 'lancedb'],
  ['lance-db', 'lancedb'],
  ['project-lance', 'lancedb'],
  ['project-lancedb', 'lancedb'],
  ['attachments', 'attachments'],
  ['attachment', 'attachments'],
  ['tool-artifacts', 'tool-artifacts'],
  ['toolartifacts', 'tool-artifacts'],
  ['artifacts', 'tool-artifacts'],
  ['artifact', 'tool-artifacts'],
  ['backups', 'backups'],
  ['backup', 'backups'],
  ['project', 'project'],
  ['project-dir', 'project'],
  ['project-data', 'project'],
  ['projects', 'projects'],
  ['all-projects', 'projects'],
  ['window-state', 'window-state'],
  ['window-state.json', 'window-state'],
])

const PROJECT_TARGETS = new Set([
  'project-sqlite',
  'lancedb',
  'attachments',
  'tool-artifacts',
  'backups',
  'project',
])

const INTERACTIVE_TARGETS = [
  {
    target: 'project-sqlite',
    label: 'Project SQLite',
    detail: 'projects/<project-id>/state.db plus WAL/SHM/journal sidecars',
  },
  {
    target: 'lancedb',
    label: 'Project LanceDB',
    detail: 'projects/<project-id>/lance',
  },
  {
    target: 'global-sqlite',
    label: 'Global SQLite',
    detail: 'xero.db plus WAL/SHM/journal sidecars',
  },
  {
    target: 'tool-artifacts',
    label: 'Tool artifacts',
    detail: 'projects/<project-id>/tool-artifacts',
  },
  {
    target: 'attachments',
    label: 'Attachments',
    detail: 'projects/<project-id>/attachments',
  },
  {
    target: 'backups',
    label: 'Project backups',
    detail: 'projects/<project-id>/backups',
  },
  {
    target: 'project',
    label: 'Entire project app-data dir',
    detail: 'projects/<project-id>',
  },
  {
    target: 'projects',
    label: 'All project app-data dirs',
    detail: 'projects/',
  },
  {
    target: 'window-state',
    label: 'Window state',
    detail: 'window-state.json',
  },
  {
    target: 'app-data',
    label: 'Everything',
    detail: 'entire Xero app-data directory',
  },
]

function usage() {
  console.log(`Usage:
  pnpm data:delete -- [options] <target> [target...]

Targets:
  global-sqlite       Delete app-data xero.db plus SQLite sidecars.
  sqlitedb            Delete per-project state.db plus SQLite sidecars. Requires --project, --repo, or --all-projects.
  lancedb             Delete per-project LanceDB directory. Requires --project, --repo, or --all-projects.
  attachments         Delete per-project staged agent attachments.
  artifacts           Delete per-project tool-artifacts.
  backups             Delete per-project project-state backups.
  project             Delete an entire app-data project directory.
  projects            Delete every directory under app-data/projects.
  window-state        Delete app-data window-state.json.
  all                 Delete the entire Xero app-data directory.

Options:
  --project VALUE     Project id, repo path, or "current". Repeatable.
  --repo PATH         Repo path to derive the project id from. Repeatable.
  --all-projects     Apply project targets to every app-data project directory.
  --app-data-dir PATH Override Xero app-data dir. Defaults to XERO_APP_DATA_DIR or the production OS path.
  -i, --interactive Select targets and projects with prompts. Default when no targets are passed.
  --dry-run          Print what would be deleted.
  -y, --yes          Required for actual deletion.
  --json             Print a machine-readable result.
  -h, --help         Show this help.

Examples:
  pnpm data:delete -- --dry-run --project current sqlitedb lancedb
  pnpm data:delete
  pnpm data:delete -- --project current sqlitedb lancedb --yes
  pnpm data:delete -- --all-projects lancedb --yes
  pnpm data:delete -- global-sqlite window-state --yes`)
}

function parseArgs(argv) {
  const options = {
    appDataDir: process.env.XERO_APP_DATA_DIR || '',
    dryRun: false,
    interactive: false,
    json: false,
    list: false,
    projects: [],
    repos: [],
    allProjects: false,
    targets: [],
    yes: false,
  }

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index]
    if (arg === '--') {
      continue
    } else if (arg === '-h' || arg === '--help') {
      usage()
      process.exit(0)
    } else if (arg === '--dry-run') {
      options.dryRun = true
    } else if (arg === '-i' || arg === '--interactive') {
      options.interactive = true
    } else if (arg === '--json') {
      options.json = true
    } else if (arg === '-y' || arg === '--yes') {
      options.yes = true
    } else if (arg === '--all-projects') {
      options.allProjects = true
    } else if (arg === '--list') {
      options.list = true
    } else if (arg === '--app-data-dir') {
      options.appDataDir = requireValue(argv, index, arg)
      index += 1
    } else if (arg.startsWith('--app-data-dir=')) {
      options.appDataDir = arg.slice('--app-data-dir='.length)
    } else if (arg === '--project') {
      options.projects.push(requireValue(argv, index, arg))
      index += 1
    } else if (arg.startsWith('--project=')) {
      options.projects.push(arg.slice('--project='.length))
    } else if (arg === '--repo') {
      options.repos.push(requireValue(argv, index, arg))
      index += 1
    } else if (arg.startsWith('--repo=')) {
      options.repos.push(arg.slice('--repo='.length))
    } else if (arg.startsWith('-')) {
      throw new Error(`Unknown option: ${arg}`)
    } else {
      options.targets.push(...splitTargetArg(arg))
    }
  }

  return options
}

function requireValue(argv, index, option) {
  const value = argv[index + 1]
  if (!value || value.startsWith('-')) {
    throw new Error(`${option} requires a value.`)
  }
  return value
}

function splitTargetArg(arg) {
  return arg
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean)
}

function defaultAppDataDir() {
  const home = os.homedir()
  if (process.platform === 'darwin') {
    return path.join(home, 'Library', 'Application Support', APP_DATA_DIRECTORY_NAME)
  }
  if (process.platform === 'win32') {
    const root = process.env.APPDATA || process.env.LOCALAPPDATA
    if (!root) {
      throw new Error('APPDATA or LOCALAPPDATA is required to locate Xero app-data on Windows.')
    }
    return path.join(root, APP_DATA_DIRECTORY_NAME)
  }
  const root = process.env.XDG_DATA_HOME || path.join(home, '.local', 'share')
  return path.join(root, APP_DATA_DIRECTORY_NAME)
}

function normalizePath(input) {
  if (!input) return ''
  const expanded =
    input === '~' || input.startsWith(`~${path.sep}`)
      ? path.join(os.homedir(), input.slice(2))
      : input
  return path.resolve(expanded)
}

function projectIdForRepo(repoPath) {
  const resolved = normalizePath(repoPath)
  if (!fs.existsSync(resolved)) {
    throw new Error(`Repo path does not exist: ${resolved}`)
  }
  const canonical = fs.realpathSync.native?.(resolved) ?? fs.realpathSync(resolved)
  const digest = createHash('sha256').update(canonical).digest('hex').slice(0, 32)
  return `project_${digest}`
}

function resolveProjectSpecifier(specifier) {
  if (specifier === 'current') {
    return projectIdForRepo(process.cwd())
  }

  const expanded = normalizePath(specifier)
  const looksLikePath =
    specifier.startsWith('.') ||
    specifier.startsWith('/') ||
    specifier.startsWith('~') ||
    specifier.includes('/') ||
    specifier.includes('\\') ||
    fs.existsSync(expanded)

  if (looksLikePath) {
    return projectIdForRepo(specifier)
  }

  if (specifier.includes('/') || specifier.includes('\\')) {
    throw new Error(`Project id must not contain path separators: ${specifier}`)
  }
  return specifier
}

function projectIdsFromAppData(appDataDir) {
  const projectsDir = path.join(appDataDir, PROJECTS_DIRECTORY)
  if (!fs.existsSync(projectsDir)) return []
  return fs
    .readdirSync(projectsDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort()
}

function normalizeTargets(rawTargets) {
  const targets = []
  for (const rawTarget of rawTargets) {
    const key = rawTarget.trim().toLowerCase()
    const target = TARGET_ALIASES.get(key)
    if (!target) {
      throw new Error(`Unknown delete target: ${rawTarget}`)
    }
    if (!targets.includes(target)) {
      targets.push(target)
    }
  }
  return targets
}

function sqliteSidecarPaths(databasePath) {
  return [
    databasePath,
    `${databasePath}-wal`,
    `${databasePath}-shm`,
    `${databasePath}-journal`,
  ]
}

function previewResults(entries) {
  return entries.map((entry) => {
    const exists = fs.existsSync(entry.path)
    return {
      ...entry,
      exists,
      action: exists ? 'would-delete' : 'missing',
    }
  })
}

function planDeletes(options) {
  const appDataDir = normalizePath(options.appDataDir || defaultAppDataDir())
  ensureSafeAppDataDir(appDataDir)

  const targets = normalizeTargets(options.targets)
  if (targets.length === 0) {
    throw new Error('Pass at least one delete target. Use --help for supported targets.')
  }

  if (targets.includes('app-data')) {
    return {
      appDataDir,
      targets,
      projectIds: [],
      entries: [deleteEntry('app-data', appDataDir)],
    }
  }

  let projectIds = [
    ...options.projects.map(resolveProjectSpecifier),
    ...options.repos.map(projectIdForRepo),
  ]
  if (options.allProjects || targets.includes('projects')) {
    projectIds = [...projectIds, ...projectIdsFromAppData(appDataDir)]
  }
  projectIds = [...new Set(projectIds)].sort()

  const needsProject = targets.some((target) => PROJECT_TARGETS.has(target))
  if (needsProject && projectIds.length === 0) {
    throw new Error(
      'Project-scoped targets require --project VALUE, --repo PATH, or --all-projects.',
    )
  }

  const entries = []
  for (const target of targets) {
    if (target === 'app-data') {
      entries.push(deleteEntry(target, appDataDir))
    } else if (target === 'global-sqlite') {
      for (const targetPath of sqliteSidecarPaths(path.join(appDataDir, GLOBAL_DATABASE_FILE_NAME))) {
        entries.push(deleteEntry(target, targetPath))
      }
    } else if (target === 'projects') {
      entries.push(deleteEntry(target, path.join(appDataDir, PROJECTS_DIRECTORY)))
    } else if (target === 'window-state') {
      entries.push(deleteEntry(target, path.join(appDataDir, 'window-state.json')))
    } else {
      for (const projectId of projectIds) {
        const projectDir = path.join(appDataDir, PROJECTS_DIRECTORY, projectId)
        if (target === 'project-sqlite') {
          for (const targetPath of sqliteSidecarPaths(
            path.join(projectDir, PROJECT_DATABASE_FILE_NAME),
          )) {
            entries.push(deleteEntry(target, targetPath, projectId))
          }
        } else if (target === 'lancedb') {
          entries.push(deleteEntry(target, path.join(projectDir, 'lance'), projectId))
        } else if (target === 'attachments') {
          entries.push(deleteEntry(target, path.join(projectDir, 'attachments'), projectId))
        } else if (target === 'tool-artifacts') {
          entries.push(deleteEntry(target, path.join(projectDir, 'tool-artifacts'), projectId))
        } else if (target === 'backups') {
          entries.push(deleteEntry(target, path.join(projectDir, 'backups'), projectId))
        } else if (target === 'project') {
          entries.push(deleteEntry(target, projectDir, projectId))
        }
      }
    }
  }

  return {
    appDataDir,
    targets,
    projectIds,
    entries: collapseCoveredEntries(dedupeEntries(entries), appDataDir),
  }
}

function deleteEntry(kind, targetPath, projectId = null) {
  return {
    kind,
    projectId,
    path: path.resolve(targetPath),
  }
}

function dedupeEntries(entries) {
  const seen = new Set()
  const deduped = []
  for (const entry of entries) {
    const key = entry.path
    if (seen.has(key)) continue
    seen.add(key)
    deduped.push(entry)
  }
  return deduped
}

function collapseCoveredEntries(entries, appDataDir) {
  const sorted = entries.slice().sort((left, right) => left.path.length - right.path.length)
  const kept = []
  for (const entry of sorted) {
    ensureInsideAppData(entry.path, appDataDir)
    if (kept.some((candidate) => isSameOrChildPath(candidate.path, entry.path))) {
      continue
    }
    kept.push(entry)
  }
  return kept
}

function isSameOrChildPath(parentPath, candidatePath) {
  const relative = path.relative(parentPath, candidatePath)
  return relative === '' || (!relative.startsWith('..') && !path.isAbsolute(relative))
}

function ensureSafeAppDataDir(appDataDir) {
  const parsed = path.parse(appDataDir)
  if (appDataDir === parsed.root) {
    throw new Error(`Refusing to use filesystem root as app-data dir: ${appDataDir}`)
  }
  if (appDataDir.length < parsed.root.length + APP_DATA_DIRECTORY_NAME.length) {
    throw new Error(`App-data dir looks too broad: ${appDataDir}`)
  }
}

function ensureInsideAppData(targetPath, appDataDir) {
  if (!isSameOrChildPath(appDataDir, targetPath)) {
    throw new Error(`Refusing to delete outside app-data: ${targetPath}`)
  }
}

async function deletePlannedEntries(plan, options) {
  const results = []
  for (const entry of plan.entries) {
    const exists = fs.existsSync(entry.path)
    const result = {
      ...entry,
      exists,
      action: exists ? (options.dryRun ? 'would-delete' : 'deleted') : 'missing',
    }
    if (exists && !options.dryRun) {
      await fs.promises.rm(entry.path, { force: true, recursive: true })
    }
    results.push(result)
  }
  return results
}

function printTextResult(plan, results, options) {
  console.log(`Xero app-data: ${plan.appDataDir}`)
  if (plan.projectIds.length > 0) {
    console.log(`Projects: ${plan.projectIds.join(', ')}`)
  }

  const existing = results.filter((result) => result.exists)
  const missing = results.filter((result) => !result.exists)
  if (options.dryRun) {
    console.log(`Dry run: ${existing.length} path(s) would be deleted.`)
  } else {
    console.log(`Deleted: ${existing.length} path(s).`)
  }

  for (const result of results) {
    const suffix = result.projectId ? ` [${result.projectId}]` : ''
    console.log(`${result.action.padEnd(12)} ${result.kind}${suffix}: ${result.path}`)
  }

  if (!options.dryRun && missing.length > 0) {
    console.log(`Missing paths skipped: ${missing.length}`)
  }
}

async function promptInteractiveOptions(baseOptions) {
  const appDataDefault = normalizePath(baseOptions.appDataDir || defaultAppDataDir())
  if (supportsArrowKeyMenus()) {
    return promptInteractiveMenuOptions(baseOptions, appDataDefault)
  }
  return promptInteractiveLineOptions(baseOptions, appDataDefault)
}

function supportsArrowKeyMenus() {
  return Boolean(process.stdin.isTTY && process.stdout.isTTY)
}

async function promptInteractiveMenuOptions(baseOptions, appDataDir) {
  console.log('Xero app-data deletion')
  console.log('Choose what to delete. A preview is shown before anything is removed.')
  console.log(`Xero app-data: ${appDataDir}`)

  const targets = await promptTargetsMenu(appDataDir)
  const interactiveOptions = {
    ...baseOptions,
    appDataDir,
    allProjects: false,
    projects: [],
    repos: [],
    targets,
    yes: false,
  }

  if (targetsNeedProjectSelection(targets)) {
    const selection = await promptProjectsMenu(appDataDir)
    interactiveOptions.projects = selection.projects
    interactiveOptions.allProjects = selection.allProjects
  }

  return interactiveOptions
}

async function promptInteractiveLineOptions(baseOptions, appDataDefault) {
  const rl = process.stdin.isTTY
    ? createInterface({
        input: process.stdin,
        output: process.stdout,
      })
    : createScriptedLinePrompt(await readStdinLines())

  try {
    console.log('Xero app-data deletion')
    console.log('Choose what to delete. A preview is shown before anything is removed.')
    console.log('')

    const appDataAnswer = await rl.question(`App-data directory [${appDataDefault}]: `)
    const appDataDir = normalizePath(appDataAnswer.trim() || appDataDefault)

    const targets = await promptTargetsLine(rl)
    const interactiveOptions = {
      ...baseOptions,
      appDataDir,
      allProjects: false,
      projects: [],
      repos: [],
      targets,
      yes: false,
    }

    if (targetsNeedProjectSelection(targets)) {
      const selection = await promptProjectsLine(rl, appDataDir)
      interactiveOptions.projects = selection.projects
      interactiveOptions.allProjects = selection.allProjects
    }

    return interactiveOptions
  } finally {
    rl.close()
  }
}

async function readStdinLines() {
  const chunks = []
  for await (const chunk of process.stdin) {
    chunks.push(Buffer.from(chunk))
  }
  return Buffer.concat(chunks).toString('utf8').split(/\r?\n/)
}

function createScriptedLinePrompt(lines) {
  let index = 0
  return {
    async question(question) {
      if (index >= lines.length) {
        throw new Error(`Missing piped answer for prompt: ${question.trim()}`)
      }
      const answer = lines[index]
      index += 1
      process.stdout.write(question)
      process.stdout.write(answer ? `${answer}\n` : '\n')
      return answer
    },
    close() {},
  }
}

async function promptTargetsMenu(appDataDir) {
  return selectFromMenu({
    title: 'Select Data To Delete',
    headerLines: [
      `Xero app-data: ${appDataDir}`,
      'Space toggles an item. Enter continues.',
    ],
    options: INTERACTIVE_TARGETS.map((option) => ({
      value: option.target,
      label: option.label,
      detail: option.detail,
    })),
    multi: true,
  })
}

async function promptProjectsMenu(appDataDir) {
  const discovered = projectIdsFromAppData(appDataDir)
  const currentProjectId = safeCurrentProjectId()
  const options = []

  if (currentProjectId) {
    options.push({
      value: { kind: 'project', project: 'current' },
      label: 'Current repo',
      detail: currentProjectId,
    })
  }
  for (const projectId of discovered) {
    options.push({
      value: { kind: 'project', project: projectId },
      label: projectId,
      detail: 'Discovered project app-data directory',
    })
  }
  if (discovered.length > 0) {
    options.push({
      value: { kind: 'all' },
      label: 'Every discovered project',
      detail: `${discovered.length} project(s) under app-data/projects`,
    })
  }
  options.push({
    value: { kind: 'manual' },
    label: 'Type project id or repo path',
    detail: 'Opens one text prompt for custom entries',
  })

  const selected = await selectFromMenu({
    title: 'Select Project Scope',
    headerLines: [
      `Xero app-data: ${appDataDir}`,
      'Space toggles an item. Enter continues.',
    ],
    options,
    multi: true,
  })

  const projects = []
  let allProjects = false
  let wantsManual = false
  for (const item of selected) {
    if (item.kind === 'all') {
      allProjects = true
    } else if (item.kind === 'manual') {
      wantsManual = true
    } else {
      projects.push(item.project)
    }
  }

  if (wantsManual) {
    const manualAnswer = await promptLine(
      'Project id or repo path, comma-separated: ',
    )
    const manual = parseProjectSelection(manualAnswer, discovered)
    projects.push(...manual.projects)
    allProjects = allProjects || manual.allProjects
  }

  return {
    allProjects,
    projects: [...new Set(projects)],
  }
}

async function selectFromMenu({ title, headerLines = [], options, multi }) {
  if (options.length === 0) {
    throw new Error('No selectable options are available.')
  }

  return new Promise((resolve, reject) => {
    const stdin = process.stdin
    const stdout = process.stdout
    const previousRawMode = stdin.isRaw
    let cursor = 0
    const selected = new Set()
    let message = ''
    let done = false

    const cleanup = () => {
      if (done) return
      done = true
      stdin.off('data', onData)
      stdin.setRawMode(previousRawMode)
      stdout.write('\x1b[?25h')
      stdout.write('\n')
    }

    const finish = () => {
      if (multi && selected.size === 0) {
        selected.add(cursor)
      }
      cleanup()
      const indexes = multi ? [...selected].sort((left, right) => left - right) : [cursor]
      resolve(indexes.map((index) => options[index].value))
    }

    const cancel = () => {
      cleanup()
      reject(new Error('Cancelled.'))
    }

    const render = () => {
      stdout.write('\x1b[2J\x1b[H\x1b[?25l')
      stdout.write(`${title}\n\n`)
      for (const line of headerLines) {
        stdout.write(`${line}\n`)
      }
      stdout.write('\n')
      stdout.write('Use Up/Down or j/k to move, Space to select, Enter to continue, q to cancel.\n\n')

      for (const [index, option] of options.entries()) {
        const focused = index === cursor ? '>' : ' '
        const marker = multi ? (selected.has(index) ? '[x]' : '[ ]') : '   '
        stdout.write(`${focused} ${marker} ${option.label}\n`)
        if (option.detail) {
          stdout.write(`      ${option.detail}\n`)
        }
      }
      if (message) {
        stdout.write(`\n${message}\n`)
      }
    }

    const move = (delta) => {
      cursor = (cursor + delta + options.length) % options.length
      message = ''
      render()
    }

    const toggle = () => {
      if (!multi) {
        finish()
        return
      }
      if (selected.has(cursor)) {
        selected.delete(cursor)
      } else {
        selected.add(cursor)
      }
      message = ''
      render()
    }

    const handleKey = (key) => {
      if (key === '\u0003' || key === 'q' || key === 'Q') {
        cancel()
      } else if (key === '\x1b[A' || key === 'k' || key === 'K') {
        move(-1)
      } else if (key === '\x1b[B' || key === 'j' || key === 'J') {
        move(1)
      } else if (key === ' ') {
        toggle()
      } else if (key === '\r' || key === '\n') {
        finish()
      }
    }

    function onData(buffer) {
      const value = buffer.toString('utf8')
      for (let index = 0; index < value.length; index += 1) {
        if (value.startsWith('\x1b[A', index) || value.startsWith('\x1b[B', index)) {
          handleKey(value.slice(index, index + 3))
          index += 2
        } else {
          handleKey(value[index])
        }
      }
    }

    stdin.setRawMode(true)
    stdin.resume()
    stdin.on('data', onData)
    render()
  })
}

async function promptLine(question) {
  const rl = createInterface({
    input: process.stdin,
    output: process.stdout,
  })
  try {
    return await rl.question(question)
  } finally {
    rl.close()
  }
}

async function promptTargetsLine(rl) {
  console.log('Delete targets:')
  for (const [index, option] of INTERACTIVE_TARGETS.entries()) {
    console.log(`  ${index + 1}. ${option.label} - ${option.detail}`)
  }
  console.log('')

  while (true) {
    const answer = await rl.question('Select targets by number or name, comma-separated: ')
    try {
      const targets = parseTargetSelection(answer)
      if (targets.length > 0) return targets
      console.log('Pick at least one target.')
    } catch (error) {
      console.log(error.message)
    }
  }
}

function parseTargetSelection(answer) {
  const tokens = answer
    .split(/[,\s]+/)
    .map((token) => token.trim())
    .filter(Boolean)
  const targets = []

  for (const token of tokens) {
    let target
    if (/^\d+$/.test(token)) {
      const index = Number(token) - 1
      target = INTERACTIVE_TARGETS[index]?.target
      if (!target) {
        throw new Error(`No target is numbered ${token}.`)
      }
    } else {
      target = TARGET_ALIASES.get(token.toLowerCase())
      if (!target) {
        throw new Error(`Unknown target: ${token}`)
      }
    }
    if (!targets.includes(target)) {
      targets.push(target)
    }
  }

  return targets
}

function targetsNeedProjectSelection(targets) {
  return (
    targets.some((target) => PROJECT_TARGETS.has(target)) &&
    !targets.includes('app-data') &&
    !targets.includes('projects')
  )
}

async function promptProjectsLine(rl, appDataDir) {
  const discovered = projectIdsFromAppData(appDataDir)
  const currentProjectId = safeCurrentProjectId()

  console.log('')
  console.log('Project selection:')
  if (currentProjectId) {
    console.log(`  current. Current repo (${currentProjectId})`)
  }
  for (const [index, projectId] of discovered.entries()) {
    console.log(`  ${index + 1}. ${projectId}`)
  }
  if (discovered.length > 0) {
    console.log('  all. Every discovered project')
  }
  console.log('')

  while (true) {
    const fallback = currentProjectId ? 'current' : ''
    const suffix = fallback ? ` [${fallback}]` : ''
    const answer = await rl.question(
      `Select projects by number, "current", "all", id, or repo path${suffix}: `,
    )
    const value = answer.trim() || fallback
    try {
      const selection = parseProjectSelection(value, discovered)
      if (selection.allProjects || selection.projects.length > 0) return selection
      console.log('Pick at least one project.')
    } catch (error) {
      console.log(error.message)
    }
  }
}

function parseProjectSelection(answer, discoveredProjects) {
  const tokens = answer
    .split(',')
    .map((token) => token.trim())
    .filter(Boolean)
  const projects = []
  let allProjects = false

  for (const token of tokens) {
    if (token.toLowerCase() === 'all') {
      allProjects = true
      continue
    }
    if (/^\d+$/.test(token)) {
      const index = Number(token) - 1
      const projectId = discoveredProjects[index]
      if (!projectId) {
        throw new Error(`No project is numbered ${token}.`)
      }
      projects.push(projectId)
      continue
    }
    projects.push(token)
  }

  return {
    allProjects,
    projects: [...new Set(projects)],
  }
}

function safeCurrentProjectId() {
  try {
    return projectIdForRepo(process.cwd())
  } catch {
    return null
  }
}

async function confirmInteractiveDelete(existingCount) {
  if (existingCount === 0) {
    console.log('No existing paths matched the selected targets.')
    return false
  }

  const rl = createInterface({
    input: process.stdin,
    output: process.stdout,
  })
  try {
    const answer = await rl.question(
      `Type DELETE to remove ${existingCount} existing path(s), or press Enter to cancel: `,
    )
    return answer.trim() === 'DELETE'
  } finally {
    rl.close()
  }
}

async function main() {
  const options = parseArgs(process.argv.slice(2))
  if (options.list) {
    usage()
    return
  }

  const shouldPrompt =
    options.interactive ||
    (options.targets.length === 0 && process.stdin.isTTY && !options.json)
  if (shouldPrompt) {
    const interactiveOptions = await promptInteractiveOptions(options)
    const plan = planDeletes(interactiveOptions)
    const preview = previewResults(plan.entries)
    printTextResult(plan, preview, {
      ...interactiveOptions,
      dryRun: true,
    })

    if (interactiveOptions.dryRun) {
      return
    }

    const existingCount = preview.filter((result) => result.exists).length
    const confirmed = await confirmInteractiveDelete(existingCount)
    if (!confirmed) {
      console.log('Cancelled. No data was deleted.')
      return
    }

    const results = await deletePlannedEntries(plan, {
      ...interactiveOptions,
      yes: true,
    })
    printTextResult(plan, results, interactiveOptions)
    return
  }

  const plan = planDeletes(options)

  if (!options.dryRun && !options.yes) {
    printTextResult(plan, previewResults(plan.entries), {
      ...options,
      dryRun: true,
    })
    throw new Error('Refusing to delete without --yes. Re-run with --yes or use --dry-run.')
  }

  const results = await deletePlannedEntries(plan, options)
  if (options.json) {
    console.log(
      JSON.stringify(
        {
          appDataDir: plan.appDataDir,
          dryRun: options.dryRun,
          targets: plan.targets,
          projectIds: plan.projectIds,
          results,
        },
        null,
        2,
      ),
    )
  } else {
    printTextResult(plan, results, options)
  }
}

main().catch((error) => {
  console.error(`delete-app-data: ${error.message}`)
  process.exit(1)
})
