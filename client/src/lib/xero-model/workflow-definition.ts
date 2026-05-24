import { z } from 'zod'

import { isoTimestampSchema } from '@xero/ui/model/shared'
import { runtimeRunApprovalModeSchema } from '@xero/ui/model/runtime'
import { agentRefSchema } from './workflow-agents'

const idSchema = z.string().trim().min(1).max(120).regex(/^[A-Za-z0-9][A-Za-z0-9_.:-]*$/)
const nonEmptyTextSchema = z.string().trim().min(1)
const optionalTextSchema = z.string().trim().optional().default('')
const jsonValueSchema = z.union([
  z.string(),
  z.number(),
  z.boolean(),
  z.null(),
  z.array(z.unknown()),
  z.record(z.unknown()),
])

export const workflowNodeIdSchema = idSchema
export const workflowEdgeIdSchema = idSchema

export const workflowNodeTypeSchema = z.enum([
  'agent',
  'router',
  'gate',
  'human_checkpoint',
  'merge',
  'terminal',
])
export type WorkflowNodeTypeDto = z.infer<typeof workflowNodeTypeSchema>

export const workflowEdgeTypeSchema = z.enum([
  'success',
  'failure',
  'conditional',
  'loop',
  'recovery',
  'manual_override',
])
export type WorkflowEdgeTypeDto = z.infer<typeof workflowEdgeTypeSchema>

export const workflowArtifactTypeSchema = z.string().trim().min(1).max(120)
export type WorkflowArtifactTypeDto = z.infer<typeof workflowArtifactTypeSchema>

export const workflowArtifactPresetSchema = z.enum([
  'text_output',
  'task_brief',
  'plan',
  'implementation_summary',
  'verification_result',
  'debug_report',
  'gap_list',
  'review_findings',
  'human_decision',
])
export type WorkflowArtifactPresetDto = z.infer<typeof workflowArtifactPresetSchema>

export const workflowNodeRunStatusSchema = z.enum([
  'pending',
  'eligible',
  'starting',
  'running',
  'waiting_on_gate',
  'succeeded',
  'failed',
  'stalled',
  'skipped',
  'cancelled',
])
export type WorkflowNodeRunStatusDto = z.infer<typeof workflowNodeRunStatusSchema>

export const workflowRunStatusSchema = z.enum([
  'queued',
  'running',
  'paused',
  'completed',
  'failed',
  'cancelled',
])
export type WorkflowRunStatusDto = z.infer<typeof workflowRunStatusSchema>

export const workflowTerminalStatusSchema = z.enum([
  'success',
  'failure',
  'cancelled',
  'needs_human',
])
export type WorkflowTerminalStatusDto = z.infer<typeof workflowTerminalStatusSchema>

export const workflowHumanCheckpointTypeSchema = z.enum([
  'human_verify',
  'decision',
  'human_action',
])
export type WorkflowHumanCheckpointTypeDto = z.infer<typeof workflowHumanCheckpointTypeSchema>

export const workflowAttemptScopeSchema = z.enum([
  'run',
  'source_node',
  'target_node',
  'artifact_group',
])
export type WorkflowAttemptScopeDto = z.infer<typeof workflowAttemptScopeSchema>

export const workflowCarryoverPolicySchema = z.enum([
  'all',
  'required_only',
  'none',
  'selected',
])
export type WorkflowCarryoverPolicyDto = z.infer<typeof workflowCarryoverPolicySchema>

export const workflowResetPolicySchema = z.enum([
  'never',
  'on_downstream_success',
  'on_terminal_success',
])
export type WorkflowResetPolicyDto = z.infer<typeof workflowResetPolicySchema>

export const workflowStallDetectorSchema = z.enum([
  'finding_count_not_decreasing',
  'same_failure_class_repeated',
  'no_artifact_progress',
  'runtime_activity_timeout',
  'retry_limit_exceeded',
])
export type WorkflowStallDetectorDto = z.infer<typeof workflowStallDetectorSchema>

export const workflowMergeWaitPolicySchema = z.enum([
  'all',
  'any',
  'quorum',
  'fail_fast',
])
export type WorkflowMergeWaitPolicyDto = z.infer<typeof workflowMergeWaitPolicySchema>

export const workflowResourceConflictModeSchema = z.enum([
  'allow_conflicts',
  'serialize_conflicts',
])
export type WorkflowResourceConflictModeDto = z.infer<
  typeof workflowResourceConflictModeSchema
>

