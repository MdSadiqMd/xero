import type { Project } from '@/components/cadence/data'
import { z } from 'zod'
import {
  mapPhase,
  mapProjectSummary,
  mapRepository,
  phaseSummarySchema,
  projectSummarySchema,
  repositorySummarySchema,
  type ProjectListItem,
  type RepositoryStatusView,
  type RepositoryView,
} from './cadence-model/project'
import {
  getLatestDecisionOutcome,
  mapOperatorApproval,
  mapResumeHistoryEntry,
  mapVerificationRecord,
  operatorApprovalSchema,
  resumeHistoryEntrySchema,
  verificationRecordSchema,
  type OperatorApprovalView,
  type ResumeHistoryEntryView,
  type VerificationRecordView,
} from './cadence-model/operator-actions'
import {
  mapNotificationBroker,
  notificationDispatchSchema,
  notificationReplyClaimSchema,
  type NotificationBrokerView,
  type NotificationDispatchDto,
} from './cadence-model/notifications'
import {
  humanizeNodeId as humanizeWorkflowNodeId,
  mapPlanningLifecycle,
  mapWorkflowHandoffPackage,
  planningLifecycleProjectionSchema,
  workflowHandoffPackageSchema,
  type PlanningLifecycleView,
  type WorkflowHandoffPackageView,
} from './cadence-model/workflow'
import {
  isoTimestampSchema,
  nonEmptyOptionalTextSchema,
  normalizeOptionalText,
  normalizeText,
  sortByNewest,
} from './cadence-model/shared'

export type { Phase, PhaseStatus, PhaseStep, Project } from '@/components/cadence/data'
export { safePercent } from './cadence-model/shared'
export * from './cadence-model/project'
export * from './cadence-model/operator-actions'
export * from './cadence-model/notifications'
export * from './cadence-model/workflow'

export const MAX_RUNTIME_STREAM_ITEMS = 40
export const MAX_RUNTIME_STREAM_TRANSCRIPTS = 20
export const MAX_RUNTIME_STREAM_TOOL_CALLS = 20
export const MAX_RUNTIME_STREAM_SKILLS = 20
export const MAX_RUNTIME_STREAM_ACTIVITY = 20
export const MAX_RUNTIME_STREAM_ACTION_REQUIRED = 10

export const projectSnapshotResponseSchema = z
  .object({
    project: projectSummarySchema,
    repository: repositorySummarySchema.nullable(),
    phases: z.array(phaseSummarySchema),
    lifecycle: planningLifecycleProjectionSchema,
    approvalRequests: z.array(operatorApprovalSchema),
    verificationRecords: z.array(verificationRecordSchema),
    resumeHistory: z.array(resumeHistoryEntrySchema),
    handoffPackages: z.array(workflowHandoffPackageSchema).optional(),
    autonomousRun: z.lazy(() => autonomousRunSchema).nullable().optional(),
    autonomousUnit: z.lazy(() => autonomousUnitSchema).nullable().optional(),
    notificationDispatches: z.array(notificationDispatchSchema).optional(),
    notificationReplyClaims: z.array(notificationReplyClaimSchema).optional(),
  })
  .superRefine((snapshot, ctx) => {
    if (snapshot.autonomousRun && snapshot.autonomousRun.projectId !== snapshot.project.id) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['autonomousRun', 'projectId'],
        message: 'Autonomous run project id must match the selected project snapshot id.',
      })
    }

    if (snapshot.autonomousUnit && snapshot.autonomousUnit.projectId !== snapshot.project.id) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['autonomousUnit', 'projectId'],
        message: 'Autonomous unit project id must match the selected project snapshot id.',
      })
    }

    if (snapshot.autonomousRun && snapshot.autonomousUnit) {
      if (snapshot.autonomousUnit.runId !== snapshot.autonomousRun.runId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['autonomousUnit', 'runId'],
          message: 'Autonomous unit run id must match the active autonomous run id.',
        })
      }
    }
  })

export const runtimeAuthPhaseSchema = z.enum([
  'idle',
  'starting',
  'awaiting_browser_callback',
  'awaiting_manual_input',
  'exchanging_code',
  'authenticated',
  'refreshing',
  'cancelled',
  'failed',
])

export const runtimeDiagnosticSchema = z.object({
  code: z.string().trim().min(1),
  message: z.string().trim().min(1),
  retryable: z.boolean(),
})

export const runtimeProviderIdSchema = z.enum(['openrouter', 'openai_codex'])

function validateRuntimeSettingsProviderModel(
  payload: { providerId: z.infer<typeof runtimeProviderIdSchema>; modelId: string },
  ctx: z.RefinementCtx,
): void {
  if (payload.providerId === 'openai_codex' && payload.modelId !== 'openai_codex') {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      path: ['modelId'],
      message: 'Cadence only supports modelId `openai_codex` for provider `openai_codex`.',
    })
  }
}

export const runtimeSettingsSchema = z
  .object({
    providerId: runtimeProviderIdSchema,
    modelId: z.string().trim().min(1),
    openrouterApiKeyConfigured: z.boolean(),
  })
  .strict()
  .superRefine((payload, ctx) => {
    validateRuntimeSettingsProviderModel(payload, ctx)
  })

export const upsertRuntimeSettingsRequestSchema = z
  .object({
    providerId: runtimeProviderIdSchema,
    modelId: z.string().trim().min(1),
    openrouterApiKey: z.string().nullable().optional(),
  })
  .strict()
  .superRefine((payload, ctx) => {
    validateRuntimeSettingsProviderModel(payload, ctx)
  })

export const runtimeSessionSchema = z.object({
  projectId: z.string().trim().min(1),
  runtimeKind: z.string().trim().min(1),
  providerId: z.string().trim().min(1),
  flowId: nonEmptyOptionalTextSchema,
  sessionId: nonEmptyOptionalTextSchema,
  accountId: nonEmptyOptionalTextSchema,
  phase: runtimeAuthPhaseSchema,
  callbackBound: z.boolean().nullable().optional(),
  authorizationUrl: z.string().url().nullable().optional(),
  redirectUri: z.string().url().nullable().optional(),
  lastErrorCode: nonEmptyOptionalTextSchema,
  lastError: runtimeDiagnosticSchema.nullable().optional(),
  updatedAt: isoTimestampSchema,
})

export const runtimeUpdatedPayloadSchema = z.object({
  projectId: z.string().trim().min(1),
  runtimeKind: z.string().trim().min(1),
  providerId: z.string().trim().min(1),
  flowId: nonEmptyOptionalTextSchema,
  sessionId: nonEmptyOptionalTextSchema,
  accountId: nonEmptyOptionalTextSchema,
  authPhase: runtimeAuthPhaseSchema,
  lastErrorCode: nonEmptyOptionalTextSchema,
  lastError: runtimeDiagnosticSchema.nullable().optional(),
  updatedAt: isoTimestampSchema,
})

export const runtimeRunStatusSchema = z.enum(['starting', 'running', 'stale', 'stopped', 'failed'])
export const runtimeRunTransportLivenessSchema = z.enum(['unknown', 'reachable', 'unreachable'])
export const runtimeRunCheckpointKindSchema = z.enum(['bootstrap', 'state', 'tool', 'action_required', 'diagnostic'])

export const runtimeRunDiagnosticSchema = z
  .object({
    code: z.string().trim().min(1),
    message: z.string().trim().min(1),
  })
  .strict()

export const runtimeRunTransportSchema = z
  .object({
    kind: z.string().trim().min(1),
    endpoint: z.string().trim().min(1),
    liveness: runtimeRunTransportLivenessSchema,
  })
  .strict()

export const runtimeRunCheckpointSchema = z
  .object({
    sequence: z.number().int().nonnegative(),
    kind: runtimeRunCheckpointKindSchema,
    summary: z.string().trim().min(1),
    createdAt: isoTimestampSchema,
  })
  .strict()

export const runtimeRunSchema = z
  .object({
    projectId: z.string().trim().min(1),
    runId: z.string().trim().min(1),
    runtimeKind: z.string().trim().min(1),
    providerId: z.string().trim().min(1),
    supervisorKind: z.string().trim().min(1),
    status: runtimeRunStatusSchema,
    transport: runtimeRunTransportSchema,
    startedAt: isoTimestampSchema,
    lastHeartbeatAt: nonEmptyOptionalTextSchema,
    lastCheckpointSequence: z.number().int().nonnegative(),
    lastCheckpointAt: nonEmptyOptionalTextSchema,
    stoppedAt: nonEmptyOptionalTextSchema,
    lastErrorCode: nonEmptyOptionalTextSchema,
    lastError: runtimeRunDiagnosticSchema.nullable().optional(),
    updatedAt: isoTimestampSchema,
    checkpoints: z.array(runtimeRunCheckpointSchema),
  })
  .strict()

export const runtimeRunUpdatedPayloadSchema = z
  .object({
    projectId: z.string().trim().min(1),
    run: runtimeRunSchema.nullable(),
  })
  .strict()
  .superRefine((payload, ctx) => {
    if (payload.run && payload.run.projectId !== payload.projectId) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['run', 'projectId'],
        message: 'Cadence received a runtime-run update for a different project than the event envelope.',
      })
    }
  })

export const autonomousRunStatusSchema = z.enum([
  'starting',
  'running',
  'paused',
  'cancelling',
  'cancelled',
  'stale',
  'failed',
  'stopped',
  'crashed',
  'completed',
])
export const autonomousRunRecoveryStateSchema = z.enum(['healthy', 'recovery_required', 'terminal', 'failed'])
export const autonomousUnitKindSchema = z.enum(['bootstrap', 'state', 'tool', 'action_required', 'diagnostic'])
export const autonomousUnitStatusSchema = z.enum([
  'pending',
  'active',
  'blocked',
  'paused',
  'completed',
  'cancelled',
  'failed',
])
export const autonomousUnitArtifactStatusSchema = z.enum(['pending', 'recorded', 'rejected', 'redacted'])
export const autonomousToolCallStateSchema = z.enum(['pending', 'running', 'succeeded', 'failed'])
export const autonomousVerificationOutcomeSchema = z.enum(['passed', 'failed', 'blocked'])

export const autonomousLifecycleReasonSchema = z
  .object({
    code: z.string().trim().min(1),
    message: z.string().trim().min(1),
  })
  .strict()

export const autonomousCommandResultSchema = z
  .object({
    exitCode: z.number().int().nullable().optional(),
    timedOut: z.boolean(),
    summary: z.string().trim().min(1),
  })
  .strict()

export const gitToolResultScopeSchema = z.enum(['staged', 'unstaged', 'worktree'])
export const webToolResultContentKindSchema = z.enum(['html', 'plain_text'])

export const toolResultSummarySchema = z.discriminatedUnion('kind', [
  z
    .object({
      kind: z.literal('command'),
      exitCode: z.number().int().nullable().optional(),
      timedOut: z.boolean(),
      stdoutTruncated: z.boolean(),
      stderrTruncated: z.boolean(),
      stdoutRedacted: z.boolean(),
      stderrRedacted: z.boolean(),
    })
    .strict(),
  z
    .object({
      kind: z.literal('file'),
      path: nonEmptyOptionalTextSchema,
      scope: nonEmptyOptionalTextSchema,
      lineCount: z.number().int().nonnegative().nullable().optional(),
      matchCount: z.number().int().nonnegative().nullable().optional(),
      truncated: z.boolean(),
    })
    .strict(),
  z
    .object({
      kind: z.literal('git'),
      scope: gitToolResultScopeSchema.nullable().optional(),
      changedFiles: z.number().int().nonnegative(),
      truncated: z.boolean(),
      baseRevision: nonEmptyOptionalTextSchema,
    })
    .strict(),
  z
    .object({
      kind: z.literal('web'),
      target: z.string().trim().min(1),
      resultCount: z.number().int().nonnegative().nullable().optional(),
      finalUrl: nonEmptyOptionalTextSchema,
      contentKind: webToolResultContentKindSchema.nullable().optional(),
      contentType: nonEmptyOptionalTextSchema,
      truncated: z.boolean(),
    })
    .strict(),
])

