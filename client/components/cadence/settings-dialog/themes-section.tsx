import { Check, Moon, Sun } from "lucide-react"
import { useMemo } from "react"
import { useTheme } from "@/src/features/theme/theme-provider"
import type { ThemeDefinition } from "@/src/features/theme/theme-definitions"
import { cn } from "@/lib/utils"
import { SectionHeader } from "./section-header"

export function ThemesSection() {
  const { themes, themeId, setThemeId } = useTheme()

  const { dark, light } = useMemo(() => {
    const dark: ThemeDefinition[] = []
    const light: ThemeDefinition[] = []
    for (const theme of themes) {
      if (theme.appearance === "light") light.push(theme)
      else dark.push(theme)
    }
    return { dark, light }
  }, [themes])

  return (
    <div className="flex flex-col gap-7">
      <SectionHeader
        title="Themes"
        description="Pick a palette for the entire app. Editor syntax highlighting and diff rendering follow the selected theme."
      />

      {dark.length > 0 ? (
        <ThemeGroup
          icon={Moon}
          label="Dark"
          themes={dark}
          activeId={themeId}
          onSelect={setThemeId}
        />
      ) : null}

      {light.length > 0 ? (
        <ThemeGroup
          icon={Sun}
          label="Light"
          themes={light}
          activeId={themeId}
          onSelect={setThemeId}
        />
      ) : null}
    </div>
  )
}

interface ThemeGroupProps {
  icon: React.ElementType
  label: string
  themes: ThemeDefinition[]
  activeId: string
  onSelect: (id: string) => void
}

function ThemeGroup({ icon: Icon, label, themes, activeId, onSelect }: ThemeGroupProps) {
  return (
    <section className="flex flex-col gap-2.5">
      <div className="flex items-center gap-2">
        <Icon className="h-3.5 w-3.5 text-muted-foreground/80" />
        <h4 className="text-[12.5px] font-semibold text-foreground">{label}</h4>
        <span className="ml-auto text-[11px] text-muted-foreground">{themes.length}</span>
      </div>
      <div className="grid grid-cols-2 gap-2">
        {themes.map((theme) => (
          <ThemeRow
            key={theme.id}
            theme={theme}
            active={theme.id === activeId}
            onSelect={() => onSelect(theme.id)}
          />
        ))}
      </div>
    </section>
  )
}

interface ThemeRowProps {
  theme: ThemeDefinition
  active: boolean
  onSelect: () => void
}

function ThemeRow({ theme, active, onSelect }: ThemeRowProps) {
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={active}
      className={cn(
        "group relative flex items-center gap-3 rounded-md border px-2.5 py-2 text-left transition-colors motion-fast",
        active
          ? "border-primary/50 bg-primary/[0.04]"
          : "border-border/60 hover:border-border hover:bg-secondary/30",
      )}
    >
      <ThemeSwatch theme={theme} />
      <div className="min-w-0 flex-1">
        <p className="truncate text-[12.5px] font-medium text-foreground">{theme.name}</p>
        <p className="mt-0.5 line-clamp-1 text-[11px] leading-[1.35] text-muted-foreground">
          {theme.description}
        </p>
      </div>
      <div
        className={cn(
          "flex h-4 w-4 shrink-0 items-center justify-center rounded-full transition-colors",
          active
            ? "bg-primary text-primary-foreground"
            : "border border-border/70 bg-transparent text-transparent",
        )}
        aria-hidden
      >
        <Check className="h-2.5 w-2.5" />
      </div>
    </button>
  )
}

function ThemeSwatch({ theme }: { theme: ThemeDefinition }) {
  const c = theme.colors
  return (
    <div
      className="relative flex h-9 w-9 shrink-0 overflow-hidden rounded-md border border-border/60"
      style={{ backgroundColor: c.background }}
      aria-hidden
    >
      <div className="w-1/3" style={{ backgroundColor: c.sidebar }} />
      <div className="flex flex-1 flex-col justify-end gap-0.5 p-1">
        <div className="h-1 w-full rounded-sm" style={{ backgroundColor: c.primary }} />
        <div className="h-1 w-full rounded-sm" style={{ backgroundColor: c.accent }} />
      </div>
    </div>
  )
}
