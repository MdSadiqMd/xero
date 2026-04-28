import { useCallback } from 'react'

import type { ProviderCredentialsSnapshotDto } from '@/src/lib/cadence-model/provider-credentials'
import type { RuntimeSessionView } from '@/src/lib/cadence-model/runtime'
import { mapRuntimeSession } from '@/src/lib/cadence-model/runtime'

import type {
  CadenceDesktopMutationActions,
  UseCadenceDesktopMutationsArgs,
} from './mutation-support'
import {
  getActiveProjectId,
  getOperatorActionError,
} from './mutation-support'

export function useProviderCredentialsMutations({
  adapter,
  refs,
  setters,
  providerCredentialsLoadStatus,
}: UseCadenceDesktopMutationsArgs): Pick<
  CadenceDesktopMutationActions,
  | 'refreshProviderCredentials'
  | 'upsertProviderCredential'
  | 'deleteProviderCredential'
  | 'startOAuthLogin'
  | 'completeOAuthCallback'
> {
  const {
    activeProjectIdRef,
    providerCredentialsRef,
    providerCredentialsLoadInFlightRef,
  } = refs
  const {
    setProviderCredentials,
    setProviderCredentialsLoadStatus,
    setProviderCredentialsLoadError,
    setProviderCredentialsSaveStatus,
    setProviderCredentialsSaveError,
  } = setters

  const applySnapshot = useCallback(
    (snapshot: ProviderCredentialsSnapshotDto) => {
      setProviderCredentials(snapshot)
      setProviderCredentialsLoadStatus('ready')
      setProviderCredentialsLoadError(null)
      return snapshot
    },
    [setProviderCredentials, setProviderCredentialsLoadError, setProviderCredentialsLoadStatus],
  )

  const refreshProviderCredentials = useCallback(
    async (options: { force?: boolean } = {}) => {
      if (providerCredentialsLoadInFlightRef.current) {
        return providerCredentialsLoadInFlightRef.current
      }

      const cached = providerCredentialsRef.current
      if (!options.force && cached && providerCredentialsLoadStatus === 'ready') {
        return cached
      }

      setProviderCredentialsLoadStatus('loading')
      setProviderCredentialsLoadError(null)

      const loadPromise = (async () => {
        try {
          const response = await adapter.listProviderCredentials()
          return applySnapshot(response)
        } catch (error) {
          setProviderCredentialsLoadStatus('error')
          setProviderCredentialsLoadError(
            getOperatorActionError(
              error,
              'Cadence could not load app-local provider credentials.',
            ),
          )
          throw error
        } finally {
          providerCredentialsLoadInFlightRef.current = null
        }
      })()

      providerCredentialsLoadInFlightRef.current = loadPromise
      return loadPromise
    },
    [
      adapter,
      applySnapshot,
      providerCredentialsLoadInFlightRef,
      providerCredentialsLoadStatus,
      providerCredentialsRef,
      setProviderCredentialsLoadError,
      setProviderCredentialsLoadStatus,
    ],
  )

  const upsertProviderCredential = useCallback<
    CadenceDesktopMutationActions['upsertProviderCredential']
  >(
    async (request) => {
      setProviderCredentialsSaveStatus('running')
      setProviderCredentialsSaveError(null)

      try {
        const response = await adapter.upsertProviderCredential(request)
        applySnapshot(response)
        return response
      } catch (error) {
        setProviderCredentialsSaveError(
          getOperatorActionError(
            error,
            'Cadence could not save the provider credential.',
          ),
        )

        try {
          await refreshProviderCredentials({ force: true })
        } catch {
          // Preserve last truthful snapshot if refresh-after-failure also fails.
        }

        throw error
      } finally {
        setProviderCredentialsSaveStatus('idle')
      }
    },
    [
      adapter,
      applySnapshot,
      refreshProviderCredentials,
      setProviderCredentialsSaveError,
      setProviderCredentialsSaveStatus,
    ],
  )

  const deleteProviderCredential = useCallback<
    CadenceDesktopMutationActions['deleteProviderCredential']
  >(
    async (providerId) => {
      setProviderCredentialsSaveStatus('running')
      setProviderCredentialsSaveError(null)

      try {
        const response = await adapter.deleteProviderCredential(providerId)
        applySnapshot(response)
        return response
      } catch (error) {
        setProviderCredentialsSaveError(
          getOperatorActionError(
            error,
            'Cadence could not remove the provider credential.',
          ),
        )

        try {
          await refreshProviderCredentials({ force: true })
        } catch {
          // Preserve last truthful snapshot if refresh-after-failure also fails.
        }

        throw error
      } finally {
        setProviderCredentialsSaveStatus('idle')
      }
    },
    [
      adapter,
      applySnapshot,
      refreshProviderCredentials,
      setProviderCredentialsSaveError,
      setProviderCredentialsSaveStatus,
    ],
  )

  const startOAuthLogin = useCallback<
    CadenceDesktopMutationActions['startOAuthLogin']
  >(
    async (request): Promise<RuntimeSessionView | null> => {
      const projectId = getActiveProjectId(
        activeProjectIdRef,
        'Cadence cannot start OAuth login without an active project.',
      )
      const session = await adapter.startOAuthLogin({
        providerId: request.providerId,
        projectId,
        originator: request.originator ?? null,
      })
      return mapRuntimeSession(session)
    },
    [activeProjectIdRef, adapter],
  )

  const completeOAuthCallback = useCallback<
    CadenceDesktopMutationActions['completeOAuthCallback']
  >(
    async (request): Promise<RuntimeSessionView | null> => {
      const projectId = getActiveProjectId(
        activeProjectIdRef,
        'Cadence cannot complete OAuth callback without an active project.',
      )
      const session = await adapter.completeOAuthCallback({
        providerId: request.providerId,
        projectId,
        flowId: request.flowId,
        manualInput: request.manualInput ?? null,
      })
      return mapRuntimeSession(session)
    },
    [activeProjectIdRef, adapter],
  )

  return {
    refreshProviderCredentials,
    upsertProviderCredential,
    deleteProviderCredential,
    startOAuthLogin,
    completeOAuthCallback,
  }
}
