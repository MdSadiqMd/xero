"use client"

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import {
  Archive,
  Loader2,
  MessageSquare,
  MoreHorizontal,
  Pin,
  PinOff,
  Plus,
  Trash2,
} from 'lucide-react'

import { cn } from '@/lib/utils'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import type { AgentSessionView } from '@/src/lib/cadence-model'

interface AgentSessionsSidebarProps {
  projectId: string | null
  projectLabel: string | null
  sessions: readonly AgentSessionView[]
  selectedSessionId: string | null
  onSelectSession: (agentSessionId: string) => void
  onCreateSession: () => void
  onArchiveSession: (agentSessionId: string) => void
  onOpenArchivedSessions: () => void
  pendingSessionId?: string | null
  isCreating?: boolean
  collapsed?: boolean
}

const PINNED_SESSIONS_STORAGE_PREFIX = 'cadence:pinned-sessions:'

function readPinnedSessionIds(projectId: string | null): Set<string> {
  if (!projectId || typeof window === 'undefined') return new Set()
  try {
    const raw = window.localStorage.getItem(`${PINNED_SESSIONS_STORAGE_PREFIX}${projectId}`)
    if (!raw) return new Set()
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) return new Set()
    return new Set(parsed.filter((id): id is string => typeof id === 'string'))
  } catch {
    return new Set()
  }
}

function writePinnedSessionIds(projectId: string | null, ids: Set<string>) {
  if (!projectId || typeof window === 'undefined') return
  try {
    window.localStorage.setItem(
      `${PINNED_SESSIONS_STORAGE_PREFIX}${projectId}`,
      JSON.stringify([...ids]),
    )
  } catch {
    // ignore storage failures (private mode, quota, etc.)
  }
}

type SessionEntryState = 'entering' | 'visible' | 'exiting'

interface SessionEntry {
  session: AgentSessionView
  state: SessionEntryState
}

