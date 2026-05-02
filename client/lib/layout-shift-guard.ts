export const LAYOUT_SHIFT_GUARD_ATTRIBUTE = 'layoutShifting'
export const LAYOUT_SHIFT_GUARD_VALUE = 'true'

export interface LayoutShiftGuardOptions {
  frames?: number
  target?: HTMLElement
}

export function startLayoutShiftGuard(options: LayoutShiftGuardOptions = {}) {
  if (typeof window === 'undefined' || typeof document === 'undefined') {
    return () => undefined
  }

  const target = options.target ?? document.documentElement
  const frames = Math.max(1, Math.floor(options.frames ?? 2))
  let cancelled = false
  let frameId: number | null = null

  const clear = () => {
    delete target.dataset[LAYOUT_SHIFT_GUARD_ATTRIBUTE]
    frameId = null
  }

  const schedule = (remainingFrames: number) => {
    frameId = window.requestAnimationFrame(() => {
      if (cancelled) {
        return
      }

      if (remainingFrames <= 1) {
        clear()
        return
      }

      schedule(remainingFrames - 1)
    })
  }

  target.dataset[LAYOUT_SHIFT_GUARD_ATTRIBUTE] = LAYOUT_SHIFT_GUARD_VALUE
  schedule(frames)

  return () => {
    cancelled = true
    if (frameId !== null) {
      window.cancelAnimationFrame(frameId)
    }
    clear()
  }
}
