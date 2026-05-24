import type { RuntimeAgentIdDto } from '@xero/ui/model/runtime'

import type {
  WorkflowDefinitionDto,
  WorkflowEdgeDto,
  WorkflowNodeDto,
} from './workflow-definition'
import type { AgentRefDto, WorkflowAgentSummaryDto } from './workflow-agents'

export type WorkflowTemplateIdDto = 'linear_handoff' | 'continuous_delivery'

export interface WorkflowTemplateSummaryDto {
  id: WorkflowTemplateIdDto
  name: string
  description: string
  nodeCount: number
  tags: string[]
}

export const WORKFLOW_TEMPLATE_LIBRARY: WorkflowTemplateSummaryDto[] = [
  {
    id: 'linear_handoff',
    name: 'Agent handoff',
    description: 'Run one agent, pass its artifact to a second agent, then finish.',
    nodeCount: 3,
    tags: ['starter', 'handoff'],
  },
  {
    id: 'continuous_delivery',
    name: 'Continuous delivery',
    description: 'Plan, build, check, recover through bounded gap/debug/review loops, then summarize.',
    nodeCount: 15,
    tags: ['recovery', 'gates', 'loops'],
  },
]

export interface InstantiateWorkflowTemplateOptions {
  projectId: string
  templateId: WorkflowTemplateIdDto
  agents?: readonly WorkflowAgentSummaryDto[]
  name?: string
}

export interface InstantiateBlankWorkflowOptions {
  projectId: string
  name?: string
}

export function instantiateBlankWorkflow({
  projectId,
  name,
}: InstantiateBlankWorkflowOptions): WorkflowDefinitionDto {
  return withBaseDefinition({
    id: createWorkflowId('blank-workflow'),
    projectId,
    name: name?.trim() || 'New workflow',
    description: '',
    startNodeId: '',
    nodes: [],
    edges: [],
  })
}

export function instantiateWorkflowTemplate({
  projectId,
  templateId,
  agents = [],
  name,
}: InstantiateWorkflowTemplateOptions): WorkflowDefinitionDto {
  if (templateId === 'continuous_delivery') {
    return instantiateContinuousDeliveryTemplate(projectId, agents, name)
  }
  return instantiateLinearHandoffTemplate(projectId, agents, name)
}

function instantiateLinearHandoffTemplate(
  projectId: string,
  agents: readonly WorkflowAgentSummaryDto[],
  name?: string,
): WorkflowDefinitionDto {
  const firstAgent = resolveBuiltInAgentRef(agents, 'plan')
  const secondAgent = resolveBuiltInAgentRef(agents, 'engineer')
  const id = createWorkflowId('agent-handoff')
  return withBaseDefinition({
    id,
    projectId,
    name: name?.trim() || 'Agent handoff',
    description: 'A small Workflow for passing one agent output to another.',
    startNodeId: 'intake',
    nodes: [
      agentNode('intake', 'Intake', 80, 120, firstAgent, 'task_brief'),
      agentNode('work', 'Work', 420, 120, secondAgent, 'implementation_summary', [
        {
          source: 'artifact',
          name: 'intake',
          required: true,
          artifactRef: 'intake.task_brief',
          promptLabel: 'Upstream task brief',
        },
      ]),
      terminalNode('done', 'Done', 760, 120, 'success'),
    ],
    edges: [
      edge('intake_to_work', 'intake', 'work', 'success', 'handoff', 10),
      edge('work_to_done', 'work', 'done', 'success', 'complete', 10),
    ],
  })
}

