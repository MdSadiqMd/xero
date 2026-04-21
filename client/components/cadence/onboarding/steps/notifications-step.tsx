import { useState } from "react"
import { Check, LoaderCircle } from "lucide-react"
import { DiscordIcon, TelegramIcon } from "@/components/cadence/brand-icons"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { cn } from "@/lib/utils"
import type { NotificationChannelId, OnboardingNotificationState } from "../types"
import { StepHeader } from "./providers-step"

interface NotificationsStepProps {
  notifications: OnboardingNotificationState[]
  onConnect: (id: NotificationChannelId, target: string) => void
  onDisconnect: (id: NotificationChannelId) => void
}

interface ChannelMeta {
  id: NotificationChannelId
  label: string
  Icon: React.ElementType
  placeholder: string
}

const CHANNELS: ChannelMeta[] = [
  { id: "telegram", label: "Telegram", Icon: TelegramIcon, placeholder: "Chat ID or @username" },
  { id: "discord", label: "Discord", Icon: DiscordIcon, placeholder: "Webhook URL" },
]

export function NotificationsStep({ notifications, onConnect, onDisconnect }: NotificationsStepProps) {
  return (
    <div>
      <StepHeader
        title="Stay in the loop"
        description="Approve runs and get updates from your preferred channel."
      />

      <div className="mt-8 flex flex-col divide-y divide-border rounded-lg border border-border bg-card/40">
        {CHANNELS.map((channel) => {
          const state = notifications.find((item) => item.id === channel.id)
          return (
            <ChannelRow
              key={channel.id}
              meta={channel}
              state={state}
              onConnect={(target) => onConnect(channel.id, target)}
              onDisconnect={() => onDisconnect(channel.id)}
            />
          )
        })}
      </div>
    </div>
  )
}

interface ChannelRowProps {
  meta: ChannelMeta
  state: OnboardingNotificationState | undefined
  onConnect: (target: string) => void
  onDisconnect: () => void
}

function ChannelRow({ meta, state, onConnect, onDisconnect }: ChannelRowProps) {
  const connected = Boolean(state?.connected)
  const [open, setOpen] = useState(false)
  const [target, setTarget] = useState(state?.target ?? "")
  const [submitting, setSubmitting] = useState(false)

  function submit() {
    if (!target.trim()) return
    setSubmitting(true)
    window.setTimeout(() => {
      onConnect(target.trim())
      setSubmitting(false)
      setOpen(false)
    }, 500)
  }

  return (
    <div className="px-4 py-3">
      <div className="flex items-center gap-3">
        <span
          className={cn(
            "flex h-8 w-8 shrink-0 items-center justify-center rounded-md border transition-colors",
            connected
              ? "border-primary/40 bg-primary/10 text-primary"
              : "border-border bg-secondary/50 text-foreground/70",
          )}
        >
          <meta.Icon className="h-4 w-4" />
        </span>
        <div className="min-w-0 flex-1">
          <p className="text-[13px] text-foreground">{meta.label}</p>
          {connected && !open ? (
            <p className="mt-0.5 truncate font-mono text-[11px] text-muted-foreground">{state?.target}</p>
          ) : null}
        </div>
        {connected && !open ? (
          <>
            <Button
              size="sm"
              variant="ghost"
              className="h-7 px-2 text-[11px] text-muted-foreground hover:text-foreground"
              onClick={() => {
                setTarget(state?.target ?? "")
                setOpen(true)
              }}
            >
              Edit
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="h-7 px-2 text-[11px] text-muted-foreground hover:text-destructive"
              onClick={onDisconnect}
            >
              Remove
            </Button>
          </>
        ) : !open ? (
          <Button
            size="sm"
            variant="outline"
            className="h-7 min-w-[96px] text-[11px]"
            onClick={() => {
              setTarget("")
              setOpen(true)
            }}
          >
            Connect
          </Button>
        ) : null}
      </div>

      {open ? (
        <div className="mt-3 flex items-center gap-2">
          <Input
            value={target}
            onChange={(event) => setTarget(event.target.value)}
            placeholder={meta.placeholder}
            className="h-8 font-mono text-[12px]"
            disabled={submitting}
            onKeyDown={(event) => {
              if (event.key === "Enter") submit()
              if (event.key === "Escape") setOpen(false)
            }}
            autoFocus
          />
          <Button
            size="sm"
            onClick={submit}
            disabled={submitting || !target.trim()}
            className="h-8 min-w-[72px] bg-primary text-[11px] hover:bg-primary/90"
          >
            {submitting ? <LoaderCircle className="h-3 w-3 animate-spin" /> : "Save"}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="h-8 text-[11px] text-muted-foreground hover:text-foreground"
            onClick={() => setOpen(false)}
            disabled={submitting}
          >
            Cancel
          </Button>
        </div>
      ) : null}
    </div>
  )
}
