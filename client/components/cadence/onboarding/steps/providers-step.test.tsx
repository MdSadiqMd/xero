import { fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}))

import { ProvidersStep } from '@/components/cadence/onboarding/steps/providers-step'
import type {
  ProviderModelCatalogDto,
  ProviderProfileDto,
  ProviderProfilesDto,
  RuntimeSessionView,
  UpsertProviderProfileRequestDto,
} from '@/src/lib/cadence-model'

function makeOpenAiProfile(overrides: Partial<ProviderProfileDto> = {}): ProviderProfileDto {
  return {
    profileId: 'openai_codex-default',
    providerId: 'openai_codex',
    runtimeKind: 'openai_codex',
    label: 'OpenAI Codex',
    modelId: 'openai_codex',
    active: false,
    readiness: {
      ready: false,
      status: 'missing',
      proofUpdatedAt: null,
    },
    migratedFromLegacy: false,
    migratedAt: null,
    ...overrides,
  }
}

function makeOpenRouterProfile(overrides: Partial<ProviderProfileDto> = {}): ProviderProfileDto {
  const ready = overrides.readiness?.ready ?? true

  return {
    profileId: 'openrouter-default',
    providerId: 'openrouter',
    runtimeKind: 'openrouter',
    label: 'OpenRouter',
    modelId: 'openai/gpt-4.1-mini',
    presetId: 'openrouter',
    active: true,
    readiness: ready
      ? {
          ready: true,
          status: 'ready',
          proof: 'stored_secret',
          proofUpdatedAt: '2026-04-20T00:00:00Z',
        }
      : {
          ready: false,
          status: 'missing',
          proofUpdatedAt: null,
        },
    migratedFromLegacy: true,
    migratedAt: '2026-04-20T00:00:00Z',
    ...overrides,
  }
}

function makeAnthropicProfile(overrides: Partial<ProviderProfileDto> = {}): ProviderProfileDto {
  const ready = overrides.readiness?.ready ?? false

  return {
    profileId: 'anthropic-default',
    providerId: 'anthropic',
    runtimeKind: 'anthropic',
    label: 'Anthropic',
    modelId: 'claude-3-7-sonnet-latest',
    presetId: 'anthropic',
    active: false,
    readiness: ready
      ? {
          ready: true,
          status: 'ready',
          proof: 'stored_secret',
          proofUpdatedAt: '2026-04-20T00:00:00Z',
        }
      : {
          ready: false,
          status: 'missing',
          proofUpdatedAt: null,
        },
    migratedFromLegacy: false,
    migratedAt: null,
    ...overrides,
  }
}

function makeGithubProfile(overrides: Partial<ProviderProfileDto> = {}): ProviderProfileDto {
  const ready = overrides.readiness?.ready ?? false

  return {
    profileId: 'github_models-default',
    providerId: 'github_models',
    runtimeKind: 'openai_compatible',
    label: 'GitHub Models',
    modelId: 'openai/gpt-4.1',
    presetId: 'github_models',
    active: false,
    readiness: ready
      ? {
          ready: true,
          status: 'ready',
          proof: 'stored_secret',
          proofUpdatedAt: '2026-04-20T00:00:00Z',
        }
      : {
          ready: false,
          status: 'missing',
          proofUpdatedAt: null,
        },
    migratedFromLegacy: false,
    migratedAt: null,
    ...overrides,
  }
}

function makeOllamaProfile(overrides: Partial<ProviderProfileDto> = {}): ProviderProfileDto {
  const ready = overrides.readiness?.ready ?? false

  return {
    profileId: 'ollama-default',
    providerId: 'ollama',
    runtimeKind: 'openai_compatible',
    label: 'Ollama',
    modelId: 'llama3.2',
    presetId: 'ollama',
    active: false,
    baseUrl: 'http://127.0.0.1:11434/v1',
    readiness: ready
      ? {
          ready: true,
          status: 'ready',
          proof: 'local',
          proofUpdatedAt: '2026-04-20T00:00:00Z',
        }
      : {
          ready: false,
          status: 'missing',
          proofUpdatedAt: null,
        },
    migratedFromLegacy: false,
    migratedAt: null,
    ...overrides,
  }
}

