"use client"

import { lazy, memo, Suspense, useCallback, useEffect, useRef, useState } from 'react'
import type { EditorView as CodeMirrorView } from '@codemirror/view'
import type {
  CreateProjectEntryRequestDto,
  CreateProjectEntryResponseDto,
  DeleteProjectEntryResponseDto,
  ListProjectFilesResponseDto,
  MoveProjectEntryRequestDto,
  MoveProjectEntryResponseDto,
  ReadProjectFileResponseDto,
  RenameProjectEntryRequestDto,
  RenameProjectEntryResponseDto,
  ReplaceInProjectRequestDto,
  ReplaceInProjectResponseDto,
  SearchProjectRequestDto,
  SearchProjectResponseDto,
  WriteProjectFileResponseDto,
} from '@/src/lib/xero-model'
import type { ExecutionPaneView } from '@/src/features/xero/use-xero-desktop-state'
import { DeleteFileDialog } from './delete-file-dialog'
import { RenameFileDialog } from './rename-file-dialog'
import { EditorEmptyState, LoadingState } from './execution-view/editor-empty-state'
import { ExplorerPane } from './execution-view/explorer-pane'
import { EditorStatusBar, EditorToolbar } from './execution-view/editor-status-bar'
import { EditorTabs } from './execution-view/editor-tabs'
import { useExecutionWorkspaceController } from './execution-view/use-execution-workspace-controller'

const LazyCodeEditor = lazy(() => import('./code-editor').then((module) => ({ default: module.CodeEditor })))
const LazyFindReplacePane = lazy(() =>
  import('./execution-view/find-replace-pane').then((module) => ({ default: module.FindReplacePane })),
)

export interface ExecutionViewProps {
  execution: ExecutionPaneView
  active?: boolean
  listProjectFiles: (projectId: string) => Promise<ListProjectFilesResponseDto>
  readProjectFile: (projectId: string, path: string) => Promise<ReadProjectFileResponseDto>
  writeProjectFile: (projectId: string, path: string, content: string) => Promise<WriteProjectFileResponseDto>
  createProjectEntry: (request: CreateProjectEntryRequestDto) => Promise<CreateProjectEntryResponseDto>
  renameProjectEntry: (request: RenameProjectEntryRequestDto) => Promise<RenameProjectEntryResponseDto>
  moveProjectEntry: (request: MoveProjectEntryRequestDto) => Promise<MoveProjectEntryResponseDto>
  deleteProjectEntry: (projectId: string, path: string) => Promise<DeleteProjectEntryResponseDto>
  searchProject: (request: SearchProjectRequestDto) => Promise<SearchProjectResponseDto>
  replaceInProject: (request: ReplaceInProjectRequestDto) => Promise<ReplaceInProjectResponseDto>
}