export const workflowNumberCompareOperatorSchema = z.enum([
  'eq',
  'neq',
  'gt',
  'gte',
  'lt',
  'lte',
])
export type WorkflowNumberCompareOperatorDto = z.infer<
  typeof workflowNumberCompareOperatorSchema
>

export type WorkflowConditionDto =
  | { kind: 'always' }
  | { kind: 'all'; conditions: WorkflowConditionDto[] }
  | { kind: 'any'; conditions: WorkflowConditionDto[] }
  | { kind: 'not'; condition: WorkflowConditionDto }
  | { kind: 'node_status'; nodeId: string; status: WorkflowNodeRunStatusDto }
  | { kind: 'artifact_exists'; artifactRef: string }
  | { kind: 'artifact_field_equals'; artifactRef: string; path: string; value: unknown }
  | { kind: 'artifact_field_in'; artifactRef: string; path: string; values: unknown[] }
  | {
      kind: 'artifact_field_number_compare'
      artifactRef: string
      path: string
      operator: WorkflowNumberCompareOperatorDto
      value: number
    }
  | { kind: 'failure_class_is'; nodeId?: string | null; failureClass: string }
  | { kind: 'loop_attempt_lt'; loopKey: string; value: number }
  | { kind: 'loop_attempt_gte'; loopKey: string; value: number }
  | { kind: 'human_decision_is'; checkpointNodeId: string; decision: string }

export const workflowConditionSchema: z.ZodType<WorkflowConditionDto> = z.lazy(() =>
  z.discriminatedUnion('kind', [
    z.object({ kind: z.literal('always') }).strict(),
    z
      .object({
        kind: z.literal('all'),
        conditions: z.array(workflowConditionSchema).min(1),
      })
      .strict(),
    z
      .object({
        kind: z.literal('any'),
        conditions: z.array(workflowConditionSchema).min(1),
      })
      .strict(),
    z
      .object({
        kind: z.literal('not'),
        condition: workflowConditionSchema,
      })
      .strict(),
    z
      .object({
        kind: z.literal('node_status'),
        nodeId: workflowNodeIdSchema,
        status: workflowNodeRunStatusSchema,
      })
      .strict(),
    z
      .object({
        kind: z.literal('artifact_exists'),
        artifactRef: nonEmptyTextSchema,
      })
      .strict(),
    z
      .object({
        kind: z.literal('artifact_field_equals'),
        artifactRef: nonEmptyTextSchema,
        path: nonEmptyTextSchema,
        value: jsonValueSchema,
      })
      .strict(),
    z
      .object({
        kind: z.literal('artifact_field_in'),
        artifactRef: nonEmptyTextSchema,
        path: nonEmptyTextSchema,
        values: z.array(jsonValueSchema).min(1),
      })
      .strict(),
    z
      .object({
        kind: z.literal('artifact_field_number_compare'),
        artifactRef: nonEmptyTextSchema,
        path: nonEmptyTextSchema,
        operator: workflowNumberCompareOperatorSchema,
        value: z.number(),
      })
      .strict(),
    z
      .object({
        kind: z.literal('failure_class_is'),
        nodeId: workflowNodeIdSchema.nullable().optional(),
        failureClass: nonEmptyTextSchema,
      })
      .strict(),
    z
      .object({
        kind: z.literal('loop_attempt_lt'),
        loopKey: nonEmptyTextSchema,
        value: z.number().int().nonnegative(),
      })
      .strict(),
    z
      .object({
        kind: z.literal('loop_attempt_gte'),
        loopKey: nonEmptyTextSchema,
        value: z.number().int().nonnegative(),
      })
      .strict(),
    z
      .object({
        kind: z.literal('human_decision_is'),
        checkpointNodeId: workflowNodeIdSchema,
        decision: nonEmptyTextSchema,
      })
      .strict(),
  ]),
)

export const workflowPositionSchema = z
  .object({
    x: z.number(),
    y: z.number(),
  })
  .strict()
export type WorkflowPositionDto = z.infer<typeof workflowPositionSchema>

export const workflowRunOverrideSchema = z
  .object({
    providerProfileId: z.string().trim().min(1).nullable().optional(),
    modelId: z.string().trim().min(1).nullable().optional(),
    thinkingEffort: z.string().trim().min(1).nullable().optional(),
    approvalMode: runtimeRunApprovalModeSchema.nullable().optional(),
    promptPreface: z.string().optional().default(''),
    planModeRequired: z.boolean().default(false),
    autoCompactEnabled: z.boolean().default(true),
  })
  .strict()