function makeBedrockProfile(overrides: Partial<ProviderProfileDto> = {}): ProviderProfileDto {
  const ready = overrides.readiness?.ready ?? false

  return {
    profileId: 'bedrock-default',
    providerId: 'bedrock',
    runtimeKind: 'anthropic',
    label: 'Amazon Bedrock',
    modelId: 'anthropic.claude-3-7-sonnet-20250219-v1:0',
    presetId: 'bedrock',
    active: false,
    region: 'us-east-1',
    readiness: ready
      ? {
          ready: true,
          status: 'ready',
          proof: 'ambient',
          proofUpdatedAt: '2026-04-20T00:00:00Z',
        }
      : {
          ready: false,
          status: 'missing',
          proofUpdatedAt: null,
        },
    migratedFromLegacy: false,
    migratedAt: null,
    ...overrides,
  }
}

function makeVertexProfile(overrides: Partial<ProviderProfileDto> = {}): ProviderProfileDto {
  const ready = overrides.readiness?.ready ?? false

  return {
    profileId: 'vertex-default',
    providerId: 'vertex',
    runtimeKind: 'anthropic',
    label: 'Google Vertex AI',
    modelId: 'claude-3-7-sonnet@20250219',
    presetId: 'vertex',
    active: false,
    region: 'us-central1',
    projectId: 'vertex-project',
    readiness: ready
      ? {
          ready: true,
          status: 'ready',
          proof: 'ambient',
          proofUpdatedAt: '2026-04-20T00:00:00Z',
        }
      : {
          ready: false,
          status: 'missing',
          proofUpdatedAt: null,
        },
    migratedFromLegacy: false,
    migratedAt: null,
    ...overrides,
  }
}

function makeProviderProfiles(overrides: Partial<ProviderProfilesDto> = {}): ProviderProfilesDto {
  return {
    activeProfileId: overrides.activeProfileId ?? 'openrouter-default',
    profiles:
      overrides.profiles ?? [makeOpenAiProfile({ active: false }), makeOpenRouterProfile({ active: true })],
    migration: overrides.migration ?? null,
  }
}