function EditorView({
  execution,
  active = true,
  listProjectFiles,
  readProjectFile,
  writeProjectFile,
  createProjectEntry,
  renameProjectEntry,
  moveProjectEntry,
  deleteProjectEntry,
  searchProject,
  replaceInProject,
}: ExecutionViewProps) {
  const projectId = execution.project.id
  const projectLabel = execution.project.repository?.displayName ?? execution.project.name
  const {
    tree,
    openTabs,
    activePath,
    setActivePath,
    expandedFolders,
    dirtyPaths,
    searchQuery,
    setSearchQuery,
    cursor,
    setCursor,
    isTreeLoading,
    pendingFilePath,
    workspaceError,
    renameTarget,
    setRenameTarget,
    deleteTarget,
    setDeleteTarget,
    newChildTarget,
    setNewChildTarget,
    activeNode,
    activeContent,
    activeSavedContent,
    activeDocumentVersion,
    activeLang,
    activeLineCount,
    isActiveDirty,
    isActiveSaving,
    isActiveLoading,
    closeTab,
    handleSelectFile,
    handleToggleFolder,
    handleSnapshotChange,
    handleDirtyChange,
    handleDocumentStatsChange,
    saveActive,
    revertActive,
    reloadProjectTree,
    handleRequestRename,
    handleRequestDelete,
    handleRequestNewFile,
    handleRequestNewFolder,
    handleMoveEntry,
    handleCopyPath,
    handleRenameSubmit,
    handleDeleteSubmit,
    handleCreateSubmit,
  } = useExecutionWorkspaceController({
    projectId,
    active,
    listProjectFiles,
    readProjectFile,
    writeProjectFile,
    createProjectEntry,
    renameProjectEntry,
    moveProjectEntry,
    deleteProjectEntry,
  })

  const [editorView, setEditorView] = useState<CodeMirrorView | null>(null)
  const [findState, setFindState] = useState<{
    open: boolean
    query: string
    token: number
  }>({ open: false, query: '', token: 0 })
  const pendingJumpRef = useRef<{ path: string; line: number; column: number } | null>(null)

  const flushEditorSnapshot = useCallback(() => {
    if (!editorView) {
      return undefined
    }

    const snapshot = editorView.state.doc.toString()
    handleSnapshotChange(snapshot)
    return snapshot
  }, [editorView, handleSnapshotChange])

  const handleSaveActive = useCallback(
    (snapshot?: string) => {
      void saveActive(snapshot ?? flushEditorSnapshot())
    },
    [flushEditorSnapshot, saveActive],
  )

  const handleSelectTab = useCallback(
    (path: string) => {
      flushEditorSnapshot()
      setActivePath(path)
    },
    [flushEditorSnapshot, setActivePath],
  )

  const handleCloseTab = useCallback(
    (path: string) => {
      flushEditorSnapshot()
      closeTab(path)
    },
    [closeTab, flushEditorSnapshot],
  )

  const handleSelectFileWithSnapshot = useCallback(
    (path: string) => {
      flushEditorSnapshot()
      void handleSelectFile(path)
    },
    [flushEditorSnapshot, handleSelectFile],
  )

  const handleMoveEntryWithSnapshot = useCallback(
    (path: string, targetParentPath: string) => {
      flushEditorSnapshot()
      return handleMoveEntry(path, targetParentPath)
    },
    [flushEditorSnapshot, handleMoveEntry],
  )

  const handleCreateSubmitWithSnapshot = useCallback(
    (name: string) => {
      flushEditorSnapshot()
      return handleCreateSubmit(name)
    },
    [flushEditorSnapshot, handleCreateSubmit],
  )

  const handleRenameSubmitWithSnapshot = useCallback(
    (newName: string) => {
      flushEditorSnapshot()
      return handleRenameSubmit(newName)
    },
    [flushEditorSnapshot, handleRenameSubmit],
  )

  const handleDeleteSubmitWithSnapshot = useCallback(() => {
    flushEditorSnapshot()
    return handleDeleteSubmit()
  }, [flushEditorSnapshot, handleDeleteSubmit])

  const handleOpenFind = useCallback(
    ({ initialQuery }: { withReplace: boolean; initialQuery: string }) => {
      flushEditorSnapshot()
      setFindState((prev) => ({
        open: true,
        query: initialQuery || prev.query,
        token: prev.token + 1,
      }))
    },
    [flushEditorSnapshot],
  )

  const handleCloseFind = useCallback(() => {
    setFindState((prev) => ({ ...prev, open: false }))
    editorView?.focus()
  }, [editorView])

  const jumpEditorToCursor = useCallback(
    (line: number, column: number) => {
      const view = editorView
      if (!view) return
      const doc = view.state.doc
      if (line < 1 || line > doc.lines) return
      const target = doc.line(line)
      const pos = Math.min(target.to, target.from + Math.max(0, column - 1))
      view.dispatch({
        selection: { anchor: pos },
        scrollIntoView: true,
      })
      view.focus()
    },
    [editorView],
  )

  const handleOpenAtLine = useCallback(
    (path: string, line: number, column: number) => {
      flushEditorSnapshot()
      if (activePath === path) {
        jumpEditorToCursor(line, column)
        return
      }
      pendingJumpRef.current = { path, line, column }
      void handleSelectFile(path)
    },
    [activePath, flushEditorSnapshot, handleSelectFile, jumpEditorToCursor],
  )

  // After a jump-triggered file open finishes, position the cursor once the
  // editor state matches the requested file.
  useEffect(() => {
    const pending = pendingJumpRef.current
    if (!pending) return
    if (pending.path !== activePath) return
    if (isActiveLoading) return
    if (!editorView) return
    pendingJumpRef.current = null
    jumpEditorToCursor(pending.line, pending.column)
  }, [activePath, isActiveLoading, editorView, jumpEditorToCursor])

  return (
    <div className="flex min-h-0 w-full min-w-0 flex-1">
      {findState.open ? (
        <Suspense
          fallback={
            <aside
              aria-label="Loading find and replace"
              className="w-[320px] shrink-0 border-r border-border bg-background"
            />
          }
        >
          <LazyFindReplacePane
            view={editorView}
            projectId={projectId}
            activePath={activePath}
            activeContent={activeContent}
            onClose={handleCloseFind}
            onOpenAtLine={handleOpenAtLine}
            searchProject={searchProject}
            replaceInProject={replaceInProject}
            initialQuery={findState.query}
            openToken={findState.token}
          />
        </Suspense>
      ) : (
        <ExplorerPane
          searchQuery={searchQuery}
          isTreeLoading={isTreeLoading}
          workspaceError={workspaceError}
          tree={tree}
          activePath={activePath}
          expandedFolders={expandedFolders}
          dirtyPaths={dirtyPaths}
          creatingEntry={newChildTarget}
          onSearchQueryChange={setSearchQuery}
          onSelectFile={handleSelectFileWithSnapshot}
          onToggleFolder={handleToggleFolder}
          onRequestRename={handleRequestRename}
          onRequestDelete={handleRequestDelete}
          onRequestNewFile={handleRequestNewFile}
          onRequestNewFolder={handleRequestNewFolder}
          onMoveEntry={handleMoveEntryWithSnapshot}
          onCancelCreate={() => setNewChildTarget(null)}
          onCreateEntry={handleCreateSubmitWithSnapshot}
          onCopyPath={handleCopyPath}
          onOpenFind={() => handleOpenFind({ withReplace: true, initialQuery: '' })}
          onReload={reloadProjectTree}
        />
      )}

      <section className="flex min-h-0 min-w-0 flex-1 flex-col">
        <EditorTabs
          openTabs={openTabs}
          activePath={activePath}
          dirtyPaths={dirtyPaths}
          pendingFilePath={pendingFilePath}
          onSelectTab={handleSelectTab}
          onCloseTab={handleCloseTab}
        />

        {activePath ? (
          <EditorToolbar
            activePath={activePath}
            isDirty={isActiveDirty}
            isSaving={isActiveSaving}
            onRevert={revertActive}
            onSave={() => {
              handleSaveActive()
            }}
          />
        ) : null}

        <div className="flex min-h-0 flex-1 flex-col bg-background">
          {activePath && activeNode?.type === 'file' ? (
            isActiveLoading ? (
              <LoadingState path={activePath} />
            ) : (
              <>
                <div className="flex-1 overflow-hidden">
                  <Suspense fallback={<LoadingState path={activePath} />}>
                    <LazyCodeEditor
                      key={activePath}
                      value={activeContent}
                      savedValue={activeSavedContent}
                      documentVersion={activeDocumentVersion}
                      filePath={activePath}
                      onSnapshotChange={handleSnapshotChange}
                      onDirtyChange={handleDirtyChange}
                      onDocumentStatsChange={handleDocumentStatsChange}
                      onSave={handleSaveActive}
                      onCursorChange={setCursor}
                      onOpenFind={handleOpenFind}
                      onViewReady={setEditorView}
                    />
                  </Suspense>
                </div>
                <EditorStatusBar
                  cursor={cursor}
                  lang={activeLang}
                  lineCount={activeLineCount}
                  isDirty={isActiveDirty}
                  isSaving={isActiveSaving}
                />
              </>
            )
          ) : (
            <EditorEmptyState loadingPath={pendingFilePath} projectLabel={projectLabel} />
          )}
        </div>
      </section>

      <RenameFileDialog
        open={!!renameTarget}
        onOpenChange={(open) => {
          if (!open) setRenameTarget(null)
        }}
        currentPath={renameTarget?.path ?? ''}
        type={renameTarget?.type ?? 'file'}
        onRename={(newName) => handleRenameSubmitWithSnapshot(newName)}
      />
      <DeleteFileDialog
        open={!!deleteTarget}
        onOpenChange={(open) => {
          if (!open) setDeleteTarget(null)
        }}
        path={deleteTarget?.path ?? ''}
        type={deleteTarget?.type ?? 'file'}
        onDelete={() => {
          void handleDeleteSubmitWithSnapshot()
        }}
      />
    </div>
  )
}

export const ExecutionView = memo(function ExecutionView(props: ExecutionViewProps) {
  return (
    <div className="flex min-h-0 min-w-0 flex-1">
      <EditorView {...props} />
    </div>
  )
})
