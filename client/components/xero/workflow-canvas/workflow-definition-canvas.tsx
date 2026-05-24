'use client'

import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import {
  Handle,
  MarkerType,
  Position,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type Connection,
  type Edge,
  type EdgeTypes,
  type Node,
  type NodeChange,
  type NodeProps,
} from '@xyflow/react'
import {
  Bot,
  CheckCircle2,
  Flag,
  GitBranch,
  GitMerge,
  Loader2,
  Lock,
  PauseCircle,
  Play,
  Plus,
  RotateCcw,
  Route,
  ShieldCheck,
  SkipForward,
  Trash2,
  Workflow,
  X,
  type LucideIcon,
} from 'lucide-react'

import '@xyflow/react/dist/style.css'
import './agent-visualization.css'

import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Textarea } from '@/components/ui/textarea'
import { cn } from '@/lib/utils'
import {
  validateWorkflowDefinition,
  type WorkflowConditionDto,
  type WorkflowDefinitionDto,
  type WorkflowEdgeDto,
  type WorkflowEdgeTypeDto,
  type WorkflowNodeDto,
  type WorkflowNodeRunStatusDto,
  type WorkflowOutputExtractionDto,
  type WorkflowRunStatusDto,
  type WorkflowTerminalStatusDto,
} from '@/src/lib/xero-model/workflow-definition'
import type {
  WorkflowArtifactRecordDto,
  WorkflowEventDto,
  WorkflowRunDto,
  WorkflowRunEdgeDecisionDto,
  WorkflowRunNodeDto,
} from '@/src/lib/xero-model/workflow-run'
import {
  agentRefKey,
  type AgentRefDto,
  type WorkflowAgentSummaryDto,
} from '@/src/lib/xero-model/workflow-agents'
import {
  AgentCanvasControls,
  AgentCanvasDots,
  AGENT_CANVAS_EMPTY_VIEWPORT,
  AGENT_CANVAS_SNAP_GRID,
  type AgentCanvasControlItem,
} from './canvas-shell'
import { PhaseBranchEdge } from './edges/phase-branch-edge'
import { CanvasNodeCard } from './nodes/canvas-node-card'

type WorkflowCanvasMode = 'view' | 'edit'
type Selection =
  | { kind: 'node'; id: string }
  | { kind: 'edge'; id: string }
  | null

export interface WorkflowDefinitionCanvasStatus {
  editing: boolean
  saving: boolean
  runningAction: boolean
  saveDisabled: boolean
  diagnosticCount: number
  diagnostics: ReadonlyArray<{ message: string; path: string }>
  errorMessage: string | null
  definition: WorkflowDefinitionDto
  run: WorkflowRunDto | null
  updateName: (value: string) => void
  updateDescription: (value: string) => void
  edit: () => void
  save: () => void
  cancel: () => void
  start: () => void
  cancelRun: (() => void) | null
  retryNodeRun: ((nodeRunId: string) => void) | null
  skipBranch: ((nodeRunId: string) => void) | null
}

interface WorkflowDefinitionCanvasProps {
  active?: boolean
  definition: WorkflowDefinitionDto
  run?: WorkflowRunDto | null
  agents?: readonly WorkflowAgentSummaryDto[]
  initialMode?: WorkflowCanvasMode
  isCreating?: boolean
  saving?: boolean
  runningAction?: boolean
  onSaveDefinition?: (definition: WorkflowDefinitionDto) => Promise<WorkflowDefinitionDto | void>
  onCancelEditing?: () => void
  onCanvasStatusChange?: (status: WorkflowDefinitionCanvasStatus | null) => void
  onStartRun?: (workflowId: string, initialInput: unknown) => Promise<WorkflowRunDto | void>
  onCancelRun?: (runId: string) => Promise<WorkflowRunDto | void>
  onRetryNodeRun?: (runId: string, nodeRunId: string) => Promise<WorkflowRunDto | void>
  onSkipBranch?: (
    runId: string,
    nodeRunId: string,
    reason?: string,
  ) => Promise<WorkflowRunDto | void>
  onResumeCheckpoint?: (
    runId: string,
    nodeRunId: string,
    decision: string,
    payload: unknown,
  ) => Promise<WorkflowRunDto | void>
  onCreateAgent?: () => void
  onEditAgent?: (ref: AgentRefDto) => void
}

interface WorkflowNodeData extends Record<string, unknown> {
  node: WorkflowNodeDto
  runNode: WorkflowRunNodeDto | null
  artifact: WorkflowArtifactRecordDto | null
  agentLabel: string | null
  isStart: boolean
  incomingCount: number
  outgoingCount: number
}

type WorkflowReactNode = Node<WorkflowNodeData, 'workflowNode'>
type WorkflowReactEdge = Edge<{ workflowEdge: WorkflowEdgeDto; label: string }>

const NODE_TYPES = {
  workflowNode: WorkflowNodeCard,
}
const EDGE_TYPES = {
  'workflow-branch': PhaseBranchEdge,
} as unknown as EdgeTypes

const FIT_VIEW_OPTIONS = { padding: 0.2, includeHiddenNodes: false, maxZoom: 0.85 } as const
const NODE_KIND_LABEL: Record<WorkflowNodeDto['type'], string> = {
  agent: 'Agent',
  router: 'Router',
  gate: 'Gate',
  human_checkpoint: 'Checkpoint',
  merge: 'Merge',
  terminal: 'Terminal',
}
const EDGE_TYPE_LABEL: Record<WorkflowEdgeTypeDto, string> = {
  success: 'Success',
  failure: 'Failure',
  conditional: 'If',
  loop: 'Loop',
  recovery: 'Recovery',
  manual_override: 'Manual',
}
const NODE_STATUS_TONE: Record<WorkflowNodeRunStatusDto, string> = {
  pending: 'border-muted-foreground/20 bg-muted/45 text-muted-foreground',
  eligible: 'border-sky-500/30 bg-sky-500/10 text-sky-700 dark:text-sky-300',
  starting: 'border-sky-500/30 bg-sky-500/10 text-sky-700 dark:text-sky-300',
  running: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300',
  waiting_on_gate: 'border-amber-500/35 bg-amber-500/10 text-amber-700 dark:text-amber-300',
  succeeded: 'border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300',
  failed: 'border-destructive/35 bg-destructive/10 text-destructive',
  stalled: 'border-orange-500/35 bg-orange-500/10 text-orange-700 dark:text-orange-300',
  skipped: 'border-muted-foreground/20 bg-muted/45 text-muted-foreground',
  cancelled: 'border-muted-foreground/20 bg-muted/45 text-muted-foreground',
}
const WORKFLOW_CONTROL_ENTRIES: {
  type: WorkflowNodeDto['type']
  label: string
  icon: LucideIcon
}[] = [
  { type: 'agent', label: 'Add agent', icon: Bot },
  { type: 'router', label: 'Add router', icon: Route },
  { type: 'gate', label: 'Add gate', icon: ShieldCheck },
  { type: 'human_checkpoint', label: 'Add checkpoint', icon: PauseCircle },
  { type: 'merge', label: 'Add merge', icon: GitMerge },
  { type: 'terminal', label: 'Add terminal', icon: CheckCircle2 },
]

export function WorkflowDefinitionCanvas(props: WorkflowDefinitionCanvasProps) {
  return (
    <ReactFlowProvider>
      <WorkflowDefinitionCanvasInner {...props} />
    </ReactFlowProvider>
  )
}

