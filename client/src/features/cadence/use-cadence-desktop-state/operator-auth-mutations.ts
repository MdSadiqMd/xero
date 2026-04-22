import { useCallback } from 'react'

import {
  CadenceDesktopError,
} from '@/src/lib/cadence-desktop'
import { mapRuntimeSession } from '@/src/lib/cadence-model/runtime'

import type {
  CadenceDesktopMutationActions,
  UseCadenceDesktopMutationsArgs,
} from './mutation-support'
import {
  getActiveProjectId,
  getOperatorActionError,
} from './mutation-support'

function getSelectedProfileId(selectedProfileId: string | null | undefined, action: string): string {
  const profileId = selectedProfileId?.trim() ?? ''
  if (profileId.length > 0) {
    return profileId
  }

  throw new CadenceDesktopError({
    code: 'provider_profiles_missing',
    errorClass: 'retryable',
    message: `Cadence could not ${action} because the selected provider profile is unavailable. Refresh Settings and retry.`,
    retryable: true,
  })
}

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
  const { activeProjectIdRef, activeProjectRef, providerProfilesRef } = refs
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
    const selectedProfileId = getSelectedProfileId(
      providerProfilesRef.current?.activeProfileId,
      'start OpenAI login',
    )

    try {
      const response = await adapter.startOpenAiLogin(projectId, {
        selectedProfileId,
        originator: 'agent-pane',
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
  }, [activeProjectIdRef, adapter, applyRuntimeSessionUpdate, providerProfilesRef, syncRuntimeSession])

  const submitOpenAiCallback = useCallback(
    async (flowId: string, options: { manualInput?: string | null } = {}) => {
      const projectId = getActiveProjectId(
        activeProjectIdRef,
        'Select an imported project before completing OpenAI login.',
      )
      const selectedProfileId = getSelectedProfileId(
        providerProfilesRef.current?.activeProfileId,
        'complete OpenAI login',
      )

      try {
        const response = await adapter.submitOpenAiCallback(projectId, flowId, {
          selectedProfileId,
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
    [activeProjectIdRef, adapter, applyRuntimeSessionUpdate, providerProfilesRef, syncRuntimeSession],
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
