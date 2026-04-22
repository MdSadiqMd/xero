/**
 * Singleton shiki highlighter for diff syntax highlighting.
 *
 * Uses the JavaScript regex engine (no WASM) and loads languages on demand.
 * All themes exported from `features/theme/theme-definitions` are bundled
 * at build time so switching themes at runtime doesn't require a network
 * request. New themes that reference additional Shiki themes must also be
 * added to the `themes` array passed to `createHighlighter`.
 */

import { createHighlighter, type Highlighter, type ThemedToken } from 'shiki'
import { THEMES } from '@/src/features/theme/theme-definitions'

let highlighterPromise: Promise<Highlighter> | null = null
const loadedLangs = new Set<string>()
const loadedThemes = new Set<string>()

/** File extension → shiki/editor language id */
const EXT_LANG_MAP: Record<string, string> = {
  // TypeScript / JavaScript
  ts: 'typescript',
  tsx: 'tsx',
  js: 'javascript',
  jsx: 'jsx',
  mts: 'typescript',
  cts: 'typescript',
  mjs: 'javascript',
  cjs: 'javascript',

  // Systems / compiled
  rs: 'rust',
  go: 'go',
  c: 'c',
  h: 'c',
  cpp: 'cpp',
  cxx: 'cpp',
  cc: 'cpp',
  hpp: 'cpp',
  hxx: 'cpp',
  hh: 'cpp',
  java: 'java',
  kt: 'kotlin',
  kts: 'kotlin',
  scala: 'scala',
  dart: 'dart',
  cs: 'csharp',

  // Scripting
  py: 'python',
  pyi: 'python',
  rb: 'ruby',
  swift: 'swift',
  lua: 'lua',
  pl: 'perl',
  pm: 'perl',
  r: 'r',
  jl: 'julia',
  ex: 'elixir',
  exs: 'elixir',
  erl: 'erlang',
  hs: 'haskell',
  clj: 'clojure',
  cljs: 'clojure',
  cljc: 'clojure',
  elm: 'elm',
  ml: 'ocaml',
  mli: 'ocaml',
  fs: 'fsharp',
  fsx: 'fsharp',
  groovy: 'groovy',
  gradle: 'groovy',

  // Shell / ops
  sh: 'bash',
  bash: 'bash',
  zsh: 'bash',
  fish: 'bash',
  ps1: 'powershell',
  psm1: 'powershell',
  psd1: 'powershell',
  tcl: 'tcl',

  // Data / config
  json: 'json',
  jsonc: 'jsonc',
  json5: 'json',
  toml: 'toml',
  yaml: 'yaml',
  yml: 'yaml',
  ini: 'properties',
  conf: 'properties',
  properties: 'properties',
  env: 'properties',

  // Web
  html: 'html',
  htm: 'html',
  xhtml: 'html',
  xml: 'xml',
  svg: 'xml',
  css: 'css',
  scss: 'scss',
  sass: 'sass',
  less: 'less',
  styl: 'stylus',
  stylus: 'stylus',
  vue: 'vue',
  svelte: 'svelte',
  php: 'php',
  phtml: 'php',

  // Markup / docs
  md: 'markdown',
  mdx: 'mdx',
  markdown: 'markdown',

  // Query / data
  sql: 'sql',
  graphql: 'graphql',
  gql: 'graphql',

  // Infra / misc
  dockerfile: 'dockerfile',
  diff: 'diff',
  patch: 'diff',
  lock: 'json',
  nix: 'nix',
  tf: 'terraform',
  hcl: 'terraform',
  proto: 'protobuf',
  cmake: 'cmake',
}

/** Basename (case-insensitive) → language id, for files without useful extensions */
const BASENAME_LANG_MAP: Record<string, string> = {
  dockerfile: 'dockerfile',
  'containerfile': 'dockerfile',
  makefile: 'makefile',
  'gnumakefile': 'makefile',
  rakefile: 'ruby',
  gemfile: 'ruby',
  procfile: 'properties',
  '.env': 'properties',
  '.gitignore': 'properties',
  '.gitattributes': 'properties',
  '.dockerignore': 'properties',
  '.npmrc': 'properties',
  '.editorconfig': 'properties',
  'cmakelists.txt': 'cmake',
}

function bundledShikiThemes(): string[] {
  const ids = new Set<string>()
  for (const theme of THEMES) {
    ids.add(theme.shiki)
  }
  return Array.from(ids)
}

function getHighlighter(): Promise<Highlighter> {
  if (!highlighterPromise) {
    const themes = bundledShikiThemes()
    highlighterPromise = createHighlighter({
      themes: themes as never,
      langs: [],
    })
    for (const theme of themes) {
      loadedThemes.add(theme)
    }
  }
  return highlighterPromise
}

export function getLangFromPath(filePath: string): string | null {
  const slash = Math.max(filePath.lastIndexOf('/'), filePath.lastIndexOf('\\'))
  const base = filePath.slice(slash + 1).toLowerCase()
  if (!base) return null

  const byBasename = BASENAME_LANG_MAP[base]
  if (byBasename) return byBasename

  const dot = base.lastIndexOf('.')
  if (dot <= 0) return null
  const ext = base.slice(dot + 1)
  return EXT_LANG_MAP[ext] ?? null
}

export type TokenizedLine = ThemedToken[]

/**
 * Tokenize a block of code into per-line token arrays.
 *
 * @param code   Source text to tokenize.
 * @param lang   Shiki language id (see {@link getLangFromPath}).
 * @param theme  Shiki theme id. Defaults to the first registered theme's
 *               shiki id. Themes not bundled at init time will be loaded
 *               on demand.
 * @returns Per-line token arrays, or `null` if tokenization fails.
 */
export async function tokenizeCode(
  code: string,
  lang: string,
  theme: string = THEMES[0].shiki,
): Promise<TokenizedLine[] | null> {
  try {
    const hl = await getHighlighter()

    if (!loadedLangs.has(lang)) {
      await hl.loadLanguage(lang as never)
      loadedLangs.add(lang)
    }

    if (!loadedThemes.has(theme)) {
      await hl.loadTheme(theme as never)
      loadedThemes.add(theme)
    }

    const { tokens } = hl.codeToTokens(code, {
      lang: lang as never,
      theme,
    })
    return tokens
  } catch {
    return null
  }
}
