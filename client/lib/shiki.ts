/** Lazy singleton shiki highlighter for code and diff syntax highlighting. */

import type { Highlighter, ThemedToken } from 'shiki'
export { getLangFromPath } from '@/lib/language-detection'

let highlighterPromise: Promise<Highlighter> | null = null
const loadedLangs = new Set<string>()
const loadedThemes = new Set<string>()
const DEFAULT_SHIKI_THEME = 'github-dark'

function getHighlighter(): Promise<Highlighter> {
  if (!highlighterPromise) {
    highlighterPromise = import('shiki').then(({ createHighlighter }) =>
      createHighlighter({
        themes: [DEFAULT_SHIKI_THEME] as never,
        langs: [],
      }),
    )
    loadedThemes.add(DEFAULT_SHIKI_THEME)
  }
  return highlighterPromise
}

async function ensureTheme(hl: Highlighter, theme: string): Promise<void> {
  if (loadedThemes.has(theme)) {
    return
  }

  await hl.loadTheme(theme as never)
  loadedThemes.add(theme)
}

async function ensureLanguage(hl: Highlighter, lang: string): Promise<void> {
  if (loadedLangs.has(lang)) {
    return
  }

  await hl.loadLanguage(lang as never)
  loadedLangs.add(lang)
}

export type TokenizedLine = ThemedToken[]

/**
 * Tokenize a block of code into per-line token arrays.
 *
 * @param code   Source text to tokenize.
 * @param lang   Shiki language id.
 * @param theme  Shiki theme id. Themes load on demand with the tokenization path.
 * @returns Per-line token arrays, or `null` if tokenization fails.
 */
export async function tokenizeCode(
  code: string,
  lang: string,
  theme: string = DEFAULT_SHIKI_THEME,
): Promise<TokenizedLine[] | null> {
  try {
    const hl = await getHighlighter()
    await ensureLanguage(hl, lang)
    await ensureTheme(hl, theme)

    const { tokens } = hl.codeToTokens(code, {
      lang: lang as never,
      theme,
    })
    return tokens
  } catch {
    return null
  }
}
