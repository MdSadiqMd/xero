import { describe, expect, it } from 'vitest'
import {
  CADENCE_SESSION_CONTEXT_CONTRACT_VERSION,
  createContextBudget,
  createPublicSessionContextRedaction,
  createRedactedSessionContextText,
  runTranscriptSchema,
  sessionContextContributorSchema,
  sessionContextPolicyDecisionSchema,
  sessionContextSnapshotSchema,
  sessionMemoryRecordSchema,
  sessionTranscriptExportPayloadSchema,
  sessionTranscriptSchema,
  type RunTranscriptDto,
  type SessionContextContributorDto,
  type SessionContextPolicyDecisionDto,
} from './session-context'

const projectId = 'project-context'
const agentSessionId = 'agent-session-context'
const runId = 'run-context'
const providerId = 'openrouter'
const modelId = 'openai/gpt-5.4'
const createdAt = '2026-04-26T10:00:00Z'

describe('session context contract', () => {
  it('accepts a strict run transcript and rejects malformed or unordered payloads', () => {
    const transcript = runTranscriptSchema.parse(makeRunTranscript())

    expect(transcript.items).toHaveLength(2)
    expect(transcript.items[0]).toMatchObject({
      itemId: 'message:1',
      actor: 'user',
      kind: 'message',
    })

    expect(() =>
      runTranscriptSchema.parse({
        ...transcript,
        items: [
          transcript.items[0],
          {
            ...transcript.items[1],
            sequence: transcript.items[0].sequence,
          },
        ],
      }),
    ).toThrow(/strictly increasing/)

    expect(() =>
      runTranscriptSchema.parse({
        ...transcript,
        items: [
          {
            ...transcript.items[0],
            unexpected: true,
          },
        ],
      }),
    ).toThrow()
  })

  it('accepts archived empty sessions and export/search-adjacent shapes', () => {
    const transcript = sessionTranscriptSchema.parse({
      contractVersion: CADENCE_SESSION_CONTEXT_CONTRACT_VERSION,
      projectId,
      agentSessionId,
      title: 'Archived context',
      summary: '',
      status: 'archived',
      archived: true,
      archivedAt: '2026-04-26T10:05:00Z',
      runs: [],
      items: [],
      usageTotals: null,
      redaction: createPublicSessionContextRedaction(),
    })

    expect(transcript.archived).toBe(true)
    expect(
      sessionTranscriptExportPayloadSchema.parse({
        contractVersion: CADENCE_SESSION_CONTEXT_CONTRACT_VERSION,
        exportId: 'export-1',
        generatedAt: '2026-04-26T10:06:00Z',
        scope: 'session',
        format: 'json',
        transcript,
        contextSnapshot: null,
        redaction: createPublicSessionContextRedaction(),
      }).transcript.items,
    ).toEqual([])

    expect(() =>
      sessionTranscriptSchema.parse({
        ...transcript,
        archivedAt: null,
      }),
    ).toThrow(/archivedAt/)
  })

  it('validates context snapshots, contributors, and compaction policy invariants', () => {
    const contributor = makeContributor('memory:project-decision', 1)
    const instruction = makeContributor('instruction:AGENTS.md', 2, {
      kind: 'instruction_file',
      text: 'Use tests rather than temporary UI.',
    })
    const policy: SessionContextPolicyDecisionDto = sessionContextPolicyDecisionSchema.parse({
      contractVersion: CADENCE_SESSION_CONTEXT_CONTRACT_VERSION,
      decisionId: 'compaction:auto:ready',
      kind: 'compaction',
      action: 'compact_now',
      trigger: 'auto',
      reasonCode: 'auto_compact_threshold_reached',
      message: 'Auto-compact should run before the next provider turn.',
      rawTranscriptPreserved: true,
      modelVisible: false,
      redaction: createPublicSessionContextRedaction(),
    })

    const snapshot = sessionContextSnapshotSchema.parse({
      contractVersion: CADENCE_SESSION_CONTEXT_CONTRACT_VERSION,
      snapshotId: 'context-snapshot-1',
      projectId,
      agentSessionId,
      runId,
      providerId,
      modelId,
      generatedAt: '2026-04-26T10:10:00Z',
      budget: createContextBudget(800, 1000),
      contributors: [contributor, instruction],
      policyDecisions: [policy],
      usageTotals: null,
      redaction: createPublicSessionContextRedaction(),
    })

    expect(snapshot.budget.pressure).toBe('high')
    expect(() =>
      sessionContextSnapshotSchema.parse({
        ...snapshot,
        contributors: [instruction, contributor],
      }),
    ).toThrow(/strictly increasing/)

    expect(() =>
      sessionContextPolicyDecisionSchema.parse({
        ...policy,
        rawTranscriptPreserved: false,
      }),
    ).toThrow(/preserve raw transcript/)

    expect(() =>
      sessionContextContributorSchema.parse({
        ...contributor,
        included: false,
      }),
    ).toThrow(/Model-visible/)
  })

  it('keeps approved memory schema explicit and redacts secret-bearing text helpers', () => {
    const redacted = createRedactedSessionContextText('Use api_key=sk-context-secret')
    expect(redacted.value).toBe('Cadence redacted sensitive session-context text.')
    expect(redacted.redaction).toMatchObject({
      redactionClass: 'secret',
      redacted: true,
    })

    const memory = sessionMemoryRecordSchema.parse({
      memoryId: 'memory-1',
      projectId,
      agentSessionId,
      scope: 'project',
      kind: 'decision',
      text: redacted.value,
      reviewState: 'approved',
      enabled: true,
      confidence: 95,
      sourceRunId: runId,
      sourceItemIds: ['message:1'],
      createdAt,
      updatedAt: createdAt,
      redaction: redacted.redaction,
    })

    const serialized = JSON.stringify(memory)
    expect(serialized).not.toContain('sk-context-secret')
    expect(memory.reviewState).toBe('approved')
  })
})

