import { Check, KeyRound, LoaderCircle } from "lucide-react"
import { AnthropicIcon, GoogleIcon, OpenAIIcon } from "@/components/cadence/brand-icons"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import type { OnboardingProviderState, ProviderId } from "../types"

interface ProvidersStepProps {
  providers: OnboardingProviderState[]
  onToggleProvider: (id: ProviderId) => void
}

interface ProviderMeta {
  id: ProviderId
  label: string
  Icon: React.ElementType
  disabled?: boolean
}

const PROVIDER_META: ProviderMeta[] = [
  { id: "openai_codex", label: "OpenAI Codex", Icon: OpenAIIcon },
  { id: "openrouter", label: "OpenRouter", Icon: KeyRound },
  { id: "anthropic", label: "Anthropic", Icon: AnthropicIcon },
  { id: "google", label: "Google", Icon: GoogleIcon, disabled: true },
]

export function ProvidersStep({ providers, onToggleProvider }: ProvidersStepProps) {
  return (
    <div>
      <StepHeader title="Connect a provider" description="Pick one or more — you can add or change these later." />

      <div className="mt-8 flex flex-col divide-y divide-border rounded-lg border border-border bg-card/40">
        {PROVIDER_META.map((meta) => {
          const state = providers.find((provider) => provider.id === meta.id)
          const status = state?.status ?? "idle"
          const connected = status === "connected"
          const connecting = status === "connecting"

          return (
            <div key={meta.id} className="flex items-center gap-3 px-4 py-3">
              <span
                className={cn(
                  "flex h-8 w-8 shrink-0 items-center justify-center rounded-md border transition-colors",
                  connected
                    ? "border-primary/40 bg-primary/10 text-primary"
                    : "border-border bg-secondary/50 text-foreground/70",
                  meta.disabled && "opacity-40",
                )}
              >
                <meta.Icon className="h-4 w-4" />
              </span>
              <p className={cn("flex-1 text-[13px] text-foreground", meta.disabled && "text-muted-foreground")}>
                {meta.label}
              </p>
              <Button
                size="sm"
                variant={connected ? "ghost" : "outline"}
                disabled={meta.disabled || connecting}
                onClick={() => onToggleProvider(meta.id)}
                className={cn(
                  "h-7 min-w-[96px] text-[11px]",
                  connected && "text-primary hover:text-primary",
                )}
              >
                {connecting ? (
                  <LoaderCircle className="h-3 w-3 animate-spin" />
                ) : connected ? (
                  <>
                    <Check className="h-3 w-3" />
                    Connected
                  </>
                ) : meta.disabled ? (
                  "Soon"
                ) : (
                  "Connect"
                )}
              </Button>
            </div>
          )
        })}
      </div>
    </div>
  )
}

interface StepHeaderProps {
  title: string
  description: string
}

export function StepHeader({ title, description }: StepHeaderProps) {
  return (
    <div>
      <h2 className="text-2xl font-semibold tracking-tight text-foreground">{title}</h2>
      <p className="mt-2 text-[13px] leading-relaxed text-muted-foreground">{description}</p>
    </div>
  )
}