export const autonomousToolResultPayloadSchema = z
  .object({
    kind: z.literal('tool_result'),
    projectId: z.string().trim().min(1),
    runId: z.string().trim().min(1),
    unitId: z.string().trim().min(1),
    attemptId: z.string().trim().min(1),
    artifactId: z.string().trim().min(1),
    toolCallId: z.string().trim().min(1),
    toolName: z.string().trim().min(1),
    toolState: autonomousToolCallStateSchema,
    commandResult: autonomousCommandResultSchema.nullable().optional(),
    toolSummary: toolResultSummarySchema.nullable().optional(),
    actionId: nonEmptyOptionalTextSchema,
    boundaryId: nonEmptyOptionalTextSchema,
  })
  .strict()

export const autonomousVerificationEvidencePayloadSchema = z
  .object({
    kind: z.literal('verification_evidence'),
    projectId: z.string().trim().min(1),
    runId: z.string().trim().min(1),
    unitId: z.string().trim().min(1),
    attemptId: z.string().trim().min(1),
    artifactId: z.string().trim().min(1),
    evidenceKind: z.string().trim().min(1),
    label: z.string().trim().min(1),
    outcome: autonomousVerificationOutcomeSchema,
    commandResult: autonomousCommandResultSchema.nullable().optional(),
    actionId: nonEmptyOptionalTextSchema,
    boundaryId: nonEmptyOptionalTextSchema,
  })
  .strict()

export const autonomousPolicyDeniedPayloadSchema = z
  .object({
    kind: z.literal('policy_denied'),
    projectId: z.string().trim().min(1),
    runId: z.string().trim().min(1),
    unitId: z.string().trim().min(1),
    attemptId: z.string().trim().min(1),
    artifactId: z.string().trim().min(1),
    diagnosticCode: z.string().trim().min(1),
    message: z.string().trim().min(1),
    toolName: nonEmptyOptionalTextSchema,
    actionId: nonEmptyOptionalTextSchema,
    boundaryId: nonEmptyOptionalTextSchema,
  })
  .strict()

export const autonomousArtifactPayloadSchema = z.discriminatedUnion('kind', [
  autonomousToolResultPayloadSchema,
  autonomousVerificationEvidencePayloadSchema,
  autonomousPolicyDeniedPayloadSchema,
])

export const autonomousRunSchema = z
  .object({
    projectId: z.string().trim().min(1),
    runId: z.string().trim().min(1),
    runtimeKind: z.string().trim().min(1),
    providerId: z.string().trim().min(1),
    supervisorKind: z.string().trim().min(1),
    status: autonomousRunStatusSchema,
    recoveryState: autonomousRunRecoveryStateSchema,
    activeUnitId: nonEmptyOptionalTextSchema,
    activeAttemptId: nonEmptyOptionalTextSchema,
    duplicateStartDetected: z.boolean(),
    duplicateStartRunId: nonEmptyOptionalTextSchema,
    duplicateStartReason: nonEmptyOptionalTextSchema,
    startedAt: isoTimestampSchema,
    lastHeartbeatAt: nonEmptyOptionalTextSchema,
    lastCheckpointAt: nonEmptyOptionalTextSchema,
    pausedAt: nonEmptyOptionalTextSchema,
    cancelledAt: nonEmptyOptionalTextSchema,
    completedAt: nonEmptyOptionalTextSchema,
    crashedAt: nonEmptyOptionalTextSchema,
    stoppedAt: nonEmptyOptionalTextSchema,
    pauseReason: autonomousLifecycleReasonSchema.nullable().optional(),
    cancelReason: autonomousLifecycleReasonSchema.nullable().optional(),
    crashReason: autonomousLifecycleReasonSchema.nullable().optional(),
    lastErrorCode: nonEmptyOptionalTextSchema,
    lastError: runtimeRunDiagnosticSchema.nullable().optional(),
    updatedAt: isoTimestampSchema,
  })
  .strict()

export const autonomousWorkflowLinkageSchema = z
  .object({
    workflowNodeId: z.string().trim().min(1),
    transitionId: z.string().trim().min(1),
    causalTransitionId: nonEmptyOptionalTextSchema,
    handoffTransitionId: z.string().trim().min(1),
    handoffPackageHash: z
      .string()
      .regex(/^[0-9a-f]{64}$/, 'Autonomous workflow linkage handoff package hashes must be lowercase 64-character hex digests.'),
  })
  .strict()

export const autonomousUnitSchema = z
  .object({
    projectId: z.string().trim().min(1),
    runId: z.string().trim().min(1),
    unitId: z.string().trim().min(1),
    sequence: z.number().int().nonnegative(),
    kind: autonomousUnitKindSchema,
    status: autonomousUnitStatusSchema,
    summary: z.string().trim().min(1),
    boundaryId: nonEmptyOptionalTextSchema,
    workflowLinkage: autonomousWorkflowLinkageSchema.nullable().optional(),
    startedAt: isoTimestampSchema,
    finishedAt: nonEmptyOptionalTextSchema,
    updatedAt: isoTimestampSchema,
    lastErrorCode: nonEmptyOptionalTextSchema,
    lastError: runtimeRunDiagnosticSchema.nullable().optional(),
  })
  .strict()

export const autonomousUnitAttemptSchema = z
  .object({
    projectId: z.string().trim().min(1),
    runId: z.string().trim().min(1),
    unitId: z.string().trim().min(1),
    attemptId: z.string().trim().min(1),
    attemptNumber: z.number().int().nonnegative(),
    childSessionId: z.string().trim().min(1),
    status: autonomousUnitStatusSchema,
    boundaryId: nonEmptyOptionalTextSchema,
    workflowLinkage: autonomousWorkflowLinkageSchema.nullable().optional(),
    startedAt: isoTimestampSchema,
    finishedAt: nonEmptyOptionalTextSchema,
    updatedAt: isoTimestampSchema,
    lastErrorCode: nonEmptyOptionalTextSchema,
    lastError: runtimeRunDiagnosticSchema.nullable().optional(),
  })
  .strict()

export const autonomousUnitArtifactSchema = z
  .object({
    projectId: z.string().trim().min(1),
    runId: z.string().trim().min(1),
    unitId: z.string().trim().min(1),
    attemptId: z.string().trim().min(1),
    artifactId: z.string().trim().min(1),
    artifactKind: z.string().trim().min(1),
    status: autonomousUnitArtifactStatusSchema,
    summary: z.string().trim().min(1),
    contentHash: nonEmptyOptionalTextSchema,
    payload: autonomousArtifactPayloadSchema.nullable().optional(),
    createdAt: isoTimestampSchema,
    updatedAt: isoTimestampSchema,
  })
  .strict()
  .superRefine((artifact, ctx) => {
    const payload = artifact.payload
    if (!payload) {
      return
    }

    if (payload.projectId !== artifact.projectId) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['payload', 'projectId'],
        message: 'Autonomous artifact payload project id must match the enclosing artifact project id.',
      })
    }

    if (payload.runId !== artifact.runId) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['payload', 'runId'],
        message: 'Autonomous artifact payload run id must match the enclosing artifact run id.',
      })
    }

    if (payload.unitId !== artifact.unitId) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['payload', 'unitId'],
        message: 'Autonomous artifact payload unit id must match the enclosing artifact unit id.',
      })
    }

    if (payload.attemptId !== artifact.attemptId) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['payload', 'attemptId'],
        message: 'Autonomous artifact payload attempt id must match the enclosing artifact attempt id.',
      })
    }

    if (payload.artifactId !== artifact.artifactId) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['payload', 'artifactId'],
        message: 'Autonomous artifact payload artifact id must match the enclosing artifact id.',
      })
    }
  })

export const autonomousUnitHistoryEntrySchema = z
  .object({
    unit: autonomousUnitSchema,
    latestAttempt: autonomousUnitAttemptSchema.nullable().optional(),
    artifacts: z.array(autonomousUnitArtifactSchema).optional(),
  })
  .strict()
  .superRefine((entry, ctx) => {
    const latestAttempt = entry.latestAttempt ?? null

    if (latestAttempt) {
      if (latestAttempt.projectId !== entry.unit.projectId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['latestAttempt', 'projectId'],
          message: 'Autonomous history attempt project id must match the enclosing unit project id.',
        })
      }

      if (latestAttempt.runId !== entry.unit.runId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['latestAttempt', 'runId'],
          message: 'Autonomous history attempt run id must match the enclosing unit run id.',
        })
      }

      if (latestAttempt.unitId !== entry.unit.unitId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['latestAttempt', 'unitId'],
          message: 'Autonomous history attempt unit id must match the enclosing unit id.',
        })
      }
    }

    entry.artifacts?.forEach((artifact, index) => {
      if (artifact.projectId !== entry.unit.projectId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['artifacts', index, 'projectId'],
          message: 'Autonomous history artifacts must reference the same project as the enclosing unit.',
        })
      }

      if (artifact.runId !== entry.unit.runId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['artifacts', index, 'runId'],
          message: 'Autonomous history artifacts must reference the same run as the enclosing unit.',
        })
      }

      if (artifact.unitId !== entry.unit.unitId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['artifacts', index, 'unitId'],
          message: 'Autonomous history artifacts must reference the same unit as the enclosing history entry.',
        })
      }

      if (latestAttempt && artifact.attemptId !== latestAttempt.attemptId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['artifacts', index, 'attemptId'],
          message: 'Autonomous history artifacts must reference the latest attempt id for the enclosing history entry.',
        })
      }
    })
  })

export const autonomousRunStateSchema = z
  .object({
    run: autonomousRunSchema.nullable(),
    unit: autonomousUnitSchema.nullable(),
    attempt: autonomousUnitAttemptSchema.nullable().optional(),
    history: z.array(autonomousUnitHistoryEntrySchema).optional(),
  })
  .strict()
  .superRefine((state, ctx) => {
    const attempt = state.attempt ?? null
    const history = state.history ?? []

    if (state.run && state.unit) {
      if (state.unit.projectId !== state.run.projectId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['unit', 'projectId'],
          message: 'Autonomous unit project id must match the autonomous run project id.',
        })
      }

      if (state.unit.runId !== state.run.runId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['unit', 'runId'],
          message: 'Autonomous unit run id must match the autonomous run run id.',
        })
      }
    }

    if (state.run && attempt) {
      if (attempt.projectId !== state.run.projectId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['attempt', 'projectId'],
          message: 'Autonomous attempt project id must match the autonomous run project id.',
        })
      }

      if (attempt.runId !== state.run.runId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['attempt', 'runId'],
          message: 'Autonomous attempt run id must match the autonomous run run id.',
        })
      }

      if (state.run.activeAttemptId && attempt.attemptId !== state.run.activeAttemptId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['attempt', 'attemptId'],
          message: 'Autonomous attempt id must match the active attempt id reported on the run.',
        })
      }
    }

    if (state.unit && attempt) {
      if (attempt.projectId !== state.unit.projectId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['attempt', 'projectId'],
          message: 'Autonomous attempt project id must match the autonomous unit project id.',
        })
      }

      if (attempt.runId !== state.unit.runId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['attempt', 'runId'],
          message: 'Autonomous attempt run id must match the autonomous unit run id.',
        })
      }

      if (attempt.unitId !== state.unit.unitId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['attempt', 'unitId'],
          message: 'Autonomous attempt unit id must match the autonomous unit id.',
        })
      }
    }

    history.forEach((entry, index) => {
      if (state.run && entry.unit.projectId !== state.run.projectId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['history', index, 'unit', 'projectId'],
          message: 'Autonomous history unit project id must match the autonomous run project id.',
        })
      }

      if (state.run && entry.unit.runId !== state.run.runId) {
        ctx.addIssue({
          code: z.ZodIssueCode.custom,
          path: ['history', index, 'unit', 'runId'],
          message: 'Autonomous history unit run id must match the autonomous run run id.',
        })
      }
    })
  })

