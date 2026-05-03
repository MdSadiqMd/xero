export interface TrimRecordCacheToByteBudgetOptions<T> {
  estimateBytes: (value: T, key: string) => number
  maxBytes: number
  maxEntries: number
  protectedKeys?: Iterable<string | null | undefined>
}

export interface TrimRecordCacheToByteBudgetResult<T> {
  evictedKeys: string[]
  records: Record<string, T>
  retainedBytes: number
}

export function trimRecordCacheToByteBudget<T>(
  records: Record<string, T>,
  options: TrimRecordCacheToByteBudgetOptions<T>,
): TrimRecordCacheToByteBudgetResult<T> {
  const protectedKeys = new Set(
    Array.from(options.protectedKeys ?? []).filter((key): key is string => Boolean(key)),
  )
  const keys = Object.keys(records)
  const entryBytes = new Map<string, number>()
  let retainedBytes = 0

  for (const key of keys) {
    const bytes = Math.max(0, Math.ceil(options.estimateBytes(records[key], key)))
    entryBytes.set(key, bytes)
    retainedBytes += bytes
  }

  const evictedKeys: string[] = []
  const shouldTrim = () =>
    keys.length - evictedKeys.length > options.maxEntries || retainedBytes > options.maxBytes

  for (const key of keys) {
    if (!shouldTrim()) {
      break
    }
    if (protectedKeys.has(key)) {
      continue
    }

    evictedKeys.push(key)
    retainedBytes = Math.max(0, retainedBytes - (entryBytes.get(key) ?? 0))
  }

  if (evictedKeys.length === 0) {
    return {
      evictedKeys,
      records,
      retainedBytes,
    }
  }

  const evicted = new Set(evictedKeys)
  const nextRecords: Record<string, T> = {}
  for (const key of keys) {
    if (!evicted.has(key)) {
      nextRecords[key] = records[key]
    }
  }

  return {
    evictedKeys,
    records: nextRecords,
    retainedBytes,
  }
}
