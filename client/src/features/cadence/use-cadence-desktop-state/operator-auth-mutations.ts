import { useCallback } from 'react'

import { mapRuntimeSession } from '@/src/lib/cadence-model/runtime'

import type {
  CadenceDesktopMutationActions,
  UseCadenceDesktopMutationsArgs,
} from './mutation-support'
import {
  getActiveProjectId,
  getOperatorActionError,
} from './mutation-support'

export function useOperatorAuthMutations({
  adapter,
  refs,
  setters,
  operations,
}: UseCadenceDesktopMutationsArgs): Pick<
  CadenceDesktopMutationActions,
  | 'startOpenAiLogin'
  | 'submitOpenAiCallback'
  | 'startRuntimeSession'
  | 'logoutRuntimeSession'
  | 'resolveOperatorAction'
  | 'resumeOperatorRun'
> {
  const { activeProjectIdRef, activeProjectRef } = refs
  const {
    setErrorMessage,
    setOperatorActionStatus,
    setPendingOperatorActionId,
    setOperatorActionError,
  } = setters
  const {
    loadProject,
    syncRuntimeSession,
    applyRuntimeSessionUpdate,
  } = operations

  const resolveOperatorAction = useCallback(
    async (
      actionId: string,
      decision: Parameters<CadenceDesktopMutationActions['resolveOperatorAction']>[1],
      options: { userAnswer?: string | null } = {},
    ) => {
      const projectId = getActiveProjectId(
        activeProjectIdRef,
        'Select an imported project before resolving an operator action.',
      )

      setOperatorActionStatus('running')
      setPendingOperatorActionId(actionId)
      setOperatorActionError(null)
      setErrorMessage(null)

      try {
        await adapter.resolveOperatorAction(projectId, actionId, decision, {
          userAnswer: options.userAnswer ?? null,
        })
        await loadProject(projectId, 'operator:resolve')
        return activeProjectIdRef.current === projectId ? activeProjectRef.current : null
      } catch (error) {
        setOperatorActionError(
          getOperatorActionError(
            error,
            'Cadence could not persist the operator decision for this project.',
          ),
        )
        throw error
      } finally {
        setOperatorActionStatus('idle')
        setPendingOperatorActionId(null)
      }
    },
    [
      activeProjectIdRef,
      activeProjectRef,
      adapter,
      loadProject,
      setErrorMessage,
      setOperatorActionError,
      setOperatorActionStatus,
      setPendingOperatorActionId,
    ],
  )

  const resumeOperatorRun = useCallback(
    async (actionId: string, options: { userAnswer?: string | null } = {}) => {
      const projectId = getActiveProjectId(
        activeProjectIdRef,
        'Select an imported project before resuming the runtime session.',
      )

      setOperatorActionStatus('running')
      setPendingOperatorActionId(actionId)
      setOperatorActionError(null)
      setErrorMessage(null)

      try {
        await adapter.resumeOperatorRun(projectId, actionId, {
          userAnswer: options.userAnswer ?? null,
        })
        await loadProject(projectId, 'operator:resume')
        return activeProjectIdRef.current === projectId ? activeProjectRef.current : null
      } catch (error) {
        setOperatorActionError(
          getOperatorActionError(
            error,
            'Cadence could not record the operator resume request for this project.',
          ),
        )
        throw error
      } finally {
        setOperatorActionStatus('idle')
        setPendingOperatorActionId(null)
      }
    },
    [
      activeProjectIdRef,
      activeProjectRef,
      adapter,
      loadProject,
      setErrorMessage,
      setOperatorActionError,
      setOperatorActionStatus,
      setPendingOperatorActionId,
    ],
  )

  const startOpenAiLogin = useCallback(async () => {
    const projectId = getActiveProjectId(
      activeProjectIdRef,
      'Select an imported project before starting OpenAI login.',
    )

    try {
      const response = await adapter.startOpenAiLogin(projectId, { originator: 'agent-pane' })
      return applyRuntimeSessionUpdate(mapRuntimeSession(response))
    } catch (error) {
      try {
        await syncRuntimeSession(projectId)
      } catch {
        // Ignore follow-up refresh failures and preserve the last truthful state.
      }

      throw error
    }
  }, [activeProjectIdRef, adapter, applyRuntimeSessionUpdate, syncRuntimeSession])

  const submitOpenAiCallback = useCallback(
    async (flowId: string, options: { manualInput?: string | null } = {}) => {
      const projectId = getActiveProjectId(
        activeProjectIdRef,
        'Select an imported project before completing OpenAI login.',
      )

      try {
        const response = await adapter.submitOpenAiCallback(projectId, flowId, {
          manualInput: options.manualInput ?? null,
        })
        return applyRuntimeSessionUpdate(mapRuntimeSession(response))
      } catch (error) {
        try {
          await syncRuntimeSession(projectId)
        } catch {
          // Ignore follow-up refresh failures and preserve the last truthful state.
        }

        throw error
      }
    },
    [activeProjectIdRef, adapter, applyRuntimeSessionUpdate, syncRuntimeSession],
  )

  const startRuntimeSession = useCallback(async () => {
    const projectId = getActiveProjectId(
      activeProjectIdRef,
      'Select an imported project before binding a runtime session.',
    )

    try {
      const response = await adapter.startRuntimeSession(projectId)
      return applyRuntimeSessionUpdate(mapRuntimeSession(response))
    } catch (error) {
      try {
        await syncRuntimeSession(projectId)
      } catch {
        // Ignore follow-up refresh failures and preserve the last truthful state.
      }

      throw error
    }
  }, [activeProjectIdRef, adapter, applyRuntimeSessionUpdate, syncRuntimeSession])

  const logoutRuntimeSession = useCallback(async () => {
    const projectId = getActiveProjectId(activeProjectIdRef, 'Select an imported project before signing out.')

    try {
      const response = await adapter.logoutRuntimeSession(projectId)
      return applyRuntimeSessionUpdate(mapRuntimeSession(response))
    } catch (error) {
      try {
        await syncRuntimeSession(projectId)
      } catch {
        // Ignore follow-up refresh failures and preserve the last truthful state.
      }

      throw error
    }
  }, [activeProjectIdRef, adapter, applyRuntimeSessionUpdate, syncRuntimeSession])

  return {
    startOpenAiLogin,
    submitOpenAiCallback,
    startRuntimeSession,
    logoutRuntimeSession,
    resolveOperatorAction,
    resumeOperatorRun,
  }
}