export const runtimeToolCallStateSchema = z.enum(['pending', 'running', 'succeeded', 'failed'])
export const runtimeSkillLifecycleStageSchema = z.enum(['discovery', 'install', 'invoke'])
export const runtimeSkillLifecycleResultSchema = z.enum(['succeeded', 'failed'])
export const runtimeSkillCacheStatusSchema = z.enum(['miss', 'hit', 'refreshed'])
export const runtimeSkillSourceSchema = z
  .object({
    repo: z.string().trim().min(1),
    path: z.string().trim().min(1),
    reference: z.string().trim().min(1),
    treeHash: z.string().regex(/^[0-9a-f]{40}$/, 'Runtime skill source tree hashes must be lowercase 40-character hex digests.'),
  })
  .strict()
export const runtimeSkillDiagnosticSchema = z
  .object({
    code: z.string().trim().min(1),
    message: z.string().trim().min(1),
    retryable: z.boolean(),
  })
  .strict()
export const runtimeStreamItemKindSchema = z.enum([
  'transcript',
  'tool',
  'skill',
  'activity',
  'action_required',
  'complete',
  'failure',
])

export const runtimeStreamItemSchema = z
  .object({
    kind: runtimeStreamItemKindSchema,
    runId: z.string().trim().min(1),
    sequence: z.number().int().positive(),
    sessionId: nonEmptyOptionalTextSchema,
    flowId: nonEmptyOptionalTextSchema,
    text: nonEmptyOptionalTextSchema,
    toolCallId: nonEmptyOptionalTextSchema,
    toolName: nonEmptyOptionalTextSchema,
    toolState: runtimeToolCallStateSchema.nullable().optional(),
    toolSummary: toolResultSummarySchema.nullable().optional(),
    skillId: nonEmptyOptionalTextSchema,
    skillStage: runtimeSkillLifecycleStageSchema.nullable().optional(),
    skillResult: runtimeSkillLifecycleResultSchema.nullable().optional(),
    skillSource: runtimeSkillSourceSchema.nullable().optional(),
    skillCacheStatus: runtimeSkillCacheStatusSchema.nullable().optional(),
    skillDiagnostic: runtimeSkillDiagnosticSchema.nullable().optional(),
    actionId: nonEmptyOptionalTextSchema,
    boundaryId: nonEmptyOptionalTextSchema,
    actionType: nonEmptyOptionalTextSchema,
    title: nonEmptyOptionalTextSchema,
    detail: nonEmptyOptionalTextSchema,
    code: nonEmptyOptionalTextSchema,
    message: nonEmptyOptionalTextSchema,
    retryable: z.boolean().nullable().optional(),
    createdAt: isoTimestampSchema,
  })
  .strict()
  .superRefine((item, ctx) => {
    const hasSkillMetadata =
      item.skillId != null
      || item.skillStage != null
      || item.skillResult != null
      || item.skillSource != null
      || item.skillCacheStatus != null
      || item.skillDiagnostic != null

    if (item.kind !== 'skill' && hasSkillMetadata) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['skillId'],
        message: `Cadence received non-skill runtime item kind \`${item.kind}\` with skill lifecycle metadata.`,
      })
    }

    switch (item.kind) {
      case 'transcript':
        if (!item.text) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['text'],
            message: 'Cadence received a runtime transcript item without a non-empty text field.',
          })
        }
        return
      case 'tool':
        if (!item.toolCallId) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['toolCallId'],
            message: 'Cadence received a runtime tool item without a non-empty toolCallId field.',
          })
        }
        if (!item.toolName) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['toolName'],
            message: 'Cadence received a runtime tool item without a non-empty toolName field.',
          })
        }
        if (!item.toolState) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['toolState'],
            message: 'Cadence received a runtime tool item without a toolState value.',
          })
        }
        return
      case 'skill':
        if (!item.skillId) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['skillId'],
            message: 'Cadence received a runtime skill item without a non-empty skillId field.',
          })
        }
        if (!item.skillStage) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['skillStage'],
            message: 'Cadence received a runtime skill item without a skillStage value.',
          })
        }
        if (!item.skillResult) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['skillResult'],
            message: 'Cadence received a runtime skill item without a skillResult value.',
          })
        }
        if (!item.skillSource) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['skillSource'],
            message: 'Cadence received a runtime skill item without skillSource metadata.',
          })
        }
        if (!item.detail) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['detail'],
            message: 'Cadence received a runtime skill item without a non-empty detail field.',
          })
        }
        if (item.skillResult === 'succeeded' && item.skillDiagnostic) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['skillDiagnostic'],
            message: 'Successful runtime skill items must not include skillDiagnostic payloads.',
          })
        }
        if (item.skillResult === 'failed' && !item.skillDiagnostic) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['skillDiagnostic'],
            message: 'Failed runtime skill items must include typed skillDiagnostic payloads.',
          })
        }
        if (item.skillStage === 'discovery' && item.skillCacheStatus) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['skillCacheStatus'],
            message: 'Discovery runtime skill items must omit skillCacheStatus because no install or invoke step has completed yet.',
          })
        }
        if ((item.skillStage === 'install' || item.skillStage === 'invoke') && item.skillResult === 'succeeded' && !item.skillCacheStatus) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['skillCacheStatus'],
            message: 'Successful install/invoke runtime skill items must include skillCacheStatus.',
          })
        }
        return
      case 'activity':
        if (!item.code) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['code'],
            message: 'Cadence received a runtime activity item without a non-empty code field.',
          })
        }
        if (!item.title) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['title'],
            message: 'Cadence received a runtime activity item without a non-empty title field.',
          })
        }
        return
      case 'action_required':
        if (!item.actionId) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['actionId'],
            message: 'Cadence received a runtime action-required item without a non-empty actionId field.',
          })
        }
        if (!item.actionType) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['actionType'],
            message: 'Cadence received a runtime action-required item without a non-empty actionType field.',
          })
        }
        if (!item.title) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['title'],
            message: 'Cadence received a runtime action-required item without a non-empty title field.',
          })
        }
        if (!item.detail) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['detail'],
            message: 'Cadence received a runtime action-required item without a non-empty detail field.',
          })
        }
        return
      case 'complete':
        if (!item.detail) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['detail'],
            message: 'Cadence received a runtime completion item without a non-empty detail field.',
          })
        }
        return
      case 'failure':
        if (!item.code) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['code'],
            message: 'Cadence received a runtime failure item without a non-empty code field.',
          })
        }
        if (!item.message) {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['message'],
            message: 'Cadence received a runtime failure item without a non-empty message field.',
          })
        }
        if (typeof item.retryable !== 'boolean') {
          ctx.addIssue({
            code: z.ZodIssueCode.custom,
            path: ['retryable'],
            message: 'Cadence received a runtime failure item without a retryable flag.',
          })
        }
        return
    }
  })

export const subscribeRuntimeStreamRequestSchema = z.object({
  projectId: z.string().trim().min(1),
  itemKinds: z.array(runtimeStreamItemKindSchema).min(1),
}).strict()

export const subscribeRuntimeStreamResponseSchema = z.object({
  projectId: z.string().trim().min(1),
  runtimeKind: z.string().trim().min(1),
  runId: z.string().trim().min(1),
  sessionId: z.string().trim().min(1),
  flowId: nonEmptyOptionalTextSchema,
  subscribedItemKinds: z.array(runtimeStreamItemKindSchema).min(1),
}).strict()

export type ProjectSnapshotResponseDto = z.infer<typeof projectSnapshotResponseSchema>

export type RuntimeAuthPhaseDto = z.infer<typeof runtimeAuthPhaseSchema>
export type RuntimeDiagnosticDto = z.infer<typeof runtimeDiagnosticSchema>
export type RuntimeProviderIdDto = z.infer<typeof runtimeProviderIdSchema>
export type RuntimeSettingsDto = z.infer<typeof runtimeSettingsSchema>
export type UpsertRuntimeSettingsRequestDto = z.infer<typeof upsertRuntimeSettingsRequestSchema>
export type RuntimeSessionDto = z.infer<typeof runtimeSessionSchema>
export type RuntimeUpdatedPayloadDto = z.infer<typeof runtimeUpdatedPayloadSchema>
export type RuntimeRunStatusDto = z.infer<typeof runtimeRunStatusSchema>
export type RuntimeRunTransportLivenessDto = z.infer<typeof runtimeRunTransportLivenessSchema>
export type RuntimeRunCheckpointKindDto = z.infer<typeof runtimeRunCheckpointKindSchema>
export type RuntimeRunDiagnosticDto = z.infer<typeof runtimeRunDiagnosticSchema>
export type RuntimeRunTransportDto = z.infer<typeof runtimeRunTransportSchema>
export type RuntimeRunCheckpointDto = z.infer<typeof runtimeRunCheckpointSchema>
export type RuntimeRunDto = z.infer<typeof runtimeRunSchema>
export type RuntimeRunUpdatedPayloadDto = z.infer<typeof runtimeRunUpdatedPayloadSchema>
export type AutonomousRunStatusDto = z.infer<typeof autonomousRunStatusSchema>
export type AutonomousRunRecoveryStateDto = z.infer<typeof autonomousRunRecoveryStateSchema>
export type AutonomousUnitKindDto = z.infer<typeof autonomousUnitKindSchema>
export type AutonomousUnitStatusDto = z.infer<typeof autonomousUnitStatusSchema>
export type AutonomousUnitArtifactStatusDto = z.infer<typeof autonomousUnitArtifactStatusSchema>
export type AutonomousWorkflowLinkageDto = z.infer<typeof autonomousWorkflowLinkageSchema>
export type AutonomousToolCallStateDto = z.infer<typeof autonomousToolCallStateSchema>
export type AutonomousVerificationOutcomeDto = z.infer<typeof autonomousVerificationOutcomeSchema>
export type AutonomousLifecycleReasonDto = z.infer<typeof autonomousLifecycleReasonSchema>
export type AutonomousCommandResultDto = z.infer<typeof autonomousCommandResultSchema>
export type GitToolResultScopeDto = z.infer<typeof gitToolResultScopeSchema>
export type WebToolResultContentKindDto = z.infer<typeof webToolResultContentKindSchema>
export type ToolResultSummaryDto = z.infer<typeof toolResultSummarySchema>
export type AutonomousToolResultPayloadDto = z.infer<typeof autonomousToolResultPayloadSchema>
export type AutonomousVerificationEvidencePayloadDto = z.infer<typeof autonomousVerificationEvidencePayloadSchema>
export type AutonomousPolicyDeniedPayloadDto = z.infer<typeof autonomousPolicyDeniedPayloadSchema>
export type AutonomousArtifactPayloadDto = z.infer<typeof autonomousArtifactPayloadSchema>
export type AutonomousRunDto = z.infer<typeof autonomousRunSchema>
export type AutonomousUnitDto = z.infer<typeof autonomousUnitSchema>
export type AutonomousUnitAttemptDto = z.infer<typeof autonomousUnitAttemptSchema>
export type AutonomousUnitArtifactDto = z.infer<typeof autonomousUnitArtifactSchema>
export type AutonomousUnitHistoryEntryDto = z.infer<typeof autonomousUnitHistoryEntrySchema>
export type AutonomousRunStateDto = z.infer<typeof autonomousRunStateSchema>
export type RuntimeToolCallStateDto = z.infer<typeof runtimeToolCallStateSchema>
export type RuntimeSkillLifecycleStageDto = z.infer<typeof runtimeSkillLifecycleStageSchema>
export type RuntimeSkillLifecycleResultDto = z.infer<typeof runtimeSkillLifecycleResultSchema>
export type RuntimeSkillCacheStatusDto = z.infer<typeof runtimeSkillCacheStatusSchema>
export type RuntimeSkillSourceDto = z.infer<typeof runtimeSkillSourceSchema>
export type RuntimeSkillDiagnosticDto = z.infer<typeof runtimeSkillDiagnosticSchema>
export type RuntimeStreamItemKindDto = z.infer<typeof runtimeStreamItemKindSchema>
export type RuntimeStreamItemDto = z.infer<typeof runtimeStreamItemSchema>
export type SubscribeRuntimeStreamRequestDto = z.infer<typeof subscribeRuntimeStreamRequestSchema>
export type SubscribeRuntimeStreamResponseDto = z.infer<typeof subscribeRuntimeStreamResponseSchema>

