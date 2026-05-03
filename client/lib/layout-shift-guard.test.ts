import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import {
  LAYOUT_SHIFT_GUARD_ATTRIBUTE,
  LAYOUT_SHIFT_GUARD_VALUE,
  startLayoutShiftGuard,
} from './layout-shift-guard'

describe('startLayoutShiftGuard', () => {
  let callbacks: FrameRequestCallback[]
  let frameId: number
  let originalRequestAnimationFrame: typeof window.requestAnimationFrame
  let originalCancelAnimationFrame: typeof window.cancelAnimationFrame

  beforeEach(() => {
    callbacks = []
    frameId = 0
    delete document.documentElement.dataset[LAYOUT_SHIFT_GUARD_ATTRIBUTE]
    originalRequestAnimationFrame = window.requestAnimationFrame
    originalCancelAnimationFrame = window.cancelAnimationFrame

    window.requestAnimationFrame = vi.fn((callback: FrameRequestCallback) => {
      callbacks.push(callback)
      frameId += 1
      return frameId
    })
    window.cancelAnimationFrame = vi.fn((id: number) => {
      callbacks[id - 1] = () => undefined
    })
  })

  afterEach(() => {
    window.requestAnimationFrame = originalRequestAnimationFrame
    window.cancelAnimationFrame = originalCancelAnimationFrame
    delete document.documentElement.dataset[LAYOUT_SHIFT_GUARD_ATTRIBUTE]
  })

  it('marks the document for two animation frames by default', () => {
    startLayoutShiftGuard()

    expect(document.documentElement.dataset[LAYOUT_SHIFT_GUARD_ATTRIBUTE]).toBe(
      LAYOUT_SHIFT_GUARD_VALUE,
    )

    callbacks.shift()?.(16)
    expect(document.documentElement.dataset[LAYOUT_SHIFT_GUARD_ATTRIBUTE]).toBe(
      LAYOUT_SHIFT_GUARD_VALUE,
    )

    callbacks.shift()?.(32)
    expect(document.documentElement.dataset[LAYOUT_SHIFT_GUARD_ATTRIBUTE]).toBeUndefined()
  })

  it('clears the guard when cancelled', () => {
    const cancel = startLayoutShiftGuard()

    cancel()

    expect(window.cancelAnimationFrame).toHaveBeenCalledWith(1)
    expect(document.documentElement.dataset[LAYOUT_SHIFT_GUARD_ATTRIBUTE]).toBeUndefined()
  })
})
