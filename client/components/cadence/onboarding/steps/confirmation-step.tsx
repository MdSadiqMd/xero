import { Check } from "lucide-react"
import { Switch } from "@/components/ui/switch"
import { cn } from "@/lib/utils"
import type { OnboardingData } from "../types"
import { StepHeader } from "./providers-step"

interface ConfirmationStepProps {
  data: OnboardingData
  onToggleLearn: (value: boolean) => void
}

const PROVIDER_LABELS: Record<OnboardingData["providers"][number]["id"], string> = {
  openai_codex: "OpenAI Codex",
  openrouter: "OpenRouter",
  anthropic: "Anthropic",
  google: "Google",
}

export function ConfirmationStep({ data, onToggleLearn }: ConfirmationStepProps) {
  const connectedProviders = data.providers.filter((provider) => provider.status === "connected")
  const connectedChannels = data.notifications.filter((channel) => channel.connected)

  const rows: Array<{ label: string; value: string; ready: boolean }> = [
    {
      label: "Providers",
      ready: connectedProviders.length > 0,
      value:
        connectedProviders.length === 0
          ? "Not set"
          : connectedProviders.map((provider) => PROVIDER_LABELS[provider.id]).join(", "),
    },
    {
      label: "Project",
      ready: Boolean(data.project),
      value: data.project?.name ?? "Not set",
    },
    {
      label: "Notifications",
      ready: connectedChannels.length > 0,
      value:
        connectedChannels.length === 0
          ? "Not set"
          : connectedChannels.map((channel) => (channel.id === "telegram" ? "Telegram" : "Discord")).join(", "),
    },
  ]

  return (
    <div>
      <StepHeader title="Review and finish" description="You can revisit any of this from Settings." />

      <dl className="mt-8 flex flex-col divide-y divide-border rounded-lg border border-border bg-card/40">
        {rows.map((row) => (
          <div key={row.label} className="flex items-center gap-3 px-4 py-3">
            <dt className="w-28 shrink-0 text-[12px] text-muted-foreground">{row.label}</dt>
            <dd className={cn("flex-1 text-[13px]", row.ready ? "text-foreground" : "text-muted-foreground")}>
              {row.value}
            </dd>
            {row.ready ? (
              <Check className="h-3.5 w-3.5 shrink-0 text-primary" strokeWidth={2.5} />
            ) : (
              <span className="h-1 w-1 shrink-0 rounded-full bg-muted-foreground/40" />
            )}
          </div>
        ))}
      </dl>

      {/* Learn environment */}
      <div className="mt-5 flex items-start gap-4 rounded-lg border border-border bg-card/40 px-4 py-4">
        <div className="min-w-0 flex-1">
          <p className="text-[13px] font-medium text-foreground">Learn my environment</p>
          <p className="mt-1 text-[12px] leading-relaxed text-muted-foreground">
            Detect your languages, package managers, and CLI tools locally to tailor suggestions.
          </p>
        </div>
        <Switch
          checked={data.learnEnvironment}
          onCheckedChange={onToggleLearn}
          aria-label="Enable environment learning"
          className="mt-0.5 shrink-0"
        />
      </div>
    </div>
  )
}
