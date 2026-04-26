import type { ComponentProps } from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import { AgentSessionsSidebar } from './agent-sessions-sidebar'
import type { AgentSessionView } from '@/src/lib/cadence-model'

const sessions: AgentSessionView[] = [
  {
    projectId: 'project-1',
    agentSessionId: 'agent-session-main',
    title: 'Main session',
    summary: 'Primary project session',
    status: 'active',
    statusLabel: 'Active',
    selected: true,
    createdAt: '2026-04-15T20:00:00Z',
    updatedAt: '2026-04-15T20:00:00Z',
    archivedAt: null,
    lastRunId: null,
    lastRuntimeKind: null,
    lastProviderId: null,
    isActive: true,
    isArchived: false,
  },
]

function renderSidebar(overrides: Partial<ComponentProps<typeof AgentSessionsSidebar>> = {}) {
  return render(
    <AgentSessionsSidebar
      projectId="project-1"
      projectLabel="Cadence"
      sessions={sessions}
      selectedSessionId="agent-session-main"
      onSelectSession={vi.fn()}
      onCreateSession={vi.fn()}
      onArchiveSession={vi.fn()}
      onOpenArchivedSessions={vi.fn()}
      {...overrides}
    />,
  )
}

describe('AgentSessionsSidebar', () => {
  it('resizes from the separator and persists the width', () => {
    const { container } = renderSidebar()

    expect(screen.getByText('Main session')).toBeVisible()

    const sidebar = container.querySelector('aside') as HTMLElement
    const separator = screen.getByRole('separator', { name: 'Resize sessions sidebar' })
    const before = Number.parseInt(sidebar.style.width, 10)

    fireEvent.keyDown(separator, { key: 'ArrowRight' })

    const after = Number.parseInt(sidebar.style.width, 10)
    expect(after).toBeGreaterThan(before)
    expect(window.localStorage.getItem('cadence.agentSessions.width')).toBe(String(after))
  })

  it('keeps the resize control hidden while collapsed', () => {
    const { container } = renderSidebar({ collapsed: true })

    const sidebar = container.querySelector('aside') as HTMLElement
    expect(sidebar.style.width).toBe('0px')
    expect(sidebar).toHaveAttribute('aria-hidden', 'true')
    expect(screen.queryByRole('separator', { name: 'Resize sessions sidebar' })).not.toBeInTheDocument()
  })
})
