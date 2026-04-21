import { useCallback, type Dispatch, type MutableRefObject, type SetStateAction } from 'react'
import {
  CadenceDesktopError,
  type CadenceDesktopAdapter,
  getDesktopErrorMessage,
} from '@/src/lib/cadence-desktop'
import {
  mapAutonomousRunInspection,
  mapProjectSummary,
  mapRuntimeRun,
  mapRuntimeSession,
  upsertProjectListItem,
  type NotificationRouteDto,
  type ProjectDetailView,
  type ProjectListItem,
  type RuntimeRunView,
  type RuntimeSessionView,
  type RuntimeSettingsDto,
} from '@/src/lib/cadence-model'

import type { ProjectLoadSource } from './project-loaders'
import type {
  AutonomousRunActionKind,
  AutonomousRunActionStatus,
  NotificationRouteMutationStatus,
  NotificationRoutesLoadResult,
  NotificationRoutesLoadStatus,
  OperatorActionErrorView,
  OperatorActionStatus,
  ProjectRemovalStatus,
  RefreshSource,
  RuntimeRunActionKind,
  RuntimeRunActionStatus,
  RuntimeSettingsLoadStatus,
  RuntimeSettingsSaveStatus,
  UseCadenceDesktopStateResult,
} from './types'

type SetState<T> = Dispatch<SetStateAction<T>>
type NotificationRouteRecords = Record<string, NotificationRouteDto[]>
type NotificationRouteStatusRecords = Record<string, NotificationRoutesLoadStatus>
type NotificationRouteErrorRecords = Record<string, OperatorActionErrorView | null>
type AutonomousInspection = ReturnType<typeof mapAutonomousRunInspection>

export type CadenceDesktopMutationActions = Pick<
  UseCadenceDesktopStateResult,
  | 'importProject'
  | 'removeProject'
  | 'listProjectFiles'
  | 'readProjectFile'
  | 'writeProjectFile'
  | 'createProjectEntry'
  | 'renameProjectEntry'
  | 'deleteProjectEntry'
  | 'startOpenAiLogin'
  | 'submitOpenAiCallback'
  | 'startAutonomousRun'
  | 'inspectAutonomousRun'
  | 'cancelAutonomousRun'
  | 'startRuntimeRun'
  | 'startRuntimeSession'
  | 'stopRuntimeRun'
  | 'logoutRuntimeSession'
  | 'resolveOperatorAction'
  | 'resumeOperatorRun'
  | 'refreshRuntimeSettings'
  | 'upsertRuntimeSettings'
  | 'refreshNotificationRoutes'
  | 'upsertNotificationRoute'
>

interface UseCadenceDesktopMutationsRefs {
  activeProjectIdRef: MutableRefObject<string | null>
  activeProjectRef: MutableRefObject<ProjectDetailView | null>
  runtimeSettingsRef: MutableRefObject<RuntimeSettingsDto | null>
  runtimeSettingsLoadInFlightRef: MutableRefObject<Promise<RuntimeSettingsDto> | null>
}

