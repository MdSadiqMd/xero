'use client'

import { useEffect, useState } from 'react'
import { FolderOpen, FolderPlus, Loader2, Sparkles } from 'lucide-react'

import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { cn } from '@/lib/utils'

type Mode = 'choose' | 'create'

export interface ProjectAddDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  isImporting: boolean
  onSelectExisting: () => Promise<boolean | void> | boolean | void
  onPickParentFolder: () => Promise<string | null>
  onCreate: (parentPath: string, name: string) => Promise<boolean>
}

export function ProjectAddDialog({
  open,
  onOpenChange,
  isImporting,
  onSelectExisting,
  onPickParentFolder,
  onCreate,
}: ProjectAddDialogProps) {
  const [mode, setMode] = useState<Mode>('choose')
  const [name, setName] = useState('')
  const [parentPath, setParentPath] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    if (open) {
      setMode('choose')
      setName('')
      setParentPath(null)
      setError(null)
      setBusy(false)
    }
  }, [open])

  const trimmedName = name.trim()
  const canCreate = trimmedName.length > 0 && parentPath !== null && !busy

  const handleSelectExisting = async () => {
    setBusy(true)
    setError(null)
    try {
      await onSelectExisting()
      onOpenChange(false)
    } finally {
      setBusy(false)
    }
  }

  const handlePickParent = async () => {
    setError(null)
    try {
      const picked = await onPickParentFolder()
      if (picked) {
        setParentPath(picked)
      }
    } catch (pickError) {
      setError(pickError instanceof Error ? pickError.message : 'Could not pick a folder.')
    }
  }

  const submitCreate = async () => {
    if (!parentPath) {
      setError('Pick a parent folder first.')
      return
    }
    if (!trimmedName) {
      setError('Project name cannot be empty.')
      return
    }
    if (trimmedName.includes('/') || trimmedName.includes('\\')) {
      setError('Project name cannot contain slashes.')
      return
    }

    setBusy(true)
    setError(null)
    try {
      const ok = await onCreate(parentPath, trimmedName)
      if (ok) {
        onOpenChange(false)
      } else {
        setError('Could not create the project. Check the project rail for details.')
      }
    } finally {
      setBusy(false)
    }
  }

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === 'Enter' && canCreate) {
      event.preventDefault()
      void submitCreate()
    }
  }

  const dialogBusy = busy || isImporting

  return (
    <Dialog open={open} onOpenChange={(next) => !dialogBusy && onOpenChange(next)}>
      <DialogContent className="sm:max-w-[480px]">
        <DialogHeader>
          <div className="flex items-center gap-2">
            <Sparkles className="h-5 w-5 text-primary" />
            <DialogTitle>Add a project</DialogTitle>
          </div>
          <DialogDescription>
            {mode === 'choose'
              ? 'Open an existing repository or scaffold a brand-new project.'
              : 'Choose where the new project folder should live and give it a name.'}
          </DialogDescription>
        </DialogHeader>

        {mode === 'choose' ? (
          <div className="grid grid-cols-1 gap-2 py-1 sm:grid-cols-2">
            <ChoiceCard
              icon={<FolderOpen className="h-5 w-5 text-primary" />}
              title="Open existing"
              description="Pick a folder that already contains a Git repository."
              disabled={dialogBusy}
              loading={dialogBusy && !mode.startsWith('create')}
              onClick={() => void handleSelectExisting()}
            />
            <ChoiceCard
              icon={<FolderPlus className="h-5 w-5 text-primary" />}
              title="Create new"
              description="Make a new folder and initialize it as a Git repository."
              disabled={dialogBusy}
              onClick={() => setMode('create')}
            />
          </div>
        ) : (
          <div className="space-y-3 py-1">
            <div className="space-y-1.5">
              <label className="text-[12px] font-medium text-muted-foreground" htmlFor="project-name">
                Project name
              </label>
              <Input
                autoFocus
                id="project-name"
                value={name}
                onChange={(event) => {
                  setName(event.target.value)
                  setError(null)
                }}
                onKeyDown={onKeyDown}
                placeholder="my-new-project"
                disabled={dialogBusy}
                className={cn(error && 'border-destructive focus-visible:ring-destructive/30')}
              />
            </div>
            <div className="space-y-1.5">
              <label className="text-[12px] font-medium text-muted-foreground">
                Parent folder
              </label>
              <div className="flex items-center gap-2">
                <div
                  className={cn(
                    'min-w-0 flex-1 truncate rounded-md border border-input bg-muted/30 px-2.5 py-1.5 font-mono text-[12px]',
                    parentPath ? 'text-foreground' : 'text-muted-foreground',
                  )}
                  title={parentPath ?? undefined}
                >
                  {parentPath ?? 'No folder selected'}
                </div>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => void handlePickParent()}
                  disabled={dialogBusy}
                >
                  <FolderOpen className="h-3.5 w-3.5" />
                  Pick
                </Button>
              </div>
              {parentPath ? (
                <p className="font-mono text-[11px] text-muted-foreground/80">
                  New project will be created at{' '}
                  <span className="text-foreground/80">
                    {trimmedName ? `${parentPath}/${trimmedName}` : `${parentPath}/…`}
                  </span>
                </p>
              ) : null}
            </div>
            {error ? <p className="text-[12px] text-destructive">{error}</p> : null}
          </div>
        )}

        <DialogFooter>
          {mode === 'choose' ? (
            <Button
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={dialogBusy}
            >
              Cancel
            </Button>
          ) : (
            <>
              <Button
                variant="ghost"
                onClick={() => {
                  setMode('choose')
                  setError(null)
                }}
                disabled={dialogBusy}
              >
                Back
              </Button>
              <Button
                onClick={() => void submitCreate()}
                disabled={!canCreate || dialogBusy}
              >
                {dialogBusy ? (
                  <>
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    Creating…
                  </>
                ) : (
                  'Create project'
                )}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

interface ChoiceCardProps {
  icon: React.ReactNode
  title: string
  description: string
  disabled?: boolean
  loading?: boolean
  onClick: () => void
}

function ChoiceCard({ icon, title, description, disabled, loading, onClick }: ChoiceCardProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={cn(
        'group flex flex-col items-start gap-2 rounded-md border border-border/70 bg-secondary/30 p-3 text-left transition-colors',
        'hover:border-primary/40 hover:bg-primary/[0.06]',
        'focus-visible:border-primary/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/30',
        'disabled:cursor-not-allowed disabled:opacity-60 disabled:hover:border-border/70 disabled:hover:bg-secondary/30',
      )}
    >
      <div className="flex h-8 w-8 items-center justify-center rounded-md border border-border/60 bg-background">
        {loading ? <Loader2 className="h-4 w-4 animate-spin text-primary" /> : icon}
      </div>
      <div className="space-y-0.5">
        <div className="text-[13px] font-semibold text-foreground">{title}</div>
        <div className="text-[11.5px] leading-snug text-muted-foreground">{description}</div>
      </div>
    </button>
  )
}
