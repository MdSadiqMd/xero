import { Monitor, Rocket } from "lucide-react"
import type { PlatformVariant } from "@/components/cadence/shell"
import { detectPlatform } from "@/components/cadence/shell"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"

const PLATFORM_OPTIONS: Array<{ value: PlatformVariant | null; label: string; hint: string }> = [
  { value: null, label: "Auto", hint: "Use detected OS" },
  { value: "macos", label: "macOS", hint: "Traffic lights · tabs right" },
  { value: "windows", label: "Windows", hint: "Tabs left · controls right" },
  { value: "linux", label: "Linux", hint: "Same as Windows, rounded" },
]

export interface DevelopmentSectionProps {
  platformOverride?: PlatformVariant | null
  onPlatformOverrideChange?: (value: PlatformVariant | null) => void
  onStartOnboarding?: () => void
}

export function DevelopmentSection({
  platformOverride,
  onPlatformOverrideChange,
  onStartOnboarding,
}: DevelopmentSectionProps) {
  const detected = detectPlatform()
  const current = platformOverride ?? null
  const currentHint = PLATFORM_OPTIONS.find((option) => option.value === current)?.hint

  return (
    <div className="flex flex-col gap-4">
      <div>
        <h3 className="text-[13px] font-semibold text-foreground">Development</h3>
        <p className="mt-1 text-[12px] text-muted-foreground">
          Developer tooling and preview options. Not visible in production builds.
        </p>
      </div>

      <div className="rounded-lg border border-border bg-card px-4 py-3">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border bg-secondary/60">
            <Monitor className="h-4 w-4 text-foreground/70" />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-[13px] font-medium text-foreground">Toolbar platform</p>
            <p className="text-[11px] text-muted-foreground">
              Override the detected platform to preview different toolbar layouts. Detected:{" "}
              <span className="font-mono text-foreground/70">{detected}</span>
            </p>
          </div>
        </div>

        <div className="mt-3 flex gap-1 rounded-lg border border-border bg-secondary/30 p-1">
          {PLATFORM_OPTIONS.map(({ value, label }) => {
            const active = current === value
            return (
              <button
                key={label}
                type="button"
                className={cn(
                  "flex-1 rounded-md py-1.5 text-[12px] font-medium transition-colors",
                  active
                    ? "bg-background text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground",
                )}
                onClick={() => onPlatformOverrideChange?.(value)}
              >
                {label}
              </button>
            )
          })}
        </div>

        {currentHint ? (
          <p className="mt-2 text-[11px] text-muted-foreground">{currentHint}</p>
        ) : null}
      </div>

      <div className="rounded-lg border border-border bg-card px-4 py-3">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-border bg-secondary/60">
            <Rocket className="h-4 w-4 text-foreground/70" />
          </div>
          <div className="min-w-0 flex-1">
            <p className="text-[13px] font-medium text-foreground">Onboarding flow</p>
            <p className="text-[11px] text-muted-foreground">
              Reopen the first-run setup flow to test provider setup, project import, and notification routing.
            </p>
          </div>
          <Button size="sm" className="h-8 text-[11px]" disabled={!onStartOnboarding} onClick={onStartOnboarding}>
            Start onboarding
          </Button>
        </div>
      </div>
    </div>
  )
}
