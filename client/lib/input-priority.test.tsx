/** @vitest-environment jsdom */

import { render, screen } from '@testing-library/react'
import { act } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import {
  createSearchIndex,
  filterSearchIndex,
  normalizeFilterQuery,
  useDebouncedValue,
} from './input-priority'

afterEach(() => {
  vi.useRealTimers()
})

function DebouncedProbe({ value }: { value: string }) {
  const debounced = useDebouncedValue(value, 250)
  return <output aria-label="debounced">{debounced}</output>
}

describe('input priority helpers', () => {
  it('debounces committed values while preserving immediate caller state', () => {
    vi.useFakeTimers()
    const { rerender } = render(<DebouncedProbe value="draft" />)

    expect(screen.getByLabelText('debounced')).toHaveTextContent('draft')

    rerender(<DebouncedProbe value="drafting" />)
    rerender(<DebouncedProbe value="drafting fast" />)

    expect(screen.getByLabelText('debounced')).toHaveTextContent('draft')

    act(() => {
      vi.advanceTimersByTime(249)
    })

    expect(screen.getByLabelText('debounced')).toHaveTextContent('draft')

    act(() => {
      vi.advanceTimersByTime(1)
    })

    expect(screen.getByLabelText('debounced')).toHaveTextContent('drafting fast')
  })

  it('builds a reusable lowercase search index for large filter derivations', () => {
    const entries = [
      { id: 'one', name: 'Release Helper', source: 'Project' },
      { id: 'two', name: 'Deploy Guard', source: 'GitHub' },
      { id: 'three', name: 'Trace Collector', source: 'Local' },
    ]
    const index = createSearchIndex(entries, (entry) => [entry.name, entry.source])

    expect(normalizeFilterQuery('  RELEASE  ')).toBe('release')
    expect(filterSearchIndex(index, 'project').map((entry) => entry.id)).toEqual(['one'])
    expect(filterSearchIndex(index, 'guard').map((entry) => entry.id)).toEqual(['two'])
    expect(filterSearchIndex(index, '', (entry) => entry.source !== 'Local').map((entry) => entry.id)).toEqual([
      'one',
      'two',
    ])
  })
})