function makeProviderModelCatalog(
  profileId: string,
  overrides: Partial<ProviderModelCatalogDto> = {},
): ProviderModelCatalogDto {
  const providerId =
    overrides.providerId ??
    (profileId.startsWith('openrouter')
      ? 'openrouter'
      : profileId.startsWith('anthropic')
        ? 'anthropic'
        : profileId.startsWith('github_models')
          ? 'github_models'
          : profileId.startsWith('ollama')
            ? 'ollama'
            : profileId.startsWith('bedrock')
              ? 'bedrock'
              : profileId.startsWith('vertex')
                ? 'vertex'
                : 'openai_codex')
  const configuredModelId =
    overrides.configuredModelId ??
    (providerId === 'openrouter'
      ? 'openai/gpt-4.1-mini'
      : providerId === 'anthropic'
        ? 'claude-3-7-sonnet-latest'
        : providerId === 'github_models'
          ? 'openai/gpt-4.1'
          : providerId === 'ollama'
            ? 'llama3.2'
            : providerId === 'bedrock'
              ? 'anthropic.claude-3-7-sonnet-20250219-v1:0'
              : providerId === 'vertex'
                ? 'claude-3-7-sonnet@20250219'
                : 'openai_codex')

  return {
    profileId,
    providerId,
    configuredModelId,
    source: overrides.source ?? 'live',
    fetchedAt: overrides.fetchedAt ?? '2026-04-21T12:00:00Z',
    lastSuccessAt: overrides.lastSuccessAt ?? '2026-04-21T12:00:00Z',
    lastRefreshError: overrides.lastRefreshError ?? null,
    models:
      overrides.models ??
      (providerId === 'openrouter'
        ? [
            {
              modelId: 'openai/gpt-4.1-mini',
              displayName: 'OpenAI GPT-4.1 Mini',
              thinking: {
                supported: true,
                effortOptions: ['minimal', 'low', 'medium', 'high', 'x_high'],
                defaultEffort: 'medium',
              },
            },
            {
              modelId: 'openrouter/anthropic/claude-3.5-sonnet',
              displayName: 'Claude 3.5 Sonnet',
              thinking: {
                supported: true,
                effortOptions: ['low', 'medium', 'high'],
                defaultEffort: 'medium',
              },
            },
          ]
        : providerId === 'anthropic'
          ? [
              {
                modelId: 'claude-3-7-sonnet-latest',
                displayName: 'Claude 3.7 Sonnet',
                thinking: {
                  supported: true,
                  effortOptions: ['low', 'medium', 'high', 'x_high'],
                  defaultEffort: 'medium',
                },
              },
              {
                modelId: 'claude-3-5-haiku-latest',
                displayName: 'Claude 3.5 Haiku',
                thinking: {
                  supported: false,
                  effortOptions: [],
                  defaultEffort: null,
                },
              },
            ]
          : providerId === 'github_models'
            ? [
                {
                  modelId: 'openai/gpt-4.1',
                  displayName: 'OpenAI GPT-4.1',
                  thinking: {
                    supported: true,
                    effortOptions: ['low', 'medium', 'high'],
                    defaultEffort: 'medium',
                  },
                },
              ]
            : providerId === 'ollama'
              ? [
                  {
                    modelId: 'llama3.2',
                    displayName: 'Llama 3.2',
                    thinking: {
                      supported: false,
                      effortOptions: [],
                      defaultEffort: null,
                    },
                  },
                ]
              : providerId === 'bedrock'
                ? [
                    {
                      modelId: 'anthropic.claude-3-7-sonnet-20250219-v1:0',
                      displayName: 'Claude 3.7 Sonnet (Bedrock)',
                      thinking: {
                        supported: true,
                        effortOptions: ['low', 'medium', 'high'],
                        defaultEffort: 'medium',
                      },
                    },
                  ]
                : providerId === 'vertex'
                  ? [
                      {
                        modelId: 'claude-3-7-sonnet@20250219',
                        displayName: 'Claude 3.7 Sonnet (Vertex)',
                        thinking: {
                          supported: true,
                          effortOptions: ['low', 'medium', 'high'],
                          defaultEffort: 'medium',
                        },
                      },
                    ]
                  : [
                      {
                        modelId: 'openai_codex',
                        displayName: 'OpenAI Codex',
                        thinking: {
                          supported: true,
                          effortOptions: ['low', 'medium', 'high'],
                          defaultEffort: 'medium',
                        },
                      },
                    ]),
  }
}

function makeRuntimeSession(overrides: Partial<RuntimeSessionView> = {}): RuntimeSessionView {
  return {
    projectId: 'project-1',
    runtimeKind: 'openai_codex',
    providerId: 'openai_codex',
    flowId: null,
    sessionId: null,
    accountId: null,
    phase: 'idle',
    phaseLabel: 'Idle',
    runtimeLabel: 'Openai Codex · Signed out',
    accountLabel: 'No account',
    sessionLabel: 'No session',
    callbackBound: null,
    authorizationUrl: null,
    redirectUri: null,
    lastErrorCode: null,
    lastError: null,
    updatedAt: '2026-04-20T00:00:00Z',
    isAuthenticated: false,
    isLoginInProgress: false,
    needsManualInput: false,
    isSignedOut: true,
    isFailed: false,
    ...overrides,
  }
}

function makeProvidersStepProps(overrides: Partial<Parameters<typeof ProvidersStep>[0]> = {}) {
  return {
    providerProfiles: makeProviderProfiles(),
    providerProfilesLoadStatus: 'ready' as const,
    providerProfilesLoadError: null,
    providerProfilesSaveStatus: 'idle' as const,
    providerProfilesSaveError: null,
    providerModelCatalogs: {
      'openai_codex-default': makeProviderModelCatalog('openai_codex-default'),
      'openrouter-default': makeProviderModelCatalog('openrouter-default'),
    },
    providerModelCatalogLoadStatuses: {
      'openai_codex-default': 'ready' as const,
      'openrouter-default': 'ready' as const,
    },
    onRefreshProviderProfiles: vi.fn(async () => makeProviderProfiles()),
    onRefreshProviderModelCatalog: vi.fn(async (profileId: string) => makeProviderModelCatalog(profileId)),
    onUpsertProviderProfile: vi.fn(async (_request: UpsertProviderProfileRequestDto) => makeProviderProfiles()),
    ...overrides,
  }
}