interface UseCadenceDesktopMutationsSetters {
  setProjects: SetState<ProjectListItem[]>
  setIsImporting: SetState<boolean>
  setProjectRemovalStatus: SetState<ProjectRemovalStatus>
  setPendingProjectRemovalId: SetState<string | null>
  setRefreshSource: SetState<RefreshSource>
  setErrorMessage: SetState<string | null>
  setOperatorActionStatus: SetState<OperatorActionStatus>
  setPendingOperatorActionId: SetState<string | null>
  setOperatorActionError: SetState<OperatorActionErrorView | null>
  setAutonomousRunActionStatus: SetState<AutonomousRunActionStatus>
  setPendingAutonomousRunAction: SetState<AutonomousRunActionKind | null>
  setAutonomousRunActionError: SetState<OperatorActionErrorView | null>
  setRuntimeRunActionStatus: SetState<RuntimeRunActionStatus>
  setPendingRuntimeRunAction: SetState<RuntimeRunActionKind | null>
  setRuntimeRunActionError: SetState<OperatorActionErrorView | null>
  setNotificationRoutes: SetState<NotificationRouteRecords>
  setNotificationRouteLoadStatuses: SetState<NotificationRouteStatusRecords>
  setNotificationRouteLoadErrors: SetState<NotificationRouteErrorRecords>
  setNotificationRouteMutationStatus: SetState<NotificationRouteMutationStatus>
  setPendingNotificationRouteId: SetState<string | null>
  setNotificationRouteMutationError: SetState<OperatorActionErrorView | null>
  setRuntimeSettings: SetState<RuntimeSettingsDto | null>
  setRuntimeSettingsLoadStatus: SetState<RuntimeSettingsLoadStatus>
  setRuntimeSettingsLoadError: SetState<OperatorActionErrorView | null>
  setRuntimeSettingsSaveStatus: SetState<RuntimeSettingsSaveStatus>
  setRuntimeSettingsSaveError: SetState<OperatorActionErrorView | null>
}

interface UseCadenceDesktopMutationsOperations {
  bootstrap: (source?: 'startup' | 'remove') => Promise<void>
  loadProject: (projectId: string, source: ProjectLoadSource) => Promise<ProjectDetailView | null>
  loadNotificationRoutes: (
    projectId: string,
    options?: { force?: boolean },
  ) => Promise<NotificationRoutesLoadResult>
  syncRuntimeSession: (projectId: string) => Promise<RuntimeSessionView>
  syncRuntimeRun: (projectId: string) => Promise<RuntimeRunView | null>
  syncAutonomousRun: (projectId: string) => Promise<ProjectDetailView['autonomousRun'] | null>
  applyRuntimeSessionUpdate: (
    runtimeSession: RuntimeSessionView,
    options?: { clearGlobalError?: boolean },
  ) => RuntimeSessionView
  applyRuntimeRunUpdate: (
    projectId: string,
    runtimeRun: RuntimeRunView | null,
    options?: { clearGlobalError?: boolean; loadError?: string | null },
  ) => RuntimeRunView | null
  applyAutonomousRunStateUpdate: (
    projectId: string,
    inspection: AutonomousInspection,
    options?: { clearGlobalError?: boolean; loadError?: string | null },
  ) => ProjectDetailView['autonomousRun']
}

interface UseCadenceDesktopMutationsArgs {
  adapter: CadenceDesktopAdapter
  refs: UseCadenceDesktopMutationsRefs
  setters: UseCadenceDesktopMutationsSetters
  operations: UseCadenceDesktopMutationsOperations
  runtimeSettingsLoadStatus: RuntimeSettingsLoadStatus
}

function getActiveProjectId(
  activeProjectIdRef: MutableRefObject<string | null>,
  errorMessage: string,
): string {
  const projectId = activeProjectIdRef.current
  if (!projectId) {
    throw new Error(errorMessage)
  }

  return projectId
}

function getOperatorActionError(error: unknown, fallback: string): OperatorActionErrorView {
  if (error instanceof CadenceDesktopError) {
    return {
      code: error.code,
      message: error.message,
      retryable: error.retryable,
    }
  }

  if (error instanceof Error && error.message.trim().length > 0) {
    return {
      code: 'operator_action_failed',
      message: error.message,
      retryable: false,
    }
  }

  return {
    code: 'operator_action_failed',
    message: fallback,
    retryable: false,
  }
}

