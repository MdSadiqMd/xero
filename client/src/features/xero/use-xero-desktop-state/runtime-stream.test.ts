import { describe, expect, it, vi } from 'vitest'
import type { MutableRefObject } from 'react'
import {
  createRuntimeStreamEventBuffer,
  mergeRuntimeStreamEvents,
} from './runtime-stream'
import {
  createRuntimeStreamView,
  type RuntimeStreamEventDto,
  type RuntimeStreamView,
} from '@/src/lib/xero-model/runtime-stream'

function makeRuntimeStreamEvent(
  sequence: number,
  overrides: Partial<RuntimeStreamEventDto['item']> = {},
): RuntimeStreamEventDto {
  const kind = overrides.kind ?? 'transcript'

  return {
    projectId: 'project-1',
    agentSessionId: 'agent-session-main',
    runtimeKind: 'openai_codex',
    runId: 'run-1',
    sessionId: 'session-1',
    flowId: 'flow-1',
    subscribedItemKinds: ['transcript', 'tool', 'skill', 'activity', 'action_required', 'complete', 'failure'],
    item: {
      kind,
      runId: 'run-1',
      sequence,
      sessionId: 'session-1',
      flowId: 'flow-1',
      text: kind === 'transcript' ? `message-${sequence}` : null,
      transcriptRole: kind === 'transcript' ? 'assistant' : null,
      toolCallId: null,
      toolName: null,
      toolState: null,
      toolSummary: null,
      skillId: null,
      skillStage: null,
      skillResult: null,
      skillSource: null,
      skillCacheStatus: null,
      skillDiagnostic: null,
      actionId: null,
      boundaryId: null,
      actionType: null,
      title: null,
      detail: null,
      code: null,
      message: null,
      retryable: null,
      createdAt: `2026-04-16T13:30:${String(sequence).padStart(2, '0')}Z`,
      ...overrides,
    },
  }
}

function makeRuntimeStream(): RuntimeStreamView {
  return createRuntimeStreamView({
    projectId: 'project-1',
    agentSessionId: 'agent-session-main',
    runtimeKind: 'openai_codex',
    runId: 'run-1',
    sessionId: 'session-1',
    flowId: 'flow-1',
    subscribedItemKinds: ['transcript', 'tool', 'skill', 'activity', 'action_required', 'complete', 'failure'],
    status: 'live',
  })
}

describe('runtime stream event coalescing', () => {
  it('flushes non-urgent stream items in one buffered update', () => {
    let stream: RuntimeStreamView | null = makeRuntimeStream()
    let scheduledFlush: (() => void) | null = null
    const updateRuntimeStream = vi.fn(
      (_projectId: string, updater: (current: RuntimeStreamView | null) => RuntimeStreamView | null) => {
        stream = updater(stream)
      },
    )

    const buffer = createRuntimeStreamEventBuffer({
      projectId: 'project-1',
      agentSessionId: 'agent-session-main',
      runtimeKind: 'openai_codex',
      runId: 'run-1',
      sessionId: 'session-1',
      flowId: 'flow-1',
      subscribedItemKinds: ['transcript'],
      runtimeActionRefreshKeysRef: { current: {} },
      updateRuntimeStream,
      scheduleRuntimeMetadataRefresh: vi.fn(),
      scheduleFlush: (callback) => {
        scheduledFlush = callback
        return vi.fn()
      },
    })

    buffer.enqueue(makeRuntimeStreamEvent(1))
    buffer.enqueue(makeRuntimeStreamEvent(2))

    expect(updateRuntimeStream).not.toHaveBeenCalled()
    scheduledFlush?.()

    expect(updateRuntimeStream).toHaveBeenCalledTimes(1)
    expect(stream?.lastSequence).toBe(2)
    expect(stream?.transcriptItems[0]?.text).toBe('message-1message-2')
  })

  it('flushes pending items immediately when an action-required event arrives', () => {
    let stream: RuntimeStreamView | null = makeRuntimeStream()
    let scheduledFlush: (() => void) | null = null
    const cancelScheduledFlush = vi.fn()
    const refreshKeysRef: MutableRefObject<Record<string, Set<string>>> = { current: {} }
    const scheduleRuntimeMetadataRefresh = vi.fn()
    const updateRuntimeStream = vi.fn(
      (_projectId: string, updater: (current: RuntimeStreamView | null) => RuntimeStreamView | null) => {
        stream = updater(stream)
      },
    )
    const buffer = createRuntimeStreamEventBuffer({
      projectId: 'project-1',
      agentSessionId: 'agent-session-main',
      runtimeKind: 'openai_codex',
      runId: 'run-1',
      sessionId: 'session-1',
      flowId: 'flow-1',
      subscribedItemKinds: ['transcript', 'action_required'],
      runtimeActionRefreshKeysRef: refreshKeysRef,
      updateRuntimeStream,
      scheduleRuntimeMetadataRefresh,
      scheduleFlush: (callback) => {
        scheduledFlush = callback
        return cancelScheduledFlush
      },
    })

    buffer.enqueue(makeRuntimeStreamEvent(1))
    buffer.enqueue(
      makeRuntimeStreamEvent(2, {
        kind: 'action_required',
        text: null,
        actionId: 'action-1',
        boundaryId: 'boundary-1',
        actionType: 'terminal_input_required',
        title: 'Terminal input required',
        detail: 'The runtime needs operator input.',
      }),
    )

    expect(updateRuntimeStream).toHaveBeenCalledTimes(1)
    expect(cancelScheduledFlush).toHaveBeenCalledTimes(1)
    expect(scheduledFlush).toBeTypeOf('function')
    expect(stream?.lastSequence).toBe(2)
    expect(stream?.actionRequired[0]?.actionId).toBe('action-1')
    expect(scheduleRuntimeMetadataRefresh).toHaveBeenCalledWith(
      'project-1',
      'runtime_stream:action_required',
    )
  })

  it('dedupes repeated stream item sequences inside a batch', () => {
    const stream = mergeRuntimeStreamEvents(makeRuntimeStream(), [
      makeRuntimeStreamEvent(1),
      makeRuntimeStreamEvent(1, { text: 'duplicate' }),
    ])

    expect(stream?.lastSequence).toBe(1)
    expect(stream?.transcriptItems).toHaveLength(1)
    expect(stream?.transcriptItems[0]?.text).toBe('message-1')
  })

  it('reports sequence gaps once while preserving the latest stream projection', () => {
    const stream = mergeRuntimeStreamEvents(makeRuntimeStream(), [
      makeRuntimeStreamEvent(1),
      makeRuntimeStreamEvent(3),
    ])

    expect(stream?.lastSequence).toBe(3)
    expect(stream?.status).toBe('stale')
    expect(stream?.lastIssue?.code).toBe('runtime_stream_sequence_gap')
    expect(stream?.lastIssue?.message).toContain('expected 2, received 3')
  })
})
