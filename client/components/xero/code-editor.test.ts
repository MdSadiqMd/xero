import { describe, expect, it, vi } from 'vitest'
import {
  countEditorLines,
  createEditorFrameScheduler,
  shouldReplaceEditorDocument,
} from './code-editor'

describe('CodeEditor hot-path helpers', () => {
  it('counts editor lines without special-casing empty documents as zero-line files', () => {
    expect(countEditorLines('')).toBe(1)
    expect(countEditorLines('one')).toBe(1)
    expect(countEditorLines('one\ntwo\n')).toBe(3)
  })

  it('only replaces the CodeMirror document for external changes or explicit document versions', () => {
    expect(
      shouldReplaceEditorDocument({
        externalValue: 'draft',
        lastSnapshot: 'draft',
        documentVersionChanged: false,
      }),
    ).toBe(false)
    expect(
      shouldReplaceEditorDocument({
        externalValue: 'reverted',
        lastSnapshot: 'draft',
        documentVersionChanged: false,
      }),
    ).toBe(true)
    expect(
      shouldReplaceEditorDocument({
        externalValue: 'draft',
        lastSnapshot: 'draft',
        documentVersionChanged: true,
      }),
    ).toBe(true)
  })

  it('coalesces repeated cursor reports into one animation frame', () => {
    const callbacks: Array<() => void> = []
    const cancelledFrames: number[] = []
    const run = vi.fn()
    const scheduler = createEditorFrameScheduler({
      requestFrame: (callback) => {
        callbacks.push(callback)
        return { id: callbacks.length, type: 'animation-frame' }
      },
      cancelFrame: (frame) => {
        cancelledFrames.push(frame.id)
      },
    })

    scheduler.schedule(run)
    scheduler.schedule(run)

    expect(callbacks).toHaveLength(1)
    expect(scheduler.isPending()).toBe(true)

    callbacks[0]()

    expect(run).toHaveBeenCalledTimes(1)
    expect(scheduler.isPending()).toBe(false)

    scheduler.schedule(run)
    expect(callbacks).toHaveLength(2)
    scheduler.cancel()

    expect(cancelledFrames).toEqual([2])
    expect(scheduler.isPending()).toBe(false)
  })
})