export function AgentSessionsSidebar({
  projectId,
  projectLabel,
  sessions,
  selectedSessionId,
  onSelectSession,
  onCreateSession,
  onArchiveSession,
  onOpenArchivedSessions,
  pendingSessionId,
  isCreating,
  collapsed = false,
}: AgentSessionsSidebarProps) {
  const activeSessions = useMemo(
    () => sessions.filter((session) => session.isActive),
    [sessions],
  )

  const [pinnedIds, setPinnedIds] = useState<Set<string>>(() => readPinnedSessionIds(projectId))

  useEffect(() => {
    setPinnedIds(readPinnedSessionIds(projectId))
  }, [projectId])

  const togglePinSession = useCallback(
    (agentSessionId: string) => {
      setPinnedIds((prev) => {
        const next = new Set(prev)
        if (next.has(agentSessionId)) {
          next.delete(agentSessionId)
        } else {
          next.add(agentSessionId)
        }
        writePinnedSessionIds(projectId, next)
        return next
      })
    },
    [projectId],
  )

  const isFirstSyncRef = useRef(true)
  const [entries, setEntries] = useState<SessionEntry[]>(() =>
    activeSessions.map((session) => ({ session, state: 'visible' as const })),
  )

  useEffect(() => {
    const isFirst = isFirstSyncRef.current
    isFirstSyncRef.current = false

    setEntries((prevEntries) => {
      const activeBySessionId = new Map(
        activeSessions.map((session) => [session.agentSessionId, session]),
      )
      const seenIds = new Set<string>()

      const next: SessionEntry[] = prevEntries.map((entry) => {
        const id = entry.session.agentSessionId
        seenIds.add(id)
        const fresh = activeBySessionId.get(id)
        if (fresh) {
          if (entry.state === 'exiting') {
            return { session: fresh, state: 'entering' }
          }
          return { session: fresh, state: entry.state }
        }
        return entry.state === 'exiting' ? entry : { ...entry, state: 'exiting' }
      })

      for (const session of activeSessions) {
        if (!seenIds.has(session.agentSessionId)) {
          next.push({ session, state: isFirst ? 'visible' : 'entering' })
        }
      }

      return next
    })
  }, [activeSessions])

  const handleEnterAnimationEnd = useCallback((agentSessionId: string) => {
    setEntries((prev) =>
      prev.map((entry) =>
        entry.session.agentSessionId === agentSessionId && entry.state === 'entering'
          ? { ...entry, state: 'visible' }
          : entry,
      ),
    )
  }, [])

  const handleExitAnimationEnd = useCallback((agentSessionId: string) => {
    setEntries((prev) =>
      prev.filter(
        (entry) =>
          !(entry.session.agentSessionId === agentSessionId && entry.state === 'exiting'),
      ),
    )
  }, [])

  const pinnedEntries = useMemo(
    () => entries.filter((entry) => pinnedIds.has(entry.session.agentSessionId)),
    [entries, pinnedIds],
  )
  const regularEntries = useMemo(
    () => entries.filter((entry) => !pinnedIds.has(entry.session.agentSessionId)),
    [entries, pinnedIds],
  )

  const renderEntry = (entry: SessionEntry, isPinned: boolean) => (
    <li
      key={entry.session.agentSessionId}
      className={cn(
        entry.state === 'entering' &&
          'animate-in fade-in-0 slide-in-from-right-4 duration-300 ease-out',
        entry.state === 'exiting' &&
          'animate-out fade-out-0 slide-out-to-left-4 fill-mode-forwards duration-300 ease-out pointer-events-none',
      )}
      onAnimationEnd={(event) => {
        if (event.target !== event.currentTarget) return
        if (entry.state === 'entering') {
          handleEnterAnimationEnd(entry.session.agentSessionId)
        } else if (entry.state === 'exiting') {
          handleExitAnimationEnd(entry.session.agentSessionId)
        }
      }}
    >
      <AgentSessionsSidebarItem
        session={entry.session}
        isActive={entry.session.agentSessionId === selectedSessionId}
        isPending={entry.session.agentSessionId === pendingSessionId}
        isPinned={isPinned}
        onSelectSession={onSelectSession}
        onArchiveSession={onArchiveSession}
        onTogglePin={togglePinSession}
        canArchive={activeSessions.length > 1 && entry.state !== 'exiting'}
      />
    </li>
  )

  return (
    <aside
      aria-hidden={collapsed}
      className={cn(
        'motion-layout-island flex shrink-0 flex-col overflow-hidden border-r border-border bg-sidebar transition-[width,border-color] motion-panel',
        collapsed ? 'w-0 border-r-transparent' : 'w-[260px]',
      )}
    >
      <div className="flex w-[260px] shrink-0 flex-col h-full">
        <div className="flex shrink-0 items-start justify-between gap-2 px-3 pt-2.5 pb-2">
          <div className="min-w-0">
            <span className="text-[10px] font-semibold uppercase tracking-[0.14em] text-muted-foreground">
              Sessions
            </span>
            {projectLabel ? (
              <p className="truncate text-[11px] text-foreground/85">{projectLabel}</p>
            ) : null}
          </div>
          <div className="flex shrink-0 items-center gap-0.5">
            <button
              aria-label="View archived sessions"
              className={cn(
                'flex h-6 w-6 items-center justify-center rounded-md text-muted-foreground transition-colors',
                'hover:bg-primary/10 hover:text-primary',
              )}
              onClick={onOpenArchivedSessions}
              type="button"
            >
              <Archive className="h-3.5 w-3.5" />
            </button>
            <button
              aria-label="New session"
              className={cn(
                'flex h-6 w-6 items-center justify-center rounded-md text-muted-foreground transition-colors',
                'hover:bg-primary/10 hover:text-primary disabled:cursor-not-allowed disabled:opacity-50',
              )}
              disabled={isCreating}
              onClick={onCreateSession}
              type="button"
            >
              {isCreating ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <Plus className="h-3.5 w-3.5" />
              )}
            </button>
          </div>
        </div>

        <div className="flex-1 overflow-y-auto scrollbar-thin">
          {entries.length === 0 ? (
            <div className="px-3 py-5 text-center text-[11px] leading-relaxed text-muted-foreground/80">
              No sessions yet. Start a new chat to begin.
            </div>
          ) : (
            <>
              {pinnedEntries.length > 0 ? (
                <div className="flex flex-col">
                  <SidebarSectionHeader label="Pinned" />
                  <ul className="flex flex-col px-1.5 pb-1.5">
                    {pinnedEntries.map((entry) => renderEntry(entry, true))}
                  </ul>
                </div>
              ) : null}
              {regularEntries.length > 0 ? (
                <div className="flex flex-col">
                  {pinnedEntries.length > 0 ? (
                    <SidebarSectionHeader label="Sessions" />
                  ) : null}
                  <ul
                    className={cn(
                      'flex flex-col px-1.5 pb-1.5',
                      pinnedEntries.length === 0 && 'pt-1.5',
                    )}
                  >
                    {regularEntries.map((entry) => renderEntry(entry, false))}
                  </ul>
                </div>
              ) : null}
            </>
          )}
        </div>
      </div>
    </aside>
  )
}

