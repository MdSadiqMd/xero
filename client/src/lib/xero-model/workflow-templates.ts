import type { RuntimeAgentIdDto } from '@xero/ui/model/runtime'

import type {
  WorkflowDefinitionDto,
  WorkflowEdgeDto,
  WorkflowNodeDto,
} from './workflow-definition'
import type { AgentRefDto, WorkflowAgentSummaryDto } from './workflow-agents'

export type WorkflowTemplateIdDto =
  | 'continuous_delivery'
  | 'gsd_auto'
  | 'release_train'
  | 'bug_triage_fix_loop'

export type WorkflowTemplateNodePositionsDto = Readonly<Record<string, Readonly<{ x: number; y: number }>>>

export interface WorkflowTemplateSummaryDto {
  id: WorkflowTemplateIdDto
  name: string
  description: string
  nodeCount: number
  tags: string[]
}

export const WORKFLOW_TEMPLATE_LIBRARY: WorkflowTemplateSummaryDto[] = [
  {
    id: 'continuous_delivery',
    name: 'Continuous delivery',
    description: 'Plan, build, check, recover through bounded gap/debug/review loops, then summarize.',
    nodeCount: 15,
    tags: ['recovery', 'gates', 'loops'],
  },
  {
    id: 'gsd_auto',
    name: 'GSD Auto',
    description: 'Continue milestones through durable delivery phases, executable verification, audit, and archive.',
    nodeCount: 13,
    tags: ['delivery-state', 'collections', 'verification'],
  },
  {
    id: 'release_train',
    name: 'Release train',
    description: 'Collect release candidates, verify each one, and archive release evidence.',
    nodeCount: 9,
    tags: ['release', 'collections', 'checks'],
  },
  {
    id: 'bug_triage_fix_loop',
    name: 'Bug triage and fix',
    description: 'Query open defects, fix one at a time, verify with commands, and close durable records.',
    nodeCount: 10,
    tags: ['bugs', 'state', 'recovery'],
  },
]

export const WORKFLOW_TEMPLATE_DEFAULT_NODE_POSITIONS: Readonly<
  Record<WorkflowTemplateIdDto, WorkflowTemplateNodePositionsDto>
> = {
  continuous_delivery: {
    goal_intake: { x: 40, y: 300 },
    plan: { x: 340, y: 300 },
    work: { x: 640, y: 300 },
    check: { x: 940, y: 80 },
    verification_router: { x: 1240, y: 300 },
    debug: { x: 940, y: 520 },
    failed: { x: 940, y: 720 },
    gap_closure: { x: 1240, y: 520 },
    review: { x: 1540, y: 300 },
    review_router: { x: 1840, y: 220 },
    fix: { x: 1840, y: 520 },
    human_verify: { x: 1840, y: 740 },
    summary: { x: 2140, y: 300 },
    success: { x: 2440, y: 300 },
    needs_human: { x: 2140, y: 740 },
  },
  gsd_auto: {
    load_milestones: { x: 40, y: 300 },
    milestone_route: { x: 340, y: 300 },
    query_phases: { x: 640, y: 260 },
    next_phase: { x: 940, y: 260 },
    phase_router: { x: 1240, y: 430 },
    milestone_intake_flow: { x: 940, y: 720 },
    partial_success: { x: 1240, y: 240 },
    process_phase_flow: { x: 1540, y: 430 },
    mark_phase_complete: { x: 1240, y: 20 },
    milestone_audit_flow: { x: 1540, y: 620 },
    next_milestone_offer: { x: 1240, y: 720 },
    success: { x: 1540, y: 820 },
    needs_human: { x: 2140, y: 620 },
  },
  release_train: {
    query_candidates: { x: 40, y: 300 },
    next_candidate: { x: 340, y: 300 },
    candidate_route: { x: 640, y: 300 },
    done: { x: 940, y: 300 },
    release_check: { x: 940, y: 520 },
    release_review: { x: 1240, y: 380 },
    archive_evidence: { x: 1540, y: 120 },
    release_human: { x: 1540, y: 520 },
    needs_human: { x: 1840, y: 520 },
  },
  bug_triage_fix_loop: {
    query_bugs: { x: 40, y: 300 },
    next_bug: { x: 340, y: 300 },
    bug_route: { x: 640, y: 300 },
    done: { x: 640, y: 520 },
    triage: { x: 940, y: 300 },
    fix_bug: { x: 1240, y: 520 },
    bug_check: { x: 1540, y: 300 },
    close_bug: { x: 1240, y: 80 },
    bug_human: { x: 1840, y: 240 },
    needs_human: { x: 1840, y: 460 },
  },
}

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
  switch (templateId) {
    case 'continuous_delivery':
      return instantiateContinuousDeliveryTemplate(projectId, agents, name)
    case 'gsd_auto':
      return instantiateGsdAutoTemplate(projectId, agents, name)
    case 'release_train':
      return instantiateReleaseTrainTemplate(projectId, agents, name)
    case 'bug_triage_fix_loop':
      return instantiateBugTriageTemplate(projectId, agents, name)
  }

  const exhaustive: never = templateId
  throw new Error(`Unsupported workflow template: ${exhaustive}`)
}