export type WorkflowRunOverrideDto = z.infer<typeof workflowRunOverrideSchema>

export const workflowInputBindingSchema = z.discriminatedUnion('source', [
  z
    .object({
      source: z.literal('run_input'),
      name: nonEmptyTextSchema,
      required: z.boolean().default(true),
      path: z.string().trim().min(1).nullable().optional(),
      promptLabel: z.string().trim().min(1).nullable().optional(),
    })
    .strict(),
  z
    .object({
      source: z.literal('artifact'),
      name: nonEmptyTextSchema,
      required: z.boolean().default(true),
      artifactRef: nonEmptyTextSchema,
      path: z.string().trim().min(1).nullable().optional(),
      promptLabel: z.string().trim().min(1).nullable().optional(),
    })
    .strict(),
])
export type WorkflowInputBindingDto = z.infer<typeof workflowInputBindingSchema>

export const workflowOutputExtractionSchema = z.enum([
  'generic_text',
  'json_object',
  'json_array',
])
export type WorkflowOutputExtractionDto = z.infer<typeof workflowOutputExtractionSchema>

export const workflowOutputContractSchema = z
  .object({
    artifactType: workflowArtifactTypeSchema,
    schemaVersion: z.number().int().positive().default(1),
    extraction: workflowOutputExtractionSchema.default('generic_text'),
    required: z.boolean().default(true),
    renderTextPath: z.string().trim().min(1).nullable().optional(),
  })
  .strict()
export type WorkflowOutputContractDto = z.infer<typeof workflowOutputContractSchema>

export const workflowFailureClassificationPolicySchema = z
  .object({
    runtimeActivityTimeoutSeconds: z.number().int().positive().nullable().optional(),
    quotaFailureClasses: z.array(nonEmptyTextSchema).default([]),
    transientFailureClasses: z.array(nonEmptyTextSchema).default([]),
  })
  .strict()
export type WorkflowFailureClassificationPolicyDto = z.infer<
  typeof workflowFailureClassificationPolicySchema
>

const workflowNodeBaseSchema = z.object({
  id: workflowNodeIdSchema,
  title: nonEmptyTextSchema,
  description: optionalTextSchema,
  position: workflowPositionSchema.default({ x: 0, y: 0 }),
})

export const workflowAgentNodeSchema = workflowNodeBaseSchema
  .extend({
    type: z.literal('agent'),
    agentRef: agentRefSchema,
    displayLabel: z.string().trim().min(1).nullable().optional(),
    inputBindings: z.array(workflowInputBindingSchema).default([]),
    outputContract: workflowOutputContractSchema.default({
      artifactType: 'text_output',
      schemaVersion: 1,
      extraction: 'generic_text',
      required: true,
    }),
    runOverrides: workflowRunOverrideSchema.nullable().optional(),
    resourceScopes: z.array(nonEmptyTextSchema).default([]),
    failurePolicy: workflowFailureClassificationPolicySchema.default({
      quotaFailureClasses: [],
      transientFailureClasses: [],
    }),
  })
  .strict()
export type WorkflowAgentNodeDto = z.infer<typeof workflowAgentNodeSchema>

export const workflowRouterNodeSchema = workflowNodeBaseSchema
  .extend({
    type: z.literal('router'),
  })
  .strict()

export const workflowGateNodeSchema = workflowNodeBaseSchema
  .extend({
    type: z.literal('gate'),
    requiredChecks: z.array(workflowConditionSchema).default([]),
    onBlocked: z.enum(['pause', 'fail']).default('pause'),
  })
  .strict()

export const workflowHumanCheckpointNodeSchema = workflowNodeBaseSchema
  .extend({
    type: z.literal('human_checkpoint'),
    checkpointType: workflowHumanCheckpointTypeSchema,
    prompt: nonEmptyTextSchema,
    decisionOptions: z.array(nonEmptyTextSchema).default([]),
  })
  .strict()

export const workflowMergeNodeSchema = workflowNodeBaseSchema
  .extend({
    type: z.literal('merge'),
    waitPolicy: workflowMergeWaitPolicySchema.default('all'),
    quorum: z.number().int().positive().nullable().optional(),
    failFast: z.boolean().default(false),
  })
  .strict()

export const workflowTerminalNodeSchema = workflowNodeBaseSchema
  .extend({
    type: z.literal('terminal'),
    terminalStatus: workflowTerminalStatusSchema,
  })
  .strict()