function instantiateContinuousDeliveryTemplate(
  projectId: string,
  agents: readonly WorkflowAgentSummaryDto[],
  name?: string,
): WorkflowDefinitionDto {
  const planAgent = resolveBuiltInAgentRef(agents, 'plan')
  const workAgent = resolveBuiltInAgentRef(agents, 'engineer')
  const checkAgent = resolveBuiltInAgentRef(agents, 'engineer')
  const debugAgent = resolveBuiltInAgentRef(agents, 'debug')
  const summaryAgent = resolveBuiltInAgentRef(agents, 'generalist')
  const id = createWorkflowId('continuous-delivery')

  return withBaseDefinition({
    id,
    projectId,
    name: name?.trim() || 'Continuous delivery',
    description:
      'A starter Workflow with typed handoffs, if/else routing, bounded recovery loops, and human escalation.',
    startNodeId: 'goal_intake',
    nodes: [
      agentNode('goal_intake', 'Goal intake', 40, 160, planAgent, 'task_brief'),
      agentNode('plan', 'Plan', 360, 160, planAgent, 'plan', [
        runInputBinding('goal', 'Goal'),
        artifactBinding('goal_intake.task_brief', 'Goal intake'),
      ]),
      agentNode('work', 'Work', 680, 160, workAgent, 'implementation_summary', [
        artifactBinding('plan.plan', 'Plan'),
      ]),
      agentNode('check', 'Check', 1000, 160, checkAgent, 'verification_result', [
        artifactBinding('work.implementation_summary', 'Implementation summary'),
      ]),
      routerNode('verification_router', 'Verification route', 1320, 160),
      agentNode('gap_closure', 'Gap closure', 1320, 420, planAgent, 'gap_list', [
        artifactBinding('check.verification_result', 'Verification result'),
      ]),
      agentNode('debug', 'Debug', 1000, 420, debugAgent, 'debug_report', [
        artifactBinding('work.implementation_summary', 'Implementation summary', false),
        artifactBinding('check.verification_result', 'Verification result', false),
      ]),
      agentNode('review', 'Review', 1640, 120, checkAgent, 'review_findings', [
        artifactBinding('check.verification_result', 'Verification result'),
      ]),
      routerNode('review_router', 'Review route', 1960, 120),
      agentNode('fix', 'Fix', 1960, 380, workAgent, 'implementation_summary', [
        artifactBinding('review.review_findings', 'Review findings'),
      ]),
      agentNode('summary', 'Summary', 2280, 120, summaryAgent, 'text_output', [
        artifactBinding('work.implementation_summary', 'Implementation summary'),
        artifactBinding('review.review_findings', 'Review findings', false),
      ]),
      humanCheckpointNode('human_verify', 'Human verification', 1640, 460),
      terminalNode('success', 'Success', 2600, 120, 'success'),
      terminalNode('failed', 'Failed', 2280, 460, 'failure'),
      terminalNode('needs_human', 'Needs human', 1960, 640, 'needs_human'),
    ],
    edges: [
      edge('goal_to_plan', 'goal_intake', 'plan', 'success', 'brief ready', 10),
      edge('plan_to_work', 'plan', 'work', 'success', 'build', 10),
      edge('work_to_check', 'work', 'check', 'success', 'verify', 10),
      edge('work_failed_to_debug', 'work', 'debug', 'recovery', 'debug', 5, {
        kind: 'node_status',
        nodeId: 'work',
        status: 'failed',
      }),
      edge('check_to_router', 'check', 'verification_router', 'success', 'route', 10),
      edge(
        'verification_passed',
        'verification_router',
        'review',
        'conditional',
        'passed',
        10,
        {
          kind: 'artifact_field_equals',
          artifactRef: 'check.verification_result',
          path: '$.status',
          value: 'passed',
        },
      ),
      edge(
        'verification_gaps',
        'verification_router',
        'gap_closure',
        'conditional',
        'gaps',
        20,
        {
          kind: 'artifact_field_in',
          artifactRef: 'check.verification_result',
          path: '$.status',
          values: ['gaps_found', 'needs_changes'],
        },
      ),
      loopEdge('gap_back_to_work', 'gap_closure', 'work', 'gap closure', 'gap_closure', 2, 'human_verify'),
      edge(
        'debug_to_work',
        'debug',
        'work',
        'loop',
        'retry work',
        30,
        {
          kind: 'artifact_field_equals',
          artifactRef: 'debug.debug_report',
          path: '$.recommended_route',
          value: 'retry_work',
        },
        {
          loopKey: 'debug_recovery',
          maxAttempts: 2,
          attemptScope: 'run',
          carryoverPolicy: 'all',
          selectedArtifactRefs: [],
          resetPolicy: 'never',
          stallDetector: 'same_failure_class_repeated',
          onExhausted: 'human_verify',
        },
      ),
      edge('review_to_router', 'review', 'review_router', 'success', 'route', 10),
      edge(
        'review_clear',
        'review_router',
        'summary',
        'conditional',
        'clear',
        10,
        {
          kind: 'artifact_field_number_compare',
          artifactRef: 'review.review_findings',
          path: '$.high_count',
          operator: 'eq',
          value: 0,
        },
      ),
      edge(
        'review_high_findings',
        'review_router',
        'fix',
        'conditional',
        'fix',
        20,
        {
          kind: 'artifact_field_number_compare',
          artifactRef: 'review.review_findings',
          path: '$.high_count',
          operator: 'gt',
          value: 0,
        },
      ),
      loopEdge('fix_back_to_review', 'fix', 'review', 'review fix', 'review_fix', 3, 'human_verify'),
      edge('summary_to_success', 'summary', 'success', 'success', 'complete', 10),
      edge('human_to_needs_human', 'human_verify', 'needs_human', 'manual_override', 'escalate', 10),
      edge('debug_to_failed', 'debug', 'failed', 'failure', 'abort', 90),
    ],
  })
}

function withBaseDefinition(params: {
  id: string
  projectId: string
  name: string
  description: string
  startNodeId: string
  nodes: WorkflowNodeDto[]
  edges: WorkflowEdgeDto[]
}): WorkflowDefinitionDto {
  return {
    schema: 'xero.workflow_definition.v1',
    id: params.id,
    projectId: params.projectId,
    name: params.name,
    description: params.description,
    version: 1,
    startNodeId: params.startNodeId,
    nodes: params.nodes,
    edges: params.edges,
    artifactContracts: [
      artifactContract('text_output', 'Text output'),
      artifactContract('task_brief', 'Task brief'),
      artifactContract('plan', 'Plan'),
      artifactContract('implementation_summary', 'Implementation summary'),
      artifactContract('verification_result', 'Verification result'),
      artifactContract('debug_report', 'Debug report'),
      artifactContract('gap_list', 'Gap list'),
      artifactContract('review_findings', 'Review findings'),
      artifactContract('human_decision', 'Human decision'),
    ],
    runPolicy: {
      concurrencyLimit: 1,
      resourceConflictPolicy: {
        mode: 'serialize_conflicts',
        defaultScopes: [],
      },
      recoveryDefaults: {
        debugMaxAttempts: 2,
        gapClosureMaxAttempts: 2,
        reviewFixMaxAttempts: 3,
      },
    },
    createdAt: null,
    updatedAt: null,
  }
}

