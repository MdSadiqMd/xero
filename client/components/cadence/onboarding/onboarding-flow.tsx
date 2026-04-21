"use client"

import { useCallback, useMemo, useState } from "react"
import { ArrowLeft, ArrowRight } from "lucide-react"
import { Button } from "@/components/ui/button"
import { OnboardingBackground } from "./onboarding-background"
import { StepIndicator } from "./step-indicator"
import { WelcomeStep } from "./steps/welcome-step"
import { ProvidersStep } from "./steps/providers-step"
import { ProjectStep } from "./steps/project-step"
import { NotificationsStep } from "./steps/notifications-step"
import { ConfirmationStep } from "./steps/confirmation-step"
import {
  INITIAL_ONBOARDING_DATA,
  type NotificationChannelId,
  type OnboardingData,
  type OnboardingProjectState,
  type OnboardingStepId,
  type ProviderId,
} from "./types"

const STEP_ORDER: Array<{ id: OnboardingStepId; showIndicator: boolean }> = [
  { id: "welcome", showIndicator: false },
  { id: "providers", showIndicator: true },
  { id: "project", showIndicator: true },
  { id: "notifications", showIndicator: true },
  { id: "confirm", showIndicator: true },
]

const INDICATOR_STEPS = STEP_ORDER.filter((step) => step.showIndicator)

export interface OnboardingFlowProps {
  onComplete: (data: OnboardingData) => void
  onDismiss: () => void
}

export function OnboardingFlow({ onComplete, onDismiss }: OnboardingFlowProps) {
  const [stepIndex, setStepIndex] = useState(0)
  const [data, setData] = useState<OnboardingData>(INITIAL_ONBOARDING_DATA)

  const currentStep = STEP_ORDER[stepIndex]

  const goTo = useCallback((target: number) => {
    setStepIndex(Math.max(0, Math.min(STEP_ORDER.length - 1, target)))
  }, [])

  const next = useCallback(() => goTo(stepIndex + 1), [goTo, stepIndex])
  const back = useCallback(() => goTo(stepIndex - 1), [goTo, stepIndex])

  const toggleProvider = useCallback((id: ProviderId) => {
    setData((current) => ({
      ...current,
      providers: current.providers.map((provider) => {
        if (provider.id !== id) return provider
        if (provider.status === "connected") return { ...provider, status: "idle" as const }
        return { ...provider, status: "connecting" as const }
      }),
    }))

    window.setTimeout(() => {
      setData((current) => ({
        ...current,
        providers: current.providers.map((provider) =>
          provider.id === id && provider.status === "connecting"
            ? { ...provider, status: "connected" as const }
            : provider,
        ),
      }))
    }, 600)
  }, [])

  const setProject = useCallback((project: OnboardingProjectState | null) => {
    setData((current) => ({ ...current, project }))
  }, [])

  const connectNotification = useCallback((id: NotificationChannelId, target: string) => {
    setData((current) => ({
      ...current,
      notifications: current.notifications.map((channel) =>
        channel.id === id ? { ...channel, target, connected: true } : channel,
      ),
    }))
  }, [])

  const disconnectNotification = useCallback((id: NotificationChannelId) => {
    setData((current) => ({
      ...current,
      notifications: current.notifications.map((channel) =>
        channel.id === id ? { ...channel, target: "", connected: false } : channel,
      ),
    }))
  }, [])

  const toggleLearn = useCallback((value: boolean) => {
    setData((current) => ({ ...current, learnEnvironment: value }))
  }, [])

  const indicatorIndex = useMemo(
    () => Math.max(0, INDICATOR_STEPS.findIndex((step) => step.id === currentStep.id)),
    [currentStep.id],
  )

  const showFooter = currentStep.id !== "welcome"
  const isConfirm = currentStep.id === "confirm"
  const primaryLabel = isConfirm ? "Enter Cadence" : "Continue"
  const handlePrimary = isConfirm ? () => onComplete(data) : next

  return (
    <div className="relative flex min-h-full flex-1 flex-col overflow-hidden bg-background text-foreground">
      <OnboardingBackground />

      {/* Header */}
      <header className="relative z-10 flex shrink-0 items-center justify-between gap-3 px-8 pt-5">
        <div className="min-w-[72px]">
          {currentStep.showIndicator ? (
            <StepIndicator total={INDICATOR_STEPS.length} currentIndex={indicatorIndex} />
          ) : null}
        </div>

        <Button
          variant="ghost"
          size="sm"
          onClick={onDismiss}
          className="text-[12px] text-muted-foreground hover:text-foreground"
        >
          Skip setup
        </Button>
      </header>

      {/* Main */}
      <main className="relative z-10 flex flex-1 items-center justify-center overflow-y-auto px-8 py-10">
        <div
          key={currentStep.id}
          className="w-full max-w-md animate-in fade-in-0 duration-300"
        >
          {currentStep.id === "welcome" ? (
            <WelcomeStep onContinue={next} onSkipAll={onDismiss} />
          ) : null}
          {currentStep.id === "providers" ? (
            <ProvidersStep providers={data.providers} onToggleProvider={toggleProvider} />
          ) : null}
          {currentStep.id === "project" ? (
            <ProjectStep project={data.project} onSetProject={setProject} />
          ) : null}
          {currentStep.id === "notifications" ? (
            <NotificationsStep
              notifications={data.notifications}
              onConnect={connectNotification}
              onDisconnect={disconnectNotification}
            />
          ) : null}
          {currentStep.id === "confirm" ? (
            <ConfirmationStep data={data} onToggleLearn={toggleLearn} />
          ) : null}
        </div>
      </main>

      {/* Footer */}
      {showFooter ? (
        <footer className="relative z-10 shrink-0">
          <div className="mx-auto flex w-full max-w-md items-center justify-between gap-2 px-8 pb-6">
            <Button
              variant="ghost"
              size="sm"
              onClick={back}
              disabled={stepIndex <= 1}
              className="h-8 gap-1.5 px-2 text-[12px] text-muted-foreground hover:text-foreground"
            >
              <ArrowLeft className="h-3.5 w-3.5" />
              Back
            </Button>

            <div className="flex items-center gap-1">
              {!isConfirm ? (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={next}
                  className="h-8 text-[12px] text-muted-foreground hover:text-foreground"
                >
                  Skip
                </Button>
              ) : null}
              <Button
                size="sm"
                onClick={handlePrimary}
                className="group h-8 gap-1.5 bg-primary px-3 text-[12px] font-medium hover:bg-primary/90"
              >
                {primaryLabel}
                <ArrowRight className="h-3.5 w-3.5 transition-transform group-hover:translate-x-0.5" />
              </Button>
            </div>
          </div>
        </footer>
      ) : null}
    </div>
  )
}
