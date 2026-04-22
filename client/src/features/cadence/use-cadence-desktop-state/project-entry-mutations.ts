import { useCallback } from 'react'

import { getDesktopErrorMessage } from '@/src/lib/cadence-desktop'
import {
  mapProjectSummary,
  upsertProjectListItem,
} from '@/src/lib/cadence-model/project'

import type {
  CadenceDesktopMutationActions,
  UseCadenceDesktopMutationsArgs,
} from './mutation-support'

export function useProjectEntryMutations({
  adapter,
  setters,
  operations,
}: UseCadenceDesktopMutationsArgs): Pick<
  CadenceDesktopMutationActions,
  | 'importProject'
  | 'removeProject'
  | 'listProjectFiles'
  | 'readProjectFile'
  | 'writeProjectFile'
  | 'createProjectEntry'
  | 'renameProjectEntry'
  | 'deleteProjectEntry'
  | 'searchProject'
  | 'replaceInProject'
> {
  const {
    setProjects,
    setIsImporting,
    setProjectRemovalStatus,
    setPendingProjectRemovalId,
    setRefreshSource,
    setErrorMessage,
  } = setters
  const { bootstrap, loadProject } = operations

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

  const searchProject = useCallback(
    async (request: Parameters<CadenceDesktopMutationActions['searchProject']>[0]) => {
      return await adapter.searchProject(request)
    },
    [adapter],
  )

  const replaceInProject = useCallback(
    async (request: Parameters<CadenceDesktopMutationActions['replaceInProject']>[0]) => {
      return await adapter.replaceInProject(request)
    },
    [adapter],
  )

  return {
    importProject,
    removeProject,
    listProjectFiles,
    readProjectFile,
    writeProjectFile,
    createProjectEntry,
    renameProjectEntry,
    deleteProjectEntry,
    searchProject,
    replaceInProject,
  }
}
