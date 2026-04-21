import { cn } from "@/lib/utils"

interface StepIndicatorProps {
  total: number
  currentIndex: number
}

export function StepIndicator({ total, currentIndex }: StepIndicatorProps) {
  return (
    <div className="flex items-center gap-1.5" role="group" aria-label="Onboarding progress">
      {Array.from({ length: total }).map((_, index) => {
        const isCurrent = index === currentIndex
        const isDone = index < currentIndex
        return (
          <span
            key={index}
            className={cn(
              "h-1 rounded-full transition-all duration-300",
              isCurrent ? "w-6 bg-primary" : isDone ? "w-1.5 bg-primary/50" : "w-1.5 bg-border",
            )}
          />
        )
      })}
    </div>
  )
}
