export type FrameDropReason = 'coalesced' | 'disabled'

export interface FrameCoalescerMetrics {
  coalescedDrops: number
  disabledDrops: number
  flushes: number
  scheduledFrames: number
  scheduledValues: number
}

interface FrameCoalescerOptions<T> {
  cancelFrame?: (id: number) => void
  getEnabled?: () => boolean
  onDrop?: (value: T, reason: FrameDropReason) => void
  onFlush: (value: T) => void
  requestFrame?: (callback: FrameRequestCallback) => number
}

export interface FrameCoalescer<T> {
  cancel: () => void
  dispose: () => void
  flush: () => void
  getMetrics: () => FrameCoalescerMetrics
  getPendingValue: () => T | null
  schedule: (value: T) => void
}

function requestDefaultFrame(callback: FrameRequestCallback): number {
  if (typeof window !== 'undefined' && typeof window.requestAnimationFrame === 'function') {
    return window.requestAnimationFrame(callback)
  }
  const timeout = setTimeout(() => callback(performance.now()), 16)
  return Number(timeout)
}

function cancelDefaultFrame(id: number): void {
  if (typeof window !== 'undefined' && typeof window.cancelAnimationFrame === 'function') {
    window.cancelAnimationFrame(id)
    return
  }
  clearTimeout(id)
}

export function createFrameCoalescer<T>({
  cancelFrame = cancelDefaultFrame,
  getEnabled = () => true,
  onDrop,
  onFlush,
  requestFrame = requestDefaultFrame,
}: FrameCoalescerOptions<T>): FrameCoalescer<T> {
  let disposed = false
  let frameId: number | null = null
  let hasPending = false
  let pendingValue: T | null = null
  const metrics: FrameCoalescerMetrics = {
    coalescedDrops: 0,
    disabledDrops: 0,
    flushes: 0,
    scheduledFrames: 0,
    scheduledValues: 0,
  }

  const clearPendingFrame = () => {
    if (frameId === null) return
    cancelFrame(frameId)
    frameId = null
  }

  const run = () => {
    frameId = null
    if (disposed || !hasPending) return

    const value = pendingValue as T
    pendingValue = null
    hasPending = false

    if (!getEnabled()) {
      metrics.disabledDrops += 1
      onDrop?.(value, 'disabled')
      return
    }

    metrics.flushes += 1
    onFlush(value)
  }

  return {
    cancel() {
      clearPendingFrame()
      pendingValue = null
      hasPending = false
    },
    dispose() {
      disposed = true
      clearPendingFrame()
      pendingValue = null
      hasPending = false
    },
    flush() {
      if (!hasPending) return
      clearPendingFrame()
      run()
    },
    getMetrics() {
      return { ...metrics }
    },
    getPendingValue() {
      return hasPending ? pendingValue : null
    },
    schedule(value) {
      if (disposed) return

      metrics.scheduledValues += 1

      if (!getEnabled()) {
        metrics.disabledDrops += 1
        onDrop?.(value, 'disabled')
        return
      }

      if (hasPending && pendingValue !== null) {
        metrics.coalescedDrops += 1
        onDrop?.(pendingValue, 'coalesced')
      }

      pendingValue = value
      hasPending = true

      if (frameId !== null) return

      metrics.scheduledFrames += 1
      frameId = requestFrame(run)
    },
  }
}

export function isDocumentHidden(): boolean {
  return typeof document !== 'undefined' && document.visibilityState === 'hidden'
}

