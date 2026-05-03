export class StaleBackendRequestError extends Error {
  scope: string

  constructor(scope: string, options: { cause?: unknown } = {}) {
    super(`Stale backend request ignored for ${scope}.`)
    this.name = 'StaleBackendRequestError'
    this.scope = scope
    if (options.cause !== undefined) {
      ;(this as Error & { cause?: unknown }).cause = options.cause
    }
  }
}

export interface BackendRequestCoordinator {
  cancelScope(scope: string): void
  runDeduped<T>(requestKey: string, work: () => Promise<T>): Promise<T>
  runLatest<T>(scope: string, requestKey: string, work: () => Promise<T>): Promise<T>
}

export function isStaleBackendRequestError(error: unknown): error is StaleBackendRequestError {
  return error instanceof StaleBackendRequestError
}

export function createBackendRequestCoordinator(): BackendRequestCoordinator {
  const inFlight = new Map<string, Promise<unknown>>()
  const latestByScope = new Map<string, number>()

  function nextSequence(scope: string): number {
    const next = (latestByScope.get(scope) ?? 0) + 1
    latestByScope.set(scope, next)
    return next
  }

  function isLatest(scope: string, sequence: number): boolean {
    return latestByScope.get(scope) === sequence
  }

  function runDeduped<T>(requestKey: string, work: () => Promise<T>): Promise<T> {
    const existing = inFlight.get(requestKey)
    if (existing) {
      return existing as Promise<T>
    }

    const promise = Promise.resolve()
      .then(work)
      .finally(() => {
        if (inFlight.get(requestKey) === promise) {
          inFlight.delete(requestKey)
        }
      })

    inFlight.set(requestKey, promise)
    return promise
  }

  return {
    cancelScope(scope) {
      nextSequence(scope)
    },

    runDeduped,

    async runLatest<T>(
      scope: string,
      requestKey: string,
      work: () => Promise<T>,
    ): Promise<T> {
      const sequence = nextSequence(scope)
      try {
        const response = await runDeduped<T>(requestKey, work)
        if (!isLatest(scope, sequence)) {
          throw new StaleBackendRequestError(scope)
        }
        return response
      } catch (error) {
        if (!isLatest(scope, sequence)) {
          throw new StaleBackendRequestError(scope, { cause: error })
        }
        throw error
      }
    },
  }
}

export function stableBackendRequestKey(parts: unknown[]): string {
  return stableStringify(parts)
}

function stableStringify(value: unknown): string {
  if (value === undefined) {
    return '"__undefined__"'
  }

  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value) ?? JSON.stringify(String(value))
  }

  if (Array.isArray(value)) {
    return `[${value.map((item) => stableStringify(item)).join(',')}]`
  }

  const record = value as Record<string, unknown>
  const keys = Object.keys(record).sort()
  return `{${keys
    .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
    .join(',')}}`
}