function templateNodePosition(templateId: WorkflowTemplateIdDto, nodeId: string): [number, number] {
  const position = WORKFLOW_TEMPLATE_DEFAULT_NODE_POSITIONS[templateId][nodeId]
  if (!position) throw new Error(`Missing default position for ${templateId}:${nodeId}`)
  return [position.x, position.y]
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
      agentNode('goal_intake', 'Goal intake', ...templateNodePosition('continuous_delivery', 'goal_intake'), planAgent, 'task_brief'),
      agentNode('plan', 'Plan', ...templateNodePosition('continuous_delivery', 'plan'), planAgent, 'plan', [
        runInputBinding('goal', 'Goal'),
        artifactBinding('goal_intake.task_brief', 'Goal intake'),
      ]),
      agentNode('work', 'Work', ...templateNodePosition('continuous_delivery', 'work'), workAgent, 'implementation_summary', [
        artifactBinding('plan.plan', 'Plan'),
      ]),
      agentNode('check', 'Check', ...templateNodePosition('continuous_delivery', 'check'), checkAgent, 'verification_result', [
        artifactBinding('work.implementation_summary', 'Implementation summary'),
      ]),
      routerNode('verification_router', 'Verification route', ...templateNodePosition('continuous_delivery', 'verification_router')),
      agentNode('gap_closure', 'Gap closure', ...templateNodePosition('continuous_delivery', 'gap_closure'), planAgent, 'gap_list', [
        artifactBinding('check.verification_result', 'Verification result'),
      ]),
      agentNode('debug', 'Debug', ...templateNodePosition('continuous_delivery', 'debug'), debugAgent, 'debug_report', [
        artifactBinding('work.implementation_summary', 'Implementation summary', false),
        artifactBinding('check.verification_result', 'Verification result', false),
      ]),
      agentNode('review', 'Review', ...templateNodePosition('continuous_delivery', 'review'), checkAgent, 'review_findings', [
        artifactBinding('check.verification_result', 'Verification result'),
      ]),
      routerNode('review_router', 'Review route', ...templateNodePosition('continuous_delivery', 'review_router')),
      agentNode('fix', 'Fix', ...templateNodePosition('continuous_delivery', 'fix'), workAgent, 'implementation_summary', [
        artifactBinding('review.review_findings', 'Review findings'),
      ]),
      agentNode('summary', 'Summary', ...templateNodePosition('continuous_delivery', 'summary'), summaryAgent, 'text_output', [
        artifactBinding('work.implementation_summary', 'Implementation summary'),
        artifactBinding('review.review_findings', 'Review findings', false),
      ]),
      humanCheckpointNode('human_verify', 'Human verification', ...templateNodePosition('continuous_delivery', 'human_verify')),
      terminalNode('success', 'Success', ...templateNodePosition('continuous_delivery', 'success'), 'success'),
      terminalNode('failed', 'Failed', ...templateNodePosition('continuous_delivery', 'failed'), 'failure'),
      terminalNode('needs_human', 'Needs human', ...templateNodePosition('continuous_delivery', 'needs_human'), 'needs_human'),
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

function instantiateGsdAutoTemplate(
  projectId: string,
  agents: readonly WorkflowAgentSummaryDto[],
  name?: string,
): WorkflowDefinitionDto {
  const planAgent = resolveBuiltInAgentRef(agents, 'plan')
  const workAgent = resolveBuiltInAgentRef(agents, 'engineer')
  const verifyAgent = resolveBuiltInAgentRef(agents, 'engineer')
  const auditAgent = resolveBuiltInAgentRef(agents, 'generalist')
  const id = createWorkflowId('gsd-auto')
  const milestoneIntakeSubgraph: WorkflowDefinitionDto['subgraphs'][number] = {
    id: 'gsd_milestone_intake',
    title: 'Milestone intake',
    description: 'Create or refresh the active milestone, requirement, and starter delivery phases.',
    startNodeId: 'milestone_intake',
    inputBindings: [runInputBinding('goal', 'Goal', true)],
    outputContract: subgraphResultOutputContract(),
    nodes: [
      agentNode('milestone_intake', 'Milestone intake', 40, 120, planAgent, 'task_brief', [
        runInputBinding('goal', 'Goal', true),
      ]),
      stateWriteNode('write_milestone', 'Write milestone', 340, 120, 'milestone', 'upsert', {
        id: 'gsd-current-milestone',
        title: '{{input.goal}}',
        summary: '{{input.goal}}',
        goal: '{{input.goal}}',
        status: 'active',
      }, 'gsd-current-milestone'),
      stateWriteNode('seed_requirement', 'Seed requirement', 640, 120, 'requirement', 'upsert', {
        id: 'gsd-requirement-1',
        milestoneId: '{{state.write_milestone.state_write_result.id}}',
        title: '{{input.goal}}',
        description: 'Primary requirement generated from the run goal.',
        status: 'open',
        priority: 100,
      }, 'gsd-requirement-1'),
      stateWriteNode('seed_phase_1', 'Seed phase 1', 940, 0, 'delivery_phase', 'upsert', {
        id: 'gsd-phase-1',
        milestoneId: '{{state.write_milestone.state_write_result.id}}',
        phaseKey: '1',
        title: 'Discuss and plan',
        summary: 'Establish scope, risks, and implementation path.',
        status: 'incomplete',
        sortOrder: 1,
      }, 'gsd-phase-1'),
      stateWriteNode('seed_phase_2', 'Seed phase 2', 1240, 0, 'delivery_phase', 'upsert', {
        id: 'gsd-phase-2',
        milestoneId: '{{state.write_milestone.state_write_result.id}}',
        phaseKey: '2',
        title: 'Implement',
        summary: 'Build the planned vertical slice.',
        status: 'incomplete',
        sortOrder: 2,
      }, 'gsd-phase-2'),
      stateWriteNode('seed_phase_3', 'Seed phase 3', 1540, 0, 'delivery_phase', 'upsert', {
        id: 'gsd-phase-3',
        milestoneId: '{{state.write_milestone.state_write_result.id}}',
        phaseKey: '3',
        title: 'Verify and polish',
        summary: 'Run checks, close gaps, and prepare the milestone audit.',
        status: 'incomplete',
        sortOrder: 3,
      }, 'gsd-phase-3'),
      terminalNode('milestone_ready', 'Milestone ready', 1840, 120, 'success'),
    ],
    edges: [
      edge('intake_to_write', 'milestone_intake', 'write_milestone', 'success', 'create', 10),
      edge('write_to_requirement', 'write_milestone', 'seed_requirement', 'success', 'requirement', 10),
      edge('requirement_to_seed_1', 'seed_requirement', 'seed_phase_1', 'success', 'seed', 10),
      edge('seed_1_to_seed_2', 'seed_phase_1', 'seed_phase_2', 'success', 'seed', 10),
      edge('seed_2_to_seed_3', 'seed_phase_2', 'seed_phase_3', 'success', 'seed', 10),
      edge('seed_3_to_ready', 'seed_phase_3', 'milestone_ready', 'success', 'ready', 10),
    ],
  }
  const phaseExecutionSubgraph: WorkflowDefinitionDto['subgraphs'][number] = {
    id: 'gsd_phase_execution',
    title: 'Phase execution',
    description: 'Discuss, plan, execute, verify, and close gaps for one selected delivery phase.',
    startNodeId: 'smart_discuss',
    inputBindings: [
      runInputBinding('goal', 'Goal', true),
      stateBinding('next_phase.collection_item', 'Delivery phase', true, '$.item'),
    ],
    outputContract: subgraphResultOutputContract(),
    nodes: [
      agentNode('smart_discuss', 'Smart discuss', 40, 80, planAgent, 'task_brief'),
      agentNode('phase_plan', 'Phase plan', 340, 80, planAgent, 'delivery_plan', [
        artifactBinding('smart_discuss.task_brief', 'Phase discussion'),
      ]),
      agentNode('phase_execute', 'Phase execute', 640, 80, workAgent, 'implementation_summary', [
        artifactBinding('phase_plan.delivery_plan', 'Phase plan'),
      ]),
      commandNode('verify_command', 'Verification command', 940, 80, 'git', ['status', '--short']),
      agentNode('phase_verify', 'Verification review', 1240, 80, verifyAgent, 'verification_result', [
        artifactBinding('phase_execute.implementation_summary', 'Implementation summary'),
        artifactBinding('verify_command.command_result', 'Command result'),
      ]),
      routerNode('verification_router', 'Verification route', 1540, 80),
      agentNode('gap_closure', 'Gap closure', 1540, 300, planAgent, 'gap_list', [
        artifactBinding('phase_verify.verification_result', 'Verification result'),
      ]),
      humanCheckpointNode('human_verify', 'Human verification', 1840, 300, ['passed', 'gaps_found', 'stop']),
      terminalNode('phase_processed', 'Phase processed', 1840, 80, 'success'),
      terminalNode('phase_needs_human', 'Needs human', 2140, 300, 'failure'),
    ],
    edges: [
      edge('discuss_to_plan', 'smart_discuss', 'phase_plan', 'success', 'plan', 10),
      edge('plan_to_execute', 'phase_plan', 'phase_execute', 'success', 'execute', 10),
      edge('execute_to_command', 'phase_execute', 'verify_command', 'success', 'check', 10),
      edge('command_to_verify', 'verify_command', 'phase_verify', 'success', 'review', 10),
      edge('verify_to_router', 'phase_verify', 'verification_router', 'success', 'route', 10),
      edge('verify_passed', 'verification_router', 'phase_processed', 'conditional', 'passed', 10, {
        kind: 'artifact_field_equals',
        artifactRef: 'phase_verify.verification_result',
        path: '$.status',
        value: 'passed',
      }),
      edge('verify_gaps_to_closure', 'verification_router', 'gap_closure', 'conditional', 'gaps', 20, {
        kind: 'artifact_field_in',
        artifactRef: 'phase_verify.verification_result',
        path: '$.status',
        values: ['gaps_found', 'needs_changes'],
      }),
      edge('verify_human_needed', 'verification_router', 'human_verify', 'conditional', 'needs human', 30, {
        kind: 'artifact_field_in',
        artifactRef: 'phase_verify.verification_result',
        path: '$.status',
        values: ['human_needed', 'failed'],
      }),
      loopEdge('gap_closure_to_execute', 'gap_closure', 'phase_execute', 'close gaps', 'gap_closure', 2, 'human_verify'),
      edge('human_verify_passed', 'human_verify', 'phase_processed', 'manual_override', 'passed', 10, {
        kind: 'human_decision_is',
        checkpointNodeId: 'human_verify',
        decision: 'passed',
      }),
      edge('human_verify_gaps', 'human_verify', 'gap_closure', 'manual_override', 'gaps', 20, {
        kind: 'human_decision_is',
        checkpointNodeId: 'human_verify',
        decision: 'gaps_found',
      }),
      edge('human_verify_stop', 'human_verify', 'phase_needs_human', 'manual_override', 'stop', 90, {
        kind: 'human_decision_is',
        checkpointNodeId: 'human_verify',
        decision: 'stop',
      }),
      edge('command_failed', 'verify_command', 'human_verify', 'failure', 'check failed', 10),
    ],
  }
  const milestoneAuditSubgraph: WorkflowDefinitionDto['subgraphs'][number] = {
    id: 'gsd_milestone_audit',
    title: 'Milestone audit and archive',
    description: 'Satisfy the seeded requirement, audit milestone coverage, complete the milestone, and archive it.',
    startNodeId: 'satisfy_requirement',
    inputBindings: [runInputBinding('goal', 'Goal', true)],
    outputContract: subgraphResultOutputContract(),
    nodes: [
      stateWriteNode('satisfy_requirement', 'Satisfy requirement', 40, 160, 'requirement', 'mark_complete', {}, undefined, 'gsd-requirement-1'),
      stateQueryNode('reload_milestones', 'Reload milestone', 340, 160, 'milestone', 'state_milestones', [
        { path: '$.status', operator: 'neq', value: 'archived', values: [] },
      ], '$.updatedAt', 1),
      stateQueryNode('query_remaining_phases', 'Remaining phases', 640, 160, 'delivery_phase', 'state_incomplete_phases', [
        { path: '$.status', operator: 'not_in', values: ['complete', 'archived'] },
      ], '$.sortOrder'),
      agentNode('audit_milestone', 'Audit milestone', 940, 160, auditAgent, 'milestone_audit', [
        runInputBinding('goal', 'Goal', true),
        stateBinding('reload_milestones.state_milestones', 'Milestone state', true),
        stateBinding('query_remaining_phases.state_incomplete_phases', 'Remaining phases', false),
      ]),
      routerNode('audit_router', 'Audit route', 1240, 160),
      stateWriteNode('complete_milestone', 'Complete milestone', 1540, 80, 'milestone', 'mark_complete', {}, undefined, '{{state.reload_milestones.state_milestones.records[0].id}}'),
      stateWriteNode('write_milestone_archive', 'Write archive', 1840, 80, 'milestone_archive', 'upsert', {
        id: '{{state.reload_milestones.state_milestones.records[0].id}}-archive',
        milestoneId: '{{state.reload_milestones.state_milestones.records[0].id}}',
        summary: '{{input.goal}}',
        goal: '{{input.goal}}',
        runId: '{{run.id}}',
      }, '{{state.reload_milestones.state_milestones.records[0].id}}-archive'),
      stateWriteNode('archive_milestone', 'Archive milestone', 2140, 80, 'milestone', 'archive', {}, undefined, '{{state.reload_milestones.state_milestones.records[0].id}}'),
      humanCheckpointNode('human_audit', 'Audit decision', 1540, 360, ['passed', 'gaps_found', 'stop']),
      terminalNode('milestone_archived', 'Milestone archived', 2440, 80, 'success'),
      terminalNode('audit_needs_human', 'Needs human', 1840, 360, 'failure'),
    ],
    edges: [
      edge('requirement_to_reload', 'satisfy_requirement', 'reload_milestones', 'success', 'reload', 10),
      edge('reload_to_remaining', 'reload_milestones', 'query_remaining_phases', 'success', 'remaining', 10),
      edge('remaining_to_audit', 'query_remaining_phases', 'audit_milestone', 'success', 'audit', 10),
      edge('audit_to_router', 'audit_milestone', 'audit_router', 'success', 'route', 10),
      edge('audit_passed', 'audit_router', 'complete_milestone', 'conditional', 'passed', 10, {
        kind: 'artifact_field_equals',
        artifactRef: 'audit_milestone.milestone_audit',
        path: '$.status',
        value: 'passed',
      }),
      edge('audit_needs_human', 'audit_router', 'human_audit', 'conditional', 'review', 20, {
        kind: 'artifact_field_in',
        artifactRef: 'audit_milestone.milestone_audit',
        path: '$.status',
        values: ['gaps_found', 'tech_debt', 'human_needed', 'failed'],
      }),
      edge('complete_to_write_archive', 'complete_milestone', 'write_milestone_archive', 'success', 'record archive', 10),
      edge('write_archive_to_archive', 'write_milestone_archive', 'archive_milestone', 'success', 'archive', 10),
      edge('archive_to_done', 'archive_milestone', 'milestone_archived', 'success', 'done', 10),
      edge('human_audit_passed', 'human_audit', 'complete_milestone', 'manual_override', 'passed', 10, {
        kind: 'human_decision_is',
        checkpointNodeId: 'human_audit',
        decision: 'passed',
      }),
      edge('human_audit_stop', 'human_audit', 'audit_needs_human', 'manual_override', 'stop', 20, {
        kind: 'any',
        conditions: [
          {
            kind: 'human_decision_is',
            checkpointNodeId: 'human_audit',
            decision: 'gaps_found',
          },
          {
            kind: 'human_decision_is',
            checkpointNodeId: 'human_audit',
            decision: 'stop',
          },
        ],
      }),
    ],
  }

  return withBaseDefinition({
    id,
    projectId,
    name: name?.trim() || 'GSD Auto',
    description:
      'A durable milestone workflow that discovers incomplete delivery phases, iterates them, verifies work, audits coverage, and archives completion state.',
    startNodeId: 'load_milestones',
    nodes: [
      stateQueryNode('load_milestones', 'Load milestones', ...templateNodePosition('gsd_auto', 'load_milestones'), 'milestone', 'state_milestones', [
        { path: '$.status', operator: 'neq', value: 'archived', values: [] },
      ], '$.updatedAt', 1),
      routerNode('milestone_route', 'Milestone route', ...templateNodePosition('gsd_auto', 'milestone_route')),
      subgraphNode('milestone_intake_flow', 'Milestone intake', ...templateNodePosition('gsd_auto', 'milestone_intake_flow'), 'gsd_milestone_intake', [
        runInputBinding('goal', 'Goal', true),
      ]),
      stateQueryNode('query_phases', 'Query incomplete phases', ...templateNodePosition('gsd_auto', 'query_phases'), 'delivery_phase', 'state_incomplete_phases', [
        { path: '$.status', operator: 'not_in', values: ['complete', 'archived'] },
      ], '$.sortOrder'),
      collectionLoopNode('next_phase', 'Next phase', ...templateNodePosition('gsd_auto', 'next_phase'), 'delivery_phase', [
        { path: '$.status', operator: 'not_in', values: ['complete', 'archived'] },
      ], {
        fromInputPath: '$.from',
        toInputPath: '$.to',
        onlyInputPath: '$.only',
      }),
      routerNode('phase_router', 'Phase route', ...templateNodePosition('gsd_auto', 'phase_router')),
      subgraphNode('process_phase_flow', 'Process phase', ...templateNodePosition('gsd_auto', 'process_phase_flow'), 'gsd_phase_execution', [
        runInputBinding('goal', 'Goal', true),
        runInputBinding('only', 'Only phase', false),
        runInputBinding('from', 'From phase', false),
        runInputBinding('to', 'To phase', false),
        stateBinding('next_phase.collection_item', 'Delivery phase', true, '$.item'),
      ]),
      stateWriteNode('mark_phase_complete', 'Mark phase complete', ...templateNodePosition('gsd_auto', 'mark_phase_complete'), 'delivery_phase', 'mark_complete', {}, undefined, '{{state.next_phase.collection_item.itemId}}'),
      subgraphNode('milestone_audit_flow', 'Audit and archive', ...templateNodePosition('gsd_auto', 'milestone_audit_flow'), 'gsd_milestone_audit', [
        runInputBinding('goal', 'Goal', true),
      ]),
      humanCheckpointNode('next_milestone_offer', 'Next milestone', ...templateNodePosition('gsd_auto', 'next_milestone_offer'), ['finish', 'start_next', 'pause']),
      terminalNode('success', 'Success', ...templateNodePosition('gsd_auto', 'success'), 'success'),
      terminalNode('partial_success', 'Partial run complete', ...templateNodePosition('gsd_auto', 'partial_success'), 'success'),
      terminalNode('needs_human', 'Needs human', ...templateNodePosition('gsd_auto', 'needs_human'), 'needs_human'),
    ],
    edges: [
      edge('load_to_route', 'load_milestones', 'milestone_route', 'success', 'route', 10),
      edge('milestone_exists', 'milestone_route', 'query_phases', 'conditional', 'continue', 10, {
        kind: 'state_collection_count_compare',
        stateRef: 'load_milestones.state_milestones',
        operator: 'gt',
        value: 0,
      }),
      edge('milestone_missing', 'milestone_route', 'milestone_intake_flow', 'conditional', 'intake', 20),
      edge('intake_to_query', 'milestone_intake_flow', 'query_phases', 'success', 'query', 10),
      edge('intake_failed', 'milestone_intake_flow', 'needs_human', 'failure', 'blocked', 10),
      edge('query_to_loop', 'query_phases', 'next_phase', 'success', 'select', 10),
      edge('loop_to_router', 'next_phase', 'phase_router', 'success', 'route', 10),
      edge('phase_available', 'phase_router', 'process_phase_flow', 'conditional', 'phase', 10, {
        kind: 'artifact_field_equals',
        artifactRef: 'next_phase.collection_item',
        path: '$.hasItem',
        value: true,
      }),
      edge('partial_run_done', 'phase_router', 'partial_success', 'conditional', 'partial done', 20, {
        kind: 'all',
        conditions: [
          {
            kind: 'artifact_field_equals',
            artifactRef: 'next_phase.collection_item',
            path: '$.hasItem',
            value: false,
          },
          {
            kind: 'artifact_field_equals',
            artifactRef: 'next_phase.collection_item',
            path: '$.partialSelection',
            value: true,
          },
        ],
      }),
      edge('no_phase_to_audit', 'phase_router', 'milestone_audit_flow', 'conditional', 'audit', 30, {
        kind: 'artifact_field_equals',
        artifactRef: 'next_phase.collection_item',
        path: '$.hasItem',
        value: false,
      }),
      edge('phase_processed', 'process_phase_flow', 'mark_phase_complete', 'success', 'complete phase', 10),
      edge('phase_blocked', 'process_phase_flow', 'needs_human', 'failure', 'blocked', 10),
      edge('phase_complete_to_query', 'mark_phase_complete', 'query_phases', 'loop', 'next phase', 10, { kind: 'always' }, {
        loopKey: 'delivery_phase_iteration',
        maxAttempts: 100,
        attemptScope: 'run',
        carryoverPolicy: 'all',
        selectedArtifactRefs: [],
        resetPolicy: 'never',
        stallDetector: 'no_artifact_progress',
        onExhausted: 'needs_human',
      }),
      edge('audit_to_next_milestone', 'milestone_audit_flow', 'next_milestone_offer', 'success', 'next', 10),
      edge('audit_blocked', 'milestone_audit_flow', 'needs_human', 'failure', 'blocked', 10),
      edge('next_milestone_finish', 'next_milestone_offer', 'success', 'manual_override', 'finish', 10, {
        kind: 'human_decision_is',
        checkpointNodeId: 'next_milestone_offer',
        decision: 'finish',
      }),
      edge('next_milestone_start', 'next_milestone_offer', 'milestone_intake_flow', 'loop', 'start next', 20, {
        kind: 'human_decision_is',
        checkpointNodeId: 'next_milestone_offer',
        decision: 'start_next',
      }, {
        loopKey: 'next_milestone',
        maxAttempts: 10,
        attemptScope: 'run',
        carryoverPolicy: 'all',
        selectedArtifactRefs: [],
        resetPolicy: 'never',
        stallDetector: 'no_artifact_progress',
        onExhausted: 'needs_human',
      }),
      edge('next_milestone_pause', 'next_milestone_offer', 'needs_human', 'manual_override', 'pause', 30, {
        kind: 'human_decision_is',
        checkpointNodeId: 'next_milestone_offer',
        decision: 'pause',
      }),
    ],
    subgraphs: [milestoneIntakeSubgraph, phaseExecutionSubgraph, milestoneAuditSubgraph],
  })
}

function instantiateReleaseTrainTemplate(
  projectId: string,
  agents: readonly WorkflowAgentSummaryDto[],
  name?: string,
): WorkflowDefinitionDto {
  const reviewAgent = resolveBuiltInAgentRef(agents, 'generalist')
  const id = createWorkflowId('release-train')
  return withBaseDefinition({
    id,
    projectId,
    name: name?.trim() || 'Release train',
    description: 'Verify and archive release candidates using the same state, collection, command, and audit primitives.',
    startNodeId: 'query_candidates',
    nodes: [
      stateQueryNode('query_candidates', 'Query candidates', ...templateNodePosition('release_train', 'query_candidates'), 'delivery_phase', 'state_release_candidates', [
        { path: '$.status', operator: 'eq', value: 'complete', values: [] },
      ], '$.sortOrder'),
      collectionLoopNode('next_candidate', 'Next candidate', ...templateNodePosition('release_train', 'next_candidate'), 'delivery_phase', [
        { path: '$.status', operator: 'eq', value: 'complete', values: [] },
      ]),
      routerNode('candidate_route', 'Candidate route', ...templateNodePosition('release_train', 'candidate_route')),
      commandNode('release_check', 'Release check', ...templateNodePosition('release_train', 'release_check'), 'git', ['status', '--short']),
      agentNode('release_review', 'Release review', ...templateNodePosition('release_train', 'release_review'), reviewAgent, 'review_findings', [
        stateBinding('next_candidate.collection_item', 'Release candidate', true, '$.item'),
        artifactBinding('release_check.command_result', 'Command result'),
      ]),
      stateWriteNode('archive_evidence', 'Archive evidence', ...templateNodePosition('release_train', 'archive_evidence'), 'verification_evidence', 'upsert', {
        id: '{{state.next_candidate.collection_item.itemId}}-release-check',
        phaseId: '{{state.next_candidate.collection_item.itemId}}',
        status: 'passed',
        summary: 'Release candidate checked and recorded.',
      }),
      terminalNode('done', 'Done', ...templateNodePosition('release_train', 'done'), 'success'),
      humanCheckpointNode('release_human', 'Release decision', ...templateNodePosition('release_train', 'release_human')),
      terminalNode('needs_human', 'Needs human', ...templateNodePosition('release_train', 'needs_human'), 'needs_human'),
    ],
    edges: [
      edge('query_to_next', 'query_candidates', 'next_candidate', 'success', 'select', 10),
      edge('next_to_route', 'next_candidate', 'candidate_route', 'success', 'route', 10),
      edge('candidate_available', 'candidate_route', 'release_check', 'conditional', 'check', 10, {
        kind: 'artifact_field_equals',
        artifactRef: 'next_candidate.collection_item',
        path: '$.hasItem',
        value: true,
      }),
      edge('candidates_done', 'candidate_route', 'done', 'conditional', 'done', 20, {
        kind: 'artifact_field_equals',
        artifactRef: 'next_candidate.collection_item',
        path: '$.hasItem',
        value: false,
      }),
      edge('check_to_review', 'release_check', 'release_review', 'success', 'review', 10),
      edge('review_to_archive', 'release_review', 'archive_evidence', 'success', 'record', 10),
      loopEdge('archive_to_query', 'archive_evidence', 'query_candidates', 'next candidate', 'release_candidate_iteration', 50, 'release_human'),
      edge('check_failed_to_human', 'release_check', 'release_human', 'failure', 'human', 10),
      edge('human_to_needs_human', 'release_human', 'needs_human', 'manual_override', 'pause', 10),
    ],
  })
}

function instantiateBugTriageTemplate(
  projectId: string,
  agents: readonly WorkflowAgentSummaryDto[],
  name?: string,
): WorkflowDefinitionDto {
  const planAgent = resolveBuiltInAgentRef(agents, 'plan')
  const workAgent = resolveBuiltInAgentRef(agents, 'engineer')
  const id = createWorkflowId('bug-triage')
  return withBaseDefinition({
    id,
    projectId,
    name: name?.trim() || 'Bug triage and fix',
    description: 'Process open requirement records as defects, fix one, verify it, and close it idempotently.',
    startNodeId: 'query_bugs',
    nodes: [
      stateQueryNode('query_bugs', 'Query open bugs', ...templateNodePosition('bug_triage_fix_loop', 'query_bugs'), 'requirement', 'state_open_bugs', [
        { path: '$.status', operator: 'eq', value: 'open', values: [] },
      ], '$.priority'),
      collectionLoopNode('next_bug', 'Next bug', ...templateNodePosition('bug_triage_fix_loop', 'next_bug'), 'requirement', [
        { path: '$.status', operator: 'eq', value: 'open', values: [] },
      ]),
      routerNode('bug_route', 'Bug route', ...templateNodePosition('bug_triage_fix_loop', 'bug_route')),
      agentNode('triage', 'Triage', ...templateNodePosition('bug_triage_fix_loop', 'triage'), planAgent, 'task_brief', [
        stateBinding('next_bug.collection_item', 'Bug record', true, '$.item'),
      ]),
      agentNode('fix_bug', 'Fix bug', ...templateNodePosition('bug_triage_fix_loop', 'fix_bug'), workAgent, 'implementation_summary', [
        artifactBinding('triage.task_brief', 'Triage brief'),
      ]),
      commandNode('bug_check', 'Bug check', ...templateNodePosition('bug_triage_fix_loop', 'bug_check'), 'git', ['status', '--short']),
      stateWriteNode('close_bug', 'Close bug', ...templateNodePosition('bug_triage_fix_loop', 'close_bug'), 'requirement', 'mark_complete', {}, undefined, '{{state.next_bug.collection_item.itemId}}'),
      humanCheckpointNode('bug_human', 'Bug decision', ...templateNodePosition('bug_triage_fix_loop', 'bug_human')),
      terminalNode('done', 'Done', ...templateNodePosition('bug_triage_fix_loop', 'done'), 'success'),
      terminalNode('needs_human', 'Needs human', ...templateNodePosition('bug_triage_fix_loop', 'needs_human'), 'needs_human'),
    ],
    edges: [
      edge('query_to_next', 'query_bugs', 'next_bug', 'success', 'select', 10),
      edge('next_to_route', 'next_bug', 'bug_route', 'success', 'route', 10),
      edge('bug_available', 'bug_route', 'triage', 'conditional', 'triage', 10, {
        kind: 'artifact_field_equals',
        artifactRef: 'next_bug.collection_item',
        path: '$.hasItem',
        value: true,
      }),
      edge('bugs_done', 'bug_route', 'done', 'conditional', 'done', 20, {
        kind: 'artifact_field_equals',
        artifactRef: 'next_bug.collection_item',
        path: '$.hasItem',
        value: false,
      }),
      edge('triage_to_fix', 'triage', 'fix_bug', 'success', 'fix', 10),
      edge('fix_to_check', 'fix_bug', 'bug_check', 'success', 'check', 10),
      edge('check_to_close', 'bug_check', 'close_bug', 'success', 'close', 10),
      loopEdge('close_to_query', 'close_bug', 'query_bugs', 'next bug', 'bug_iteration', 50, 'bug_human'),
      edge('check_failed_to_human', 'bug_check', 'bug_human', 'failure', 'human', 10),
      edge('human_to_needs_human', 'bug_human', 'needs_human', 'manual_override', 'pause', 10),
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
  subgraphs?: WorkflowDefinitionDto['subgraphs']
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
    subgraphs: params.subgraphs ?? [],
    artifactContracts: [
      artifactContract('text_output', 'Text output', textOutputSchema()),
      artifactContract('task_brief', 'Task brief', taskBriefSchema()),
      artifactContract('delivery_plan', 'Delivery plan', deliveryPlanSchema()),
      artifactContract('plan', 'Plan', deliveryPlanSchema()),
      artifactContract('implementation_summary', 'Implementation summary', implementationSummarySchema()),
      artifactContract('verification_result', 'Verification result', verificationResultSchema()),
      artifactContract('debug_report', 'Debug report', debugReportSchema()),
      artifactContract('gap_list', 'Gap list', gapListSchema()),
      artifactContract('review_findings', 'Review findings', reviewFindingsSchema()),
      artifactContract('human_decision', 'Human decision', humanDecisionSchema()),
      artifactContract('milestone_audit', 'Milestone audit', milestoneAuditSchema()),
      artifactContract('command_result', 'Command result', commandResultSchema()),
      artifactContract('subgraph_result', 'Subgraph result', subgraphResultSchema()),
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

function subgraphNode(
  id: string,
  title: string,
  x: number,
  y: number,
  subgraphId: string,
  inputBindings: Extract<WorkflowNodeDto, { type: 'subgraph' }>['inputBindings'] = [],
): WorkflowNodeDto {
  return {
    id,
    title,
    description: '',
    position: { x, y },
    type: 'subgraph',
    subgraphId,
    inputBindings,
    outputContract: subgraphResultOutputContract(),
  }
}

function subgraphResultOutputContract(): Extract<WorkflowNodeDto, { type: 'subgraph' }>['outputContract'] {
  return {
    artifactType: 'subgraph_result',
    schemaVersion: 1,
    extraction: 'json_object',
    required: true,
    renderTextPath: '$.summary',
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
      renderTextPath: defaultRenderTextPath(artifactType),
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

function stateQueryNode(
  id: string,
  title: string,
  x: number,
  y: number,
  entityType: Extract<WorkflowNodeDto, { type: 'state_query' }>['query']['entityType'],
  outputArtifactType: string,
  filters: Extract<WorkflowNodeDto, { type: 'state_query' }>['query']['filters'] = [],
  orderBy?: string,
  limit?: number,
): WorkflowNodeDto {
  return {
    id,
    title,
    description: '',
    position: { x, y },
    type: 'state_query',
    query: {
      entityType,
      filters,
      orderBy: orderBy ?? null,
      limit: limit ?? null,
      includeArchived: false,
    },
    outputArtifactType,
  }
}

function stateWriteNode(
  id: string,
  title: string,
  x: number,
  y: number,
  entityType: Extract<WorkflowNodeDto, { type: 'state_write' }>['operation']['entityType'],
  action: Extract<WorkflowNodeDto, { type: 'state_write' }>['operation']['action'],
  payload: Record<string, unknown>,
  idempotencyKey?: string,
  targetId?: string,
): WorkflowNodeDto {
  return {
    id,
    title,
    description: '',
    position: { x, y },
    type: 'state_write',
    inputBindings: [],
    operation: {
      entityType,
      action,
      idempotencyKey: idempotencyKey ?? null,
      targetId: targetId ?? null,
      payload,
      outputArtifactType: 'state_write_result',
    },
  }
}

function collectionLoopNode(
  id: string,
  title: string,
  x: number,
  y: number,
  entityType: Extract<WorkflowNodeDto, { type: 'collection_loop' }>['collection']['entityType'],
  filters: Extract<WorkflowNodeDto, { type: 'collection_loop' }>['collection']['filters'] = [],
  controls: Extract<WorkflowNodeDto, { type: 'collection_loop' }>['controls'] = {},
): WorkflowNodeDto {
  return {
    id,
    title,
    description: '',
    position: { x, y },
    type: 'collection_loop',
    collection: {
      entityType,
      filters,
      orderBy: entityType === 'requirement' ? '$.priority' : '$.sortOrder',
      limit: null,
      includeArchived: false,
    },
    itemArtifactType: 'collection_item',
    itemVariableName: 'item',
    sortKey: '$.sortOrder',
    afterItemRequery: true,
    maxItemCount: 100,
    maxRuntimeSeconds: null,
    controls,
  }
}

function commandNode(
  id: string,
  title: string,
  x: number,
  y: number,
  command: string,
  args: string[],
): WorkflowNodeDto {
  return {
    id,
    title,
    description: '',
    position: { x, y },
    type: 'command',
    command,
    args,
    allowedCommands: [command],
    workingDirectory: null,
    timeoutSeconds: 120,
    successExitCodes: [0],
    outputContract: {
      artifactType: 'command_result',
      schemaVersion: 1,
      extraction: 'json_object',
      required: true,
      renderTextPath: '$.stdout',
    },
    parser: {
      extraction: 'generic_text',
      renderTextPath: '$.stdout',
    },
  }
}

function humanCheckpointNode(
  id: string,
  title: string,
  x: number,
  y: number,
  decisionOptions: string[] = ['continue', 'stop'],
): WorkflowNodeDto {
  return {
    id,
    title,
    description: 'Pause the Workflow for user judgment before continuing.',
    position: { x, y },
    type: 'human_checkpoint',
    checkpointType: 'human_verify',
    prompt: 'Review the current artifacts and choose how the Workflow should continue.',
    decisionOptions,
    resumePayloadSchema: null,
    stateUpdates: [],
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
  required = false,
): Extract<WorkflowNodeDto, { type: 'agent' }>['inputBindings'][number] {
  return {
    source: 'run_input',
    name,
    required,
    path: `$.${name}`,
    promptLabel,
  }
}

function stateBinding(
  stateRef: string,
  promptLabel: string,
  required = true,
  path?: string,
): Extract<WorkflowNodeDto, { type: 'agent' }>['inputBindings'][number] {
  return {
    source: 'state',
    name: stateRef.replace(/[^A-Za-z0-9_]+/g, '_'),
    required,
    stateRef,
    path,
    promptLabel,
  }
}

function artifactContract(artifactType: string, displayName: string, jsonSchema: Record<string, unknown>) {
  return {
    artifactType,
    schemaVersion: 1,
    jsonSchema,
    displayName,
    description: '',
  }
}

function defaultRenderTextPath(artifactType: string): string | null {
  if (artifactType === 'text_output') return null
  if (artifactType === 'gap_list') return '$.summary'
  if (artifactType === 'review_findings') return '$.summary'
  if (artifactType === 'verification_result') return '$.summary'
  if (artifactType === 'debug_report') return '$.summary'
  if (artifactType === 'human_decision') return '$.decision'
  if (artifactType === 'milestone_audit') return '$.summary'
  return '$.summary'
}

function textOutputSchema(): Record<string, unknown> {
  return objectSchema({
    text: { type: 'string', minLength: 1 },
  }, ['text'])
}

function taskBriefSchema(): Record<string, unknown> {
  return objectSchema({
    summary: { type: 'string', minLength: 1 },
    goal: { type: 'string', minLength: 1 },
    constraints: { type: 'array', items: { type: 'string' } },
    acceptanceCriteria: { type: 'array', items: { type: 'string' } },
  }, ['summary', 'goal'])
}

function deliveryPlanSchema(): Record<string, unknown> {
  return objectSchema({
    summary: { type: 'string', minLength: 1 },
    steps: {
      type: 'array',
      minItems: 1,
      items: objectSchema({
        id: { type: 'string', minLength: 1 },
        title: { type: 'string', minLength: 1 },
        status: { type: 'string', enum: ['pending', 'in_progress', 'blocked', 'done'] },
      }, ['id', 'title', 'status']),
    },
    risks: { type: 'array', items: { type: 'string' } },
  }, ['summary', 'steps'])
}

function implementationSummarySchema(): Record<string, unknown> {
  return objectSchema({
    summary: { type: 'string', minLength: 1 },
    changedFiles: { type: 'array', items: { type: 'string' } },
    testsRun: { type: 'array', items: { type: 'string' } },
    followUps: { type: 'array', items: { type: 'string' } },
  }, ['summary'])
}

function verificationResultSchema(): Record<string, unknown> {
  return objectSchema({
    status: { type: 'string', enum: ['passed', 'gaps_found', 'needs_changes', 'human_needed', 'failed'] },
    summary: { type: 'string', minLength: 1 },
    evidence: {
      type: 'array',
      items: objectSchema({
        command: { type: 'string' },
        status: { type: 'string', enum: ['passed', 'failed', 'skipped'] },
        output: { type: 'string' },
      }, ['status']),
    },
    gaps: {
      type: 'array',
      items: objectSchema({
        id: { type: 'string', minLength: 1 },
        summary: { type: 'string', minLength: 1 },
        severity: { type: 'string', enum: ['low', 'medium', 'high', 'critical'] },
      }, ['id', 'summary']),
    },
  }, ['status', 'summary'])
}

function debugReportSchema(): Record<string, unknown> {
  return objectSchema({
    summary: { type: 'string', minLength: 1 },
    failureClass: { type: 'string' },
    recommendedRoute: { type: 'string', enum: ['retry_work', 'ask_human', 'fail'] },
    recommended_route: { type: 'string', enum: ['retry_work', 'ask_human', 'fail'] },
    diagnostics: { type: 'array', items: { type: 'string' } },
  }, ['summary', 'recommended_route'])
}

function gapListSchema(): Record<string, unknown> {
  return objectSchema({
    summary: { type: 'string', minLength: 1 },
    gaps: {
      type: 'array',
      items: objectSchema({
        id: { type: 'string', minLength: 1 },
        summary: { type: 'string', minLength: 1 },
        severity: { type: 'string', enum: ['low', 'medium', 'high', 'critical'] },
        status: { type: 'string', enum: ['open', 'closed', 'deferred'] },
      }, ['id', 'summary']),
    },
  }, ['summary', 'gaps'])
}

function reviewFindingsSchema(): Record<string, unknown> {
  return objectSchema({
    summary: { type: 'string', minLength: 1 },
    high_count: { type: 'integer' },
    findings: {
      type: 'array',
      items: objectSchema({
        id: { type: 'string', minLength: 1 },
        severity: { type: 'string', enum: ['low', 'medium', 'high', 'critical'] },
        summary: { type: 'string', minLength: 1 },
      }, ['id', 'severity', 'summary']),
    },
  }, ['summary', 'high_count', 'findings'])
}

function humanDecisionSchema(): Record<string, unknown> {
  return objectSchema({
    decision: { type: 'string', minLength: 1 },
    payload: {},
  }, ['decision'])
}

function milestoneAuditSchema(): Record<string, unknown> {
  return objectSchema({
    status: { type: 'string', enum: ['passed', 'gaps_found', 'tech_debt', 'human_needed', 'failed'] },
    summary: { type: 'string', minLength: 1 },
    unsatisfiedRequirements: { type: 'array', items: { type: 'string' } },
    evidence: { type: 'array', items: { type: 'string' } },
  }, ['status', 'summary', 'unsatisfiedRequirements'])
}

function commandResultSchema(): Record<string, unknown> {
  return objectSchema({
    status: { type: 'string', enum: ['passed', 'failed'] },
    command: { type: 'string', minLength: 1 },
    args: { type: 'array', items: { type: 'string' } },
    workingDirectory: { type: 'string' },
    exitCode: { type: 'integer' },
    timedOut: { type: 'boolean' },
    stdout: { type: 'string' },
    stderr: { type: 'string' },
    parsed: {},
    parseError: { type: ['string', 'null'] },
  }, ['status', 'command', 'args', 'exitCode', 'timedOut', 'stdout', 'stderr'])
}

function subgraphResultSchema(): Record<string, unknown> {
  return objectSchema({
    status: { type: 'string', enum: ['succeeded', 'failed', 'cancelled', 'needs_human'] },
    subgraphId: { type: 'string', minLength: 1 },
    summary: { type: 'string', minLength: 1 },
  }, ['status', 'subgraphId', 'summary'])
}

function objectSchema(
  properties: Record<string, unknown>,
  required: string[] = [],
): Record<string, unknown> {
  return {
    type: 'object',
    required,
    properties,
    additionalProperties: false,
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
