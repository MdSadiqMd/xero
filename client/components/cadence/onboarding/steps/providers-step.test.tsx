import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}))

import { ProvidersStep } from '@/components/cadence/onboarding/steps/providers-step'
import type {
  ProviderProfileDto,
  ProviderProfilesDto,
  RuntimeSessionView,
  UpsertProviderProfileRequestDto,
} from '@/src/lib/cadence-model'

function makeOpenAiProfile(overrides: Partial<ProviderProfileDto> = {}): ProviderProfileDto {
  return {
    profileId: 'openai_codex-default',
    providerId: 'openai_codex',
    label: 'OpenAI Codex',
    modelId: 'openai_codex',
    active: false,
    readiness: {
      ready: false,
      status: 'missing',
      credentialUpdatedAt: null,
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
    label: 'OpenRouter',
    modelId: 'openai/gpt-4.1-mini',
    active: true,
    readiness: ready
      ? {
          ready: true,
          status: 'ready',
          credentialUpdatedAt: '2026-04-20T00:00:00Z',
        }
      : {
          ready: false,
          status: 'missing',
          credentialUpdatedAt: null,
        },
    migratedFromLegacy: true,
    migratedAt: '2026-04-20T00:00:00Z',
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

describe('ProvidersStep', () => {
  it('renders migrated active profiles, keeps saved keys blank, and validates label/model edits', async () => {
    const onUpsertProviderProfile = vi.fn(async (_request: UpsertProviderProfileRequestDto) => makeProviderProfiles())

    render(
      <ProvidersStep
        providerProfiles={makeProviderProfiles()}
        providerProfilesLoadStatus="ready"
        providerProfilesLoadError={null}
        providerProfilesSaveStatus="idle"
        providerProfilesSaveError={null}
        onRefreshProviderProfiles={vi.fn(async () => makeProviderProfiles())}
        onUpsertProviderProfile={onUpsertProviderProfile}
        onSetActiveProviderProfile={vi.fn(async (_profileId: string) => makeProviderProfiles())}
      />,
    )

    expect(screen.getByText('Active profile')).toBeVisible()
    expect(screen.getByText('Ready')).toBeVisible()
    expect(screen.getByText('Migrated')).toBeVisible()
    expect(screen.getByText('Migrated 2026-04-20T00:00:00Z')).toBeVisible()
    expect(screen.getAllByText('Unavailable')).toHaveLength(2)

    fireEvent.click(screen.getByRole('button', { name: 'Edit setup' }))

    const labelInput = screen.getByLabelText('Profile label') as HTMLInputElement
    const modelInput = screen.getByLabelText('Model ID') as HTMLInputElement
    const keyInput = screen.getByLabelText('API Key') as HTMLInputElement

    expect(labelInput).toHaveValue('OpenRouter')
    expect(modelInput).toHaveValue('openai/gpt-4.1-mini')
    expect(keyInput).toHaveValue('')

    fireEvent.change(labelInput, { target: { value: '   ' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(screen.getByText('Profile label is required.')).toBeVisible()

    fireEvent.change(labelInput, { target: { value: 'Team OpenRouter' } })
    fireEvent.change(modelInput, { target: { value: '' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))
    expect(screen.getByText('Model ID is required.')).toBeVisible()

    fireEvent.change(modelInput, { target: { value: 'openrouter/anthropic/claude-3.5-sonnet' } })
    fireEvent.click(screen.getByRole('button', { name: 'Save' }))

    await waitFor(() =>
      expect(onUpsertProviderProfile).toHaveBeenCalledWith({
        profileId: 'openrouter-default',
        providerId: 'openrouter',
        label: 'Team OpenRouter',
        modelId: 'openrouter/anthropic/claude-3.5-sonnet',
        activate: true,
      }),
    )
  })

  it('switches active profile truth without leaving stale active badges behind', async () => {
    let providerProfiles = makeProviderProfiles({
      activeProfileId: 'openai_codex-default',
      profiles: [makeOpenAiProfile({ active: true }), makeOpenRouterProfile({ active: false, migratedFromLegacy: false, migratedAt: null })],
    })

    const onSetActiveProviderProfile = vi.fn(async (_profileId: string) => {
      providerProfiles = makeProviderProfiles({
        activeProfileId: 'openrouter-default',
        profiles: [makeOpenAiProfile({ active: false }), makeOpenRouterProfile({ active: true, migratedFromLegacy: false, migratedAt: null })],
      })
      return providerProfiles
    })

    const { rerender } = render(
      <ProvidersStep
        providerProfiles={providerProfiles}
        providerProfilesLoadStatus="ready"
        providerProfilesLoadError={null}
        providerProfilesSaveStatus="idle"
        providerProfilesSaveError={null}
        onRefreshProviderProfiles={vi.fn(async () => providerProfiles)}
        onUpsertProviderProfile={vi.fn(async (_request: UpsertProviderProfileRequestDto) => providerProfiles)}
        onSetActiveProviderProfile={onSetActiveProviderProfile}
      />,
    )

    fireEvent.click(screen.getByRole('button', { name: 'Use this profile' }))
    await waitFor(() => expect(onSetActiveProviderProfile).toHaveBeenCalledWith('openrouter-default'))

    rerender(
      <ProvidersStep
        providerProfiles={providerProfiles}
        providerProfilesLoadStatus="ready"
        providerProfilesLoadError={null}
        providerProfilesSaveStatus="idle"
        providerProfilesSaveError={null}
        onRefreshProviderProfiles={vi.fn(async () => providerProfiles)}
        onUpsertProviderProfile={vi.fn(async (_request: UpsertProviderProfileRequestDto) => providerProfiles)}
        onSetActiveProviderProfile={onSetActiveProviderProfile}
      />,
    )

    expect(screen.getAllByText('Active profile')).toHaveLength(1)
    expect(screen.getByText('Using this')).toBeVisible()
  })

  it('scopes OpenAI auth copy to the selected profile and uses onboarding project guidance when no project is selected', () => {
    render(
      <ProvidersStep
        providerProfiles={makeProviderProfiles({
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
        })}
        providerProfilesLoadStatus="ready"
        providerProfilesLoadError={null}
        providerProfilesSaveStatus="idle"
        providerProfilesSaveError={null}
        runtimeSession={makeRuntimeSession()}
        hasSelectedProject={false}
        onRefreshProviderProfiles={vi.fn(async () => makeProviderProfiles())}
        onUpsertProviderProfile={vi.fn(async (_request: UpsertProviderProfileRequestDto) => makeProviderProfiles())}
        onSetActiveProviderProfile={vi.fn(async (_profileId: string) => makeProviderProfiles())}
        onStartLogin={vi.fn(async () => makeRuntimeSession())}
        onLogout={vi.fn(async () => makeRuntimeSession())}
      />,
    )

    expect(screen.getByText('Choose a project next')).toBeVisible()
    expect(
      screen.getByText('After you choose a project, Cadence can sign in the selected OpenAI profile.'),
    ).toBeVisible()
    expect(
      screen.getByText(
        'OpenAI sign-in only runs against the selected profile OpenAI Alt (zz-openai-alt). Select this profile first to manage auth.',
      ),
    ).toBeVisible()
    expect(screen.queryByRole('button', { name: 'Sign in' })).not.toBeInTheDocument()
  })

  it('shows the shared selected-profile mismatch recovery copy without forking onboarding provider logic', () => {
    render(
      <ProvidersStep
        providerProfiles={makeProviderProfiles({
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
        })}
        providerProfilesLoadStatus="ready"
        providerProfilesLoadError={null}
        providerProfilesSaveStatus="idle"
        providerProfilesSaveError={null}
        runtimeSession={makeRuntimeSession({
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
        })}
        hasSelectedProject
        onRefreshProviderProfiles={vi.fn(async () => makeProviderProfiles())}
        onUpsertProviderProfile={vi.fn(async (_request: UpsertProviderProfileRequestDto) => makeProviderProfiles())}
        onSetActiveProviderProfile={vi.fn(async (_profileId: string) => makeProviderProfiles())}
        onStartLogin={vi.fn(async () => makeRuntimeSession())}
        onLogout={vi.fn(async () => makeRuntimeSession())}
      />,
    )

    expect(
      screen.getByText(
        'Settings now select provider profile OpenRouter Work (openrouter-work), but the persisted runtime session still reflects OpenAI Codex.',
      ),
    ).toBeVisible()
    expect(
      screen.getByText('Rebind the selected profile so durable runtime truth matches Settings.'),
    ).toBeVisible()
    expect(screen.getByText('OpenRouter Work')).toBeVisible()
    expect(screen.getByText('Using this')).toBeVisible()
  })

  it('shows typed save errors while keeping the last truthful provider snapshot visible', () => {
    render(
      <ProvidersStep
        providerProfiles={makeProviderProfiles()}
        providerProfilesLoadStatus="ready"
        providerProfilesLoadError={null}
        providerProfilesSaveStatus="idle"
        providerProfilesSaveError={{
          code: 'provider_profiles_write_failed',
          message: 'Cadence could not save the selected provider profile.',
          retryable: true,
        }}
        onRefreshProviderProfiles={vi.fn(async () => makeProviderProfiles())}
        onUpsertProviderProfile={vi.fn(async (_request: UpsertProviderProfileRequestDto) => makeProviderProfiles())}
        onSetActiveProviderProfile={vi.fn(async (_profileId: string) => makeProviderProfiles())}
      />,
    )

    expect(screen.getByText('Cadence could not save the selected provider profile.')).toBeVisible()
    expect(screen.getByText('OpenRouter')).toBeVisible()
    expect(screen.getByText('OpenAI Codex')).toBeVisible()
    expect(screen.getByText('Ready')).toBeVisible()
  })
})
