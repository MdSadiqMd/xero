"use client"

import { Loader2 } from "lucide-react"
import { cn } from "@/lib/utils"

interface PanelHeaderProps {
  icon: React.ComponentType<{ className?: string }>
  title: string
  description?: string
  busy?: boolean
  right?: React.ReactNode
}

export function PanelHeader({
  icon: Icon,
  title,
  description,
  busy,
  right,
}: PanelHeaderProps) {
  return (
    <header className="flex items-start justify-between gap-3">
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2 text-[11px] font-medium text-foreground/85">
          <Icon className="h-3.5 w-3.5 text-primary" />
          <span className="truncate">{title}</span>
        </div>
        {description ? (
          <p className="mt-0.5 text-[10.5px] leading-snug text-muted-foreground">
            {description}
          </p>
        ) : null}
      </div>
      <div className="flex shrink-0 items-center gap-1">
        {busy ? (
          <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />
        ) : null}
        {right}
      </div>
    </header>
  )
}

interface UnderlineTabsProps {
  ariaLabel?: string
  className?: string
  children: React.ReactNode
}

export function UnderlineTabs({ ariaLabel, className, children }: UnderlineTabsProps) {
  return (
    <div
      role="tablist"
      aria-label={ariaLabel}
      className={cn(
        "flex shrink-0 items-center gap-0.5 overflow-x-auto border-b border-border/60",
        "[scrollbar-width:none] [&::-webkit-scrollbar]:hidden",
        className,
      )}
    >
      {children}
    </div>
  )
}

interface UnderlineTabProps {
  active: boolean
  icon?: React.ComponentType<{ className?: string }>
  label: string
  count?: number | null
  onClick: () => void
}

export function UnderlineTab({
  active,
  icon: Icon,
  label,
  count,
  onClick,
}: UnderlineTabProps) {
  return (
    <button
      type="button"
      role="tab"
      aria-selected={active}
      onClick={onClick}
      className={cn(
        "relative inline-flex shrink-0 items-center gap-1.5 px-2.5 py-1.5 text-[11px] transition-colors",
        active ? "text-foreground" : "text-muted-foreground hover:text-foreground",
      )}
    >
      {Icon ? <Icon className="h-3.5 w-3.5" /> : null}
      <span>{label}</span>
      {count != null && count > 0 ? (
        <span
          className={cn(
            "rounded px-1 text-[9.5px] tabular-nums",
            active ? "bg-primary/20 text-primary" : "bg-secondary/60 text-muted-foreground",
          )}
        >
          {count}
        </span>
      ) : null}
      {active ? (
        <span className="absolute inset-x-1 -bottom-px h-px bg-primary" />
      ) : null}
    </button>
  )
}
