import type { ReactNode } from 'react'

import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import { instantiateBlankWorkflow } from '@/src/lib/xero-model/workflow-templates'

const fitViewSpy = vi.hoisted(() => vi.fn())

vi.mock('@xyflow/react', () => ({
  ControlButton: ({
    children,
    ...props
  }: {
    children: ReactNode
    [key: string]: unknown
  }) => <button type="button" {...props}>{children}</button>,
  Controls: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  Handle: () => null,
  MarkerType: { ArrowClosed: 'arrowclosed' },
  Position: { Left: 'left', Right: 'right' },
  ReactFlowProvider: ({ children }: { children: ReactNode }) => <>{children}</>,
  ReactFlow: ({
    children,
    nodes,
    fitViewOptions,
  }: {
    children: ReactNode
    nodes: unknown[]
    fitViewOptions?: { maxZoom?: number }
  }) => (
    <div
      data-testid="workflow-react-flow"
      data-node-count={nodes.length}
      data-fit-max-zoom={fitViewOptions?.maxZoom ?? ''}
    >
      {children}
    </div>
  ),
  useReactFlow: () => ({
    fitView: fitViewSpy,
    getViewport: () => ({ x: 0, y: 0, zoom: 1 }),
    zoomIn: vi.fn(),
    zoomOut: vi.fn(),
  }),
  useOnViewportChange: () => undefined,
}))

import { WorkflowDefinitionCanvas } from './workflow-definition-canvas'

describe('WorkflowDefinitionCanvas', () => {
  it('opens blank workflow drafts as an empty canvas with capped fit zoom', () => {
    const definition = instantiateBlankWorkflow({ projectId: 'project-1' })

    render(
      <WorkflowDefinitionCanvas
        definition={definition}
        initialMode="edit"
        isCreating
        onSaveDefinition={vi.fn()}
      />,
    )

    expect(screen.getByTestId('workflow-react-flow')).toHaveAttribute('data-node-count', '0')
    expect(screen.getByTestId('workflow-react-flow')).toHaveAttribute('data-fit-max-zoom', '0.85')
    expect(screen.queryByText('Done')).toBeNull()
    expect(screen.getByText('Build a blank workflow')).toBeInTheDocument()
    expect(screen.getByText('Add agent')).toBeInTheDocument()
    expect(screen.getByText('Add router')).toBeInTheDocument()
  })

  it('makes the first blank-canvas node the workflow start node', async () => {
    const onCanvasStatusChange = vi.fn()
    const definition = instantiateBlankWorkflow({ projectId: 'project-1' })

    render(
      <WorkflowDefinitionCanvas
        definition={definition}
        initialMode="edit"
        isCreating
        onSaveDefinition={vi.fn()}
        onCanvasStatusChange={onCanvasStatusChange}
      />,
    )

    const addAgentButton = screen.getByText('Add agent').closest('button')
    expect(addAgentButton).not.toBeNull()
    fireEvent.click(addAgentButton as HTMLButtonElement)

    await waitFor(() =>
      expect(screen.getByTestId('workflow-react-flow')).toHaveAttribute('data-node-count', '1'),
    )
    await waitFor(() => {
      const latestStatus = onCanvasStatusChange.mock.calls
        .map((call) => call[0])
        .filter(Boolean)
        .at(-1)
      expect(latestStatus.definition.startNodeId).toBe('agent')
    })
  })
})
