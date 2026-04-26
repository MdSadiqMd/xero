import type { PlatformVariant } from "@/components/cadence/shell"
import { detectPlatform } from "@/components/cadence/shell"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { SectionHeader } from "./section-header"

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
    <div className="flex flex-col gap-7">
      <SectionHeader
        title="Development"
        description="Developer tooling and preview options. Not visible in production builds."
      />

      <section className="flex flex-col gap-3">
        <div className="flex items-baseline justify-between gap-3">
          <h4 className="text-[12.5px] font-semibold text-foreground">Toolbar platform</h4>
          <span className="text-[11px] text-muted-foreground">
            Detected <span className="font-mono text-foreground/80">{detected}</span>
          </span>
        </div>
        <p className="-mt-1 text-[12px] leading-[1.5] text-muted-foreground">
          Override the detected platform to preview different toolbar layouts.
        </p>

        <div className="flex gap-1 rounded-lg border border-border/70 bg-secondary/30 p-1">
          {PLATFORM_OPTIONS.map(({ value, label }) => {
            const active = current === value
            return (
              <button
                key={label}
                type="button"
                className={cn(
                  "flex-1 rounded-md py-1.5 text-[12.5px] font-medium transition-all motion-fast",
                  active
                    ? "bg-background text-foreground shadow-sm ring-1 ring-border/40"
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
          <p className="text-[11.5px] text-muted-foreground">
            <span className="text-muted-foreground/70">Behavior:</span> {currentHint}
          </p>
        ) : null}
      </section>

      <section className="flex items-center justify-between gap-4 border-t border-border/50 pt-5">
        <div className="min-w-0 flex-1">
          <h4 className="text-[12.5px] font-semibold text-foreground">Onboarding flow</h4>
          <p className="mt-0.5 text-[12px] leading-[1.5] text-muted-foreground">
            Reopen the first-run setup flow to test provider setup, project import, and notification routing.
          </p>
        </div>
        <Button
          size="sm"
          variant="outline"
          className="h-8 shrink-0 text-[12px]"
          disabled={!onStartOnboarding}
          onClick={onStartOnboarding}
        >
          Start onboarding
        </Button>
      </section>
    </div>
  )
}