export const workflowNodeSchema = z.discriminatedUnion('type', [
  workflowAgentNodeSchema,
  workflowRouterNodeSchema,
  workflowGateNodeSchema,
  workflowHumanCheckpointNodeSchema,
  workflowMergeNodeSchema,
  workflowTerminalNodeSchema,
])
export type WorkflowNodeDto = z.infer<typeof workflowNodeSchema>

export const workflowLoopPolicySchema = z
  .object({
    loopKey: nonEmptyTextSchema,
    maxAttempts: z.number().int().positive(),
    attemptScope: workflowAttemptScopeSchema.default('run'),
    carryoverPolicy: workflowCarryoverPolicySchema.default('all'),
    selectedArtifactRefs: z.array(nonEmptyTextSchema).default([]),
    resetPolicy: workflowResetPolicySchema.default('never'),
    stallDetector: workflowStallDetectorSchema.nullable().optional(),
    onExhausted: workflowNodeIdSchema,
  })
  .strict()
export type WorkflowLoopPolicyDto = z.infer<typeof workflowLoopPolicySchema>

export const workflowEdgeSchema = z
  .object({
    id: workflowEdgeIdSchema,
    fromNodeId: workflowNodeIdSchema,
    toNodeId: workflowNodeIdSchema,
    type: workflowEdgeTypeSchema,
    label: z.string().trim().max(80).optional().default(''),
    priority: z.number().int().min(0).default(100),
    condition: workflowConditionSchema.default({ kind: 'always' }),
    loopPolicy: workflowLoopPolicySchema.nullable().optional(),
  })
  .strict()
export type WorkflowEdgeDto = z.infer<typeof workflowEdgeSchema>

export const workflowArtifactContractSchema = z
  .object({
    artifactType: workflowArtifactTypeSchema,
    schemaVersion: z.number().int().positive().default(1),
    jsonSchema: z.record(z.unknown()).nullable().optional(),
    displayName: nonEmptyTextSchema,
    description: optionalTextSchema,
  })
  .strict()
export type WorkflowArtifactContractDto = z.infer<typeof workflowArtifactContractSchema>

export const workflowRunPolicySchema = z
  .object({
    defaultProviderProfileId: z.string().trim().min(1).nullable().optional(),
    defaultModelId: z.string().trim().min(1).nullable().optional(),
    approvalMode: runtimeRunApprovalModeSchema.nullable().optional(),
    concurrencyLimit: z.number().int().positive().max(16).default(1),
    nodeTimeoutSeconds: z.number().int().positive().nullable().optional(),
    resourceConflictPolicy: z
      .object({
        mode: workflowResourceConflictModeSchema.default('serialize_conflicts'),
        defaultScopes: z.array(nonEmptyTextSchema).default([]),
      })
      .strict()
      .default({
        mode: 'serialize_conflicts',
        defaultScopes: [],
      }),
    recoveryDefaults: z
      .object({
        debugMaxAttempts: z.number().int().nonnegative().default(2),
        gapClosureMaxAttempts: z.number().int().nonnegative().default(2),
        reviewFixMaxAttempts: z.number().int().nonnegative().default(3),
      })
      .strict()
      .default({
        debugMaxAttempts: 2,
        gapClosureMaxAttempts: 2,
        reviewFixMaxAttempts: 3,
      }),
  })
  .strict()
export type WorkflowRunPolicyDto = z.infer<typeof workflowRunPolicySchema>

export const workflowDefinitionSchema = z
  .object({
    schema: z.literal('xero.workflow_definition.v1').default('xero.workflow_definition.v1'),
    id: workflowNodeIdSchema,
    projectId: nonEmptyTextSchema,
    name: nonEmptyTextSchema,
    description: optionalTextSchema,
    version: z.number().int().positive().default(1),
    startNodeId: workflowNodeIdSchema,
    nodes: z.array(workflowNodeSchema).min(1),
    edges: z.array(workflowEdgeSchema).default([]),
    artifactContracts: z.array(workflowArtifactContractSchema).default([]),
    runPolicy: workflowRunPolicySchema.default({
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
    }),
    createdAt: isoTimestampSchema.nullable().optional(),
    updatedAt: isoTimestampSchema.nullable().optional(),
  })
  .strict()
export type WorkflowDefinitionDto = z.infer<typeof workflowDefinitionSchema>

export const workflowDefinitionSummarySchema = z
  .object({
    id: nonEmptyTextSchema,
    projectId: nonEmptyTextSchema,
    name: nonEmptyTextSchema,
    description: z.string(),
    activeVersionId: nonEmptyTextSchema,
    activeVersionNumber: z.number().int().positive(),
    createdAt: isoTimestampSchema,
    updatedAt: isoTimestampSchema,
  })
  .strict()
