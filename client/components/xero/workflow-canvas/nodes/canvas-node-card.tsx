'use client'

import { type CSSProperties, type ReactNode } from 'react'
import { type LucideIcon } from 'lucide-react'

import { cn } from '@/lib/utils'

export interface CanvasNodeCardProps {
  title: string
  subtitle: string
  icon: LucideIcon
  tone: string
  iconClassName: string
  badges?: ReactNode
  detail?: ReactNode
  chips?: ReactNode
  footer?: ReactNode
  selected?: boolean
  width?: number
  testId?: string
  attributes?: Record<string, string | undefined>
  className?: string
}

export function CanvasNodeCard({
  title,
  subtitle,
  icon: Icon,
  tone,
  iconClassName,
  badges,
  detail,
  chips,
  footer,
  selected = false,
  width = 260,
  testId,
  attributes,
  className,
}: CanvasNodeCardProps) {
  const dataAttributes = Object.fromEntries(
    Object.entries(attributes ?? {}).filter(([, value]) => value !== undefined),
  )
  return (
    <div
      className={cn('agent-card overflow-hidden text-card-foreground', selected && 'selected', className)}
      style={{ width } satisfies CSSProperties}
      data-testid={testId}
      {...dataAttributes}
    >
      <div className="agent-card-tone-strip" data-tone={tone} />
      <div className="space-y-2 px-3 py-2.5">
        <div className="flex items-start gap-2">
          <span
            className={cn(
              'mt-px inline-flex h-6 w-6 shrink-0 items-center justify-center rounded-md ring-1',
              iconClassName,
            )}
          >
            <Icon className="h-3.5 w-3.5" aria-hidden="true" />
          </span>
          <div className="min-w-0 flex-1">
            <div className="flex min-w-0 items-center gap-1.5">
              <span className="truncate text-[12.5px] font-semibold text-foreground/95">
                {title}
              </span>
              {badges}
            </div>
            <p className="mt-0.5 truncate font-mono text-[10px] text-muted-foreground/80">
              {subtitle}
            </p>
          </div>
        </div>

        {detail ? (
          <p className="agent-node-detail line-clamp-2 text-[10.5px] leading-relaxed text-muted-foreground">
            {detail}
          </p>
        ) : null}

        {chips ? (
          <div className="agent-node-chip-row flex flex-wrap items-center gap-1">
            {chips}
          </div>
        ) : null}

        {footer}
      </div>
    </div>
  )
}
