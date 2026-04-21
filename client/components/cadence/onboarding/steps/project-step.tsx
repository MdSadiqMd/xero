import { useState } from "react"
import { Check, FolderGit2, FolderOpen, Loader2, X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import type { OnboardingProjectState } from "../types"
import { StepHeader } from "./providers-step"

interface ProjectStepProps {
  project: OnboardingProjectState | null
  onSetProject: (project: OnboardingProjectState | null) => void
}

export function ProjectStep({ project, onSetProject }: ProjectStepProps) {
  const [isPicking, setIsPicking] = useState(false)

  function mockPick() {
    setIsPicking(true)
    window.setTimeout(() => {
      onSetProject({ name: "cadence-app", path: "~/Documents/dev/cadence-app" })
      setIsPicking(false)
    }, 650)
  }

  return (
    <div>
      <StepHeader
        title="Add a project"
        description="Point Cadence at a local repository. Nothing leaves your machine."
      />

      {project ? (
        <div className="mt-8 flex items-center gap-3 rounded-lg border border-primary/40 bg-card/60 px-4 py-3">
          <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-primary/40 bg-primary/10 text-primary">
            <FolderGit2 className="h-4 w-4" />
          </span>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-1.5">
              <p className="truncate text-[13px] font-medium text-foreground">{project.name}</p>
              <Check className="h-3 w-3 text-primary" strokeWidth={3} />
            </div>
            <p className="mt-0.5 truncate font-mono text-[11px] text-muted-foreground">{project.path}</p>
          </div>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => onSetProject(null)}
            className="shrink-0 text-muted-foreground hover:text-foreground"
            aria-label="Remove project"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      ) : (
        <button
          type="button"
          onClick={mockPick}
          disabled={isPicking}
          className={cn(
            "mt-8 flex w-full items-center gap-3 rounded-lg border border-dashed border-border bg-card/30 px-4 py-5 text-left transition-colors",
            "hover:border-primary/40 hover:bg-card/50",
            isPicking && "border-primary/40 bg-card/50",
          )}
        >
          <span
            className={cn(
              "flex h-9 w-9 shrink-0 items-center justify-center rounded-md border transition-colors",
              isPicking
                ? "border-primary/40 bg-primary/10 text-primary"
                : "border-border bg-secondary/50 text-muted-foreground",
            )}
          >
            {isPicking ? <Loader2 className="h-4 w-4 animate-spin" /> : <FolderOpen className="h-4 w-4" />}
          </span>
          <div className="flex-1">
            <p className="text-[13px] font-medium text-foreground">
              {isPicking ? "Indexing repository…" : "Choose a folder"}
            </p>
            <p className="mt-0.5 text-[11px] text-muted-foreground">
              {isPicking ? "Reading branches and project structure." : "Select a local Git repository."}
            </p>
          </div>
        </button>
      )}
    </div>
  )
}