export type WorkflowDefinitionSummaryDto = z.infer<
  typeof workflowDefinitionSummarySchema
>

export const workflowValidationSeveritySchema = z.enum(['error', 'warning'])
export type WorkflowValidationSeverityDto = z.infer<
  typeof workflowValidationSeveritySchema
>

export const workflowValidationDiagnosticSchema = z
  .object({
    severity: workflowValidationSeveritySchema,
    code: nonEmptyTextSchema,
    path: nonEmptyTextSchema,
    message: nonEmptyTextSchema,
  })
  .strict()
export type WorkflowValidationDiagnosticDto = z.infer<
  typeof workflowValidationDiagnosticSchema
>

export const workflowValidationReportSchema = z
  .object({
    status: z.enum(['valid', 'invalid']),
    diagnostics: z.array(workflowValidationDiagnosticSchema),
  })
  .strict()
export type WorkflowValidationReportDto = z.infer<typeof workflowValidationReportSchema>

export function validateWorkflowDefinition(input: unknown): WorkflowValidationReportDto {
  const parsed = workflowDefinitionSchema.safeParse(input)
  if (!parsed.success) {
    return {
      status: 'invalid',
      diagnostics: parsed.error.issues.map((issue) => ({
        severity: 'error',
        code: 'schema_invalid',
        path: issue.path.length > 0 ? issue.path.join('.') : '$',
        message: issue.message,
      })),
    }
  }

  const diagnostics = validateWorkflowDefinitionGraph(parsed.data)
  return {
    status: diagnostics.some((diagnostic) => diagnostic.severity === 'error')
      ? 'invalid'
      : 'valid',
    diagnostics,
  }
}

function validateWorkflowDefinitionGraph(
  definition: WorkflowDefinitionDto,
): WorkflowValidationDiagnosticDto[] {
  const diagnostics: WorkflowValidationDiagnosticDto[] = []
  const nodeIds = new Set<string>()
  const edgeIds = new Set<string>()
  const producedArtifactRefs = new Set<string>()

  definition.nodes.forEach((node, index) => {
    if (nodeIds.has(node.id)) {
      diagnostics.push(error('duplicate_node_id', `nodes.${index}.id`, `Node id \`${node.id}\` is duplicated.`))
    }
    nodeIds.add(node.id)
    if (node.type === 'agent') {
      producedArtifactRefs.add(`${node.id}.${node.outputContract.artifactType}`)
    }
  })

  if (!nodeIds.has(definition.startNodeId)) {
    diagnostics.push(error('start_node_missing', 'startNodeId', 'The start node must exist.'))
  }

  const outgoingDefaults = new Map<string, string>()
  const outgoingEdges = new Map<string, WorkflowEdgeDto[]>()

  definition.edges.forEach((edge, index) => {
    if (edgeIds.has(edge.id)) {
      diagnostics.push(error('duplicate_edge_id', `edges.${index}.id`, `Edge id \`${edge.id}\` is duplicated.`))
    }
    edgeIds.add(edge.id)
    if (!nodeIds.has(edge.fromNodeId)) {
      diagnostics.push(error('edge_source_missing', `edges.${index}.fromNodeId`, `Edge \`${edge.id}\` references a missing source node.`))
    }
    if (!nodeIds.has(edge.toNodeId)) {
      diagnostics.push(error('edge_target_missing', `edges.${index}.toNodeId`, `Edge \`${edge.id}\` references a missing target node.`))
    }
    if (edge.condition.kind === 'always') {
      const previous = outgoingDefaults.get(edge.fromNodeId)
      if (previous) {
        diagnostics.push(error('duplicate_default_edge', `edges.${index}.condition`, `Node \`${edge.fromNodeId}\` has more than one default else edge.`))
      } else {
        outgoingDefaults.set(edge.fromNodeId, edge.id)
      }
    }
    if (edge.type === 'loop' || edge.loopPolicy) {
      if (!edge.loopPolicy) {
        diagnostics.push(error('loop_policy_missing', `edges.${index}.loopPolicy`, `Loop edge \`${edge.id}\` must declare a loop policy.`))
      } else if (!nodeIds.has(edge.loopPolicy.onExhausted)) {
        diagnostics.push(error('loop_exhaustion_target_missing', `edges.${index}.loopPolicy.onExhausted`, `Loop edge \`${edge.id}\` must route exhaustion to an existing node.`))
      }
    }
    const entries = outgoingEdges.get(edge.fromNodeId) ?? []
    entries.push(edge)
    outgoingEdges.set(edge.fromNodeId, entries)
  })

  definition.nodes.forEach((node, index) => {
    if (node.type === 'agent') {
      node.inputBindings.forEach((binding, bindingIndex) => {
        if (binding.source === 'artifact' && !producedArtifactRefs.has(binding.artifactRef)) {
          diagnostics.push(error('artifact_ref_missing', `nodes.${index}.inputBindings.${bindingIndex}.artifactRef`, `Artifact reference \`${binding.artifactRef}\` is not produced by any agent node.`))
        }
      })
    }
    if (node.type === 'merge' && node.waitPolicy === 'quorum' && !node.quorum) {
      diagnostics.push(error('merge_quorum_missing', `nodes.${index}.quorum`, 'Quorum merge nodes must declare a quorum.'))
    }
  })

  definition.edges.forEach((edge, index) => {
    collectConditionArtifactRefs(edge.condition).forEach((artifactRef) => {
      if (!producedArtifactRefs.has(artifactRef)) {
        diagnostics.push(error('condition_artifact_ref_missing', `edges.${index}.condition`, `Condition references missing artifact \`${artifactRef}\`.`))
      }
    })
    collectConditionNodeRefs(edge.condition).forEach((nodeId) => {
      if (!nodeIds.has(nodeId)) {
        diagnostics.push(error('condition_node_ref_missing', `edges.${index}.condition`, `Condition references missing node \`${nodeId}\`.`))
      }
    })
  })

  diagnostics.push(...detectUnboundedCycles(definition, outgoingEdges))
  return diagnostics
}

