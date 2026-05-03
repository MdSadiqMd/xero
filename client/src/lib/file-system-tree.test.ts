import { describe, expect, it } from 'vitest'
import {
  applyProjectFileListing,
  createEmptyProjectFileTreeStore,
  getProjectFileTreeStoreStats,
  isFolderLoaded,
  materializeProjectFileTree,
  trimProjectFileTreeStoreToBudget,
  type ProjectFileTreeStore,
} from './file-system-tree'
import type { ListProjectFilesResponseDto } from './xero-model/project'

function listing(path: string, children: ListProjectFilesResponseDto['root']['children']): ListProjectFilesResponseDto {
  return {
    projectId: 'project-1',
    path,
    root: {
      name: path === '/' ? 'root' : path.split('/').pop() ?? 'folder',
      path,
      type: 'folder',
      children,
      childrenLoaded: true,
    },
    truncated: false,
    omittedEntryCount: 0,
  }
}

describe('project file tree store', () => {
  it('hydrates folder listings incrementally without inventing unloaded descendants', () => {
    let store: ProjectFileTreeStore = createEmptyProjectFileTreeStore()
    store = applyProjectFileListing(
      store,
      listing('/', [
        { name: 'src', path: '/src', type: 'folder', childrenLoaded: false },
        { name: 'README.md', path: '/README.md', type: 'file', childrenLoaded: true },
      ]),
    )

    let tree = materializeProjectFileTree(store)
    expect(tree.children?.map((node) => node.path)).toEqual(['/src', '/README.md'])
    expect(tree.children?.[0]?.children).toBeUndefined()
    expect(isFolderLoaded(store, '/src')).toBe(false)

    store = applyProjectFileListing(
      store,
      listing('/src', [{ name: 'main.ts', path: '/src/main.ts', type: 'file', childrenLoaded: true }]),
    )

    tree = materializeProjectFileTree(store)
    expect(tree.children?.[0]?.children?.map((node) => node.path)).toEqual(['/src/main.ts'])
    expect(isFolderLoaded(store, '/src')).toBe(true)
  })

  it('replaces stale children when a folder is reloaded', () => {
    let store = createEmptyProjectFileTreeStore()
    store = applyProjectFileListing(
      store,
      listing('/', [
        { name: 'old.ts', path: '/old.ts', type: 'file', childrenLoaded: true },
        { name: 'src', path: '/src', type: 'folder', childrenLoaded: false },
      ]),
    )
    store = applyProjectFileListing(
      store,
      listing('/', [{ name: 'src', path: '/src', type: 'folder', childrenLoaded: false }]),
    )

    expect(materializeProjectFileTree(store).children?.map((node) => node.path)).toEqual(['/src'])
  })

  it('reports approximate retained bytes for hydrated project tree stores', () => {
    const store = applyProjectFileListing(
      createEmptyProjectFileTreeStore(),
      listing('/', [
        { name: 'src', path: '/src', type: 'folder', childrenLoaded: false },
        { name: 'README.md', path: '/README.md', type: 'file', childrenLoaded: true },
      ]),
    )

    expect(getProjectFileTreeStoreStats(store)).toMatchObject({
      childListCount: 2,
      nodeCount: 3,
      unloadedFolderCount: 1,
    })
    expect(getProjectFileTreeStoreStats(store).byteSize).toBeGreaterThan(0)
  })

  it('prunes unprotected hydrated folders when the project tree exceeds its byte budget', () => {
    let store = createEmptyProjectFileTreeStore()
    store = applyProjectFileListing(
      store,
      listing('/', [
        { name: 'src', path: '/src', type: 'folder', childrenLoaded: false },
        { name: 'vendor', path: '/vendor', type: 'folder', childrenLoaded: false },
      ]),
    )
    store = applyProjectFileListing(
      store,
      listing('/src', [{ name: 'main.ts', path: '/src/main.ts', type: 'file', childrenLoaded: true }]),
    )
    store = applyProjectFileListing(
      store,
      listing(
        '/vendor',
        Array.from({ length: 80 }, (_, index) => ({
          name: `package-${index}.js`,
          path: `/vendor/package-${index}.js`,
          type: 'file' as const,
          childrenLoaded: true,
        })),
      ),
    )

    const before = getProjectFileTreeStoreStats(store)
    const trimmed = trimProjectFileTreeStoreToBudget(store, {
      maxBytes: Math.max(1, before.byteSize - 1_000),
      protectedPaths: ['/src/main.ts'],
    })

    expect(trimmed.prunedFolderCount).toBeGreaterThan(0)
    expect(trimmed.stats.byteSize).toBeLessThan(before.byteSize)
    expect(isFolderLoaded(trimmed.store, '/src')).toBe(true)
    expect(isFolderLoaded(trimmed.store, '/vendor')).toBe(false)
    expect(materializeProjectFileTree(trimmed.store).children?.map((node) => node.path)).toEqual([
      '/src',
      '/vendor',
    ])
  })
})
