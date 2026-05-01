import { ShieldCheck, ShieldQuestion } from "lucide-react"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import type {
  EnvironmentPermissionDecisionStatusDto,
  EnvironmentPermissionRequestDto,
} from "@/src/lib/xero-model/environment"
import { StepHeader } from "./providers-step"

interface EnvironmentAccessStepProps {
  permissionRequests: EnvironmentPermissionRequestDto[]
  decisions: Record<string, EnvironmentPermissionDecisionStatusDto | "pending">
  onDecisionChange: (
    requestId: string,
    status: EnvironmentPermissionDecisionStatusDto,
  ) => void
  disabled?: boolean
}

const KIND_LABELS: Record<EnvironmentPermissionRequestDto["kind"], string> = {
  os_permission: "OS permission",
  protected_path: "Protected files",
  network_access: "Network access",
  installation_action: "Installation",
}

export function EnvironmentAccessStep({
  permissionRequests,
  decisions,
  onDecisionChange,
  disabled = false,
}: EnvironmentAccessStepProps) {
  return (
    <div>
      <StepHeader
        title="Review environment access"
        description="Xero needs your explicit approval before using access that the silent local probe cannot request on its own."
      />

      <div className="mt-7 flex flex-col gap-2 animate-in fade-in-0 slide-in-from-bottom-1 motion-enter [animation-delay:60ms] [animation-fill-mode:both]">
        {permissionRequests.map((request) => (
          <div
            key={request.id}
            className="rounded-lg border border-border bg-card/40 px-3.5 py-3"
          >
            <div className="flex items-start gap-3">
              <span
                className={cn(
                  "mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-md border",
                  request.optional
                    ? "border-border bg-secondary/50 text-muted-foreground"
                    : "border-primary/40 bg-primary/10 text-primary",
                )}
              >
                {request.optional ? (
                  <ShieldQuestion className="h-4 w-4" />
                ) : (
                  <ShieldCheck className="h-4 w-4" />
                )}
              </span>

              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-1.5">
                  <p className="text-[13px] font-medium text-foreground">{request.title}</p>
                  <Badge variant="secondary" className="px-1.5 py-0 text-[10px] font-medium">
                    {KIND_LABELS[request.kind]}
                  </Badge>
                  {request.optional ? (
                    <Badge variant="outline" className="px-1.5 py-0 text-[10px] font-medium">
                      Optional
                    </Badge>
                  ) : null}
                </div>
                <p className="mt-1 text-[11.5px] leading-relaxed text-muted-foreground">
                  {request.reason}
                </p>
              </div>
            </div>

            <EnvironmentAccessDecisionControl
              request={request}
              decision={decisions[request.id] ?? (request.optional ? "skipped" : "pending")}
              disabled={disabled}
              onDecisionChange={onDecisionChange}
            />
          </div>
        ))}
      </div>
    </div>
  )
}

function EnvironmentAccessDecisionControl({
  request,
  decision,
  disabled,
  onDecisionChange,
}: {
  request: EnvironmentPermissionRequestDto
  decision: EnvironmentPermissionDecisionStatusDto | "pending"
  disabled: boolean
  onDecisionChange: (
    requestId: string,
    status: EnvironmentPermissionDecisionStatusDto,
  ) => void
}) {
  const isGranted = decision === "granted"
  const isSkipped = decision === "skipped"

  return (
    <div className="mt-3 flex flex-wrap items-center gap-2">
      <Button
        type="button"
        size="sm"
        variant={isGranted ? "default" : "outline"}
        disabled={disabled}
        onClick={() => onDecisionChange(request.id, "granted")}
        className="h-7 px-2.5 text-[11.5px]"
      >
        {isGranted ? "Allowed" : `Allow ${request.title}`}
      </Button>

      {request.optional ? (
        <Button
          type="button"
          size="sm"
          variant={isSkipped ? "secondary" : "ghost"}
          disabled={disabled}
          onClick={() => onDecisionChange(request.id, "skipped")}
          className="h-7 px-2.5 text-[11.5px] text-muted-foreground"
        >
          Skip optional access
        </Button>
      ) : null}
    </div>
  )
}
