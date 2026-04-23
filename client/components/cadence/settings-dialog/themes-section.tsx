import { Check, Moon, Sun } from "lucide-react"
import { useTheme } from "@/src/features/theme/theme-provider"
import type { ThemeDefinition } from "@/src/features/theme/theme-definitions"
import { cn } from "@/lib/utils"

export function ThemesSection() {
  const { themes, themeId, setThemeId } = useTheme()

  return (
    <div className="flex flex-col gap-5">
      <div>
        <h3 className="text-[14px] font-semibold text-foreground">Themes</h3>
        <p className="mt-1.5 text-[13px] text-muted-foreground">
          Pick a palette for the entire app. Editor syntax highlighting and diff
          rendering follow the selected theme.
        </p>
      </div>

      <div className="grid gap-3">
        {themes.map((theme) => (
          <ThemeCard
            key={theme.id}
            theme={theme}
            active={theme.id === themeId}
            onSelect={() => setThemeId(theme.id)}
          />
        ))}
      </div>
    </div>
  )
}

interface ThemeCardProps {
  theme: ThemeDefinition
  active: boolean
  onSelect: () => void
}

function ThemeCard({ theme, active, onSelect }: ThemeCardProps) {
  const Icon = theme.appearance === "light" ? Sun : Moon
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={active}
      className={cn(
        "group flex items-center gap-3.5 rounded-lg border px-3.5 py-3 text-left transition-colors",
        active
          ? "border-primary/60 bg-primary/[0.06]"
          : "border-border bg-card hover:border-border/80 hover:bg-secondary/30",
      )}
    >
      <ThemeSwatch theme={theme} />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <Icon
            className={cn(
              "h-3.5 w-3.5 shrink-0",
              active ? "text-primary" : "text-muted-foreground",
            )}
          />
          <p className="text-[13.5px] font-medium text-foreground">{theme.name}</p>
          <span
            className={cn(
              "rounded-sm px-1.5 py-px text-[10.5px] font-medium uppercase tracking-[0.08em]",
              theme.appearance === "light"
                ? "bg-amber-500/10 text-amber-600 dark:text-amber-300"
                : "bg-slate-500/10 text-muted-foreground",
            )}
          >
            {theme.appearance}
          </span>
        </div>
        <p className="mt-1 text-[12px] text-muted-foreground">{theme.description}</p>
      </div>
      <div
        className={cn(
          "flex h-6 w-6 shrink-0 items-center justify-center rounded-full border transition-colors",
          active
            ? "border-primary bg-primary text-primary-foreground"
            : "border-border bg-transparent text-transparent group-hover:border-border/80",
        )}
        aria-hidden
      >
        <Check className="h-3.5 w-3.5" />
      </div>
    </button>
  )
}

function ThemeSwatch({ theme }: { theme: ThemeDefinition }) {
  const c = theme.colors
  return (
    <div
      className="relative h-11 w-14 shrink-0 overflow-hidden rounded-md border border-border/70 shadow-sm"
      style={{ backgroundColor: c.background }}
      aria-hidden
    >
      <div
        className="absolute inset-y-0 left-0 w-3"
        style={{ backgroundColor: c.sidebar }}
      />
      <div
        className="absolute left-4 top-1.5 h-1 w-6 rounded-sm"
        style={{ backgroundColor: c.primary }}
      />
      <div
        className="absolute left-4 top-4 h-1 w-5 rounded-sm"
        style={{ backgroundColor: c.foreground, opacity: 0.7 }}
      />
      <div
        className="absolute left-4 top-6.5 h-1 w-4 rounded-sm"
        style={{ backgroundColor: c.mutedForeground }}
      />
      <div
        className="absolute left-4 bottom-1.5 h-1 w-3 rounded-sm"
        style={{ backgroundColor: c.accent }}
      />
    </div>
  )
}
