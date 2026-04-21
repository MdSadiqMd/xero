import { ArrowRight } from "lucide-react"
import { Button } from "@/components/ui/button"

interface WelcomeStepProps {
  onContinue: () => void
  onSkipAll: () => void
}

export function WelcomeStep({ onContinue, onSkipAll }: WelcomeStepProps) {
  return (
    <div className="flex flex-col items-center text-center">
      <svg className="text-primary" fill="none" height="28" viewBox="0 0 24 24" width="28">
        <path d="M4 4h6v6H4V4Z" fill="currentColor" />
        <path d="M14 4h6v6h-6V4Z" fill="currentColor" fillOpacity="0.3" />
        <path d="M4 14h6v6H4v-6Z" fill="currentColor" fillOpacity="0.3" />
        <path d="M14 14h6v6h-6v-6Z" fill="currentColor" />
      </svg>

      <h1 className="mt-8 text-3xl font-semibold tracking-tight text-foreground">
        Welcome to Cadence
      </h1>
      <p className="mt-3 max-w-sm text-[14px] leading-relaxed text-muted-foreground">
        Three quick steps — connect a provider, add a project, pick a notification channel.
      </p>

      <div className="mt-10 flex items-center gap-2">
        <Button
          size="lg"
          onClick={onContinue}
          className="group h-10 gap-2 bg-primary px-5 text-[13px] font-medium hover:bg-primary/90"
        >
          Get started
          <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-0.5" />
        </Button>
        <Button
          size="lg"
          variant="ghost"
          onClick={onSkipAll}
          className="h-10 text-[13px] text-muted-foreground hover:text-foreground"
        >
          Skip
        </Button>
      </div>
    </div>
  )
}
