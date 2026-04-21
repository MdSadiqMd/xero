import type { AgentPaneView } from '@/src/features/cadence/use-cadence-desktop-state'
import type {
  RuntimeRunView,
  RuntimeSessionView,
  RuntimeStreamStatus,
} from '@/src/lib/cadence-model'

import { displayValue } from './shared-helpers'
import { hasUsableRuntimeRunId } from './runtime-stream-helpers'

interface ComposerModelOption {
  value: string
  label: string
}

interface ComposerModelGroup {
  id: string
  label: string
  items: ComposerModelOption[]
}

const SAMPLE_COMPOSER_MODEL_GROUPS: ComposerModelGroup[] = [
  {
    id: 'openai_codex',
    label: 'OpenAI Codex',
    items: [
      { value: 'openai_codex', label: 'openai_codex' },
      { value: 'codex-mini-latest', label: 'codex-mini-latest' },
    ],
  },
  {
    id: 'openai',
    label: 'OpenAI',
    items: [
      { value: 'gpt-4.1', label: 'gpt-4.1' },
      { value: 'gpt-4.1-mini', label: 'gpt-4.1-mini' },
      { value: 'o4-mini', label: 'o4-mini' },
      { value: 'o3', label: 'o3' },
      { value: 'o3-mini', label: 'o3-mini' },
    ],
  },
  {
    id: 'anthropic',
    label: 'Anthropic',
    items: [
      { value: 'anthropic/claude-3.7-sonnet', label: 'claude-3.7-sonnet' },
      { value: 'anthropic/claude-3.5-sonnet', label: 'claude-3.5-sonnet' },
      { value: 'anthropic/claude-3.5-haiku', label: 'claude-3.5-haiku' },
    ],
  },
  {
    id: 'google',
    label: 'Google',
    items: [
      { value: 'gemini-2.5-pro', label: 'gemini-2.5-pro' },
      { value: 'gemini-2.5-flash', label: 'gemini-2.5-flash' },
    ],
  },
  {
    id: 'deepseek',
    label: 'DeepSeek',
    items: [
      { value: 'deepseek/deepseek-chat-v3-0324', label: 'deepseek-chat-v3-0324' },
      { value: 'deepseek/deepseek-r1-0528', label: 'deepseek-r1-0528' },
    ],
  },
  {
    id: 'meta_llama',
    label: 'Meta Llama',
    items: [
      { value: 'meta-llama/llama-4-maverick', label: 'llama-4-maverick' },
      { value: 'meta-llama/llama-4-scout', label: 'llama-4-scout' },
    ],
  },
  {
    id: 'mistral',
    label: 'Mistral',
    items: [
      { value: 'mistral/magistral-medium-2506', label: 'magistral-medium-2506' },
      { value: 'mistral/devstral-medium', label: 'devstral-medium' },
    ],
  },
  {
    id: 'moonshot',
    label: 'Moonshot',
    items: [{ value: 'moonshotai/kimi-k2', label: 'kimi-k2' }],
  },
  {
    id: 'x_ai',
    label: 'xAI',
    items: [
      { value: 'x-ai/grok-3-beta', label: 'grok-3-beta' },
      { value: 'x-ai/grok-3-mini-beta', label: 'grok-3-mini-beta' },
    ],
  },
]

export function getComposerModelGroups(
  selectedProviderId: string,
  selectedProviderLabel: string,
  currentModelId: string,
): ComposerModelGroup[] {
  const groups = SAMPLE_COMPOSER_MODEL_GROUPS.map((group) => ({
    ...group,
    items: [...group.items],
  }))

  const currentExists = groups.some((group) => group.items.some((item) => item.value === currentModelId))
  if (currentExists) {
    return groups
  }

  const fallbackLabel = selectedProviderLabel.trim().length > 0 ? selectedProviderLabel : 'Selected provider'
  const fallbackGroupIndex = groups.findIndex((group) => group.id === selectedProviderId)
  const fallbackItem = { value: currentModelId, label: currentModelId }

  if (fallbackGroupIndex >= 0) {
    groups[fallbackGroupIndex] = {
      ...groups[fallbackGroupIndex],
      items: [fallbackItem, ...groups[fallbackGroupIndex].items],
    }
    return groups
  }

  return [{ id: selectedProviderId, label: fallbackLabel, items: [fallbackItem] }, ...groups]
}

export function getSelectedProviderId(agent: AgentPaneView, runtimeSession: RuntimeSessionView | null): string {
  return agent.selectedProviderId ?? runtimeSession?.providerId ?? 'openai_codex'
}

export function getSelectedProviderLabel(agent: AgentPaneView, runtimeSession: RuntimeSessionView | null): string {
  return agent.selectedProviderLabel ?? (getSelectedProviderId(agent, runtimeSession) === 'openrouter' ? 'OpenRouter' : 'OpenAI Codex')
}

export function getComposerPlaceholder(
  runtimeSession: RuntimeSessionView | null,
  streamStatus: RuntimeStreamStatus,
  runtimeRun: RuntimeRunView | null,
  streamRunId: string | undefined,
  options: { selectedProviderId: string; openrouterApiKeyConfigured: boolean; providerMismatch: boolean },
): string {
  if (!runtimeSession) {
    if (options.selectedProviderId === 'openrouter') {
      return options.openrouterApiKeyConfigured
        ? 'Bind OpenRouter from the Agent tab to start.'
        : 'Configure an OpenRouter API key in Settings to start.'
    }

    return 'Connect a provider to start.'
  }

  if (options.providerMismatch) {
    return `Rebind ${options.selectedProviderId === 'openrouter' ? 'OpenRouter' : 'the selected provider'} before trusting new live activity.`
  }

  if (!runtimeSession.isAuthenticated) {
    if (runtimeSession.isLoginInProgress) {
      return options.selectedProviderId === 'openrouter'
        ? 'Finish the OpenRouter bind to continue.'
        : 'Finish the login flow to continue.'
    }

    return options.selectedProviderId === 'openrouter'
      ? options.openrouterApiKeyConfigured
        ? 'Bind OpenRouter from the Agent tab to start.'
        : 'Configure an OpenRouter API key in Settings to start.'
      : 'Connect a provider to start.'
  }

  if (!hasUsableRuntimeRunId(runtimeRun)) {
    return 'Start or reconnect a supervised run to create the run-scoped live feed for this imported project.'
  }

  switch (streamStatus) {
    case 'live':
      return 'Live activity streaming. Composer is read-only.'
    case 'complete':
      return 'Run completed.'
    case 'stale':
      return 'Stream went stale — retry to refresh.'
    case 'error':
      return 'Stream failed — retry to restore.'
    case 'subscribing':
      return 'Connecting to the live transcript.'
    case 'replaying':
      return `Cadence is replaying recent run-scoped activity for ${displayValue(streamRunId, runtimeRun.runId)} while the live feed catches up.`
    case 'idle':
      return 'Waiting for first event…'
  }
}
