export type OnboardingStepId = "welcome" | "providers" | "project" | "notifications" | "confirm"

export type ProviderId = "openai_codex" | "openrouter" | "anthropic" | "google"

export interface OnboardingProviderState {
  id: ProviderId
  status: "idle" | "connecting" | "connected"
}

export interface OnboardingProjectState {
  name: string
  path: string
}

export type NotificationChannelId = "telegram" | "discord"

export interface OnboardingNotificationState {
  id: NotificationChannelId
  target: string
  connected: boolean
}

export interface OnboardingData {
  providers: OnboardingProviderState[]
  project: OnboardingProjectState | null
  notifications: OnboardingNotificationState[]
  learnEnvironment: boolean
}

export const INITIAL_ONBOARDING_DATA: OnboardingData = {
  providers: [
    { id: "openai_codex", status: "idle" },
    { id: "openrouter", status: "idle" },
    { id: "anthropic", status: "idle" },
    { id: "google", status: "idle" },
  ],
  project: null,
  notifications: [
    { id: "telegram", target: "", connected: false },
    { id: "discord", target: "", connected: false },
  ],
  learnEnvironment: true,
}