function getProviderCard(label: string): HTMLElement {
  const card = screen
    .getAllByText(label)
    .map((node) => node.closest('.rounded-lg'))
    .find((value): value is HTMLElement => value instanceof HTMLElement)

  if (!card) {
    throw new Error(`Could not find provider card for ${label}`)
  }

  return card
}

describe('ProvidersStep', () => {
  it('renders migrated provider profiles, keeps saved keys blank, and validates label edits', async () => {
    const onUpsertProviderProfile = vi.fn(async (_request: UpsertProviderProfileRequestDto) => makeProviderProfiles())

    render(
      <ProvidersStep
        {...makeProvidersStepProps({
          onUpsertProviderProfile,
        })}
      />,
    )

    expect(screen.queryByText('Active')).not.toBeInTheDocument()
    expect(screen.getByText('Ready')).toBeVisible()
    expect(screen.getByText('GitHub Models')).toBeVisible()

    fireEvent.click(within(getProviderCard('OpenRouter')).getByRole('button', { name: 'API key' }))

    const labelInput = screen.getByLabelText('Profile label') as HTMLInputElement
    const keyInput = screen.getByLabelText('API Key') as HTMLInputElement

    expect(labelInput).toHaveValue('OpenRouter')
    expect(screen.queryByLabelText('Model')).not.toBeInTheDocument()
    expect(keyInput).toHaveValue('')

    fireEvent.change(labelInput, { target: { value: '   ' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(screen.getByText('Profile label is required.')).toBeVisible()

    fireEvent.change(labelInput, { target: { value: 'Team OpenRouter' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() =>
      expect(onUpsertProviderProfile).toHaveBeenCalledWith({
        profileId: 'openrouter-default',
        providerId: 'openrouter',
        runtimeKind: 'openrouter',
        label: 'Team OpenRouter',
        modelId: 'openai/gpt-4.1-mini',
        presetId: 'openrouter',
        baseUrl: null,
        apiVersion: null,
        region: null,
        projectId: null,
        apiKey: null,
        activate: false,
      }),
    )
  })

  it('creates and clears Anthropic profiles from the onboarding provider step without special-case UI', async () => {
    const secret = 'sk-ant-test-secret'

    let providerProfiles = makeProviderProfiles({
      activeProfileId: 'openrouter-default',
      profiles: [makeOpenAiProfile({ active: false }), makeOpenRouterProfile({ active: true })],
    })

    const onUpsertProviderProfile = vi.fn(async (request: UpsertProviderProfileRequestDto) => {
      const anthropicReady = typeof request.apiKey === 'string' && request.apiKey.trim().length > 0
      providerProfiles = makeProviderProfiles({
        activeProfileId: providerProfiles.activeProfileId,
        profiles: [
          makeOpenAiProfile({ active: false }),
          makeOpenRouterProfile({ active: true }),
          makeAnthropicProfile({
            active: false,
            label: request.label,
            modelId: request.modelId,
            readiness: anthropicReady
              ? {
                  ready: true,
                  status: 'ready',
                  proofUpdatedAt: '2026-04-20T12:00:00Z',
                }
              : {
                  ready: false,
                  status: 'missing',
                  proofUpdatedAt: null,
                },
          }),
        ],
      })

      return providerProfiles
    })

    const { rerender } = render(
      <ProvidersStep
        {...makeProvidersStepProps({
          providerProfiles,
          onRefreshProviderProfiles: vi.fn(async () => providerProfiles),
          onUpsertProviderProfile,
        })}
      />,
    )

    expect(within(getProviderCard('Anthropic')).queryByRole('button', { name: 'Select' })).not.toBeInTheDocument()

    fireEvent.click(within(getProviderCard('Anthropic')).getByRole('button', { name: 'API key' }))
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(screen.getByText('Anthropic requires an API key.')).toBeVisible()

    fireEvent.change(screen.getByLabelText('API Key'), { target: { value: secret } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() =>
      expect(onUpsertProviderProfile).toHaveBeenCalledWith({
        profileId: 'anthropic-default',
        providerId: 'anthropic',
        runtimeKind: 'anthropic',
        label: 'Anthropic',
        modelId: 'claude-3-7-sonnet-latest',
        presetId: 'anthropic',
        baseUrl: null,
        apiVersion: null,
        region: null,
        projectId: null,
        apiKey: secret,
        activate: false,
      }),
    )

    rerender(
      <ProvidersStep
        {...makeProvidersStepProps({
          providerProfiles,
          onRefreshProviderProfiles: vi.fn(async () => providerProfiles),
          onUpsertProviderProfile,
        })}
      />,
    )

    fireEvent.click(within(getProviderCard('Anthropic')).getByRole('button', { name: 'API key' }))
    fireEvent.click(screen.getByRole('button', { name: 'Clear' }))
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() =>
      expect(onUpsertProviderProfile).toHaveBeenCalledWith({
        profileId: 'anthropic-default',
        providerId: 'anthropic',
        runtimeKind: 'anthropic',
        label: 'Anthropic',
        modelId: 'claude-3-7-sonnet-latest',
        presetId: 'anthropic',
        baseUrl: null,
        apiVersion: null,
        region: null,
        projectId: null,
        apiKey: '',
        activate: false,
      }),
    )
  })


  it('creates GitHub profiles from onboarding with the generic request shape and keeps tokens redacted', async () => {
    const secret = 'ghp_test_secret'

    let providerProfiles = makeProviderProfiles({
      activeProfileId: 'openrouter-default',
      profiles: [makeOpenAiProfile({ active: false }), makeOpenRouterProfile({ active: true })],
    })

    const onUpsertProviderProfile = vi.fn(async (request: UpsertProviderProfileRequestDto) => {
      const githubReady = typeof request.apiKey === 'string' && request.apiKey.trim().length > 0
      providerProfiles = makeProviderProfiles({
        activeProfileId: providerProfiles.activeProfileId,
        profiles: [
          makeOpenAiProfile({ active: false }),
          makeOpenRouterProfile({ active: true }),
          makeGithubProfile({
            active: false,
            label: request.label,
            modelId: request.modelId,
            readiness: githubReady
              ? {
                  ready: true,
                  status: 'ready',
                  proofUpdatedAt: '2026-04-20T12:00:00Z',
                }
              : {
                  ready: false,
                  status: 'missing',
                  proofUpdatedAt: null,
                },
          }),
        ],
      })

      return providerProfiles
    })

    render(
      <ProvidersStep
        {...makeProvidersStepProps({
          providerProfiles,
          onRefreshProviderProfiles: vi.fn(async () => providerProfiles),
          onUpsertProviderProfile,
        })}
      />,
    )

    expect(within(getProviderCard('GitHub Models')).queryByRole('button', { name: 'Select' })).not.toBeInTheDocument()

    fireEvent.click(within(getProviderCard('GitHub Models')).getByRole('button', { name: 'API key' }))
    const keyInput = screen.getByLabelText('API Key') as HTMLInputElement
    expect(keyInput).toHaveValue('')
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(screen.getByText('GitHub Models requires an API key.')).toBeVisible()

    fireEvent.change(keyInput, { target: { value: secret } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() =>
      expect(onUpsertProviderProfile).toHaveBeenCalledWith({
        profileId: 'github_models-default',
        providerId: 'github_models',
        runtimeKind: 'openai_compatible',
        label: 'GitHub Models',
        modelId: 'openai/gpt-4.1',
        presetId: 'github_models',
        baseUrl: null,
        apiVersion: null,
        region: null,
        projectId: null,
        apiKey: secret,
        activate: false,
      }),
    )
  })

  it('saves Ollama onboarding profiles without app-local API-key state', async () => {
    const onUpsertProviderProfile = vi.fn(async (_request: UpsertProviderProfileRequestDto) => makeProviderProfiles())

    render(
      <ProvidersStep
        {...makeProvidersStepProps({
          onUpsertProviderProfile,
        })}
      />,
    )

    fireEvent.click(within(getProviderCard('Ollama')).getByRole('button', { name: 'Endpoint' }))

    expect(
      screen.getByText('Cadence treats Ollama as a local endpoint. No app-local API key is stored for this provider profile.'),
    ).toBeVisible()
    expect(screen.queryByLabelText('API Key')).not.toBeInTheDocument()

    fireEvent.change(screen.getByLabelText('Base URL'), {
      target: { value: 'http://127.0.0.1:11434/v1' },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() =>
      expect(onUpsertProviderProfile).toHaveBeenCalledWith({
        profileId: 'ollama-default',
        providerId: 'ollama',
        runtimeKind: 'openai_compatible',
        label: 'Ollama',
        modelId: 'llama3.2',
        presetId: 'ollama',
        baseUrl: 'http://127.0.0.1:11434/v1',
        apiVersion: null,
        region: null,
        projectId: null,
        apiKey: null,
        activate: false,
      }),
    )
  })

  it('requires Bedrock ambient metadata without rendering API-key UI in onboarding', async () => {
    const onUpsertProviderProfile = vi.fn(async (_request: UpsertProviderProfileRequestDto) => makeProviderProfiles())

    render(
      <ProvidersStep
        {...makeProvidersStepProps({
          onUpsertProviderProfile,
        })}
      />,
    )

    fireEvent.click(within(getProviderCard('Amazon Bedrock')).getByRole('button', { name: 'Cloud config' }))

    expect(
      screen.getByText('Cadence uses ambient desktop credentials for Amazon Bedrock. No app-local API key is stored for this provider profile.'),
    ).toBeVisible()
    expect(screen.queryByLabelText('API Key')).not.toBeInTheDocument()

    fireEvent.change(screen.getByLabelText('Region'), { target: { value: '' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(screen.getByText('Amazon Bedrock requires a region.')).toBeVisible()

    fireEvent.change(screen.getByLabelText('Region'), { target: { value: 'us-east-1' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() =>
      expect(onUpsertProviderProfile).toHaveBeenCalledWith({
        profileId: 'bedrock-default',
        providerId: 'bedrock',
        runtimeKind: 'anthropic',
        label: 'Amazon Bedrock',
        modelId: 'anthropic.claude-3-7-sonnet-20250219-v1:0',
        presetId: 'bedrock',
        baseUrl: null,
        apiVersion: null,
        region: 'us-east-1',
        projectId: null,
        apiKey: null,
        activate: false,
      }),
    )
  })

  it('requires Vertex ambient metadata without rendering API-key UI in onboarding', async () => {
    const onUpsertProviderProfile = vi.fn(async (_request: UpsertProviderProfileRequestDto) => makeProviderProfiles())

    render(
      <ProvidersStep
        {...makeProvidersStepProps({
          onUpsertProviderProfile,
        })}
      />,
    )

    fireEvent.click(within(getProviderCard('Google Vertex AI')).getByRole('button', { name: 'Cloud config' }))

    expect(
      screen.getByText('Cadence uses ambient desktop credentials for Google Vertex AI. No app-local API key is stored for this provider profile.'),
    ).toBeVisible()
    expect(screen.queryByLabelText('API Key')).not.toBeInTheDocument()

    fireEvent.change(screen.getByLabelText('Region'), { target: { value: '' } })
    fireEvent.change(screen.getByLabelText('Project ID'), { target: { value: '' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(screen.getByText('Google Vertex AI requires a region.')).toBeVisible()

    fireEvent.change(screen.getByLabelText('Region'), { target: { value: 'us-central1' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(screen.getByText('Google Vertex AI requires a project ID.')).toBeVisible()

    fireEvent.change(screen.getByLabelText('Project ID'), { target: { value: 'vertex-project' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() =>
      expect(onUpsertProviderProfile).toHaveBeenCalledWith({
        profileId: 'vertex-default',
        providerId: 'vertex',
        runtimeKind: 'anthropic',
        label: 'Google Vertex AI',
        modelId: 'claude-3-7-sonnet@20250219',
        presetId: 'vertex',
        baseUrl: null,
        apiVersion: null,
        region: 'us-central1',
        projectId: 'vertex-project',
        apiKey: null,
        activate: false,
      }),
    )
  })

  it('does not render provider selection controls or active badges', () => {
    const providerProfiles = makeProviderProfiles({
      activeProfileId: 'openai_codex-default',
      profiles: [makeOpenAiProfile({ active: true }), makeOpenRouterProfile({ active: false, migratedFromLegacy: false, migratedAt: null })],
    })

    render(
      <ProvidersStep
        {...makeProvidersStepProps({
          providerProfiles,
          onRefreshProviderProfiles: vi.fn(async () => providerProfiles),
          onUpsertProviderProfile: vi.fn(async (_request: UpsertProviderProfileRequestDto) => providerProfiles),
        })}
      />,
    )

    expect(screen.queryByRole('button', { name: 'Select' })).not.toBeInTheDocument()
    expect(screen.queryByText('Active')).not.toBeInTheDocument()
    expect(within(getProviderCard('OpenRouter')).getByRole('button', { name: 'API key' })).toBeVisible()
  })

  it('keeps OpenAI auth scoped to sign-in controls and disables them until a project is selected', () => {
    render(
      <ProvidersStep
        {...makeProvidersStepProps({
          providerProfiles: makeProviderProfiles({
            activeProfileId: 'zz-openai-alt',
            profiles: [
              makeOpenAiProfile({ active: false }),
              makeOpenAiProfile({
                profileId: 'zz-openai-alt',
                label: 'OpenAI Alt',
                active: true,
              }),
              makeOpenRouterProfile({ active: false, migratedFromLegacy: false, migratedAt: null }),
            ],
          }),
          runtimeSession: makeRuntimeSession(),
          hasSelectedProject: false,
          onStartLogin: vi.fn(async () => makeRuntimeSession()),
          onLogout: vi.fn(async () => makeRuntimeSession()),
        })}
      />,
    )

    const openAiCard = getProviderCard('OpenAI Codex')
    const openAiSignIn = within(openAiCard).getByRole('button', { name: 'Sign in' })
    expect(openAiSignIn).toBeVisible()
    expect(openAiSignIn).toBeDisabled()
    expect(screen.getAllByRole('button', { name: 'Sign in' })).toHaveLength(1)
    expect(screen.queryByText('OpenAI Alt')).not.toBeInTheDocument()
    expect(screen.queryByText('Choose a project next')).not.toBeInTheDocument()
    expect(within(openAiCard).queryByRole('button', { name: 'Select' })).not.toBeInTheDocument()
    expect(within(openAiCard).queryByRole('button', { name: 'API key' })).not.toBeInTheDocument()
  })

  it('keeps runtime mismatch recovery copy out of provider setup cards', () => {
    render(
      <ProvidersStep
        {...makeProvidersStepProps({
          providerProfiles: makeProviderProfiles({
            activeProfileId: 'openrouter-work',
            profiles: [
              makeOpenAiProfile({ active: false }),
              makeOpenRouterProfile({
                profileId: 'openrouter-work',
                label: 'OpenRouter Work',
                active: true,
                migratedFromLegacy: false,
                migratedAt: null,
              }),
            ],
          }),
          runtimeSession: makeRuntimeSession({
            providerId: 'openai_codex',
            runtimeKind: 'openai_codex',
            phase: 'authenticated',
            phaseLabel: 'Authenticated',
            runtimeLabel: 'OpenAI Codex · Signed in',
            accountLabel: 'operator',
            sessionLabel: 'session-1',
            sessionId: 'session-1',
            accountId: 'acct-1',
            isAuthenticated: true,
            isSignedOut: false,
          }),
          hasSelectedProject: true,
          onStartLogin: vi.fn(async () => makeRuntimeSession()),
          onLogout: vi.fn(async () => makeRuntimeSession()),
        })}
      />,
    )

    expect(
      screen.queryByText(
        'Configured provider profile OpenRouter Work (openrouter-work) no longer matches the persisted runtime session for OpenAI Codex.',
      ),
    ).not.toBeInTheDocument()
    expect(screen.queryByText('Rebind this profile so durable runtime truth matches Settings.')).not.toBeInTheDocument()
    expect(screen.getByText('OpenRouter Work')).toBeVisible()
    expect(screen.queryByText('Active')).not.toBeInTheDocument()
  })

  it('shows typed save errors while keeping the last truthful provider snapshot visible', () => {
    render(
      <ProvidersStep
        {...makeProvidersStepProps({
          providerProfilesSaveError: {
            code: 'provider_profiles_write_failed',
            message: 'Cadence could not save the provider profile.',
            retryable: true,
          },
        })}
      />,
    )

    expect(screen.getByText('Cadence could not save the provider profile.')).toBeVisible()
    expect(screen.getByText('OpenRouter')).toBeVisible()
    expect(screen.getByText('OpenAI Codex')).toBeVisible()
    expect(screen.getByText('Ready')).toBeVisible()
  })
})