export interface RuntimeSessionView {
  projectId: string
  runtimeKind: string
  providerId: string
  flowId: string | null
  sessionId: string | null
  accountId: string | null
  phase: RuntimeAuthPhaseDto
  phaseLabel: string
  runtimeLabel: string
  accountLabel: string
  sessionLabel: string
  callbackBound: boolean | null
  authorizationUrl: string | null
  redirectUri: string | null
  lastErrorCode: string | null
  lastError: RuntimeDiagnosticDto | null
  updatedAt: string
  isAuthenticated: boolean
  isLoginInProgress: boolean
  needsManualInput: boolean
  isSignedOut: boolean
  isFailed: boolean
}

export interface RuntimeRunTransportView {
  kind: string
  endpoint: string
  liveness: RuntimeRunTransportLivenessDto
  livenessLabel: string
}

export interface RuntimeRunCheckpointView {
  sequence: number
  kind: RuntimeRunCheckpointKindDto
  kindLabel: string
  summary: string
  createdAt: string
}

export interface RuntimeRunView {
  projectId: string
  runId: string
  runtimeKind: string
  providerId: string
  runtimeLabel: string
  supervisorKind: string
  supervisorLabel: string
  status: RuntimeRunStatusDto
  statusLabel: string
  transport: RuntimeRunTransportView
  startedAt: string
  lastHeartbeatAt: string | null
  lastCheckpointSequence: number
  lastCheckpointAt: string | null
  stoppedAt: string | null
  lastErrorCode: string | null
  lastError: RuntimeRunDiagnosticDto | null
  updatedAt: string
  checkpoints: RuntimeRunCheckpointView[]
  latestCheckpoint: RuntimeRunCheckpointView | null
  checkpointCount: number
  hasCheckpoints: boolean
  isActive: boolean
  isTerminal: boolean
  isStale: boolean
  isFailed: boolean
}

export interface AutonomousLifecycleReasonView {
  code: string
  message: string
}

export interface AutonomousRunView {
  projectId: string
  runId: string
  runtimeKind: string
  providerId: string
  runtimeLabel: string
  supervisorKind: string
  supervisorLabel: string
  status: AutonomousRunStatusDto
  statusLabel: string
  recoveryState: AutonomousRunRecoveryStateDto
  recoveryLabel: string
  activeUnitId: string | null
  activeAttemptId: string | null
  duplicateStartDetected: boolean
  duplicateStartRunId: string | null
  duplicateStartReason: string | null
  startedAt: string
  lastHeartbeatAt: string | null
  lastCheckpointAt: string | null
  pausedAt: string | null
  cancelledAt: string | null
  completedAt: string | null
  crashedAt: string | null
  stoppedAt: string | null
  pauseReason: AutonomousLifecycleReasonView | null
  cancelReason: AutonomousLifecycleReasonView | null
  crashReason: AutonomousLifecycleReasonView | null
  lastErrorCode: string | null
  lastError: RuntimeRunDiagnosticDto | null
  updatedAt: string
  isActive: boolean
  needsRecovery: boolean
  isTerminal: boolean
  isFailed: boolean
}

export interface AutonomousWorkflowLinkageView {
  workflowNodeId: string
  transitionId: string
  causalTransitionId: string | null
  handoffTransitionId: string
  handoffPackageHash: string
}

export interface AutonomousWorkflowHandoffView {
  handoffTransitionId: string
  causalTransitionId: string | null
  fromNodeId: string
  toNodeId: string
  transitionKind: string
  transitionKindLabel: string
  packageHash: string
  createdAt: string
}

export type AutonomousWorkflowLinkageSource = 'unit' | 'attempt'
export type AutonomousWorkflowContextState = 'ready' | 'awaiting_snapshot' | 'awaiting_handoff'

export interface AutonomousWorkflowContextView {
  linkage: AutonomousWorkflowLinkageView
  linkageSource: AutonomousWorkflowLinkageSource
  linkedNodeLabel: string
  linkedStage: PlanningLifecycleStageView | null
  activeLifecycleStage: PlanningLifecycleStageView | null
  handoff: AutonomousWorkflowHandoffView | null
  pendingApproval: OperatorApprovalView | null
  state: AutonomousWorkflowContextState
  stateLabel: string
  detail: string
}

export interface AutonomousUnitView {
  projectId: string
  runId: string
  unitId: string
  sequence: number
  kind: AutonomousUnitKindDto
  kindLabel: string
  status: AutonomousUnitStatusDto
  statusLabel: string
  summary: string
  boundaryId: string | null
  workflowLinkage: AutonomousWorkflowLinkageView | null
  startedAt: string
  finishedAt: string | null
  updatedAt: string
  lastErrorCode: string | null
  lastError: RuntimeRunDiagnosticDto | null
  isActive: boolean
  isTerminal: boolean
  isFailed: boolean
}

export interface AutonomousUnitAttemptView {
  projectId: string
  runId: string
  unitId: string
  attemptId: string
  attemptNumber: number
  childSessionId: string
  status: AutonomousUnitStatusDto
  statusLabel: string
  boundaryId: string | null
  workflowLinkage: AutonomousWorkflowLinkageView | null
  startedAt: string
  finishedAt: string | null
  updatedAt: string
  lastErrorCode: string | null
  lastError: RuntimeRunDiagnosticDto | null
  isActive: boolean
  isTerminal: boolean
  isFailed: boolean
}

export interface AutonomousCommandResultView {
  exitCode: number | null
  timedOut: boolean
  summary: string
}

export interface AutonomousUnitArtifactView {
  projectId: string
  runId: string
  unitId: string
  attemptId: string
  artifactId: string
  artifactKind: string
  artifactKindLabel: string
  status: AutonomousUnitArtifactStatusDto
  statusLabel: string
  summary: string
  contentHash: string | null
  payload: AutonomousArtifactPayloadDto | null
  createdAt: string
  updatedAt: string
  detail: string | null
  commandResult: AutonomousCommandResultView | null
  toolSummary: ToolResultSummaryDto | null
  toolName: string | null
  toolState: AutonomousToolCallStateDto | null
  toolStateLabel: string | null
  evidenceKind: string | null
  verificationOutcome: AutonomousVerificationOutcomeDto | null
  verificationOutcomeLabel: string | null
  diagnosticCode: string | null
  actionId: string | null
  boundaryId: string | null
  isToolResult: boolean
  isVerificationEvidence: boolean
  isPolicyDenied: boolean
}

export interface AutonomousUnitHistoryEntryView {
  unit: AutonomousUnitView
  latestAttempt: AutonomousUnitAttemptView | null
  artifacts: AutonomousUnitArtifactView[]
}

export interface AutonomousRunInspectionView {
  autonomousRun: AutonomousRunView | null
  autonomousUnit: AutonomousUnitView | null
  autonomousAttempt: AutonomousUnitAttemptView | null
  autonomousHistory: AutonomousUnitHistoryEntryView[]
  autonomousRecentArtifacts: AutonomousUnitArtifactView[]
}

export type RuntimeStreamStatus = 'idle' | 'subscribing' | 'replaying' | 'live' | 'complete' | 'stale' | 'error'

export interface RuntimeStreamIssueView {
  code: string
  message: string
  retryable: boolean
  observedAt: string
}

interface RuntimeStreamBaseItemView {
  id: string
  runId: string
  sequence: number
  createdAt: string
}

export interface RuntimeStreamTranscriptItemView extends RuntimeStreamBaseItemView {
  kind: 'transcript'
  text: string
}

export interface RuntimeStreamToolItemView extends RuntimeStreamBaseItemView {
  kind: 'tool'
  toolCallId: string
  toolName: string
  toolState: RuntimeToolCallStateDto
  detail: string | null
  toolSummary: ToolResultSummaryDto | null
}

export interface RuntimeStreamSkillItemView extends RuntimeStreamBaseItemView {
  kind: 'skill'
  skillId: string
  stage: RuntimeSkillLifecycleStageDto
  result: RuntimeSkillLifecycleResultDto
  detail: string
  source: RuntimeSkillSourceDto
  cacheStatus: RuntimeSkillCacheStatusDto | null
  diagnostic: RuntimeSkillDiagnosticDto | null
}

export interface RuntimeStreamActivityItemView extends RuntimeStreamBaseItemView {
  kind: 'activity'
  code: string
  title: string
  detail: string | null
}

export interface RuntimeStreamActionRequiredItemView extends RuntimeStreamBaseItemView {
  kind: 'action_required'
  actionId: string
  boundaryId: string | null
  actionType: string
  title: string
  detail: string
}

export interface RuntimeStreamCompleteItemView extends RuntimeStreamBaseItemView {
  kind: 'complete'
  detail: string
}

export interface RuntimeStreamFailureItemView extends RuntimeStreamBaseItemView {
  kind: 'failure'
  code: string
  message: string
  retryable: boolean
}

export type RuntimeStreamViewItem =
  | RuntimeStreamTranscriptItemView
  | RuntimeStreamToolItemView
  | RuntimeStreamSkillItemView
  | RuntimeStreamActivityItemView
  | RuntimeStreamActionRequiredItemView
  | RuntimeStreamCompleteItemView
  | RuntimeStreamFailureItemView

export interface RuntimeStreamEventDto {
  projectId: string
  runtimeKind: string
  runId: string
  sessionId: string
  flowId: string | null
  subscribedItemKinds: RuntimeStreamItemKindDto[]
  item: RuntimeStreamItemDto
}

export interface RuntimeStreamView {
  projectId: string
  runtimeKind: string
  runId: string | null
  sessionId: string | null
  flowId: string | null
  subscribedItemKinds: RuntimeStreamItemKindDto[]
  status: RuntimeStreamStatus
  items: RuntimeStreamViewItem[]
  transcriptItems: RuntimeStreamTranscriptItemView[]
  toolCalls: RuntimeStreamToolItemView[]
  skillItems: RuntimeStreamSkillItemView[]
  activityItems: RuntimeStreamActivityItemView[]
  actionRequired: RuntimeStreamActionRequiredItemView[]
  completion: RuntimeStreamCompleteItemView | null
  failure: RuntimeStreamFailureItemView | null
  lastIssue: RuntimeStreamIssueView | null
  lastItemAt: string | null
  lastSequence: number | null
}

