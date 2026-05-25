import { cn } from "@/lib/utils"

interface StepIndicatorProps {
  total: number
  currentIndex: number
}

export function StepIndicator({ total, currentIndex }: StepIndicatorProps) {
  return (
    <div
      className="flex items-center gap-2"
      role="group"
      aria-label={`Step ${currentIndex + 1} of ${total}`}
    >
      {Array.from({ length: total }).map((_, index) => {
        const isCurrent = index === currentIndex
        const isDone = index < currentIndex
        return (
          <span
            key={index}
            aria-current={isCurrent ? "step" : undefined}
            className={cn(
              "h-1.5 rounded-full transition-[width,background-color] motion-standard",
              isCurrent
                ? "w-9 bg-primary"
                : isDone
                  ? "w-2 bg-primary/50"
                  : "w-2 bg-border",
            )}
          />
        )
      })}
      <span className="ml-2 text-[12px] font-medium tabular-nums text-muted-foreground">
        {currentIndex + 1}
        <span className="text-muted-foreground/50">/{total}</span>
      </span>
    </div>
  )
}