function WorkflowDefinitionCanvasInner({
  active = true,
  definition,
  run = null,
  agents = [],
  initialMode = 'view',
  isCreating = false,
  saving = false,
  runningAction = false,
  onSaveDefinition,
  onCancelEditing,
  onCanvasStatusChange,
  onStartRun,
  onCancelRun,
  onRetryNodeRun,
  onSkipBranch,
  onResumeCheckpoint,
  onCreateAgent,
  onEditAgent,
}: WorkflowDefinitionCanvasProps) {
  const reactFlow = useReactFlow()
  const [mode, setMode] = useState<WorkflowCanvasMode>(initialMode)
  const [draft, setDraft] = useState<WorkflowDefinitionDto>(() => cloneDefinition(definition))
  const [selection, setSelection] = useState<Selection>(null)
  const [startDialogOpen, setStartDialogOpen] = useState(false)
  const [startGoal, setStartGoal] = useState('')
  const [localError, setLocalError] = useState<string | null>(null)
  const [canvasLocked, setCanvasLocked] = useState(false)
  const [snapToGrid, setSnapToGrid] = useState(true)
  const [entering, setEntering] = useState(true)
  const [dragging, setDragging] = useState(false)
  const editable = mode === 'edit'
  const canvasInteractionsLocked = editable && canvasLocked

  useEffect(() => {
    setMode(initialMode)
    setDraft(cloneDefinition(definition))
    setSelection(null)
    setLocalError(null)
  }, [definition.id, definition.version, definition.updatedAt, initialMode])

  useEffect(() => {
    setEntering(true)
    let secondFrame: number | null = null
    const firstFrame = window.requestAnimationFrame(() => {
      secondFrame = window.requestAnimationFrame(() => setEntering(false))
    })
    return () => {
      window.cancelAnimationFrame(firstFrame)
      if (secondFrame !== null) window.cancelAnimationFrame(secondFrame)
    }
  }, [definition.id, isCreating])

  const effectiveDefinition = editable ? draft : definition
  const validation = useMemo(
    () => validateWorkflowDefinition(effectiveDefinition),
    [effectiveDefinition],
  )
  const selectedNode = selection?.kind === 'node'
    ? effectiveDefinition.nodes.find((node) => node.id === selection.id) ?? null
    : null
  const selectedEdge = selection?.kind === 'edge'
    ? effectiveDefinition.edges.find((edge) => edge.id === selection.id) ?? null
    : null
  const latestRunNodeByNodeId = useMemo(() => latestRunNodesByNodeId(run), [run])
  const latestArtifactByNodeId = useMemo(() => latestArtifactsByNodeId(run), [run])
  const matchedEdgeIds = useMemo(
    () => new Set((run?.edgeDecisions ?? []).map((decision) => decision.edgeId)),
    [run?.edgeDecisions],
  )
  const agentOptions = useMemo(
    () => agents.map((agent) => ({ agent, key: agentRefKey(agent.ref) })),
    [agents],
  )
  const nodes = useMemo<WorkflowReactNode[]>(
    () =>
      effectiveDefinition.nodes.map((node) => {
        const incomingCount = effectiveDefinition.edges.filter(
          (edge) => edge.toNodeId === node.id,
        ).length
        const outgoingCount = effectiveDefinition.edges.filter(
          (edge) => edge.fromNodeId === node.id,
        ).length
        return {
          id: node.id,
          type: 'workflowNode',
          position: node.position,
          data: {
            node,
            runNode: latestRunNodeByNodeId.get(node.id) ?? null,
            artifact: latestArtifactByNodeId.get(node.id) ?? null,
            agentLabel: node.type === 'agent' ? labelForAgentRef(node.agentRef, agents) : null,
            isStart: node.id === effectiveDefinition.startNodeId,
            incomingCount,
            outgoingCount,
          },
          draggable: editable && !canvasInteractionsLocked,
          selectable: true,
          selected: selection?.kind === 'node' && selection.id === node.id,
        }
      }),
    [
      agents,
      canvasInteractionsLocked,
      editable,
      effectiveDefinition.edges,
      effectiveDefinition.nodes,
      latestArtifactByNodeId,
      latestRunNodeByNodeId,
      selection,
    ],
  )
  const edges = useMemo<WorkflowReactEdge[]>(
    () =>
      effectiveDefinition.edges.map((edge) => ({
        id: edge.id,
        source: edge.fromNodeId,
        target: edge.toNodeId,
        type: 'workflow-branch',
        markerEnd: { type: MarkerType.ArrowClosed },
        data: { workflowEdge: edge, label: edge.label || EDGE_TYPE_LABEL[edge.type] },
        animated: run?.status === 'running' && matchedEdgeIds.has(edge.id),
        className: cn(
          'workflow-definition-edge',
          'agent-edge-phase-branch',
          matchedEdgeIds.has(edge.id) && 'workflow-definition-edge--matched',
          edge.type === 'loop' && 'workflow-definition-edge--loop',
          edge.type === 'recovery' && 'workflow-definition-edge--recovery',
        ),
        style: matchedEdgeIds.has(edge.id)
          ? { strokeWidth: 2.2, stroke: 'hsl(var(--primary))' }
          : undefined,
      })),
    [effectiveDefinition.edges, matchedEdgeIds, run?.status],
  )

  const canSave = editable && validation.status === 'valid' && Boolean(onSaveDefinition)
  const waitingCheckpoint = useMemo(() => {
    if (!run || run.status !== 'paused') return null
    return run.nodes.find((node) => node.status === 'waiting_on_gate') ?? null
  }, [run])

  const updateDefinition = useCallback((updater: (current: WorkflowDefinitionDto) => WorkflowDefinitionDto) => {
    setDraft((current) => updater(cloneDefinition(current)))
  }, [])

  const updateSelectedNode = useCallback(
    (updater: (node: WorkflowNodeDto) => WorkflowNodeDto) => {
      if (!selectedNode) return
      updateDefinition((current) => ({
        ...current,
        nodes: current.nodes.map((node) => (node.id === selectedNode.id ? updater(node) : node)),
      }))
    },
    [selectedNode, updateDefinition],
  )

  const updateSelectedEdge = useCallback(
    (updater: (edge: WorkflowEdgeDto) => WorkflowEdgeDto) => {
      if (!selectedEdge) return
      updateDefinition((current) => ({
        ...current,
        edges: current.edges.map((edge) => (edge.id === selectedEdge.id ? updater(edge) : edge)),
      }))
    },
    [selectedEdge, updateDefinition],
  )

  const handleNodeDragStop = useCallback(
    (_event: unknown, node: WorkflowReactNode) => {
      setDragging(false)
      if (!editable || canvasInteractionsLocked) return
      updateDefinition((current) => ({
        ...current,
        nodes: current.nodes.map((entry) =>
          entry.id === node.id ? { ...entry, position: node.position } : entry,
        ),
      }))
    },
    [canvasInteractionsLocked, editable, updateDefinition],
  )

  const handleNodesChange = useCallback(
    (changes: NodeChange<WorkflowReactNode>[]) => {
      if (!editable || canvasInteractionsLocked) return
      const moved = new Map<string, { x: number; y: number }>()
      for (const change of changes) {
        if (change.type !== 'position' || !change.position) continue
        moved.set(change.id, change.position)
      }
      if (moved.size === 0) return
      updateDefinition((current) => ({
        ...current,
        nodes: current.nodes.map((node) => {
          const position = moved.get(node.id)
          return position ? { ...node, position } : node
        }),
      }))
    },
    [canvasInteractionsLocked, editable, updateDefinition],
  )

  const handleConnect = useCallback(
    (connection: Connection) => {
      if (!editable || canvasInteractionsLocked || !connection.source || !connection.target) return
      updateDefinition((current) => {
        const id = uniqueId('edge', current.edges.map((edge) => edge.id))
        return {
          ...current,
          edges: [
            ...current.edges,
            {
              id,
              fromNodeId: connection.source ?? '',
              toNodeId: connection.target ?? '',
              type: 'success',
              label: '',
              priority: 100,
              condition: { kind: 'always' },
              loopPolicy: null,
            },
          ],
        }
      })
    },
    [canvasInteractionsLocked, editable, updateDefinition],
  )

  const addNode = useCallback(
    (type: WorkflowNodeDto['type']) => {
      updateDefinition((current) => {
        const id = uniqueId(type.replace('_', '-'), current.nodes.map((node) => node.id))
        const offset = current.nodes.length * 36
        const node = createNode(type, id, agents[0]?.ref ?? {
          kind: 'built_in',
          runtimeAgentId: 'generalist',
          version: 1,
        }, { x: 140 + offset, y: 180 + offset })
        return {
          ...current,
          nodes: [...current.nodes, node],
          startNodeId: current.nodes.length === 0 ? id : current.startNodeId,
        }
      })
      setSelection(null)
    },
    [agents, updateDefinition],
  )

  const deleteSelected = useCallback(() => {
    if (!selection) return
    updateDefinition((current) => {
      if (selection.kind === 'edge') {
        return {
          ...current,
          edges: current.edges.filter((edge) => edge.id !== selection.id),
        }
      }
      const nextNodes = current.nodes.filter((node) => node.id !== selection.id)
      const nextEdges = current.edges.filter(
        (edge) => edge.fromNodeId !== selection.id && edge.toNodeId !== selection.id,
      )
      return {
        ...current,
        nodes: nextNodes,
        edges: nextEdges,
        startNodeId:
          current.startNodeId === selection.id ? nextNodes[0]?.id ?? '' : current.startNodeId,
      }
    })
    setSelection(null)
  }, [selection, updateDefinition])

  const handleSave = useCallback(async () => {
    if (!onSaveDefinition || validation.status !== 'valid') return
    setLocalError(null)
    try {
      const saved = await onSaveDefinition(draft)
      if (saved) setDraft(cloneDefinition(saved))
      setMode('view')
    } catch (error) {
      setLocalError(error instanceof Error ? error.message : 'Xero could not save the Workflow.')
    }
  }, [draft, onSaveDefinition, validation.status])

  const handleStart = useCallback(async () => {
    if (!onStartRun) return
    setLocalError(null)
    try {
      await onStartRun(definition.id, {
        goal: startGoal.trim(),
      })
      setStartDialogOpen(false)
      setStartGoal('')
    } catch (error) {
      setLocalError(error instanceof Error ? error.message : 'Xero could not start the Workflow.')
    }
  }, [definition.id, onStartRun, startGoal])

  const handleCancelRun = useCallback(async () => {
    if (!run || !onCancelRun) return
    setLocalError(null)
    try {
      await onCancelRun(run.id)
    } catch (error) {
      setLocalError(error instanceof Error ? error.message : 'Xero could not cancel the Workflow run.')
    }
  }, [onCancelRun, run])

  const handleRetryNodeRun = useCallback(
    async (nodeRunId: string) => {
      if (!run || !onRetryNodeRun) return
      setLocalError(null)
      try {
        await onRetryNodeRun(run.id, nodeRunId)
      } catch (error) {
        setLocalError(error instanceof Error ? error.message : 'Xero could not retry the Workflow node.')
      }
    },
    [onRetryNodeRun, run],
  )

  const handleSkipBranch = useCallback(
    async (nodeRunId: string) => {
      if (!run || !onSkipBranch) return
      setLocalError(null)
      try {
        await onSkipBranch(run.id, nodeRunId, 'Skipped from the Workflow canvas.')
      } catch (error) {
        setLocalError(error instanceof Error ? error.message : 'Xero could not skip the Workflow branch.')
      }
    },
    [onSkipBranch, run],
  )

  const handleResumeCheckpoint = useCallback(
    async (decision: string) => {
      if (!run || !waitingCheckpoint || !onResumeCheckpoint) return
      setLocalError(null)
      try {
        await onResumeCheckpoint(run.id, waitingCheckpoint.id, decision, { decision })
      } catch (error) {
        setLocalError(error instanceof Error ? error.message : 'Xero could not resume the Workflow.')
      }
    },
    [onResumeCheckpoint, run, waitingCheckpoint],
  )

  const handleEdit = useCallback(() => {
    setDraft(cloneDefinition(definition))
    setMode('edit')
  }, [definition])

  const handleCancelEditing = useCallback(() => {
    if (isCreating) {
      onCancelEditing?.()
      return
    }
    setDraft(cloneDefinition(definition))
    setMode('view')
    setSelection(null)
  }, [definition, isCreating, onCancelEditing])

  const updateWorkflowName = useCallback(
    (name: string) => updateDefinition((current) => ({ ...current, name })),
    [updateDefinition],
  )

  const updateWorkflowDescription = useCallback(
    (description: string) => updateDefinition((current) => ({ ...current, description })),
    [updateDefinition],
  )

  const fitWorkflowView = useCallback(() => {
    void reactFlow.fitView({ ...FIT_VIEW_OPTIONS, duration: 420 })
  }, [reactFlow])

  const handleResetLayout = useCallback(() => {
    updateDefinition(autoLayoutWorkflowDefinition)
    window.requestAnimationFrame(fitWorkflowView)
  }, [fitWorkflowView, updateDefinition])

  const workflowControlItems = useMemo<AgentCanvasControlItem[]>(
    () =>
      editable
        ? WORKFLOW_CONTROL_ENTRIES.map((entry) => {
            const Icon = entry.icon
            return {
              key: entry.type,
              label: entry.label,
              title: entry.label,
              disabled: canvasInteractionsLocked,
              onClick: () => addNode(entry.type),
              children: <Icon className="h-[18px] w-[18px]" aria-hidden="true" />,
            }
          })
        : [],
    [addNode, canvasInteractionsLocked, editable],
  )

  useEffect(() => {
    onCanvasStatusChange?.({
      editing: editable,
      saving,
      runningAction,
      saveDisabled: !canSave || saving,
      diagnosticCount: validation.diagnostics.length,
      diagnostics: validation.diagnostics,
      errorMessage: localError,
      definition: effectiveDefinition,
      run,
      updateName: updateWorkflowName,
      updateDescription: updateWorkflowDescription,
      edit: handleEdit,
      save: () => {
        void handleSave()
      },
      cancel: handleCancelEditing,
      start: () => setStartDialogOpen(true),
      cancelRun:
        run && isActiveRun(run.status) && onCancelRun
          ? () => {
              void handleCancelRun()
            }
          : null,
      retryNodeRun:
        run && onRetryNodeRun
          ? (nodeRunId: string) => {
              void handleRetryNodeRun(nodeRunId)
            }
          : null,
      skipBranch:
        run && onSkipBranch
          ? (nodeRunId: string) => {
              void handleSkipBranch(nodeRunId)
            }
          : null,
    })
    return () => onCanvasStatusChange?.(null)
  }, [
    canSave,
    editable,
    effectiveDefinition,
    handleCancelEditing,
    handleCancelRun,
    handleEdit,
    handleRetryNodeRun,
    handleSave,
    handleSkipBranch,
    localError,
    onCancelRun,
    onCanvasStatusChange,
    onRetryNodeRun,
    onSkipBranch,
    run,
    runningAction,
    saving,
    updateWorkflowDescription,
    updateWorkflowName,
    validation.diagnostics,
  ])

  return (
    <div
      className={cn(
        'agent-visualization relative h-full w-full overflow-hidden',
        canvasInteractionsLocked && 'is-locked',
        entering && 'is-workflow-entering',
        dragging && 'is-dragging',
        editable && 'is-editing',
        selection?.kind === 'node' && 'is-node-focused',
      )}
    >
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={NODE_TYPES}
        edgeTypes={EDGE_TYPES}
        fitView
        fitViewOptions={FIT_VIEW_OPTIONS}
        defaultViewport={AGENT_CANVAS_EMPTY_VIEWPORT}
        minZoom={0.2}
        maxZoom={2}
        nodesDraggable={editable && !canvasInteractionsLocked}
        nodesConnectable={editable && !canvasInteractionsLocked}
        elementsSelectable
        snapToGrid={snapToGrid}
        snapGrid={AGENT_CANVAS_SNAP_GRID}
        onNodesChange={handleNodesChange}
        onConnect={handleConnect}
        onNodeDragStart={() => setDragging(true)}
        onEdgeClick={(_, edge) => setSelection({ kind: 'edge', id: edge.id })}
        onNodeClick={(_, node) => setSelection({ kind: 'node', id: node.id })}
        onNodeDragStop={handleNodeDragStop}
        onPaneClick={() => setSelection(null)}
        proOptions={{ hideAttribution: true }}
      >
        <AgentCanvasDots />
        <AgentCanvasControls
          showLayoutControls={editable}
          layoutControlsDisabled={canvasInteractionsLocked}
          locked={canvasLocked}
          snapToGrid={snapToGrid}
          extraControls={workflowControlItems}
          onFitView={fitWorkflowView}
          onToggleLock={() => setCanvasLocked((current) => !current)}
          onToggleSnapToGrid={() => setSnapToGrid((current) => !current)}
          onResetLayout={handleResetLayout}
        />
      </ReactFlow>

      {editable && effectiveDefinition.nodes.length === 0 ? (
        <WorkflowDraftEmptyState
          onAddAgent={() => addNode('agent')}
          onAddRouter={() => addNode('router')}
          onAddCheckpoint={() => addNode('human_checkpoint')}
          onAddTerminal={() => addNode('terminal')}
          onCreateAgent={onCreateAgent}
        />
      ) : null}

      {editable && (selectedNode || selectedEdge) ? (
        <WorkflowPropertiesPanel
          agents={agentOptions}
          definition={effectiveDefinition}
          selectedNode={selectedNode}
          selectedEdge={selectedEdge}
          diagnostics={validation.diagnostics.filter((diagnostic) =>
            selection?.kind === 'node'
              ? diagnostic.path.includes(selection.id)
              : selection?.kind === 'edge'
                ? diagnostic.path.includes(selection.id)
                : false,
          )}
          onClose={() => setSelection(null)}
          onDelete={deleteSelected}
          onUpdateNode={updateSelectedNode}
          onUpdateEdge={updateSelectedEdge}
          onSetStartNode={(nodeId) =>
            updateDefinition((current) => ({ ...current, startNodeId: nodeId }))
          }
          onCreateAgent={onCreateAgent}
          onEditAgent={onEditAgent}
        />
      ) : !editable && (selectedNode || selectedEdge) ? (
        <WorkflowDetailsPanel
          node={selectedNode}
          edge={selectedEdge}
          runNode={selectedNode ? latestRunNodeByNodeId.get(selectedNode.id) ?? null : null}
          artifact={selectedNode ? latestArtifactByNodeId.get(selectedNode.id) ?? null : null}
          edgeDecision={selectedEdge ? latestEdgeDecision(run, selectedEdge.id) : null}
          events={run?.events ?? []}
          agentLabel={selectedNode?.type === 'agent' ? labelForAgentRef(selectedNode.agentRef, agents) : null}
          running={runningAction}
          onRetryNodeRun={onRetryNodeRun ? handleRetryNodeRun : null}
          onSkipBranch={onSkipBranch ? handleSkipBranch : null}
          onClose={() => setSelection(null)}
        />
      ) : null}

      {waitingCheckpoint && onResumeCheckpoint ? (
        <CheckpointResumeBar
          node={effectiveDefinition.nodes.find((node) => node.id === waitingCheckpoint.nodeId) ?? null}
          onResume={handleResumeCheckpoint}
          running={runningAction}
        />
      ) : null}

      <Dialog open={startDialogOpen} onOpenChange={setStartDialogOpen}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>Start {definition.name}</DialogTitle>
            <DialogDescription>
              The goal is passed into the Workflow as durable run input.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-2">
            <Label htmlFor="workflow-start-goal">Goal</Label>
            <Textarea
              id="workflow-start-goal"
              value={startGoal}
              onChange={(event) => setStartGoal(event.target.value)}
              placeholder="Describe the outcome this Workflow should produce."
              className="min-h-28"
            />
          </div>
          <DialogFooter>
            <Button type="button" variant="ghost" onClick={() => setStartDialogOpen(false)}>
              Cancel
            </Button>
            <Button type="button" onClick={() => void handleStart()} disabled={runningAction || startGoal.trim().length === 0}>
              {runningAction ? <Loader2 className="size-4 animate-spin" /> : <Play className="size-4" />}
              Start
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}

function WorkflowNodeCard({ data, selected }: NodeProps<WorkflowReactNode>) {
  const node = data.node
  const Icon = iconForNodeType(node.type)
  const status = data.runNode?.status ?? null
  const isStart = data.isStart
  return (
    <>
      <Handle
        type="target"
        position={Position.Left}
        className={handleClassForNode(node.type)}
      />
      <Handle
        type="source"
        position={Position.Right}
        className={handleClassForNode(node.type)}
      />
      <CanvasNodeCard
        title={node.title}
        subtitle={NODE_KIND_LABEL[node.type]}
        icon={Icon}
        tone={nodeTone(node.type)}
        iconClassName={nodeIconTone(node.type)}
        selected={selected}
        detail={
          node.type === 'agent'
            ? data.agentLabel ?? 'Choose an agent'
            : node.type === 'terminal'
              ? humanize(node.terminalStatus)
              : node.description || `${data.incomingCount} in · ${data.outgoingCount} out`
        }
        badges={
          <>
            {isStart ? (
              <Badge
                variant="outline"
                className="h-5 px-1.5 text-[9.5px] font-medium border-amber-500/40 bg-amber-500/12 text-amber-700 dark:text-amber-300"
              >
                <Flag className="mr-0.5 h-2.5 w-2.5" aria-hidden="true" />
                start
              </Badge>
            ) : null}
            {status ? (
              <Badge
                variant="outline"
                className={cn('h-5 px-1.5 text-[9.5px] font-medium', NODE_STATUS_TONE[status])}
              >
                {humanize(status)}
              </Badge>
            ) : null}
          </>
        }
        chips={
          <>
            {node.type === 'agent' ? (
              <Badge
                variant="outline"
                className="h-5 px-1.5 text-[9.5px] font-medium border-sky-500/30 bg-sky-500/10 text-sky-700 dark:text-sky-300"
              >
                {node.outputContract.artifactType}
              </Badge>
            ) : null}
            {data.artifact ? (
              <Badge
                variant="outline"
                className="h-5 px-1.5 text-[9.5px] font-medium border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300"
              >
                artifact
              </Badge>
            ) : null}
          </>
        }
      />
    </>
  )
}

function WorkflowDraftEmptyState({
  onAddAgent,
  onAddRouter,
  onAddCheckpoint,
  onAddTerminal,
  onCreateAgent,
}: {
  onAddAgent: () => void
  onAddRouter: () => void
  onAddCheckpoint: () => void
  onAddTerminal: () => void
  onCreateAgent?: () => void
}) {
  return (
    <div className="pointer-events-none absolute inset-0 z-[6] flex items-center justify-center px-6">
      <div
        className="pointer-events-auto flex w-full max-w-md flex-col items-center text-center"
        onPointerDown={(event) => event.stopPropagation()}
      >
        <div className="flex h-12 w-12 items-center justify-center rounded-2xl border border-border bg-card/80 shadow-sm">
          <Workflow className="h-6 w-6 text-foreground" aria-hidden="true" />
        </div>
        <h3 className="mt-5 text-[22px] font-semibold tracking-tight text-foreground">
          Build a blank workflow
        </h3>
        <p className="mt-2 max-w-sm text-[12.5px] leading-relaxed text-muted-foreground">
          Add the first node, then connect agents, routers, checkpoints, and terminals into a run path.
        </p>
        <div className="mt-6 grid w-full grid-cols-2 gap-2">
          <WorkflowDraftAction icon={Bot} label="Add agent" onClick={onAddAgent} />
          <WorkflowDraftAction icon={Route} label="Add router" onClick={onAddRouter} />
          <WorkflowDraftAction icon={PauseCircle} label="Add checkpoint" onClick={onAddCheckpoint} />
          <WorkflowDraftAction icon={CheckCircle2} label="Add terminal" onClick={onAddTerminal} />
        </div>
        {onCreateAgent ? (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="mt-3 text-muted-foreground hover:text-foreground"
            onClick={onCreateAgent}
          >
            <Plus className="h-3.5 w-3.5" />
            Create agent
          </Button>
        ) : null}
      </div>
    </div>
  )
}

function WorkflowDraftAction({
  icon: Icon,
  label,
  onClick,
}: {
  icon: LucideIcon
  label: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      className="group flex h-10 items-center gap-2 rounded-lg border border-border/70 bg-card/80 px-3 text-left text-[12px] font-medium text-foreground/85 shadow-sm transition-colors hover:border-primary/40 hover:bg-primary/[0.04] hover:text-foreground focus-visible:border-primary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/30"
      onClick={onClick}
    >
      <Icon className="h-3.5 w-3.5 shrink-0 text-muted-foreground transition-colors group-hover:text-primary" />
      <span className="min-w-0 truncate">{label}</span>
    </button>
  )
}

function WorkflowPropertiesPanel({
  agents,
  definition,
  selectedNode,
  selectedEdge,
  diagnostics,
  onClose,
  onDelete,
  onUpdateNode,
  onUpdateEdge,
  onSetStartNode,
  onCreateAgent,
  onEditAgent,
}: {
  agents: { agent: WorkflowAgentSummaryDto; key: string }[]
  definition: WorkflowDefinitionDto
  selectedNode: WorkflowNodeDto | null
  selectedEdge: WorkflowEdgeDto | null
  diagnostics: { message: string; path: string }[]
  onClose: () => void
  onDelete: () => void
  onUpdateNode: (updater: (node: WorkflowNodeDto) => WorkflowNodeDto) => void
  onUpdateEdge: (updater: (edge: WorkflowEdgeDto) => WorkflowEdgeDto) => void
  onSetStartNode: (nodeId: string) => void
  onCreateAgent?: () => void
  onEditAgent?: (ref: AgentRefDto) => void
}) {
  const title = selectedNode ? selectedNode.title : selectedEdge?.label || selectedEdge?.id || ''
  return (
    <div
      className="agent-properties-panel pointer-events-auto absolute bottom-4 left-5 z-30 flex max-h-[calc(100%-5rem)] w-[300px] flex-col overflow-hidden rounded-lg border border-border/60 bg-card/95 text-[11.5px] text-card-foreground shadow-[0_8px_28px_-12px_rgba(0,0,0,0.55)] backdrop-blur-md"
      onPointerDown={(event) => event.stopPropagation()}
      onWheel={(event) => event.stopPropagation()}
    >
      <header className="flex items-center gap-2 border-b border-border/50 px-3 py-1.5">
        <span className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded bg-primary/10 text-primary">
          {selectedNode ? <Workflow className="h-3 w-3" /> : <GitBranch className="h-3 w-3" />}
        </span>
        <p className="min-w-0 flex-1 truncate text-[12px] font-semibold leading-none text-foreground">
          {title}
        </p>
        <Button type="button" size="icon-sm" variant="ghost" onClick={onDelete} className="size-5 text-muted-foreground hover:text-destructive" aria-label="Delete selected workflow item">
          <Trash2 className="h-3 w-3" />
        </Button>
        <Button type="button" size="icon-sm" variant="ghost" onClick={onClose} className="size-5 text-muted-foreground hover:text-foreground" aria-label="Close properties">
          <X className="h-3 w-3" />
        </Button>
      </header>
      <div className="min-h-0 space-y-4 overflow-y-auto px-3 py-3">
        {diagnostics.length > 0 ? (
          <div className="space-y-1 rounded-md border border-destructive/25 bg-destructive/10 px-2 py-2 text-[11px] text-destructive">
            {diagnostics.slice(0, 3).map((diagnostic) => (
              <p key={`${diagnostic.path}:${diagnostic.message}`}>{diagnostic.message}</p>
            ))}
          </div>
        ) : null}
        {selectedNode ? (
          <NodeEditor
            agents={agents}
            definition={definition}
            node={selectedNode}
            onUpdate={onUpdateNode}
            onSetStartNode={onSetStartNode}
            onCreateAgent={onCreateAgent}
            onEditAgent={onEditAgent}
          />
        ) : selectedEdge ? (
          <EdgeEditor definition={definition} edge={selectedEdge} onUpdate={onUpdateEdge} />
        ) : null}
      </div>
    </div>
  )
}

function NodeEditor({
  agents,
  definition,
  node,
  onUpdate,
  onSetStartNode,
  onCreateAgent,
  onEditAgent,
}: {
  agents: { agent: WorkflowAgentSummaryDto; key: string }[]
  definition: WorkflowDefinitionDto
  node: WorkflowNodeDto
  onUpdate: (updater: (node: WorkflowNodeDto) => WorkflowNodeDto) => void
  onSetStartNode: (nodeId: string) => void
  onCreateAgent?: () => void
  onEditAgent?: (ref: AgentRefDto) => void
}) {
  return (
    <>
      <Field label="Title">
        <Input
          value={node.title}
          onChange={(event) => onUpdate((current) => ({ ...current, title: event.target.value }))}
          className="h-8 text-[12px]"
        />
      </Field>
      <Field label="Description">
        <Textarea
          value={node.description}
          onChange={(event) => onUpdate((current) => ({ ...current, description: event.target.value }))}
          className="min-h-20 text-[12px]"
        />
      </Field>
      <Field label="Start">
        <Button
          type="button"
          variant={definition.startNodeId === node.id ? 'secondary' : 'outline'}
          size="sm"
          className="h-7 text-[11px]"
          onClick={() => onSetStartNode(node.id)}
          disabled={definition.startNodeId === node.id}
        >
          <Lock className="size-3" />
          {definition.startNodeId === node.id ? 'Start node' : 'Make start node'}
        </Button>
      </Field>
      {node.type === 'agent' ? (
        <AgentNodeEditor
          agents={agents}
          node={node}
          onUpdate={onUpdate}
          onCreateAgent={onCreateAgent}
          onEditAgent={onEditAgent}
        />
      ) : null}
      {node.type === 'gate' ? (
        <Field label="Blocked behavior">
          <Select
            value={node.onBlocked}
            onValueChange={(value) => onUpdate((current) => current.type === 'gate' ? { ...current, onBlocked: value as 'pause' | 'fail' } : current)}
          >
            <SelectTrigger className="h-8 text-[12px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="pause">Pause</SelectItem>
              <SelectItem value="fail">Fail</SelectItem>
            </SelectContent>
          </Select>
        </Field>
      ) : null}
      {node.type === 'human_checkpoint' ? (
        <>
          <Field label="Checkpoint prompt">
            <Textarea
              value={node.prompt}
              onChange={(event) => onUpdate((current) => current.type === 'human_checkpoint' ? { ...current, prompt: event.target.value } : current)}
              className="min-h-20 text-[12px]"
            />
          </Field>
          <Field label="Decisions">
            <Input
              value={node.decisionOptions.join(', ')}
              onChange={(event) =>
                onUpdate((current) =>
                  current.type === 'human_checkpoint'
                    ? {
                        ...current,
                        decisionOptions: event.target.value
                          .split(',')
                          .map((entry) => entry.trim())
                          .filter(Boolean),
                      }
                    : current,
                )
              }
              className="h-8 text-[12px]"
            />
          </Field>
        </>
      ) : null}
      {node.type === 'terminal' ? (
        <Field label="Terminal status">
          <Select
            value={node.terminalStatus}
            onValueChange={(value) =>
              onUpdate((current) =>
                current.type === 'terminal'
                  ? { ...current, terminalStatus: value as WorkflowTerminalStatusDto }
                  : current,
              )
            }
          >
            <SelectTrigger className="h-8 text-[12px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="success">Success</SelectItem>
              <SelectItem value="failure">Failure</SelectItem>
              <SelectItem value="cancelled">Cancelled</SelectItem>
              <SelectItem value="needs_human">Needs human</SelectItem>
            </SelectContent>
          </Select>
        </Field>
      ) : null}
    </>
  )
}

function AgentNodeEditor({
  agents,
  node,
  onUpdate,
  onCreateAgent,
  onEditAgent,
}: {
  agents: { agent: WorkflowAgentSummaryDto; key: string }[]
  node: Extract<WorkflowNodeDto, { type: 'agent' }>
  onUpdate: (updater: (node: WorkflowNodeDto) => WorkflowNodeDto) => void
  onCreateAgent?: () => void
  onEditAgent?: (ref: AgentRefDto) => void
}) {
  const selectedKey = agentRefKey(node.agentRef)
  return (
    <>
      <Field label="Agent">
        <Select
          value={selectedKey}
          onValueChange={(value) => {
            const match = agents.find((entry) => entry.key === value)
            if (!match) return
            onUpdate((current) => current.type === 'agent' ? { ...current, agentRef: match.agent.ref } : current)
          }}
        >
          <SelectTrigger className="h-8 text-[12px]">
            <SelectValue placeholder="Choose agent" />
          </SelectTrigger>
          <SelectContent>
            {agents.map(({ agent, key }) => (
              <SelectItem key={key} value={key}>
                {agent.displayName}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <div className="mt-2 flex gap-2">
          {onCreateAgent ? (
            <Button type="button" variant="outline" size="sm" className="h-7 text-[11px]" onClick={onCreateAgent}>
              <Plus className="size-3" />
              New
            </Button>
          ) : null}
          {onEditAgent && node.agentRef.kind === 'custom' ? (
            <Button type="button" variant="ghost" size="sm" className="h-7 text-[11px]" onClick={() => onEditAgent(node.agentRef)}>
              Edit
            </Button>
          ) : null}
        </div>
      </Field>
      <Field label="Output artifact">
        <Input
          value={node.outputContract.artifactType}
          onChange={(event) =>
            onUpdate((current) =>
              current.type === 'agent'
                ? {
                    ...current,
                    outputContract: {
                      ...current.outputContract,
                      artifactType: event.target.value,
                    },
                  }
                : current,
            )
          }
          className="h-8 text-[12px]"
        />
      </Field>
      <Field label="Extraction">
        <Select
          value={node.outputContract.extraction}
          onValueChange={(value) =>
            onUpdate((current) =>
              current.type === 'agent'
                ? {
                    ...current,
                    outputContract: {
                      ...current.outputContract,
                      extraction: value as WorkflowOutputExtractionDto,
                    },
                  }
                : current,
            )
          }
        >
          <SelectTrigger className="h-8 text-[12px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="generic_text">Generic text</SelectItem>
            <SelectItem value="json_object">JSON object</SelectItem>
            <SelectItem value="json_array">JSON array</SelectItem>
          </SelectContent>
        </Select>
      </Field>
    </>
  )
}

function EdgeEditor({
  definition,
  edge,
  onUpdate,
}: {
  definition: WorkflowDefinitionDto
  edge: WorkflowEdgeDto
  onUpdate: (updater: (edge: WorkflowEdgeDto) => WorkflowEdgeDto) => void
}) {
  return (
    <>
      <Field label="Label">
        <Input
          value={edge.label}
          onChange={(event) => onUpdate((current) => ({ ...current, label: event.target.value }))}
          className="h-8 text-[12px]"
        />
      </Field>
      <Field label="Type">
        <Select
          value={edge.type}
          onValueChange={(value) =>
            onUpdate((current) => ({
              ...current,
              type: value as WorkflowEdgeTypeDto,
              loopPolicy:
                value === 'loop'
                  ? current.loopPolicy ?? {
                      loopKey: `${current.fromNodeId}_loop`,
                      maxAttempts: 2,
                      attemptScope: 'run',
                      carryoverPolicy: 'all',
                      selectedArtifactRefs: [],
                      resetPolicy: 'never',
                      stallDetector: null,
                      onExhausted: current.toNodeId,
                    }
                  : null,
            }))
          }
        >
          <SelectTrigger className="h-8 text-[12px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {Object.entries(EDGE_TYPE_LABEL).map(([value, label]) => (
              <SelectItem key={value} value={value}>
                {label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <Field label="Priority">
        <Input
          type="number"
          value={edge.priority}
          onChange={(event) => onUpdate((current) => ({ ...current, priority: Number(event.target.value) || 0 }))}
          className="h-8 text-[12px]"
        />
      </Field>
      <ConditionEditor edge={edge} onUpdate={onUpdate} />
      {edge.loopPolicy ? (
        <>
          <Field label="Loop key">
            <Input
              value={edge.loopPolicy.loopKey}
              onChange={(event) =>
                onUpdate((current) =>
                  current.loopPolicy
                    ? { ...current, loopPolicy: { ...current.loopPolicy, loopKey: event.target.value } }
                    : current,
                )
              }
              className="h-8 text-[12px]"
            />
          </Field>
          <Field label="Max attempts">
            <Input
              type="number"
              min={1}
              value={edge.loopPolicy.maxAttempts}
              onChange={(event) =>
                onUpdate((current) =>
                  current.loopPolicy
                    ? {
                        ...current,
                        loopPolicy: {
                          ...current.loopPolicy,
                          maxAttempts: Math.max(1, Number(event.target.value) || 1),
                        },
                      }
                    : current,
                )
              }
              className="h-8 text-[12px]"
            />
          </Field>
          <Field label="On exhausted">
            <Select
              value={edge.loopPolicy.onExhausted}
              onValueChange={(value) =>
                onUpdate((current) =>
                  current.loopPolicy
                    ? { ...current, loopPolicy: { ...current.loopPolicy, onExhausted: value } }
                    : current,
                )
              }
            >
              <SelectTrigger className="h-8 text-[12px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {definition.nodes.map((node) => (
                  <SelectItem key={node.id} value={node.id}>
                    {node.title}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
        </>
      ) : null}
    </>
  )
}

function ConditionEditor({
  edge,
  onUpdate,
}: {
  edge: WorkflowEdgeDto
  onUpdate: (updater: (edge: WorkflowEdgeDto) => WorkflowEdgeDto) => void
}) {
  const condition = edge.condition
  const conditionKind = supportedConditionKind(condition)
  const setCondition = (next: WorkflowConditionDto) => {
    onUpdate((current) => ({ ...current, condition: next }))
  }
  return (
    <>
      <Field label="Condition">
        <Select
          value={conditionKind}
          onValueChange={(value) => setCondition(defaultCondition(value))}
        >
          <SelectTrigger className="h-8 text-[12px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="always">Always</SelectItem>
            <SelectItem value="artifact_exists">Artifact exists</SelectItem>
            <SelectItem value="artifact_field_equals">Artifact field equals</SelectItem>
            <SelectItem value="artifact_field_number_compare">Number comparison</SelectItem>
            <SelectItem value="human_decision_is">Human decision</SelectItem>
          </SelectContent>
        </Select>
      </Field>
      {'artifactRef' in condition ? (
        <Field label="Artifact ref">
          <Input
            value={condition.artifactRef}
            onChange={(event) =>
              setCondition({ ...condition, artifactRef: event.target.value } as WorkflowConditionDto)
            }
            className="h-8 text-[12px]"
          />
        </Field>
      ) : null}
      {'path' in condition ? (
        <Field label="JSON path">
          <Input
            value={condition.path}
            onChange={(event) =>
              setCondition({ ...condition, path: event.target.value } as WorkflowConditionDto)
            }
            className="h-8 text-[12px]"
          />
        </Field>
      ) : null}
      {condition.kind === 'artifact_field_equals' ? (
        <Field label="Value">
          <Input
            value={String(condition.value ?? '')}
            onChange={(event) => setCondition({ ...condition, value: event.target.value })}
            className="h-8 text-[12px]"
          />
        </Field>
      ) : null}
      {condition.kind === 'artifact_field_number_compare' ? (
        <>
          <Field label="Operator">
            <Select
              value={condition.operator}
              onValueChange={(value) =>
                setCondition({
                  ...condition,
                  operator: value as typeof condition.operator,
                })
              }
            >
              <SelectTrigger className="h-8 text-[12px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {['eq', 'neq', 'gt', 'gte', 'lt', 'lte'].map((operator) => (
                  <SelectItem key={operator} value={operator}>
                    {operator}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </Field>
          <Field label="Value">
            <Input
              type="number"
              value={condition.value}
              onChange={(event) =>
                setCondition({ ...condition, value: Number(event.target.value) || 0 })
              }
              className="h-8 text-[12px]"
            />
          </Field>
        </>
      ) : null}
      {condition.kind === 'human_decision_is' ? (
        <>
          <Field label="Checkpoint">
            <Input
              value={condition.checkpointNodeId}
              onChange={(event) =>
                setCondition({ ...condition, checkpointNodeId: event.target.value })
              }
              className="h-8 text-[12px]"
            />
          </Field>
          <Field label="Decision">
            <Input
              value={condition.decision}
              onChange={(event) => setCondition({ ...condition, decision: event.target.value })}
              className="h-8 text-[12px]"
            />
          </Field>
        </>
      ) : null}
    </>
  )
}

function WorkflowDetailsPanel({
  node,
  edge,
  runNode,
  artifact,
  edgeDecision,
  events,
  agentLabel,
  running,
  onRetryNodeRun,
  onSkipBranch,
  onClose,
}: {
  node: WorkflowNodeDto | null
  edge: WorkflowEdgeDto | null
  runNode: WorkflowRunNodeDto | null
  artifact: WorkflowArtifactRecordDto | null
  edgeDecision: WorkflowRunEdgeDecisionDto | null
  events: readonly WorkflowEventDto[]
  agentLabel: string | null
  running: boolean
  onRetryNodeRun: ((nodeRunId: string) => void) | null
  onSkipBranch: ((nodeRunId: string) => void) | null
  onClose: () => void
}) {
  const canRetry = runNode ? isRetryableRunNodeStatus(runNode.status) && onRetryNodeRun : false
  const canSkip = runNode ? isSkippableRunNodeStatus(runNode.status) && onSkipBranch : false
  const timeline = workflowTimelineEvents(events, runNode, edge).slice(0, 6)
  return (
    <div
      className="agent-properties-panel pointer-events-auto absolute bottom-4 left-5 z-30 flex max-h-[calc(100%-5rem)] w-[300px] flex-col overflow-hidden rounded-lg border border-border/60 bg-card/95 text-[12px] text-card-foreground shadow-[0_8px_28px_-12px_rgba(0,0,0,0.55)] backdrop-blur-md"
      onPointerDown={(event) => event.stopPropagation()}
      onWheel={(event) => event.stopPropagation()}
    >
      <header className="flex items-center gap-2 border-b border-border/50 px-3 py-1.5">
        <span className="inline-flex h-5 w-5 shrink-0 items-center justify-center rounded bg-primary/10 text-primary">
          {node ? <Workflow className="h-3 w-3" /> : <GitBranch className="h-3 w-3" />}
        </span>
        <p className="min-w-0 flex-1 truncate text-[12px] font-semibold leading-none text-foreground">
          {node?.title ?? edge?.label ?? edge?.id}
        </p>
        <Button type="button" size="icon-sm" variant="ghost" onClick={onClose} className="size-5 text-muted-foreground hover:text-foreground" aria-label="Close details">
          <X className="h-3 w-3" />
        </Button>
      </header>
      <div className="min-h-0 space-y-4 overflow-y-auto px-3 py-3">
        {node ? (
          <>
            <ReadOnlyRow label="Type" value={NODE_KIND_LABEL[node.type]} />
            {agentLabel ? <ReadOnlyRow label="Agent" value={agentLabel} /> : null}
            {runNode ? <ReadOnlyRow label="Status" value={humanize(runNode.status)} /> : null}
            {runNode?.failureClass ? <ReadOnlyRow label="Failure" value={runNode.failureClass} /> : null}
            {runNode && (canRetry || canSkip) ? (
              <section className="space-y-1.5">
                <h3 className="text-[9.5px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
                  Operations
                </h3>
                <div className="flex flex-wrap gap-1.5">
                  {canRetry ? (
                    <Button
                      type="button"
                      size="sm"
                      variant="secondary"
                      disabled={running}
                      className="h-7 px-2 text-[11px]"
                      onClick={() => onRetryNodeRun?.(runNode.id)}
                    >
                      {running ? <Loader2 className="size-3 animate-spin" /> : <RotateCcw className="size-3" />}
                      Retry
                    </Button>
                  ) : null}
                  {canSkip ? (
                    <Button
                      type="button"
                      size="sm"
                      variant="outline"
                      disabled={running}
                      className="h-7 px-2 text-[11px]"
                      onClick={() => onSkipBranch?.(runNode.id)}
                    >
                      {running ? <Loader2 className="size-3 animate-spin" /> : <SkipForward className="size-3" />}
                      Skip
                    </Button>
                  ) : null}
                </div>
              </section>
            ) : null}
            {artifact ? (
              <section className="space-y-1.5">
                <h3 className="text-[9.5px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
                  Artifact
                </h3>
                <div className="rounded-md border border-border/45 bg-background/45 px-2 py-2 text-[11.5px] leading-relaxed">
                  <p className="font-medium text-foreground">{artifact.artifactType} v{artifact.schemaVersion}</p>
                  <p className="mt-1 max-h-28 overflow-hidden text-muted-foreground [white-space:pre-wrap]">
                    {artifact.renderText ?? summarizeJson(artifact.payload)}
                  </p>
                </div>
              </section>
            ) : null}
            {timeline.length > 0 ? <WorkflowEventTimeline events={timeline} /> : null}
          </>
        ) : edge ? (
          <>
            <ReadOnlyRow label="Type" value={EDGE_TYPE_LABEL[edge.type]} />
            <ReadOnlyRow label="From" value={edge.fromNodeId} />
            <ReadOnlyRow label="To" value={edge.toNodeId} />
            <ReadOnlyRow label="Condition" value={conditionSummary(edge.condition)} />
            {edge.loopPolicy ? <ReadOnlyRow label="Loop" value={`${edge.loopPolicy.loopKey}, max ${edge.loopPolicy.maxAttempts}`} /> : null}
            {edgeDecision ? (
              <section className="space-y-1.5">
                <h3 className="text-[9.5px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
                  Decision evidence
                </h3>
                <pre className="max-h-40 overflow-auto rounded-md border border-border/45 bg-background/45 p-2 text-[10.5px] text-foreground/80">
                  {JSON.stringify(edgeDecision.evidence, null, 2)}
                </pre>
              </section>
            ) : null}
            {timeline.length > 0 ? <WorkflowEventTimeline events={timeline} /> : null}
          </>
        ) : null}
      </div>
    </div>
  )
}

function WorkflowEventTimeline({ events }: { events: readonly WorkflowEventDto[] }) {
  return (
    <section className="space-y-1.5">
      <h3 className="text-[9.5px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
        Timeline
      </h3>
      <ol className="space-y-1.5">
        {events.map((event) => (
          <li
            key={event.id}
            className="rounded-md border border-border/45 bg-background/35 px-2 py-1.5"
          >
            <div className="flex items-center justify-between gap-2">
              <span className="min-w-0 truncate text-[11px] font-medium text-foreground/90">
                {humanize(event.eventType)}
              </span>
              <time className="shrink-0 text-[9.5px] text-muted-foreground">
                {compactTime(event.createdAt)}
              </time>
            </div>
            <p className="mt-0.5 line-clamp-2 text-[10.5px] text-muted-foreground">
              {timelineEventSummary(event)}
            </p>
          </li>
        ))}
      </ol>
    </section>
  )
}

function CheckpointResumeBar({
  node,
  running,
  onResume,
}: {
  node: WorkflowNodeDto | null
  running: boolean
  onResume: (decision: string) => void
}) {
  const options = node?.type === 'human_checkpoint' && node.decisionOptions.length > 0
    ? node.decisionOptions
    : ['continue']
  return (
    <div className="pointer-events-auto absolute bottom-5 right-5 z-30 flex max-w-md items-center gap-2 rounded-lg border border-amber-500/25 bg-card/95 px-3 py-2 text-[12px] shadow-lg backdrop-blur-md">
      <PauseCircle className="size-4 shrink-0 text-amber-500" />
      <p className="min-w-0 flex-1 truncate text-muted-foreground">
        {node?.type === 'human_checkpoint' ? node.prompt : 'Workflow is paused at a gate.'}
      </p>
      {options.map((option) => (
        <Button key={option} type="button" size="sm" className="h-7 text-[11px]" disabled={running} onClick={() => onResume(option)}>
          {running ? <Loader2 className="size-3 animate-spin" /> : null}
          {humanize(option)}
        </Button>
      ))}
    </div>
  )
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="space-y-1.5">
      <Label className="text-[10px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
        {label}
      </Label>
      {children}
    </div>
  )
}

function ReadOnlyRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="space-y-0.5">
      <dt className="text-[9.5px] font-semibold uppercase tracking-[0.12em] text-muted-foreground">
        {label}
      </dt>
      <dd className="break-words text-[12px] text-foreground/90">{value}</dd>
    </div>
  )
}

function createNode(
  type: WorkflowNodeDto['type'],
  id: string,
  fallbackAgentRef: AgentRefDto,
  position: { x: number; y: number },
): WorkflowNodeDto {
  const base = {
    id,
    title: humanize(id),
    description: '',
    position,
  }
  if (type === 'agent') {
    return {
      ...base,
      type,
      agentRef: fallbackAgentRef,
      displayLabel: null,
      inputBindings: [],
      outputContract: {
        artifactType: 'text_output',
        schemaVersion: 1,
        extraction: 'generic_text',
        required: true,
      },
      runOverrides: null,
      resourceScopes: [],
      failurePolicy: {
        quotaFailureClasses: [],
        transientFailureClasses: [],
      },
    }
  }
  if (type === 'router') return { ...base, type }
  if (type === 'gate') return { ...base, type, requiredChecks: [], onBlocked: 'pause' }
  if (type === 'human_checkpoint') {
    return {
      ...base,
      type,
      checkpointType: 'human_verify',
      prompt: 'Review the Workflow state and choose a decision.',
      decisionOptions: ['continue'],
    }
  }
  if (type === 'merge') {
    return { ...base, type, waitPolicy: 'all', quorum: null, failFast: false }
  }
  return { ...base, type: 'terminal', terminalStatus: 'success' }
}

function autoLayoutWorkflowDefinition(definition: WorkflowDefinitionDto): WorkflowDefinitionDto {
  const byId = new Map(definition.nodes.map((node) => [node.id, node]))
  const outgoing = new Map<string, string[]>()
  const incomingCounts = new Map<string, number>()

  for (const node of definition.nodes) {
    outgoing.set(node.id, [])
    incomingCounts.set(node.id, 0)
  }
  for (const edge of definition.edges) {
    if (!byId.has(edge.fromNodeId) || !byId.has(edge.toNodeId)) continue
    outgoing.get(edge.fromNodeId)?.push(edge.toNodeId)
    incomingCounts.set(edge.toNodeId, (incomingCounts.get(edge.toNodeId) ?? 0) + 1)
  }

  const queue = definition.nodes
    .filter((node) => (incomingCounts.get(node.id) ?? 0) === 0)
    .map((node) => node.id)
  const visited = new Set<string>()
  const ordered: string[] = []

  while (queue.length > 0) {
    const id = queue.shift()
    if (!id || visited.has(id)) continue
    visited.add(id)
    ordered.push(id)
    for (const target of outgoing.get(id) ?? []) {
      incomingCounts.set(target, Math.max(0, (incomingCounts.get(target) ?? 0) - 1))
      if ((incomingCounts.get(target) ?? 0) === 0) queue.push(target)
    }
  }

  for (const node of definition.nodes) {
    if (!visited.has(node.id)) ordered.push(node.id)
  }

  const positionById = new Map(
    ordered.map((id, index) => [id, { x: 120 + index * 360, y: 220 }]),
  )
  return {
    ...definition,
    nodes: definition.nodes.map((node) => ({
      ...node,
      position: positionById.get(node.id) ?? node.position,
    })),
  }
}

function latestRunNodesByNodeId(run: WorkflowRunDto | null): Map<string, WorkflowRunNodeDto> {
  const map = new Map<string, WorkflowRunNodeDto>()
  for (const node of run?.nodes ?? []) {
    const previous = map.get(node.nodeId)
    if (!previous || previous.attemptNumber <= node.attemptNumber) {
      map.set(node.nodeId, node)
    }
  }
  return map
}

function latestArtifactsByNodeId(run: WorkflowRunDto | null): Map<string, WorkflowArtifactRecordDto> {
  const nodeIdByRunId = new Map((run?.nodes ?? []).map((node) => [node.id, node.nodeId]))
  const map = new Map<string, WorkflowArtifactRecordDto>()
  for (const artifact of run?.artifacts ?? []) {
    const nodeId = nodeIdByRunId.get(artifact.producerNodeRunId)
    if (!nodeId) continue
    map.set(nodeId, artifact)
  }
  return map
}

function latestEdgeDecision(
  run: WorkflowRunDto | null,
  edgeId: string,
): WorkflowRunEdgeDecisionDto | null {
  const decisions = run?.edgeDecisions.filter((decision) => decision.edgeId === edgeId) ?? []
  return decisions.at(-1) ?? null
}

function labelForAgentRef(
  ref: AgentRefDto,
  agents: readonly WorkflowAgentSummaryDto[],
): string {
  return agents.find((agent) => agentRefKey(agent.ref) === agentRefKey(ref))?.displayName ??
    (ref.kind === 'built_in' ? humanize(ref.runtimeAgentId) : ref.definitionId)
}

function iconForNodeType(type: WorkflowNodeDto['type']) {
  switch (type) {
    case 'agent':
      return Bot
    case 'router':
      return Route
    case 'gate':
      return ShieldCheck
    case 'human_checkpoint':
      return PauseCircle
    case 'merge':
      return GitMerge
    case 'terminal':
      return CheckCircle2
  }
}

function nodeTone(type: WorkflowNodeDto['type']): string {
  switch (type) {
    case 'agent':
      return 'amber'
    case 'router':
      return 'sky'
    case 'gate':
      return 'emerald'
    case 'human_checkpoint':
      return 'rose'
    case 'merge':
      return 'violet'
    case 'terminal':
      return 'foreground'
  }
}

function nodeIconTone(type: WorkflowNodeDto['type']): string {
  switch (type) {
    case 'agent':
      return 'bg-amber-500/12 text-amber-500 ring-amber-500/30'
    case 'router':
      return 'bg-sky-500/12 text-sky-500 ring-sky-500/30'
    case 'gate':
      return 'bg-emerald-500/12 text-emerald-500 ring-emerald-500/30'
    case 'human_checkpoint':
      return 'bg-rose-500/12 text-rose-500 ring-rose-500/30'
    case 'merge':
      return 'bg-violet-500/12 text-violet-500 ring-violet-500/30'
    case 'terminal':
      return 'bg-foreground/10 text-muted-foreground ring-foreground/20'
  }
}

function handleClassForNode(type: WorkflowNodeDto['type']): string {
  switch (type) {
    case 'agent':
      return '!bg-amber-500'
    case 'router':
      return '!bg-sky-500'
    case 'gate':
      return '!bg-emerald-500'
    case 'human_checkpoint':
      return '!bg-rose-500'
    case 'merge':
      return '!bg-violet-500'
    case 'terminal':
      return '!bg-foreground'
  }
}

function defaultCondition(kind: string): WorkflowConditionDto {
  if (kind === 'artifact_exists') return { kind, artifactRef: 'node.text_output' }
  if (kind === 'artifact_field_equals') {
    return { kind, artifactRef: 'node.text_output', path: '$.status', value: 'passed' }
  }
  if (kind === 'artifact_field_number_compare') {
    return {
      kind,
      artifactRef: 'node.text_output',
      path: '$.count',
      operator: 'gt',
      value: 0,
    }
  }
  if (kind === 'human_decision_is') {
    return { kind, checkpointNodeId: 'human_checkpoint', decision: 'continue' }
  }
  return { kind: 'always' }
}

function supportedConditionKind(condition: WorkflowConditionDto): string {
  if (
    condition.kind === 'always' ||
    condition.kind === 'artifact_exists' ||
    condition.kind === 'artifact_field_equals' ||
    condition.kind === 'artifact_field_number_compare' ||
    condition.kind === 'human_decision_is'
  ) {
    return condition.kind
  }
  return 'always'
}

function conditionSummary(condition: WorkflowConditionDto): string {
  switch (condition.kind) {
    case 'always':
      return 'Always'
    case 'artifact_exists':
      return `${condition.artifactRef} exists`
    case 'artifact_field_equals':
      return `${condition.artifactRef}${condition.path} = ${String(condition.value)}`
    case 'artifact_field_number_compare':
      return `${condition.artifactRef}${condition.path} ${condition.operator} ${condition.value}`
    case 'human_decision_is':
      return `${condition.checkpointNodeId} decision is ${condition.decision}`
    case 'all':
      return 'All conditions'
    case 'any':
      return 'Any condition'
    case 'not':
      return 'Not condition'
    case 'node_status':
      return `${condition.nodeId} is ${humanize(condition.status)}`
    case 'artifact_field_in':
      return `${condition.artifactRef}${condition.path} in ${condition.values.length} values`
    case 'failure_class_is':
      return `Failure class is ${condition.failureClass}`
    case 'loop_attempt_lt':
      return `${condition.loopKey} attempts < ${condition.value}`
    case 'loop_attempt_gte':
      return `${condition.loopKey} attempts >= ${condition.value}`
  }
}

function uniqueId(prefix: string, existing: readonly string[]): string {
  const normalized = prefix.replace(/[^A-Za-z0-9_-]+/g, '_')
  let candidate = normalized
  let index = 2
  const seen = new Set(existing)
  while (seen.has(candidate)) {
    candidate = `${normalized}_${index}`
    index += 1
  }
  return candidate
}

function cloneDefinition(definition: WorkflowDefinitionDto): WorkflowDefinitionDto {
  return JSON.parse(JSON.stringify(definition)) as WorkflowDefinitionDto
}

function summarizeJson(value: unknown): string {
  if (typeof value === 'string') return value
  try {
    return JSON.stringify(value, null, 2)
  } catch {
    return String(value)
  }
}

function workflowTimelineEvents(
  events: readonly WorkflowEventDto[],
  runNode: WorkflowRunNodeDto | null,
  edge: WorkflowEdgeDto | null,
): WorkflowEventDto[] {
  const filtered = events.filter((event) => {
    if (runNode) return event.nodeRunId === runNode.id
    if (!edge) return true
    const payload = event.event
    return isRecord(payload) && payload.edgeId === edge.id
  })
  return [...filtered].sort((left, right) => right.createdAt.localeCompare(left.createdAt))
}

function timelineEventSummary(event: WorkflowEventDto): string {
  const payload = event.event
  if (!isRecord(payload)) return summarizeJson(payload)
  if (typeof payload.metric === 'string') return humanize(payload.metric)
  if (typeof payload.failureClass === 'string') return payload.failureClass
  if (typeof payload.reason === 'string') return payload.reason
  if (typeof payload.edgeId === 'string') {
    const matched = typeof payload.matched === 'boolean'
      ? payload.matched ? 'matched' : 'not matched'
      : 'evaluated'
    return `${payload.edgeId} ${matched}`
  }
  if (typeof payload.nodeId === 'string') return payload.nodeId
  return summarizeJson(payload)
}

function compactTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function humanize(value: string): string {
  return value
    .replace(/[_-]+/g, ' ')
    .replace(/\b\w/g, (letter) => letter.toUpperCase())
}

function isActiveRun(status: WorkflowRunStatusDto): boolean {
  return status === 'queued' || status === 'running' || status === 'paused'
}

function isRetryableRunNodeStatus(status: WorkflowNodeRunStatusDto): boolean {
  return status === 'failed' || status === 'stalled' || status === 'skipped' || status === 'cancelled'
}

function isSkippableRunNodeStatus(status: WorkflowNodeRunStatusDto): boolean {
  return status === 'pending'
    || status === 'eligible'
    || status === 'starting'
    || status === 'running'
    || status === 'waiting_on_gate'
}
