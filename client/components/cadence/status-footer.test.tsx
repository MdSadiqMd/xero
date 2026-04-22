import { render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { StatusFooter } from './status-footer'

describe('StatusFooter', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-04-22T18:02:16Z'))
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('renders live last-commit hash, message, and relative time when provided', () => {
    render(
      <StatusFooter
        git={{
          branch: 'main',
          hasChanges: true,
          changedFiles: 1,
          lastCommit: {
            sha: 'c3e529f1c4e2a7d0d4cf759f9080e7f507dc9f4a',
            message: 'feat: wire live commit metadata',
            committedAt: '2026-04-22T18:00:16Z',
          },
        }}
        runtime={{
          provider: 'OpenAI Codex',
          state: 'idle',
        }}
      />,
    )

    expect(screen.getByText('c3e529f')).toBeVisible()
    expect(screen.getByText('feat: wire live commit metadata')).toBeVisible()
    expect(screen.getByText(/2 minutes ago/)).toBeVisible()
  })
})
