import { useDeferredValue, useEffect, useMemo, useState } from 'react'

export function normalizeFilterQuery(query: string): string {
  return query.trim().toLowerCase()
}

export function useDebouncedValue<T>(value: T, delayMs: number): T {
  const [debounced, setDebounced] = useState(value)

  useEffect(() => {
    const timeout = window.setTimeout(() => setDebounced(value), delayMs)
    return () => window.clearTimeout(timeout)
  }, [delayMs, value])

  return debounced
}

export function useDeferredFilterQuery(query: string): string {
  const deferredQuery = useDeferredValue(query)
  return useMemo(() => normalizeFilterQuery(deferredQuery), [deferredQuery])
}

export interface SearchIndexEntry<T> {
  item: T
  haystack: string
}

export function createSearchIndex<T>(
  items: ReadonlyArray<T>,
  getParts: (item: T) => ReadonlyArray<string | null | undefined>,
): SearchIndexEntry<T>[] {
  const index: SearchIndexEntry<T>[] = []

  for (const item of items) {
    index.push({
      item,
      haystack: getParts(item)
        .filter((value): value is string => typeof value === 'string' && value.length > 0)
        .join('\u0000')
        .toLowerCase(),
    })
  }

  return index
}

export function filterSearchIndex<T>(
  index: ReadonlyArray<SearchIndexEntry<T>>,
  query: string,
  predicate?: (item: T) => boolean,
): T[] {
  const normalizedQuery = normalizeFilterQuery(query)
  const filtered: T[] = []

  for (const entry of index) {
    if (predicate && !predicate(entry.item)) {
      continue
    }
    if (normalizedQuery && !entry.haystack.includes(normalizedQuery)) {
      continue
    }
    filtered.push(entry.item)
  }

  return filtered
}