export interface ProjectDetailView extends Project {
  branchLabel: string
  runtimeLabel: string
  phaseProgressPercent: number
  lifecycle: PlanningLifecycleView
  repository: RepositoryView | null
  repositoryStatus: RepositoryStatusView | null
  approvalRequests: OperatorApprovalView[]
  pendingApprovalCount: number
  latestDecisionOutcome: OperatorDecisionOutcomeView | null
  verificationRecords: VerificationRecordView[]
  resumeHistory: ResumeHistoryEntryView[]
  handoffPackages: WorkflowHandoffPackageView[]
  notificationBroker: NotificationBrokerView
  runtimeSession?: RuntimeSessionView | null
  runtimeRun?: RuntimeRunView | null
  autonomousRun?: AutonomousRunView | null
  autonomousUnit?: AutonomousUnitView | null
  autonomousAttempt?: AutonomousUnitAttemptView | null
  autonomousHistory: AutonomousUnitHistoryEntryView[]
  autonomousRecentArtifacts: AutonomousUnitArtifactView[]
}

function getAutonomousWorkflowContextStateLabel(state: AutonomousWorkflowContextState): string {
  switch (state) {
    case 'ready':
      return 'In sync'
    case 'awaiting_snapshot':
      return 'Snapshot lag'
    case 'awaiting_handoff':
      return 'Handoff pending'
  }
}

function mapAutonomousWorkflowHandoff(pkg: WorkflowHandoffPackageView): AutonomousWorkflowHandoffView {
  return {
    handoffTransitionId: pkg.handoffTransitionId,
    causalTransitionId: pkg.causalTransitionId,
    fromNodeId: pkg.fromNodeId,
    toNodeId: pkg.toNodeId,
    transitionKind: pkg.transitionKind,
    transitionKindLabel: humanizeRuntimeKind(pkg.transitionKind),
    packageHash: pkg.packageHash,
    createdAt: pkg.createdAt,
  }
}

export function deriveAutonomousWorkflowContext(options: {
  lifecycle: PlanningLifecycleView
  handoffPackages: WorkflowHandoffPackageView[]
  approvalRequests: OperatorApprovalView[]
  autonomousUnit: AutonomousUnitView | null
  autonomousAttempt?: AutonomousUnitAttemptView | null
}): AutonomousWorkflowContextView | null {
  const attemptLinkage = options.autonomousAttempt?.workflowLinkage ?? null
  const unitLinkage = options.autonomousUnit?.workflowLinkage ?? null
  const linkage = attemptLinkage ?? unitLinkage
  if (!linkage) {
    return null
  }

  const linkageSource: AutonomousWorkflowLinkageSource = attemptLinkage ? 'attempt' : 'unit'
  const linkedStage = options.lifecycle.stages.find((stage) => stage.nodeId === linkage.workflowNodeId) ?? null
  const activeLifecycleStage = options.lifecycle.activeStage
  const linkedNodeLabel = linkedStage?.nodeLabel ?? humanizeWorkflowNodeId(linkage.workflowNodeId)
  const matchingHandoffPackage = sortByNewest(
    options.handoffPackages.filter((pkg) => pkg.handoffTransitionId === linkage.handoffTransitionId),
    (pkg) => pkg.createdAt,
  )[0] ?? null
  const handoff = matchingHandoffPackage ? mapAutonomousWorkflowHandoff(matchingHandoffPackage) : null
  const pendingApproval =
    options.approvalRequests.find(
      (approval) => approval.isPending && approval.gateNodeId === linkage.workflowNodeId,
    ) ?? null

  const activeStageMismatch = Boolean(activeLifecycleStage && activeLifecycleStage.nodeId !== linkage.workflowNodeId)
  const handoffHashMismatch = Boolean(handoff && handoff.packageHash !== linkage.handoffPackageHash)

  let state: AutonomousWorkflowContextState
  let detail: string

  if (!linkedStage) {
    state = 'awaiting_snapshot'
    detail =
      'Cadence has persisted autonomous workflow linkage for this boundary, but the selected project snapshot has not exposed the linked lifecycle node yet.'
  } else if (activeStageMismatch) {
    state = 'awaiting_snapshot'
    detail = `Cadence is keeping lifecycle progression anchored to snapshot truth while the linked node \`${linkedStage.stageLabel}\` waits for the active lifecycle stage to catch up.`
  } else if (handoffHashMismatch) {
    state = 'awaiting_snapshot'
    detail =
      'Cadence found the linked handoff transition in the selected project snapshot, but the persisted handoff hash has not caught up to the autonomous linkage yet.'
  } else if (!handoff) {
    state = 'awaiting_handoff'
    detail =
      'Cadence has persisted autonomous workflow linkage for this boundary, but the linked handoff package is not visible in the selected project snapshot yet.'
  } else {
    state = 'ready'
    detail =
      'Lifecycle stage, autonomous linkage, and handoff package all agree on backend truth for this boundary.'
  }

  if (pendingApproval) {
    detail = `${detail} Pending approval \`${pendingApproval.title}\` is still blocking continuation at this linked node.`
  }

  return {
    linkage,
    linkageSource,
    linkedNodeLabel,
    linkedStage,
    activeLifecycleStage,
    handoff,
    pendingApproval,
    state,
    stateLabel: getAutonomousWorkflowContextStateLabel(state),
    detail,
  }
}

function timestampToSortValue(value: string | null): number {
  if (!value) {
    return Number.NEGATIVE_INFINITY
  }

  const parsed = Date.parse(value)
  return Number.isFinite(parsed) ? parsed : Number.NEGATIVE_INFINITY
}

function humanizeRuntimeKind(runtimeKind: string): string {
  return runtimeKind
    .split(/[_-]+/)
    .filter((part) => part.length > 0)
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ')
}

function getRuntimePhaseLabel(phase: RuntimeAuthPhaseDto): string {
  switch (phase) {
    case 'idle':
      return 'Signed out'
    case 'starting':
      return 'Starting login'
    case 'awaiting_browser_callback':
      return 'Awaiting browser'
    case 'awaiting_manual_input':
      return 'Awaiting manual input'
    case 'exchanging_code':
      return 'Signing in'
    case 'authenticated':
      return 'Authenticated'
    case 'refreshing':
      return 'Refreshing session'
    case 'cancelled':
      return 'Login cancelled'
    case 'failed':
      return 'Login failed'
  }
}

function getRuntimeLabel(runtimeKind: string, phase: RuntimeAuthPhaseDto): string {
  if (phase === 'idle' || phase === 'failed' || phase === 'cancelled') {
    return 'Runtime unavailable'
  }

  return `${humanizeRuntimeKind(runtimeKind)} · ${getRuntimePhaseLabel(phase)}`
}

export function getRuntimeRunStatusLabel(status: RuntimeRunStatusDto): string {
  switch (status) {
    case 'starting':
      return 'Supervisor starting'
    case 'running':
      return 'Supervisor running'
    case 'stale':
      return 'Supervisor stale'
    case 'stopped':
      return 'Run stopped'
    case 'failed':
      return 'Run failed'
  }
}

export function getRuntimeRunTransportLivenessLabel(liveness: RuntimeRunTransportLivenessDto): string {
  switch (liveness) {
    case 'unknown':
      return 'Probe unknown'
    case 'reachable':
      return 'Control reachable'
    case 'unreachable':
      return 'Control unreachable'
  }
}

export function getRuntimeRunCheckpointKindLabel(kind: RuntimeRunCheckpointKindDto): string {
  switch (kind) {
    case 'bootstrap':
      return 'Bootstrap'
    case 'state':
      return 'State'
    case 'tool':
      return 'Tool'
    case 'action_required':
      return 'Action required'
    case 'diagnostic':
      return 'Diagnostic'
  }
}

function getRuntimeRunLabel(runtimeKind: string, status: RuntimeRunStatusDto): string {
  return `${humanizeRuntimeKind(runtimeKind)} · ${getRuntimeRunStatusLabel(status)}`
}

export function getAutonomousRunStatusLabel(status: AutonomousRunStatusDto): string {
  switch (status) {
    case 'starting':
      return 'Autonomous run starting'
    case 'running':
      return 'Autonomous run active'
    case 'paused':
      return 'Autonomous run paused'
    case 'cancelling':
      return 'Autonomous run cancelling'
    case 'cancelled':
      return 'Autonomous run cancelled'
    case 'stale':
      return 'Autonomous run stale'
    case 'failed':
      return 'Autonomous run failed'
    case 'stopped':
      return 'Autonomous run stopped'
    case 'crashed':
      return 'Autonomous run crashed'
    case 'completed':
      return 'Autonomous run completed'
  }
}

export function getAutonomousRunRecoveryLabel(recoveryState: AutonomousRunRecoveryStateDto): string {
  switch (recoveryState) {
    case 'healthy':
      return 'Recovery healthy'
    case 'recovery_required':
      return 'Recovery required'
    case 'terminal':
      return 'Terminal state'
    case 'failed':
      return 'Recovery failed'
  }
}

export function getAutonomousUnitKindLabel(kind: AutonomousUnitKindDto): string {
  switch (kind) {
    case 'bootstrap':
      return 'Bootstrap'
    case 'state':
      return 'State'
    case 'tool':
      return 'Tool'
    case 'action_required':
      return 'Action required'
    case 'diagnostic':
      return 'Diagnostic'
  }
}

export function getAutonomousUnitStatusLabel(status: AutonomousUnitStatusDto): string {
  switch (status) {
    case 'pending':
      return 'Pending'
    case 'active':
      return 'Active'
    case 'blocked':
      return 'Blocked'
    case 'paused':
      return 'Paused'
    case 'completed':
      return 'Completed'
    case 'cancelled':
      return 'Cancelled'
    case 'failed':
      return 'Failed'
  }
}

function getAutonomousRunLabel(runtimeKind: string, status: AutonomousRunStatusDto): string {
  return `${humanizeRuntimeKind(runtimeKind)} · ${getAutonomousRunStatusLabel(status)}`
}

function capRecent<T>(values: T[], limit: number): T[] {
  return values.length <= limit ? values : values.slice(values.length - limit)
}

function uniqueRuntimeStreamKinds(kinds: RuntimeStreamItemKindDto[]): RuntimeStreamItemKindDto[] {
  return Array.from(new Set(kinds))
}

function ensureRuntimeStreamText(value: string | null | undefined, field: string, kind: string): string {
  const normalized = normalizeOptionalText(value)
  if (!normalized) {
    throw new Error(`Cadence received a ${kind} item without a non-empty ${field}.`)
  }

  return normalized
}

function runtimeStreamItemId(kind: RuntimeStreamItemKindDto, runId: string, sequence: number): string {
  return `${kind}:${runId}:${sequence}`
}

function runtimeStreamActionRequiredItemId(runId: string, actionId: string): string {
  return `action_required:${runId}:${actionId}`
}