function SidebarSectionHeader({ label }: { label: string }) {
  return (
    <div className="px-3 pt-2 pb-1 text-[9px] font-semibold uppercase tracking-[0.14em] text-muted-foreground/70">
      {label}
    </div>
  )
}

interface AgentSessionsSidebarItemProps {
  session: AgentSessionView
  isActive: boolean
  isPending: boolean
  isPinned: boolean
  canArchive: boolean
  onSelectSession: (agentSessionId: string) => void
  onArchiveSession: (agentSessionId: string) => void
  onTogglePin: (agentSessionId: string) => void
}

function AgentSessionsSidebarItem({
  session,
  isActive,
  isPending,
  isPinned,
  canArchive,
  onSelectSession,
  onArchiveSession,
  onTogglePin,
}: AgentSessionsSidebarItemProps) {
  const formattedCreatedAt = formatRelativeDate(session.createdAt)

  return (
    <div className="group relative">
      <button
        className={cn(
          'flex w-full items-center gap-2 rounded-md px-2 py-2 text-left transition-colors',
          isActive ? 'bg-primary/[0.08]' : 'hover:bg-secondary/50',
        )}
        onClick={() => onSelectSession(session.agentSessionId)}
        type="button"
      >
        <div
          className={cn(
            'flex h-6 w-6 shrink-0 items-center justify-center rounded-md border transition-colors',
            isActive
              ? 'border-primary/45 bg-primary/15 text-primary'
              : 'border-border/70 bg-secondary/70 text-muted-foreground group-hover:border-border group-hover:bg-secondary group-hover:text-foreground',
          )}
        >
          <MessageSquare className="h-3 w-3" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-1 pr-5">
            <span
              className={cn(
                'truncate text-[12px] font-medium leading-tight',
                isActive ? 'text-foreground' : 'text-foreground/85 group-hover:text-foreground',
              )}
            >
              {session.title}
            </span>
            {isPinned ? (
              <Pin
                aria-hidden
                className="h-2.5 w-2.5 shrink-0 -rotate-45 text-muted-foreground/70"
              />
            ) : null}
          </div>
          {formattedCreatedAt ? (
            <div className="mt-0.5 truncate text-[10px] text-muted-foreground">
              {formattedCreatedAt}
            </div>
          ) : null}
        </div>
      </button>

      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <button
            aria-label={`Session actions for ${session.title}`}
            className={cn(
              'absolute right-1 top-1 z-10 flex h-5 w-5 items-center justify-center rounded-md text-muted-foreground transition-colors',
              'hover:bg-secondary hover:text-foreground disabled:opacity-50',
              isActive || isPending
                ? 'opacity-100'
                : 'opacity-0 group-hover:opacity-100 focus-visible:opacity-100',
            )}
            disabled={isPending}
            type="button"
          >
            {isPending ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <MoreHorizontal className="h-3.5 w-3.5" />
            )}
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem
            onSelect={(event) => {
              event.preventDefault()
              onTogglePin(session.agentSessionId)
            }}
          >
            {isPinned ? (
              <>
                <PinOff className="h-4 w-4" />
                Unpin
              </>
            ) : (
              <>
                <Pin className="h-4 w-4" />
                Pin
              </>
            )}
          </DropdownMenuItem>
          {canArchive ? (
            <>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onSelect={(event) => {
                  event.preventDefault()
                  onArchiveSession(session.agentSessionId)
                }}
                variant="destructive"
              >
                <Trash2 className="h-4 w-4" />
                Archive
              </DropdownMenuItem>
            </>
          ) : null}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  )
}

function formatRelativeDate(isoTimestamp: string): string | null {
  const parsed = Date.parse(isoTimestamp)
  if (!Number.isFinite(parsed)) {
    return null
  }

  const now = Date.now()
  const diffSeconds = Math.floor((now - parsed) / 1000)

  if (diffSeconds < 60) return 'Just now'
  if (diffSeconds < 3600) {
    const minutes = Math.floor(diffSeconds / 60)
    return `${minutes}m ago`
  }
  if (diffSeconds < 86400) {
    const hours = Math.floor(diffSeconds / 3600)
    return `${hours}h ago`
  }
  if (diffSeconds < 86400 * 7) {
    const days = Math.floor(diffSeconds / 86400)
    return `${days}d ago`
  }

  return new Date(parsed).toLocaleDateString()
}
