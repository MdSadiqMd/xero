import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'

export function loadRootDotenv(repoRoot, baseEnv = process.env) {
  const dotenvPath = resolve(repoRoot, '.env')
  if (!existsSync(dotenvPath)) return { ...baseEnv }

  const parsed = parseDotenv(readFileSync(dotenvPath, 'utf8'))
  return {
    ...parsed,
    ...baseEnv,
  }
}

export function parseDotenv(text) {
  const env = {}
  for (const rawLine of text.split(/\r?\n/)) {
    const line = rawLine.trim()
    if (!line || line.startsWith('#')) continue
    const separatorIndex = line.indexOf('=')
    if (separatorIndex <= 0) continue

    const key = line.slice(0, separatorIndex).trim()
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(key)) continue

    let value = line.slice(separatorIndex + 1).trim()
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1)
    } else {
      const commentIndex = value.search(/[ \t]#/)
      if (commentIndex >= 0) value = value.slice(0, commentIndex).trimEnd()
    }
    env[key] = value
  }
  return env
}