function normalizeRuntimeStreamItem(event: RuntimeStreamEventDto): RuntimeStreamViewItem {
  const projectId = normalizeOptionalText(event.projectId)
  if (!projectId) {
    throw new Error('Cadence received a runtime stream item without a selected project id.')
  }

  const expectedRunId = normalizeOptionalText(event.runId)
  const expectedSessionId = normalizeOptionalText(event.sessionId)
  const eventFlowId = normalizeOptionalText(event.flowId)
  const itemRunId = normalizeOptionalText(event.item.runId)
  const itemSessionId = normalizeOptionalText(event.item.sessionId)
  const itemFlowId = normalizeOptionalText(event.item.flowId)

  if (!expectedRunId || !itemRunId || itemRunId !== expectedRunId) {
    throw new Error('Cadence received a runtime stream item for an unexpected run id at the desktop adapter boundary.')
  }

  if (expectedSessionId && itemSessionId && itemSessionId !== expectedSessionId) {
    throw new Error(
      `Cadence received a runtime stream item for an unexpected session (${itemSessionId}) while ${expectedSessionId} is active.`,
    )
  }

  if (eventFlowId && itemFlowId && itemFlowId !== eventFlowId) {
    throw new Error(`Cadence received a runtime stream item for an unexpected auth flow (${itemFlowId}).`)
  }

  switch (event.item.kind) {
    case 'transcript': {
      const text = ensureRuntimeStreamText(event.item.text, 'text', 'transcript')
      return {
        id: runtimeStreamItemId('transcript', itemRunId, event.item.sequence),
        kind: 'transcript',
        runId: itemRunId,
        sequence: event.item.sequence,
        createdAt: event.item.createdAt,
        text,
      }
    }
    case 'tool': {
      const toolCallId = ensureRuntimeStreamText(event.item.toolCallId, 'toolCallId', 'tool')
      const toolName = ensureRuntimeStreamText(event.item.toolName, 'toolName', 'tool')
      const toolState = event.item.toolState
      if (!toolState) {
        throw new Error('Cadence received a runtime tool item without a toolState value.')
      }

      return {
        id: runtimeStreamItemId('tool', itemRunId, event.item.sequence),
        kind: 'tool',
        runId: itemRunId,
        sequence: event.item.sequence,
        createdAt: event.item.createdAt,
        toolCallId,
        toolName,
        toolState,
        detail: normalizeOptionalText(event.item.detail),
        toolSummary: event.item.toolSummary ?? null,
      }
    }
    case 'skill': {
      const skillId = ensureRuntimeStreamText(event.item.skillId, 'skillId', 'skill')
      const stage = event.item.skillStage
      const result = event.item.skillResult
      const source = event.item.skillSource
      const detail = ensureRuntimeStreamText(event.item.detail, 'detail', 'skill')

      if (!stage) {
        throw new Error('Cadence received a runtime skill item without a skillStage value.')
      }
      if (!result) {
        throw new Error('Cadence received a runtime skill item without a skillResult value.')
      }
      if (!source) {
        throw new Error('Cadence received a runtime skill item without skillSource metadata.')
      }

      return {
        id: runtimeStreamItemId('skill', itemRunId, event.item.sequence),
        kind: 'skill',
        runId: itemRunId,
        sequence: event.item.sequence,
        createdAt: event.item.createdAt,
        skillId,
        stage,
        result,
        detail,
        source,
        cacheStatus: event.item.skillCacheStatus ?? null,
        diagnostic: event.item.skillDiagnostic ?? null,
      }
    }
    case 'activity': {
      const code = ensureRuntimeStreamText(event.item.code, 'code', 'activity')
      const title = ensureRuntimeStreamText(event.item.title, 'title', 'activity')
      return {
        id: runtimeStreamItemId('activity', itemRunId, event.item.sequence),
        kind: 'activity',
        runId: itemRunId,
        sequence: event.item.sequence,
        createdAt: event.item.createdAt,
        code,
        title,
        detail: normalizeOptionalText(event.item.detail),
      }
    }
    case 'action_required': {
      const actionId = ensureRuntimeStreamText(event.item.actionId, 'actionId', 'action-required')
      const actionType = ensureRuntimeStreamText(event.item.actionType, 'actionType', 'action-required')
      const title = ensureRuntimeStreamText(event.item.title, 'title', 'action-required')
      const detail = ensureRuntimeStreamText(event.item.detail, 'detail', 'action-required')
      return {
        id: runtimeStreamActionRequiredItemId(itemRunId, actionId),
        kind: 'action_required',
        runId: itemRunId,
        sequence: event.item.sequence,
        createdAt: event.item.createdAt,
        actionId,
        boundaryId: normalizeOptionalText(event.item.boundaryId),
        actionType,
        title,
        detail,
      }
    }
    case 'complete': {
      const detail = ensureRuntimeStreamText(event.item.detail, 'detail', 'complete')
      return {
        id: runtimeStreamItemId('complete', itemRunId, event.item.sequence),
        kind: 'complete',
        runId: itemRunId,
        sequence: event.item.sequence,
        createdAt: event.item.createdAt,
        detail,
      }
    }
    case 'failure': {
      const code = ensureRuntimeStreamText(event.item.code, 'code', 'failure')
      const message = ensureRuntimeStreamText(event.item.message, 'message', 'failure')
      if (typeof event.item.retryable !== 'boolean') {
        throw new Error('Cadence received a runtime failure item without a retryable flag.')
      }

      return {
        id: runtimeStreamItemId('failure', itemRunId, event.item.sequence),
        kind: 'failure',
        runId: itemRunId,
        sequence: event.item.sequence,
        createdAt: event.item.createdAt,
        code,
        message,
        retryable: event.item.retryable,
      }
    }
  }
}

export function createRuntimeStreamView(options: {
  projectId: string
  runtimeKind: string
  runId?: string | null
  sessionId?: string | null
  flowId?: string | null
  subscribedItemKinds?: RuntimeStreamItemKindDto[]
  status?: RuntimeStreamStatus
}): RuntimeStreamView {
  return {
    projectId: options.projectId,
    runtimeKind: normalizeText(options.runtimeKind, 'openai_codex'),
    runId: normalizeOptionalText(options.runId),
    sessionId: normalizeOptionalText(options.sessionId),
    flowId: normalizeOptionalText(options.flowId),
    subscribedItemKinds: uniqueRuntimeStreamKinds(options.subscribedItemKinds ?? []),
    status: options.status ?? 'idle',
    items: [],
    transcriptItems: [],
    toolCalls: [],
    skillItems: [],
    activityItems: [],
    actionRequired: [],
    completion: null,
    failure: null,
    lastIssue: null,
    lastItemAt: null,
    lastSequence: null,
  }
}

export function createRuntimeStreamFromSubscription(
  response: SubscribeRuntimeStreamResponseDto,
  status: RuntimeStreamStatus = 'subscribing',
): RuntimeStreamView {
  return createRuntimeStreamView({
    projectId: response.projectId,
    runtimeKind: response.runtimeKind,
    runId: response.runId,
    sessionId: response.sessionId,
    flowId: response.flowId ?? null,
    subscribedItemKinds: response.subscribedItemKinds,
    status,
  })
}

export function mergeRuntimeStreamEvent(
  current: RuntimeStreamView | null,
  event: RuntimeStreamEventDto,
): RuntimeStreamView {
  if (current && current.projectId !== event.projectId) {
    throw new Error(
      `Cadence received a runtime stream item for ${event.projectId} while ${current.projectId} is the selected project.`,
    )
  }

  if (current?.runId && current.runId !== event.runId) {
    return current
  }

  const base =
    current ??
    createRuntimeStreamView({
      projectId: event.projectId,
      runtimeKind: event.runtimeKind,
      runId: event.runId,
      sessionId: event.sessionId,
      flowId: event.flowId,
      subscribedItemKinds: event.subscribedItemKinds,
      status: 'subscribing',
    })

  if (base.lastSequence !== null) {
    if (event.item.sequence < base.lastSequence) {
      throw new Error(
        `Cadence rejected non-monotonic runtime stream sequence ${event.item.sequence} for run ${event.runId}; last sequence was ${base.lastSequence}.`,
      )
    }

    if (event.item.sequence === base.lastSequence) {
      return base
    }
  }

  const nextItem = normalizeRuntimeStreamItem(event)
  const nextItems =
    nextItem.kind === 'action_required'
      ? capRecent(
          [
            ...base.items.filter(
              (item) => !(item.kind === 'action_required' && item.runId === nextItem.runId && item.actionId === nextItem.actionId),
            ),
            nextItem,
          ],
          MAX_RUNTIME_STREAM_ITEMS,
        )
      : capRecent([...base.items, nextItem], MAX_RUNTIME_STREAM_ITEMS)
  const nextToolCalls =
    nextItem.kind === 'tool'
      ? capRecent(
          [
            ...base.toolCalls.filter((toolCall) => toolCall.toolCallId !== nextItem.toolCallId),
            nextItem,
          ],
          MAX_RUNTIME_STREAM_TOOL_CALLS,
        )
      : base.toolCalls
  const nextSkillItems =
    nextItem.kind === 'skill'
      ? capRecent([...base.skillItems, nextItem], MAX_RUNTIME_STREAM_SKILLS)
      : base.skillItems
  const nextTranscriptItems =
    nextItem.kind === 'transcript'
      ? capRecent([...base.transcriptItems, nextItem], MAX_RUNTIME_STREAM_TRANSCRIPTS)
      : base.transcriptItems
  const nextActivityItems =
    nextItem.kind === 'activity'
      ? capRecent([...base.activityItems, nextItem], MAX_RUNTIME_STREAM_ACTIVITY)
      : base.activityItems
  const nextActionRequired =
    nextItem.kind === 'action_required'
      ? capRecent(
          [
            ...base.actionRequired.filter((actionRequiredItem) => actionRequiredItem.actionId !== nextItem.actionId),
            nextItem,
          ],
          MAX_RUNTIME_STREAM_ACTION_REQUIRED,
        )
      : base.actionRequired

  return {
    ...base,
    runtimeKind: normalizeText(event.runtimeKind, base.runtimeKind),
    runId: normalizeOptionalText(event.runId) ?? base.runId,
    sessionId: normalizeOptionalText(event.sessionId) ?? base.sessionId,
    flowId: normalizeOptionalText(event.flowId) ?? base.flowId,
    subscribedItemKinds: uniqueRuntimeStreamKinds(event.subscribedItemKinds),
    status:
      nextItem.kind === 'complete'
        ? 'complete'
        : nextItem.kind === 'failure'
          ? nextItem.retryable
            ? 'stale'
            : 'error'
          : 'live',
    items: nextItems,
    transcriptItems: nextTranscriptItems,
    toolCalls: nextToolCalls,
    skillItems: nextSkillItems,
    activityItems: nextActivityItems,
    actionRequired: nextActionRequired,
    completion: nextItem.kind === 'complete' ? nextItem : base.completion,
    failure: nextItem.kind === 'failure' ? nextItem : null,
    lastIssue:
      nextItem.kind === 'failure'
        ? {
            code: nextItem.code,
            message: nextItem.message,
            retryable: nextItem.retryable,
            observedAt: nextItem.createdAt,
          }
        : null,
    lastItemAt: nextItem.createdAt,
    lastSequence: nextItem.sequence,
  }
}

