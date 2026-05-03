import { render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import type { FileSystemNode } from '@/src/lib/file-system-tree'
import { FileTree, flattenFileTreeRows, type MatchInfo } from './file-tree'

function makeTree(children: FileSystemNode[]): FileSystemNode {
  return {
    id: '/',
    name: 'root',
    path: '/',
    type: 'folder',
    children,
  }
}

function makeFile(path: string): FileSystemNode {
  return {
    id: path,
    name: path.split('/').pop() ?? path,
    path,
    type: 'file',
  }
}

function makeFolder(path: string, children: FileSystemNode[]): FileSystemNode {
  return {
    id: path,
    name: path.split('/').pop() ?? path,
    path,
    type: 'folder',
    children,
  }
}

function rowKey(row: ReturnType<typeof flattenFileTreeRows>[number]): string {
  if (row.kind === 'node') return row.node.path
  if (row.kind === 'create') return `create:${row.parentPath}`
  return `continuation:${row.path}`
}

function renderFileTree(root: FileSystemNode, selectedPath: string | null = null) {
  return render(
    <FileTree
      root={root}
      selectedPath={selectedPath}
      expandedFolders={new Set(['/src'])}
      dirtyPaths={new Set()}
      onSelectFile={vi.fn()}
      onToggleFolder={vi.fn()}
      onRequestRename={vi.fn()}
      onRequestDelete={vi.fn()}
      onRequestNewFile={vi.fn()}
      onRequestNewFolder={vi.fn()}
      onMoveEntry={vi.fn()}
      onCancelCreate={vi.fn()}
      onCreateEntry={vi.fn(async () => null)}
      onCopyPath={vi.fn()}
    />,
  )
}

describe('FileTree virtualization', () => {
  it('flattens expanded folders and inline create rows into a linear render model', () => {
    const root = makeTree([
      makeFolder('/src', [
        makeFile('/src/app.tsx'),
        makeFolder('/src/components', [makeFile('/src/components/button.tsx')]),
      ]),
      makeFile('/README.md'),
    ])

    const rows = flattenFileTreeRows({
      root,
      expandedFolders: new Set(['/src']),
      search: null,
      creatingEntry: { parentPath: '/src', type: 'file' },
    })

    expect(rows.map(rowKey)).toEqual([
      '/src',
      'create:/src',
      '/src/app.tsx',
      '/src/components',
      '/README.md',
    ])
  })

  it('expands search ancestors without mounting unrelated branches', () => {
    const root = makeTree([
      makeFolder('/src', [makeFile('/src/app.tsx')]),
      makeFolder('/docs', [makeFile('/docs/guide.md')]),
    ])
    const search: MatchInfo = {
      matchedPaths: new Set(['/docs/guide.md']),
      ancestorPaths: new Set(['/docs']),
    }

    const rows = flattenFileTreeRows({
      root,
      expandedFolders: new Set(),
      search,
      creatingEntry: null,
    })

    expect(rows.map(rowKey)).toEqual([
      '/docs',
      '/docs/guide.md',
    ])
  })

  it('surfaces folder continuation rows when a listing is capped', () => {
    const root = {
      ...makeTree([makeFile('/a.ts')]),
      truncated: true,
      omittedEntryCount: 42,
    }

    const rows = flattenFileTreeRows({
      root,
      expandedFolders: new Set(['/']),
      search: null,
      creatingEntry: null,
    })

    expect(rows.map(rowKey)).toEqual(['/a.ts', 'continuation:/'])
    expect(rows[rows.length - 1]).toMatchObject({ kind: 'continuation', omittedEntryCount: 42 })
  })

  it('windows large explorer trees instead of mounting every file row', () => {
    const root = makeTree(
      Array.from({ length: 1_000 }, (_, index) => makeFile(`/file-${String(index).padStart(4, '0')}.ts`)),
    )

    renderFileTree(root)

    expect(screen.getByRole('tree')).toBeInTheDocument()
    expect(screen.getByText('file-0000.ts')).toBeInTheDocument()
    expect(screen.queryByText('file-0999.ts')).not.toBeInTheDocument()
  })

  it('keeps the selected row mounted after virtualizing a large tree', async () => {
    const root = makeTree(
      Array.from({ length: 1_000 }, (_, index) => makeFile(`/file-${String(index).padStart(4, '0')}.ts`)),
    )

    renderFileTree(root, '/file-0950.ts')

    await waitFor(() => expect(screen.getByText('file-0950.ts')).toBeInTheDocument())
    expect(screen.getByText('file-0950.ts').closest('[role="treeitem"]')).toHaveAttribute('aria-selected', 'true')
  })
})
