import { describe, expect, it } from 'vitest'

import {
  StaleBackendRequestError,
  createBackendRequestCoordinator,
  stableBackendRequestKey,
} from './backend-request-coordinator'

describe('backend request coordinator', () => {
  it('deduplicates identical in-flight request keys', async () => {
    const coordinator = createBackendRequestCoordinator()
    let calls = 0

    const first = coordinator.runDeduped('repository-diff:project-1:unstaged', async () => {
      calls += 1
      return { patch: '+one' }
    })
    const second = coordinator.runDeduped('repository-diff:project-1:unstaged', async () => {
      calls += 1
      return { patch: '+two' }
    })

    await expect(first).resolves.toEqual({ patch: '+one' })
    await expect(second).resolves.toEqual({ patch: '+one' })
    expect(calls).toBe(1)
  })

  it('rejects older latest-wins responses for the same scope', async () => {
    const coordinator = createBackendRequestCoordinator()
    let resolveFirst: (value: string) => void = () => undefined

    const first = coordinator.runLatest(
      'visible-search',
      'search:one',
      () =>
        new Promise<string>((resolve) => {
          resolveFirst = resolve
        }),
    )
    const second = coordinator.runLatest('visible-search', 'search:two', async () => 'second')

    await expect(second).resolves.toBe('second')
    resolveFirst('first')
    await expect(first).rejects.toBeInstanceOf(StaleBackendRequestError)
  })

  it('uses stable keys for object arguments independent of property order', () => {
    expect(
      stableBackendRequestKey([
        'search_project',
        { projectId: 'project-1', query: 'needle', regex: false },
      ]),
    ).toBe(
      stableBackendRequestKey([
        'search_project',
        { regex: false, query: 'needle', projectId: 'project-1' },
      ]),
    )
  })
})
