import type { ComponentProps, ReactNode } from 'react'
import { fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { AgentDockSidebar } from './agent-dock-sidebar'
import { createXeroHighChurnStore } from '@/src/features/xero/use-xero-desktop-state/high-churn-store'
import type { AgentSessionView } from '@/src/lib/xero-model'

vi.mock('@/components/xero/agent-runtime/live-agent-runtime', () => ({
  LiveAgentRuntimeView: ({ agent }: { agent: unknown }) =>
    agent ? <div data-testid="live-agent-runtime">runtime</div> : null,
}))

// Radix DropdownMenu in jsdom uses pointer-capture APIs that don't behave the
// way fireEvent simulates them, which makes menu-item click handlers flaky.
// Replace it with plain elements so the test can assert the wiring directly.
vi.mock('@/components/ui/dropdown-menu', () => ({
  DropdownMenu: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  DropdownMenuTrigger: ({ children }: { asChild?: boolean; children: ReactNode }) => <>{children}</>,
  DropdownMenuContent: ({ children }: { children: ReactNode }) => (
    <div role="menu">{children}</div>
  ),
  DropdownMenuItem: ({
    children,
    onSelect,
    disabled,
  }: {
    children: ReactNode
    onSelect?: (event: { preventDefault: () => void }) => void
    disabled?: boolean
  }) => (
    <button
      disabled={disabled}
      onClick={() => onSelect?.({ preventDefault: () => undefined })}
      role="menuitem"
      type="button"
    >
      {children}
    </button>
  ),
  DropdownMenuSeparator: () => <hr />,
}))

const sessions: AgentSessionView[] = [
  {
    projectId: 'project-1',
    agentSessionId: 'session-a',
    title: 'First session',
    summary: '',
    status: 'active',
    statusLabel: 'Active',
    selected: true,
    createdAt: '2026-04-15T20:00:00Z',
    updatedAt: '2026-04-15T20:00:00Z',
    archivedAt: null,
    lastRunId: null,
    lastRuntimeKind: null,
    lastProviderId: null,
    lineage: null,
    isActive: true,
    isArchived: false,
  },
  {
    projectId: 'project-1',
    agentSessionId: 'session-b',
    title: 'Second session',
    summary: '',
    status: 'active',
    statusLabel: 'Active',
    selected: false,
    createdAt: '2026-04-16T20:00:00Z',
    updatedAt: '2026-04-16T20:00:00Z',
    archivedAt: null,
    lastRunId: null,
    lastRuntimeKind: null,
    lastProviderId: null,
    lineage: null,
    isActive: true,
    isArchived: false,
  },
]

const dummyAgent = {
  project: {
    id: 'project-1',
    selectedAgentSessionId: 'session-a',
    selectedAgentSession: sessions[0],
    agentSessions: sessions,
  },
} as unknown as ComponentProps<typeof AgentDockSidebar>['agent']

function renderDock(
  overrides: Partial<ComponentProps<typeof AgentDockSidebar>> = {},
) {
  const highChurnStore = createXeroHighChurnStore()
  return render(
    <AgentDockSidebar
      open
      agent={dummyAgent}
      highChurnStore={highChurnStore}
      sessions={sessions}
      selectedSessionId="session-a"
      onClose={vi.fn()}
      onSelectSession={vi.fn()}
      onCreateSession={vi.fn()}
      {...overrides}
    />,
  )
}

describe('AgentDockSidebar', () => {
  afterEach(() => {
    window.localStorage.clear()
  })

  it('renders the live agent runtime when open with an agent', () => {
    renderDock()
    expect(screen.getByTestId('live-agent-runtime')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Switch agent session' })).toHaveTextContent(
      'First session',
    )
  })

  it('shows the empty state when no agent is available', () => {
    renderDock({ agent: null })
    expect(screen.queryByTestId('live-agent-runtime')).not.toBeInTheDocument()
    expect(screen.getByText('No active session')).toBeVisible()
    expect(screen.getByRole('button', { name: /New session/ })).toBeVisible()
  })

  it('triggers onCreateSession when "New session" is selected from the dropdown', () => {
    const onCreateSession = vi.fn()
    renderDock({ onCreateSession })

    fireEvent.click(screen.getByRole('menuitem', { name: 'New session' }))

    expect(onCreateSession).toHaveBeenCalledTimes(1)
  })

  it('triggers onSelectSession when a sibling session is chosen', () => {
    const onSelectSession = vi.fn()
    renderDock({ onSelectSession })

    fireEvent.click(screen.getByRole('menuitem', { name: /Second session/ }))

    expect(onSelectSession).toHaveBeenCalledWith('session-b')
  })

  it('calls onClose when the close button is clicked', () => {
    const onClose = vi.fn()
    renderDock({ onClose })

    fireEvent.click(screen.getByRole('button', { name: 'Close agent dock' }))

    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('hides the sidebar (width 0, aria-hidden) when closed', () => {
    renderDock({ open: false })

    const aside = screen.getByLabelText('Agent dock')
    expect(aside.getAttribute('aria-hidden')).toBe('true')
    expect((aside as HTMLElement).style.width).toBe('0px')
  })
})
