import { describe, expect, it } from 'vitest'

import { trimRecordCacheToByteBudget } from './memory-budget'

describe('memory budget helpers', () => {
  it('evicts oldest unprotected records until entry and byte budgets fit', () => {
    const result = trimRecordCacheToByteBudget(
      {
        first: 'a'.repeat(80),
        second: 'b'.repeat(80),
        third: 'c'.repeat(80),
      },
      {
        estimateBytes: (value) => value.length,
        maxBytes: 160,
        maxEntries: 2,
        protectedKeys: ['second'],
      },
    )

    expect(result.evictedKeys).toEqual(['first'])
    expect(Object.keys(result.records)).toEqual(['second', 'third'])
    expect(result.retainedBytes).toBe(160)
  })

  it('keeps protected records even when they exceed the requested byte ceiling', () => {
    const result = trimRecordCacheToByteBudget(
      {
        active: 'a'.repeat(200),
        stale: 'b'.repeat(80),
      },
      {
        estimateBytes: (value) => value.length,
        maxBytes: 100,
        maxEntries: 1,
        protectedKeys: ['active'],
      },
    )

    expect(result.evictedKeys).toEqual(['stale'])
    expect(Object.keys(result.records)).toEqual(['active'])
    expect(result.retainedBytes).toBe(200)
  })
})