function makeRunTranscript(): RunTranscriptDto {
  return {
    contractVersion: CADENCE_SESSION_CONTEXT_CONTRACT_VERSION,
    projectId,
    agentSessionId,
    runId,
    providerId,
    modelId,
    status: 'completed',
    sourceKind: 'owned_agent',
    startedAt: createdAt,
    completedAt: '2026-04-26T10:01:00Z',
    usageTotals: {
      projectId,
      runId,
      providerId,
      modelId,
      inputTokens: 100,
      outputTokens: 50,
      totalTokens: 150,
      estimatedCostMicros: 10,
      source: 'provider',
      updatedAt: '2026-04-26T10:01:00Z',
    },
    items: [
      {
        contractVersion: CADENCE_SESSION_CONTEXT_CONTRACT_VERSION,
        itemId: 'message:1',
        projectId,
        agentSessionId,
        runId,
        providerId,
        modelId,
        sourceKind: 'owned_agent',
        sourceTable: 'agent_messages',
        sourceId: '1',
        sequence: 1,
        createdAt,
        kind: 'message',
        actor: 'user',
        title: 'User message',
        text: 'Implement priority 4 phase 0.',
        summary: null,
        toolCallId: null,
        toolName: null,
        toolState: null,
        filePath: null,
        checkpointKind: null,
        actionId: null,
        redaction: createPublicSessionContextRedaction(),
      },
      {
        contractVersion: CADENCE_SESSION_CONTEXT_CONTRACT_VERSION,
        itemId: 'tool_call:1',
        projectId,
        agentSessionId,
        runId,
        providerId,
        modelId,
        sourceKind: 'owned_agent',
        sourceTable: 'agent_tool_calls',
        sourceId: 'tool-1',
        sequence: 2,
        createdAt: '2026-04-26T10:00:10Z',
        kind: 'tool_call',
        actor: 'tool',
        title: 'Tool call',
        text: null,
        summary: 'read succeeded.',
        toolCallId: 'tool-1',
        toolName: 'read',
        toolState: 'succeeded',
        filePath: null,
        checkpointKind: null,
        actionId: null,
        redaction: createPublicSessionContextRedaction(),
      },
    ],
    redaction: createPublicSessionContextRedaction(),
  }
}

function makeContributor(
  contributorId: string,
  sequence: number,
  overrides: Partial<SessionContextContributorDto> = {},
): SessionContextContributorDto {
  return {
    contributorId,
    kind: 'approved_memory',
    label: 'Project decision',
    projectId,
    agentSessionId,
    runId,
    sourceId: contributorId,
    sequence,
    estimatedTokens: 20,
    estimatedChars: 80,
    included: true,
    modelVisible: true,
    text: 'Use the native owned-agent runtime.',
    redaction: createPublicSessionContextRedaction(),
    ...overrides,
  }
}
