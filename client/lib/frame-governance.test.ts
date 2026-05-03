import { describe, expect, it, vi } from 'vitest'

import { createFrameCoalescer } from './frame-governance'

function createFrameController() {
  let nextId = 1
  const frames = new Map<number, FrameRequestCallback>()

  return {
    cancelFrame(id: number) {
      frames.delete(id)
    },
    flushFrame() {
      const [id, callback] = frames.entries().next().value ?? []
      if (!id || !callback) throw new Error('No pending frame')
      frames.delete(id)
      callback(0)
    },
    get pendingCount() {
      return frames.size
    },
    requestFrame(callback: FrameRequestCallback) {
      const id = nextId
      nextId += 1
      frames.set(id, callback)
      return id
    },
  }
}

describe('createFrameCoalescer', () => {
  it('flushes only the latest value once per animation frame', () => {
    const frames = createFrameController()
    const onFlush = vi.fn()
    const onDrop = vi.fn()
    const coalescer = createFrameCoalescer<number>({
      cancelFrame: frames.cancelFrame,
      onDrop,
      onFlush,
      requestFrame: frames.requestFrame,
    })

    for (let index = 0; index < 200; index += 1) {
      coalescer.schedule(index)
    }

    expect(frames.pendingCount).toBe(1)
    expect(onFlush).not.toHaveBeenCalled()

    frames.flushFrame()

    expect(onFlush).toHaveBeenCalledTimes(1)
    expect(onFlush).toHaveBeenCalledWith(199)
    expect(onDrop).toHaveBeenCalledTimes(199)
    expect(coalescer.getMetrics()).toMatchObject({
      coalescedDrops: 199,
      flushes: 1,
      scheduledFrames: 1,
      scheduledValues: 200,
    })
  })

  it('drops values while disabled and cancels pending work on dispose', () => {
    const frames = createFrameController()
    const onFlush = vi.fn()
    const onDrop = vi.fn()
    let enabled = true
    const coalescer = createFrameCoalescer<number>({
      cancelFrame: frames.cancelFrame,
      getEnabled: () => enabled,
      onDrop,
      onFlush,
      requestFrame: frames.requestFrame,
    })

    coalescer.schedule(1)
    expect(frames.pendingCount).toBe(1)

    enabled = false
    coalescer.flush()

    expect(onFlush).not.toHaveBeenCalled()
    expect(onDrop).toHaveBeenCalledWith(1, 'disabled')

    coalescer.schedule(2)
    expect(frames.pendingCount).toBe(0)

    enabled = true
    coalescer.schedule(3)
    expect(frames.pendingCount).toBe(1)
    coalescer.dispose()
    expect(frames.pendingCount).toBe(0)

    coalescer.schedule(4)
    expect(onFlush).not.toHaveBeenCalled()
    expect(coalescer.getMetrics()).toMatchObject({
      disabledDrops: 2,
      flushes: 0,
      scheduledFrames: 2,
      scheduledValues: 3,
    })
  })
})