export function useCadenceDesktopMutations({
  adapter,
  refs,
  setters,
  operations,
  runtimeSettingsLoadStatus,
}: UseCadenceDesktopMutationsArgs): CadenceDesktopMutationActions {
  const {
    activeProjectIdRef,
    activeProjectRef,
    runtimeSettingsRef,
    runtimeSettingsLoadInFlightRef,
  } = refs
  const {
    setProjects,
    setIsImporting,
    setProjectRemovalStatus,
    setPendingProjectRemovalId,
    setRefreshSource,
    setErrorMessage,
    setOperatorActionStatus,
    setPendingOperatorActionId,
    setOperatorActionError,
    setAutonomousRunActionStatus,
    setPendingAutonomousRunAction,
    setAutonomousRunActionError,
    setRuntimeRunActionStatus,
    setPendingRuntimeRunAction,
    setRuntimeRunActionError,
    setNotificationRoutes,
    setNotificationRouteLoadStatuses,
    setNotificationRouteLoadErrors,
    setNotificationRouteMutationStatus,
    setPendingNotificationRouteId,
    setNotificationRouteMutationError,
    setRuntimeSettings,
    setRuntimeSettingsLoadStatus,
    setRuntimeSettingsLoadError,
    setRuntimeSettingsSaveStatus,
    setRuntimeSettingsSaveError,
  } = setters
  const {
    bootstrap,
    loadProject,
    loadNotificationRoutes,
    syncRuntimeSession,
    syncRuntimeRun,
    syncAutonomousRun,
    applyRuntimeSessionUpdate,
    applyRuntimeRunUpdate,
    applyAutonomousRunStateUpdate,
  } = operations

  const importProject = useCallback(async () => {
    setIsImporting(true)
    setRefreshSource('import')
    setErrorMessage(null)

    try {
      const selectedPath = await adapter.pickRepositoryFolder()
      if (!selectedPath) {
        return
      }

      const response = await adapter.importRepository(selectedPath)
      const summary = mapProjectSummary(response.project)
      setProjects((currentProjects) => upsertProjectListItem(currentProjects, summary))
      await loadProject(summary.id, 'import')
    } catch (error) {
      setErrorMessage(getDesktopErrorMessage(error))
    } finally {
      setIsImporting(false)
    }
  }, [adapter, loadProject, setErrorMessage, setIsImporting, setProjects, setRefreshSource])

  const removeProject = useCallback(
    async (projectId: string) => {
      if (!projectId.trim()) {
        return
      }

      setProjectRemovalStatus('running')
      setPendingProjectRemovalId(projectId)
      setRefreshSource('remove')
      setErrorMessage(null)

      try {
        await adapter.removeProject(projectId)
        await bootstrap('remove')
      } catch (error) {
        setErrorMessage(getDesktopErrorMessage(error))
      } finally {
        setPendingProjectRemovalId(null)
        setProjectRemovalStatus('idle')
      }
    },
    [adapter, bootstrap, setErrorMessage, setPendingProjectRemovalId, setProjectRemovalStatus, setRefreshSource],
  )

  const listProjectFiles = useCallback(
    async (projectId: string) => {
      return await adapter.listProjectFiles(projectId)
    },
    [adapter],
  )

  const readProjectFile = useCallback(
    async (projectId: string, path: string) => {
      return await adapter.readProjectFile(projectId, path)
    },
    [adapter],
  )

  const writeProjectFile = useCallback(
    async (projectId: string, path: string, content: string) => {
      return await adapter.writeProjectFile(projectId, path, content)
    },
    [adapter],
  )

  const createProjectEntry = useCallback(
    async (request: Parameters<CadenceDesktopMutationActions['createProjectEntry']>[0]) => {
      return await adapter.createProjectEntry(request)
    },
    [adapter],
  )

  const renameProjectEntry = useCallback(
    async (request: Parameters<CadenceDesktopMutationActions['renameProjectEntry']>[0]) => {
      return await adapter.renameProjectEntry(request)
    },
    [adapter],
  )

  const deleteProjectEntry = useCallback(
    async (projectId: string, path: string) => {
      return await adapter.deleteProjectEntry(projectId, path)
    },
    [adapter],
  )

  const refreshRuntimeSettings = useCallback(
    async (options: { force?: boolean } = {}) => {
      if (runtimeSettingsLoadInFlightRef.current) {
        return runtimeSettingsLoadInFlightRef.current
      }

      const cachedRuntimeSettings = runtimeSettingsRef.current
      if (!options.force && cachedRuntimeSettings && runtimeSettingsLoadStatus === 'ready') {
        return cachedRuntimeSettings
      }

      setRuntimeSettingsLoadStatus('loading')
      setRuntimeSettingsLoadError(null)

      const loadPromise = (async () => {
        try {
          const response = await adapter.getRuntimeSettings()
          setRuntimeSettings(response)
          setRuntimeSettingsLoadStatus('ready')
          setRuntimeSettingsLoadError(null)
          return response
        } catch (error) {
          setRuntimeSettingsLoadStatus('error')
          setRuntimeSettingsLoadError(
            getOperatorActionError(error, 'Cadence could not load app-global runtime settings.'),
          )
          throw error
        } finally {
          runtimeSettingsLoadInFlightRef.current = null
        }
      })()

      runtimeSettingsLoadInFlightRef.current = loadPromise
      return loadPromise
    },
    [
      adapter,
      runtimeSettingsLoadInFlightRef,
      runtimeSettingsLoadStatus,
      runtimeSettingsRef,
      setRuntimeSettings,
      setRuntimeSettingsLoadError,
      setRuntimeSettingsLoadStatus,
    ],
  )

  const upsertRuntimeSettings = useCallback(
    async (request: Parameters<CadenceDesktopMutationActions['upsertRuntimeSettings']>[0]) => {
      setRuntimeSettingsSaveStatus('running')
      setRuntimeSettingsSaveError(null)

      try {
        const response = await adapter.upsertRuntimeSettings(request)
        setRuntimeSettings(response)
        setRuntimeSettingsLoadStatus('ready')
        setRuntimeSettingsLoadError(null)
        setRuntimeSettingsSaveError(null)
        return response
      } catch (error) {
        setRuntimeSettingsSaveError(
          getOperatorActionError(error, 'Cadence could not save app-global runtime settings.'),
        )
        throw error
      } finally {
        setRuntimeSettingsSaveStatus('idle')
      }
    },
    [
      adapter,
      setRuntimeSettings,
      setRuntimeSettingsLoadError,
      setRuntimeSettingsLoadStatus,
      setRuntimeSettingsSaveError,
      setRuntimeSettingsSaveStatus,
    ],
  )

  const refreshNotificationRoutes = useCallback(
    async (options: { force?: boolean } = {}) => {
      const projectId = getActiveProjectId(
        activeProjectIdRef,
        'Select an imported project before loading notification routes.',
      )

      const result = await loadNotificationRoutes(projectId, {
        force: options.force ?? false,
      })

      if (result.loadError) {
        setNotificationRouteLoadStatuses((currentStatuses) => ({
          ...currentStatuses,
          [projectId]: 'error',
        }))
        setNotificationRouteLoadErrors((currentErrors) => ({
          ...currentErrors,
          [projectId]: result.loadError,
        }))
      }

      return result.routes
    },
    [
      activeProjectIdRef,
      loadNotificationRoutes,
      setNotificationRouteLoadErrors,
      setNotificationRouteLoadStatuses,
    ],
  )

  const upsertNotificationRoute = useCallback(
    async (request: Parameters<CadenceDesktopMutationActions['upsertNotificationRoute']>[0]) => {
      const projectId = getActiveProjectId(
        activeProjectIdRef,
        'Select an imported project before saving a notification route.',
      )

      const trimmedRouteId = request.routeId.trim()
      setNotificationRouteMutationStatus('running')
      setPendingNotificationRouteId(trimmedRouteId.length > 0 ? trimmedRouteId : null)
      setNotificationRouteMutationError(null)

      try {
        const response = await adapter.upsertNotificationRoute({
          ...request,
          projectId,
        })

        setNotificationRoutes((currentRoutes) => {
          const existingRoutes = currentRoutes[projectId] ?? []
          const nextRoutes = [
            response.route,
            ...existingRoutes.filter((route) => route.routeId !== response.route.routeId),
          ]

          return {
            ...currentRoutes,
            [projectId]: nextRoutes,
          }
        })
        setNotificationRouteLoadStatuses((currentStatuses) => ({
          ...currentStatuses,
          [projectId]: 'ready',
        }))
        setNotificationRouteLoadErrors((currentErrors) => ({
          ...currentErrors,
          [projectId]: null,
        }))

        void loadNotificationRoutes(projectId, { force: true })
        return response.route
      } catch (error) {
        setNotificationRouteMutationError(
          getOperatorActionError(error, 'Cadence could not save the notification route for this project.'),
        )

        try {
          await loadNotificationRoutes(projectId, { force: true })
        } catch {
          // Preserve the last truthful route list when refresh-after-failure also fails.
        }

        throw error
      } finally {
        setNotificationRouteMutationStatus('idle')
        setPendingNotificationRouteId(null)
      }
    },
    [
      activeProjectIdRef,
      adapter,
      loadNotificationRoutes,
      setNotificationRouteLoadErrors,
      setNotificationRouteLoadStatuses,
      setNotificationRouteMutationError,
      setNotificationRouteMutationStatus,
      setNotificationRoutes,
      setPendingNotificationRouteId,
    ],
  )

  const resolveOperatorAction = useCallback(
    async (actionId: string, decision: Parameters<CadenceDesktopMutationActions['resolveOperatorAction']>[1], options: { userAnswer?: string | null } = {}) => {
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

  const startAutonomousRun = useCallback(async () => {
    const projectId = getActiveProjectId(
      activeProjectIdRef,
      'Select an imported project before starting an autonomous run.',
    )

    setAutonomousRunActionStatus('running')
    setPendingAutonomousRunAction('start')
    setAutonomousRunActionError(null)

    try {
      const response = await adapter.startAutonomousRun(projectId)
      return applyAutonomousRunStateUpdate(projectId, mapAutonomousRunInspection(response), {
        clearGlobalError: false,
        loadError: null,
      })
    } catch (error) {
      setAutonomousRunActionError(
        getOperatorActionError(
          error,
          'Cadence could not start or inspect the autonomous run for this project.',
        ),
      )

      try {
        await syncAutonomousRun(projectId)
      } catch {
        // Ignore follow-up refresh failures and preserve the last truthful state.
      }

      throw error
    } finally {
      setAutonomousRunActionStatus('idle')
      setPendingAutonomousRunAction(null)
    }
  }, [
    activeProjectIdRef,
    adapter,
    applyAutonomousRunStateUpdate,
    setAutonomousRunActionError,
    setAutonomousRunActionStatus,
    setPendingAutonomousRunAction,
    syncAutonomousRun,
  ])

  const inspectAutonomousRun = useCallback(async () => {
    const projectId = getActiveProjectId(
      activeProjectIdRef,
      'Select an imported project before inspecting autonomous run truth.',
    )

    setAutonomousRunActionStatus('running')
    setPendingAutonomousRunAction('inspect')
    setAutonomousRunActionError(null)

    try {
      return await syncAutonomousRun(projectId)
    } catch (error) {
      setAutonomousRunActionError(
        getOperatorActionError(
          error,
          'Cadence could not inspect the autonomous run truth for this project.',
        ),
      )
      throw error
    } finally {
      setAutonomousRunActionStatus('idle')
      setPendingAutonomousRunAction(null)
    }
  }, [
    activeProjectIdRef,
    setAutonomousRunActionError,
    setAutonomousRunActionStatus,
    setPendingAutonomousRunAction,
    syncAutonomousRun,
  ])

  const cancelAutonomousRun = useCallback(
    async (runId: string) => {
      const projectId = getActiveProjectId(
        activeProjectIdRef,
        'Select an imported project before cancelling the autonomous run.',
      )

      setAutonomousRunActionStatus('running')
      setPendingAutonomousRunAction('cancel')
      setAutonomousRunActionError(null)

      try {
        const response = await adapter.cancelAutonomousRun(projectId, runId)
        return applyAutonomousRunStateUpdate(projectId, mapAutonomousRunInspection(response), {
          clearGlobalError: false,
          loadError: null,
        })
      } catch (error) {
        setAutonomousRunActionError(
          getOperatorActionError(error, 'Cadence could not cancel the autonomous run for this project.'),
        )

        try {
          await syncAutonomousRun(projectId)
        } catch {
          // Ignore follow-up refresh failures and preserve the last truthful state.
        }

        throw error
      } finally {
        setAutonomousRunActionStatus('idle')
        setPendingAutonomousRunAction(null)
      }
    },
    [
      activeProjectIdRef,
      adapter,
      applyAutonomousRunStateUpdate,
      setAutonomousRunActionError,
      setAutonomousRunActionStatus,
      setPendingAutonomousRunAction,
      syncAutonomousRun,
    ],
  )

  const startRuntimeRun = useCallback(async () => {
    const projectId = getActiveProjectId(
      activeProjectIdRef,
      'Select an imported project before starting a supervised runtime run.',
    )

    setRuntimeRunActionStatus('running')
    setPendingRuntimeRunAction('start')
    setRuntimeRunActionError(null)

    try {
      const response = await adapter.startRuntimeRun(projectId)
      return applyRuntimeRunUpdate(projectId, mapRuntimeRun(response), {
        clearGlobalError: false,
        loadError: null,
      })
    } catch (error) {
      setRuntimeRunActionError(
        getOperatorActionError(
          error,
          'Cadence could not start or reconnect the supervised runtime run for this project.',
        ),
      )

      try {
        await syncRuntimeRun(projectId)
      } catch {
        // Ignore follow-up refresh failures and preserve the last truthful state.
      }

      throw error
    } finally {
      setRuntimeRunActionStatus('idle')
      setPendingRuntimeRunAction(null)
    }
  }, [
    activeProjectIdRef,
    adapter,
    applyRuntimeRunUpdate,
    setPendingRuntimeRunAction,
    setRuntimeRunActionError,
    setRuntimeRunActionStatus,
    syncRuntimeRun,
  ])

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

  const stopRuntimeRun = useCallback(
    async (runId: string) => {
      const projectId = getActiveProjectId(
        activeProjectIdRef,
        'Select an imported project before stopping the supervised runtime run.',
      )

      setRuntimeRunActionStatus('running')
      setPendingRuntimeRunAction('stop')
      setRuntimeRunActionError(null)

      try {
        const response = await adapter.stopRuntimeRun(projectId, runId)
        return applyRuntimeRunUpdate(projectId, response ? mapRuntimeRun(response) : null, {
          clearGlobalError: false,
          loadError: null,
        })
      } catch (error) {
        setRuntimeRunActionError(
          getOperatorActionError(error, 'Cadence could not stop the supervised runtime run for this project.'),
        )

        try {
          await syncRuntimeRun(projectId)
        } catch {
          // Ignore follow-up refresh failures and preserve the last truthful state.
        }

        throw error
      } finally {
        setRuntimeRunActionStatus('idle')
        setPendingRuntimeRunAction(null)
      }
    },
    [
      activeProjectIdRef,
      adapter,
      applyRuntimeRunUpdate,
      setPendingRuntimeRunAction,
      setRuntimeRunActionError,
      setRuntimeRunActionStatus,
      syncRuntimeRun,
    ],
  )

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
    importProject,
    removeProject,
    listProjectFiles,
    readProjectFile,
    writeProjectFile,
    createProjectEntry,
    renameProjectEntry,
    deleteProjectEntry,
    startOpenAiLogin,
    submitOpenAiCallback,
    startAutonomousRun,
    inspectAutonomousRun,
    cancelAutonomousRun,
    startRuntimeRun,
    startRuntimeSession,
    stopRuntimeRun,
    logoutRuntimeSession,
    resolveOperatorAction,
    resumeOperatorRun,
    refreshRuntimeSettings,
    upsertRuntimeSettings,
    refreshNotificationRoutes,
    upsertNotificationRoute,
  }
}