function detectUnboundedCycles(
  definition: WorkflowDefinitionDto,
  outgoingEdges: Map<string, WorkflowEdgeDto[]>,
): WorkflowValidationDiagnosticDto[] {
  const diagnostics: WorkflowValidationDiagnosticDto[] = []
  const visiting = new Set<string>()
  const visited = new Set<string>()
  const edgeStack: WorkflowEdgeDto[] = []

  const visit = (nodeId: string) => {
    if (visiting.has(nodeId)) {
      const cycleStart = edgeStack.findIndex((edge) => edge.fromNodeId === nodeId)
      const cycle = cycleStart >= 0 ? edgeStack.slice(cycleStart) : edgeStack
      const hasBoundedLoop = cycle.some((edge) => edge.type === 'loop' && edge.loopPolicy)
      if (!hasBoundedLoop) {
        diagnostics.push(error('cycle_without_loop_policy', 'edges', `Cycle \`${cycle.map((edge) => edge.id).join(' -> ')}\` must include an explicit bounded loop edge.`))
      }
      return
    }
    if (visited.has(nodeId)) return
    visiting.add(nodeId)
    for (const edge of outgoingEdges.get(nodeId) ?? []) {
      edgeStack.push(edge)
      visit(edge.toNodeId)
      edgeStack.pop()
    }
    visiting.delete(nodeId)
    visited.add(nodeId)
  }

  if (definition.nodes.some((node) => node.id === definition.startNodeId)) {
    visit(definition.startNodeId)
  }
  return diagnostics
}

function collectConditionArtifactRefs(condition: WorkflowConditionDto): string[] {
  switch (condition.kind) {
    case 'artifact_exists':
    case 'artifact_field_equals':
    case 'artifact_field_in':
    case 'artifact_field_number_compare':
      return [condition.artifactRef]
    case 'all':
    case 'any':
      return condition.conditions.flatMap(collectConditionArtifactRefs)
    case 'not':
      return collectConditionArtifactRefs(condition.condition)
    default:
      return []
  }
}

function collectConditionNodeRefs(condition: WorkflowConditionDto): string[] {
  switch (condition.kind) {
    case 'node_status':
      return [condition.nodeId]
    case 'failure_class_is':
      return condition.nodeId ? [condition.nodeId] : []
    case 'human_decision_is':
      return [condition.checkpointNodeId]
    case 'all':
    case 'any':
      return condition.conditions.flatMap(collectConditionNodeRefs)
    case 'not':
      return collectConditionNodeRefs(condition.condition)
    default:
      return []
  }
}

function error(
  code: string,
  path: string,
  message: string,
): WorkflowValidationDiagnosticDto {
  return { severity: 'error', code, path, message }
}