function agentNode(
  id: string,
  title: string,
  x: number,
  y: number,
  agentRef: AgentRefDto,
  artifactType: string,
  inputBindings: WorkflowNodeDto extends infer _ ? NonNullable<Extract<WorkflowNodeDto, { type: 'agent' }>['inputBindings']> : never = [],
): WorkflowNodeDto {
  return {
    id,
    title,
    description: '',
    position: { x, y },
    type: 'agent',
    agentRef,
    displayLabel: null,
    inputBindings,
    outputContract: {
      artifactType,
      schemaVersion: 1,
      extraction: artifactType === 'text_output' ? 'generic_text' : 'json_object',
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

function routerNode(id: string, title: string, x: number, y: number): WorkflowNodeDto {
  return {
    id,
    title,
    description: '',
    position: { x, y },
    type: 'router',
  }
}

function humanCheckpointNode(id: string, title: string, x: number, y: number): WorkflowNodeDto {
  return {
    id,
    title,
    description: 'Pause the Workflow for user judgment before continuing.',
    position: { x, y },
    type: 'human_checkpoint',
    checkpointType: 'human_verify',
    prompt: 'Review the current artifacts and choose how the Workflow should continue.',
    decisionOptions: ['continue', 'stop'],
  }
}

function terminalNode(
  id: string,
  title: string,
  x: number,
  y: number,
  terminalStatus: Extract<WorkflowNodeDto, { type: 'terminal' }>['terminalStatus'],
): WorkflowNodeDto {
  return {
    id,
    title,
    description: '',
    position: { x, y },
    type: 'terminal',
    terminalStatus,
  }
}

function edge(
  id: string,
  fromNodeId: string,
  toNodeId: string,
  type: WorkflowEdgeDto['type'],
  label: string,
  priority: number,
  condition: WorkflowEdgeDto['condition'] = { kind: 'always' },
  loopPolicy: WorkflowEdgeDto['loopPolicy'] = null,
): WorkflowEdgeDto {
  return {
    id,
    fromNodeId,
    toNodeId,
    type,
    label,
    priority,
    condition,
    loopPolicy,
  }
}

function loopEdge(
  id: string,
  fromNodeId: string,
  toNodeId: string,
  label: string,
  loopKey: string,
  maxAttempts: number,
  onExhausted: string,
): WorkflowEdgeDto {
  return edge(id, fromNodeId, toNodeId, 'loop', label, 30, { kind: 'always' }, {
    loopKey,
    maxAttempts,
    attemptScope: 'run',
    carryoverPolicy: 'all',
    selectedArtifactRefs: [],
    resetPolicy: 'never',
    stallDetector: 'no_artifact_progress',
    onExhausted,
  })
}

function artifactBinding(
  artifactRef: string,
  promptLabel: string,
  required = true,
): Extract<WorkflowNodeDto, { type: 'agent' }>['inputBindings'][number] {
  return {
    source: 'artifact',
    name: artifactRef.replace(/[^A-Za-z0-9_]+/g, '_'),
    required,
    artifactRef,
    promptLabel,
  }
}

function runInputBinding(
  name: string,
  promptLabel: string,
): Extract<WorkflowNodeDto, { type: 'agent' }>['inputBindings'][number] {
  return {
    source: 'run_input',
    name,
    required: false,
    promptLabel,
  }
}

function artifactContract(artifactType: string, displayName: string) {
  return {
    artifactType,
    schemaVersion: 1,
    jsonSchema: null,
    displayName,
    description: '',
  }
}

function resolveBuiltInAgentRef(
  agents: readonly WorkflowAgentSummaryDto[],
  runtimeAgentId: RuntimeAgentIdDto,
): AgentRefDto {
  const match = agents.find(
    (agent) => agent.ref.kind === 'built_in' && agent.ref.runtimeAgentId === runtimeAgentId,
  )
  if (match) return match.ref
  return {
    kind: 'built_in',
    runtimeAgentId,
    version: 1,
  }
}

function createWorkflowId(prefix: string): string {
  const suffix =
    typeof crypto !== 'undefined' && typeof crypto.getRandomValues === 'function'
      ? Array.from(crypto.getRandomValues(new Uint8Array(6)), (byte) =>
          byte.toString(16).padStart(2, '0'),
        ).join('')
      : Math.random().toString(16).slice(2, 14)
  return `${prefix}-${suffix}`
}
