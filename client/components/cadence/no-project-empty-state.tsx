import { FolderPlus, Loader2, Lock } from "lucide-react"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

interface NoProjectEmptyStateProps {
  isDesktopRuntime: boolean
  isImporting: boolean
  onImport: () => void
}

export function NoProjectEmptyState({ isDesktopRuntime, isImporting, onImport }: NoProjectEmptyStateProps) {
  return (
    <div className="relative flex flex-1 items-center justify-center overflow-hidden bg-background">
      {/* Subtle single-glow background */}
      <div
        aria-hidden
        className="pointer-events-none absolute left-1/2 top-1/2 h-[420px] w-[680px] -translate-x-1/2 -translate-y-[55%] rounded-full opacity-[0.06] blur-[120px]"
        style={{
          background:
            "radial-gradient(closest-side, #d4a574 0%, rgba(212,165,116,0.4) 45%, transparent 75%)",
        }}
      />

      <div className="relative flex w-full max-w-sm flex-col items-center px-8 text-center">
        <BrandGlyph />

        <h2 className="mt-6 text-xl font-semibold tracking-tight text-foreground">
          {isDesktopRuntime ? "Add your first project" : "Open Cadence desktop to continue"}
        </h2>
        <p className="mt-2 text-[13px] leading-relaxed text-muted-foreground">
          {isDesktopRuntime
            ? "Import a local Git repository to start planning and running work."
            : "Project import is only available inside the Tauri desktop runtime."}
        </p>

        <div className="mt-6">
          {isDesktopRuntime ? (
            <Button
              onClick={onImport}
              disabled={isImporting}
              size="sm"
              className="h-9 gap-2 bg-primary px-4 text-[12px] font-medium hover:bg-primary/90"
            >
              {isImporting ? (
                <>
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  Importing…
                </>
              ) : (
                <>
                  <FolderPlus className="h-3.5 w-3.5" />
                  Import repository
                </>
              )}
            </Button>
          ) : (
            <span className="inline-flex items-center gap-1.5 rounded-full border border-border bg-card/40 px-2.5 py-1 text-[11px] text-muted-foreground">
              <Lock className="h-3 w-3" />
              Desktop runtime required
            </span>
          )}
        </div>
      </div>
    </div>
  )
}

function BrandGlyph() {
  // A larger, softer echo of the Cadence mark used in the titlebar.
  return (
    <div className="relative">
      <div className="absolute inset-0 -z-10 rounded-3xl bg-primary/10 blur-2xl" />
      <div className="flex h-14 w-14 items-center justify-center rounded-2xl border border-border bg-card/60">
        <svg className="text-primary" fill="none" height="26" viewBox="0 0 24 24" width="26">
          <path d="M4 4h6v6H4V4Z" fill="currentColor" />
          <path d="M14 4h6v6h-6V4Z" fill="currentColor" fillOpacity="0.3" />
          <path d="M4 14h6v6H4v-6Z" fill="currentColor" fillOpacity="0.3" />
          <path d="M14 14h6v6h-6v-6Z" fill="currentColor" />
        </svg>
      </div>
    </div>
  )
}