export function applyRuntimeStreamIssue(
  current: RuntimeStreamView | null,
  options: {
    projectId: string
    runtimeKind: string
    runId?: string | null
    sessionId?: string | null
    flowId?: string | null
    subscribedItemKinds?: RuntimeStreamItemKindDto[]
    code: string
    message: string
    retryable: boolean
    observedAt?: string
  },
): RuntimeStreamView {
  const observedAt = options.observedAt ?? new Date().toISOString()
  const base =
    current ??
    createRuntimeStreamView({
      projectId: options.projectId,
      runtimeKind: options.runtimeKind,
      runId: options.runId,
      sessionId: options.sessionId,
      flowId: options.flowId,
      subscribedItemKinds: options.subscribedItemKinds,
      status: options.retryable ? 'stale' : 'error',
    })

  return {
    ...base,
    runtimeKind: normalizeText(options.runtimeKind, base.runtimeKind),
    runId: normalizeOptionalText(options.runId) ?? base.runId,
    sessionId: normalizeOptionalText(options.sessionId) ?? base.sessionId,
    flowId: normalizeOptionalText(options.flowId) ?? base.flowId,
    subscribedItemKinds: uniqueRuntimeStreamKinds(options.subscribedItemKinds ?? base.subscribedItemKinds),
    status: options.retryable ? 'stale' : 'error',
    lastIssue: {
      code: normalizeText(options.code, 'runtime_stream_issue'),
      message: normalizeText(options.message, 'Cadence could not project runtime activity for this project.'),
      retryable: options.retryable,
      observedAt,
    },
    lastItemAt: base.lastItemAt ?? observedAt,
    lastSequence: base.lastSequence,
  }
}

export function getRuntimeStreamStatusLabel(status: RuntimeStreamStatus): string {
  switch (status) {
    case 'idle':
      return 'No live stream'
    case 'subscribing':
      return 'Connecting stream'
    case 'replaying':
      return 'Replaying recent activity'
    case 'live':
      return 'Streaming live activity'
    case 'complete':
      return 'Stream complete'
    case 'stale':
      return 'Stream stale'
    case 'error':
      return 'Stream failed'
  }
}

export function mapProjectSnapshot(
  snapshot: ProjectSnapshotResponseDto,
  options: { notificationDispatches?: NotificationDispatchDto[] } = {},
): ProjectDetailView {
  const summary = mapProjectSummary(snapshot.project)
  const approvalRequests = snapshot.approvalRequests.map(mapOperatorApproval)
  const verificationRecords = snapshot.verificationRecords.map(mapVerificationRecord)
  const resumeHistory = snapshot.resumeHistory.map(mapResumeHistoryEntry)
  const handoffPackages = (snapshot.handoffPackages ?? [])
    .filter((pkg) => pkg.projectId === snapshot.project.id)
    .map(mapWorkflowHandoffPackage)
  const notificationDispatches = options.notificationDispatches ?? snapshot.notificationDispatches ?? []
  const notificationBroker = mapNotificationBroker(snapshot.project.id, notificationDispatches)

  if (!snapshot.lifecycle) {
    throw new Error('Cadence received a project snapshot without the required lifecycle projection.')
  }

  const autonomousRun = snapshot.autonomousRun ? mapAutonomousRun(snapshot.autonomousRun) : null
  const autonomousUnit = snapshot.autonomousUnit ? mapAutonomousUnit(snapshot.autonomousUnit) : null

  return {
    ...summary,
    phases: snapshot.phases.map(mapPhase),
    lifecycle: mapPlanningLifecycle(snapshot.lifecycle),
    repository: snapshot.repository ? mapRepository(snapshot.repository) : null,
    repositoryStatus: null,
    approvalRequests,
    pendingApprovalCount: approvalRequests.filter((approval) => approval.isPending).length,
    latestDecisionOutcome: getLatestDecisionOutcome(approvalRequests),
    verificationRecords,
    resumeHistory,
    handoffPackages,
    notificationBroker,
    runtimeSession: null,
    runtimeRun: null,
    autonomousRun,
    autonomousUnit,
    autonomousAttempt: null,
    autonomousHistory: [],
    autonomousRecentArtifacts: [],
  }
}

export function mapRuntimeSession(runtime: RuntimeSessionDto): RuntimeSessionView {
  const runtimeKind = normalizeText(runtime.runtimeKind, 'openai_codex')
  const providerId = normalizeText(runtime.providerId, 'provider-unavailable')
  const accountId = normalizeOptionalText(runtime.accountId)
  const sessionId = normalizeOptionalText(runtime.sessionId)

  return {
    projectId: runtime.projectId,
    runtimeKind,
    providerId,
    flowId: normalizeOptionalText(runtime.flowId),
    sessionId,
    accountId,
    phase: runtime.phase,
    phaseLabel: getRuntimePhaseLabel(runtime.phase),
    runtimeLabel: getRuntimeLabel(runtimeKind, runtime.phase),
    accountLabel: accountId ?? 'Not signed in',
    sessionLabel: sessionId ?? 'No session',
    callbackBound: runtime.callbackBound ?? null,
    authorizationUrl: normalizeOptionalText(runtime.authorizationUrl),
    redirectUri: normalizeOptionalText(runtime.redirectUri),
    lastErrorCode: normalizeOptionalText(runtime.lastErrorCode),
    lastError: runtime.lastError ?? null,
    updatedAt: runtime.updatedAt,
    isAuthenticated: runtime.phase === 'authenticated',
    isLoginInProgress: [
      'starting',
      'awaiting_browser_callback',
      'awaiting_manual_input',
      'exchanging_code',
      'refreshing',
    ].includes(runtime.phase),
    needsManualInput: runtime.phase === 'awaiting_manual_input',
    isSignedOut: runtime.phase === 'idle',
    isFailed: runtime.phase === 'failed' || runtime.phase === 'cancelled',
  }
}

export function mapRuntimeRunCheckpoint(checkpoint: RuntimeRunCheckpointDto): RuntimeRunCheckpointView {
  return {
    sequence: checkpoint.sequence,
    kind: checkpoint.kind,
    kindLabel: getRuntimeRunCheckpointKindLabel(checkpoint.kind),
    summary: normalizeText(checkpoint.summary, 'Durable checkpoint recorded.'),
    createdAt: checkpoint.createdAt,
  }
}

export function mapRuntimeRun(runtimeRun: RuntimeRunDto): RuntimeRunView {
  const runtimeKind = normalizeText(runtimeRun.runtimeKind, 'openai_codex')
  const providerId = normalizeText(runtimeRun.providerId, 'provider-unavailable')
  const supervisorKind = normalizeText(runtimeRun.supervisorKind, 'detached_pty')
  const checkpoints = runtimeRun.checkpoints
    .map(mapRuntimeRunCheckpoint)
    .sort((left, right) => left.sequence - right.sequence)
  const latestCheckpoint = checkpoints[checkpoints.length - 1] ?? null

  return {
    projectId: runtimeRun.projectId,
    runId: normalizeText(runtimeRun.runId, 'run-unavailable'),
    runtimeKind,
    providerId,
    runtimeLabel: getRuntimeRunLabel(runtimeKind, runtimeRun.status),
    supervisorKind,
    supervisorLabel: humanizeRuntimeKind(supervisorKind),
    status: runtimeRun.status,
    statusLabel: getRuntimeRunStatusLabel(runtimeRun.status),
    transport: {
      kind: normalizeText(runtimeRun.transport.kind, 'tcp'),
      endpoint: normalizeText(runtimeRun.transport.endpoint, 'Unavailable'),
      liveness: runtimeRun.transport.liveness,
      livenessLabel: getRuntimeRunTransportLivenessLabel(runtimeRun.transport.liveness),
    },
    startedAt: runtimeRun.startedAt,
    lastHeartbeatAt: normalizeOptionalText(runtimeRun.lastHeartbeatAt),
    lastCheckpointSequence: runtimeRun.lastCheckpointSequence,
    lastCheckpointAt: normalizeOptionalText(runtimeRun.lastCheckpointAt),
    stoppedAt: normalizeOptionalText(runtimeRun.stoppedAt),
    lastErrorCode: normalizeOptionalText(runtimeRun.lastErrorCode),
    lastError: runtimeRun.lastError ?? null,
    updatedAt: runtimeRun.updatedAt,
    checkpoints,
    latestCheckpoint,
    checkpointCount: checkpoints.length,
    hasCheckpoints: checkpoints.length > 0,
    isActive: runtimeRun.status === 'starting' || runtimeRun.status === 'running',
    isTerminal: runtimeRun.status === 'stopped' || runtimeRun.status === 'failed',
    isStale: runtimeRun.status === 'stale',
    isFailed: runtimeRun.status === 'failed',
  }
}

export function mapAutonomousRun(autonomousRun: AutonomousRunDto): AutonomousRunView {
  const runtimeKind = normalizeText(autonomousRun.runtimeKind, 'openai_codex')
  const providerId = normalizeText(autonomousRun.providerId, 'provider-unavailable')
  const supervisorKind = normalizeText(autonomousRun.supervisorKind, 'detached_pty')

  return {
    projectId: autonomousRun.projectId,
    runId: normalizeText(autonomousRun.runId, 'autonomous-run-unavailable'),
    runtimeKind,
    providerId,
    runtimeLabel: getAutonomousRunLabel(runtimeKind, autonomousRun.status),
    supervisorKind,
    supervisorLabel: humanizeRuntimeKind(supervisorKind),
    status: autonomousRun.status,
    statusLabel: getAutonomousRunStatusLabel(autonomousRun.status),
    recoveryState: autonomousRun.recoveryState,
    recoveryLabel: getAutonomousRunRecoveryLabel(autonomousRun.recoveryState),
    activeUnitId: normalizeOptionalText(autonomousRun.activeUnitId),
    activeAttemptId: normalizeOptionalText(autonomousRun.activeAttemptId),
    duplicateStartDetected: autonomousRun.duplicateStartDetected,
    duplicateStartRunId: normalizeOptionalText(autonomousRun.duplicateStartRunId),
    duplicateStartReason: normalizeOptionalText(autonomousRun.duplicateStartReason),
    startedAt: autonomousRun.startedAt,
    lastHeartbeatAt: normalizeOptionalText(autonomousRun.lastHeartbeatAt),
    lastCheckpointAt: normalizeOptionalText(autonomousRun.lastCheckpointAt),
    pausedAt: normalizeOptionalText(autonomousRun.pausedAt),
    cancelledAt: normalizeOptionalText(autonomousRun.cancelledAt),
    completedAt: normalizeOptionalText(autonomousRun.completedAt),
    crashedAt: normalizeOptionalText(autonomousRun.crashedAt),
    stoppedAt: normalizeOptionalText(autonomousRun.stoppedAt),
    pauseReason: autonomousRun.pauseReason ?? null,
    cancelReason: autonomousRun.cancelReason ?? null,
    crashReason: autonomousRun.crashReason ?? null,
    lastErrorCode: normalizeOptionalText(autonomousRun.lastErrorCode),
    lastError: autonomousRun.lastError ?? null,
    updatedAt: autonomousRun.updatedAt,
    isActive: autonomousRun.status === 'starting' || autonomousRun.status === 'running',
    needsRecovery: autonomousRun.recoveryState === 'recovery_required',
    isTerminal: ['cancelled', 'stopped', 'completed'].includes(autonomousRun.status),
    isFailed: ['failed', 'crashed'].includes(autonomousRun.status),
  }
}

function mapAutonomousWorkflowLinkage(
  workflowLinkage: AutonomousWorkflowLinkageDto,
): AutonomousWorkflowLinkageView {
  return {
    workflowNodeId: normalizeText(workflowLinkage.workflowNodeId, 'workflow-node-unavailable'),
    transitionId: normalizeText(workflowLinkage.transitionId, 'workflow-transition-unavailable'),
    causalTransitionId: normalizeOptionalText(workflowLinkage.causalTransitionId),
    handoffTransitionId: normalizeText(
      workflowLinkage.handoffTransitionId,
      'workflow-handoff-transition-unavailable',
    ),
    handoffPackageHash: normalizeText(
      workflowLinkage.handoffPackageHash,
      'workflow-handoff-package-hash-unavailable',
    ),
  }
}

export function mapAutonomousUnit(autonomousUnit: AutonomousUnitDto): AutonomousUnitView {
  return {
    projectId: autonomousUnit.projectId,
    runId: autonomousUnit.runId,
    unitId: normalizeText(autonomousUnit.unitId, 'autonomous-unit-unavailable'),
    sequence: autonomousUnit.sequence,
    kind: autonomousUnit.kind,
    kindLabel: getAutonomousUnitKindLabel(autonomousUnit.kind),
    status: autonomousUnit.status,
    statusLabel: getAutonomousUnitStatusLabel(autonomousUnit.status),
    summary: normalizeText(autonomousUnit.summary, 'Autonomous unit boundary recorded.'),
    boundaryId: normalizeOptionalText(autonomousUnit.boundaryId),
    workflowLinkage: autonomousUnit.workflowLinkage
      ? mapAutonomousWorkflowLinkage(autonomousUnit.workflowLinkage)
      : null,
    startedAt: autonomousUnit.startedAt,
    finishedAt: normalizeOptionalText(autonomousUnit.finishedAt),
    updatedAt: autonomousUnit.updatedAt,
    lastErrorCode: normalizeOptionalText(autonomousUnit.lastErrorCode),
    lastError: autonomousUnit.lastError ?? null,
    isActive: autonomousUnit.status === 'active',
    isTerminal: ['completed', 'cancelled', 'failed'].includes(autonomousUnit.status),
    isFailed: autonomousUnit.status === 'failed',
  }
}

function getAutonomousArtifactKindLabel(artifactKind: string): string {
  switch (artifactKind) {
    case 'tool_result':
      return 'Tool result'
    case 'verification_evidence':
      return 'Verification evidence'
    case 'policy_denied':
      return 'Policy denied'
    default:
      return humanizeRuntimeKind(artifactKind)
  }
}

function getAutonomousArtifactStatusLabel(status: AutonomousUnitArtifactStatusDto): string {
  switch (status) {
    case 'pending':
      return 'Pending'
    case 'recorded':
      return 'Recorded'
    case 'rejected':
      return 'Rejected'
    case 'redacted':
      return 'Redacted'
  }
}

function getAutonomousToolCallStateLabel(state: AutonomousToolCallStateDto): string {
  switch (state) {
    case 'pending':
      return 'Pending'
    case 'running':
      return 'Running'
    case 'succeeded':
      return 'Succeeded'
    case 'failed':
      return 'Failed'
  }
}

function getAutonomousVerificationOutcomeLabel(outcome: AutonomousVerificationOutcomeDto): string {
  switch (outcome) {
    case 'passed':
      return 'Passed'
    case 'failed':
      return 'Failed'
    case 'blocked':
      return 'Blocked'
  }
}

export function mapAutonomousAttempt(autonomousAttempt: AutonomousUnitAttemptDto): AutonomousUnitAttemptView {
  return {
    projectId: autonomousAttempt.projectId,
    runId: autonomousAttempt.runId,
    unitId: autonomousAttempt.unitId,
    attemptId: normalizeText(autonomousAttempt.attemptId, 'autonomous-attempt-unavailable'),
    attemptNumber: autonomousAttempt.attemptNumber,
    childSessionId: normalizeText(autonomousAttempt.childSessionId, 'child-session-unavailable'),
    status: autonomousAttempt.status,
    statusLabel: getAutonomousUnitStatusLabel(autonomousAttempt.status),
    boundaryId: normalizeOptionalText(autonomousAttempt.boundaryId),
    workflowLinkage: autonomousAttempt.workflowLinkage
      ? mapAutonomousWorkflowLinkage(autonomousAttempt.workflowLinkage)
      : null,
    startedAt: autonomousAttempt.startedAt,
    finishedAt: normalizeOptionalText(autonomousAttempt.finishedAt),
    updatedAt: autonomousAttempt.updatedAt,
    lastErrorCode: normalizeOptionalText(autonomousAttempt.lastErrorCode),
    lastError: autonomousAttempt.lastError ?? null,
    isActive: autonomousAttempt.status === 'active',
    isTerminal: ['completed', 'cancelled', 'failed'].includes(autonomousAttempt.status),
    isFailed: autonomousAttempt.status === 'failed',
  }
}

function mapAutonomousCommandResult(commandResult: AutonomousCommandResultDto): AutonomousCommandResultView {
  return {
    exitCode: commandResult.exitCode ?? null,
    timedOut: commandResult.timedOut,
    summary: normalizeText(commandResult.summary, 'Autonomous command result recorded.'),
  }
}

function getAutonomousArtifactDetail(
  artifact: AutonomousUnitArtifactDto,
  commandResult: AutonomousCommandResultView | null,
): string | null {
  const payload = artifact.payload ?? null
  if (!payload) {
    return normalizeOptionalText(artifact.summary)
  }

  switch (payload.kind) {
    case 'tool_result':
      return commandResult?.summary ?? normalizeOptionalText(artifact.summary)
    case 'verification_evidence':
      return commandResult?.summary ?? normalizeOptionalText(payload.label) ?? normalizeOptionalText(artifact.summary)
    case 'policy_denied':
      return normalizeOptionalText(payload.message) ?? normalizeOptionalText(artifact.summary)
  }
}

export function mapAutonomousArtifact(artifact: AutonomousUnitArtifactDto): AutonomousUnitArtifactView {
  const payload = artifact.payload ?? null
  const commandResult =
    payload != null
      && (payload.kind === 'tool_result' || payload.kind === 'verification_evidence')
      && payload.commandResult
      ? mapAutonomousCommandResult(payload.commandResult)
      : null

  let toolSummary: ToolResultSummaryDto | null = null
  let toolName: string | null = null
  let toolState: AutonomousToolCallStateDto | null = null
  let toolStateLabel: string | null = null
  let evidenceKind: string | null = null
  let verificationOutcome: AutonomousVerificationOutcomeDto | null = null
  let verificationOutcomeLabel: string | null = null
  let diagnosticCode: string | null = null
  let actionId: string | null = null
  let boundaryId: string | null = null

  switch (payload?.kind) {
    case 'tool_result':
      toolSummary = payload.toolSummary ?? null
      toolName = normalizeOptionalText(payload.toolName)
      toolState = payload.toolState
      toolStateLabel = getAutonomousToolCallStateLabel(payload.toolState)
      actionId = normalizeOptionalText(payload.actionId)
      boundaryId = normalizeOptionalText(payload.boundaryId)
      break
    case 'verification_evidence':
      evidenceKind = normalizeOptionalText(payload.evidenceKind)
      verificationOutcome = payload.outcome
      verificationOutcomeLabel = getAutonomousVerificationOutcomeLabel(payload.outcome)
      actionId = normalizeOptionalText(payload.actionId)
      boundaryId = normalizeOptionalText(payload.boundaryId)
      break
    case 'policy_denied':
      toolName = normalizeOptionalText(payload.toolName)
      diagnosticCode = normalizeOptionalText(payload.diagnosticCode)
      actionId = normalizeOptionalText(payload.actionId)
      boundaryId = normalizeOptionalText(payload.boundaryId)
      break
  }

  return {
    projectId: artifact.projectId,
    runId: artifact.runId,
    unitId: artifact.unitId,
    attemptId: artifact.attemptId,
    artifactId: normalizeText(artifact.artifactId, 'autonomous-artifact-unavailable'),
    artifactKind: artifact.artifactKind,
    artifactKindLabel: getAutonomousArtifactKindLabel(artifact.artifactKind),
    status: artifact.status,
    statusLabel: getAutonomousArtifactStatusLabel(artifact.status),
    summary: normalizeText(artifact.summary, 'Autonomous artifact recorded.'),
    contentHash: normalizeOptionalText(artifact.contentHash),
    payload,
    createdAt: artifact.createdAt,
    updatedAt: artifact.updatedAt,
    detail: getAutonomousArtifactDetail(artifact, commandResult),
    commandResult,
    toolSummary,
    toolName,
    toolState,
    toolStateLabel,
    evidenceKind,
    verificationOutcome,
    verificationOutcomeLabel,
    diagnosticCode,
    actionId,
    boundaryId,
    isToolResult: artifact.artifactKind === 'tool_result',
    isVerificationEvidence: artifact.artifactKind === 'verification_evidence',
    isPolicyDenied: artifact.artifactKind === 'policy_denied',
  }
}

export function mapAutonomousHistoryEntry(entry: AutonomousUnitHistoryEntryDto): AutonomousUnitHistoryEntryView {
  return {
    unit: mapAutonomousUnit(entry.unit),
    latestAttempt: entry.latestAttempt ? mapAutonomousAttempt(entry.latestAttempt) : null,
    artifacts: sortByNewest((entry.artifacts ?? []).map(mapAutonomousArtifact), (artifact) => artifact.updatedAt || artifact.createdAt),
  }
}

export function mapAutonomousRunInspection(autonomousState: AutonomousRunStateDto): AutonomousRunInspectionView {
  const autonomousHistory = (autonomousState.history ?? []).map(mapAutonomousHistoryEntry)
  const autonomousRecentArtifacts = sortByNewest(
    autonomousHistory.flatMap((entry) => entry.artifacts),
    (artifact) => artifact.updatedAt || artifact.createdAt,
  ).slice(0, 5)

  return {
    autonomousRun: autonomousState.run ? mapAutonomousRun(autonomousState.run) : null,
    autonomousUnit: autonomousState.unit ? mapAutonomousUnit(autonomousState.unit) : null,
    autonomousAttempt: autonomousState.attempt ? mapAutonomousAttempt(autonomousState.attempt) : null,
    autonomousHistory,
    autonomousRecentArtifacts,
  }
}

export function mergeRuntimeUpdated(
  currentRuntime: RuntimeSessionView | null,
  payload: RuntimeUpdatedPayloadDto,
): RuntimeSessionView {
  if (currentRuntime && timestampToSortValue(payload.updatedAt) < timestampToSortValue(currentRuntime.updatedAt)) {
    return currentRuntime
  }

  const nextFlowId = normalizeOptionalText(payload.flowId)
  const currentFlowId = currentRuntime?.flowId ?? null

  return mapRuntimeSession({
    projectId: payload.projectId,
    runtimeKind: payload.runtimeKind,
    providerId: payload.providerId,
    flowId: nextFlowId,
    sessionId: normalizeOptionalText(payload.sessionId),
    accountId: normalizeOptionalText(payload.accountId),
    phase: payload.authPhase,
    callbackBound: currentFlowId === nextFlowId ? currentRuntime?.callbackBound ?? null : null,
    authorizationUrl: currentFlowId === nextFlowId ? currentRuntime?.authorizationUrl ?? null : null,
    redirectUri: currentFlowId === nextFlowId ? currentRuntime?.redirectUri ?? null : null,
    lastErrorCode: normalizeOptionalText(payload.lastErrorCode),
    lastError: payload.lastError ?? null,
    updatedAt: payload.updatedAt,
  })
}

export function applyRuntimeSession(
  project: ProjectDetailView,
  runtimeSession: RuntimeSessionView | null,
): ProjectDetailView {
  if (!runtimeSession) {
    return {
      ...project,
      runtimeSession: null,
    }
  }

  return {
    ...project,
    runtime: runtimeSession.runtimeLabel,
    runtimeLabel: runtimeSession.runtimeLabel,
    runtimeSession,
  }
}

export function applyRuntimeRun(
  project: ProjectDetailView,
  runtimeRun: RuntimeRunView | null,
): ProjectDetailView {
  return {
    ...project,
    runtimeRun: runtimeRun ?? null,
  }
}
