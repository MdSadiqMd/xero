import { describe, expect, it } from 'vitest'
import {
  createProviderSetupRecipeUpsertRequest,
  getProviderSetupRecipeMissingFields,
  listProviderSetupRecipes,
  providerSetupRecipeSchema,
  recommendProviderSetup,
} from './provider-setup'
import type { ProviderProfileDto, ProviderProfilesDto } from './provider-profiles'

function readyProof(proof: ProviderProfileDto['readiness']['proof']): ProviderProfileDto['readiness'] {
  return {
    ready: true,
    status: 'ready',
    proof,
    proofUpdatedAt: '2026-04-26T12:00:00Z',
  }
}

const missingReadiness: ProviderProfileDto['readiness'] = {
  ready: false,
  status: 'missing',
  proofUpdatedAt: null,
}

function profile(overrides: Partial<ProviderProfileDto>): ProviderProfileDto {
  return {
    profileId: 'openrouter-default',
    providerId: 'openrouter',
    runtimeKind: 'openrouter',
    label: 'OpenRouter',
    modelId: 'openai/gpt-4.1-mini',
    presetId: 'openrouter',
    active: false,
    readiness: missingReadiness,
    migratedFromLegacy: false,
    migratedAt: null,
    ...overrides,
  }
}

function profiles(
  entries: ProviderProfileDto[],
  activeProfileId = entries.find((entry) => entry.active)?.profileId ?? entries[0]?.profileId ?? 'openrouter-default',
): ProviderProfilesDto {
  return {
    activeProfileId,
    profiles: entries.map((entry) => ({
      ...entry,
      active: entry.profileId === activeProfileId,
    })),
    migration: null,
  }
}

describe('provider setup recipes', () => {
  it('validates OpenAI-compatible recipe metadata and generated upsert requests', () => {
    const recipes = listProviderSetupRecipes()
    expect(recipes.map((recipe) => recipe.recipeId)).toEqual([
      'litellm',
      'lm_studio',
      'groq',
      'together',
      'deepseek',
      'custom_openai_compatible',
    ])
    expect(recipes.every((recipe) => providerSetupRecipeSchema.safeParse(recipe).success)).toBe(true)

    expect(createProviderSetupRecipeUpsertRequest('groq', {
      apiKey: 'gsk-test',
      activate: true,
    })).toEqual({
      profileId: 'openai_api-groq',
      providerId: 'openai_api',
      runtimeKind: 'openai_compatible',
      label: 'Groq',
      modelId: 'llama-3.3-70b-versatile',
      presetId: 'openai_api',
      baseUrl: 'https://api.groq.com/openai/v1',
      apiVersion: null,
      region: null,
      projectId: null,
      apiKey: 'gsk-test',
      activate: true,
    })
  })

  it('reports recipe-required fields and supports local endpoints without fake keys', () => {
    expect(getProviderSetupRecipeMissingFields('custom_openai_compatible', {})).toEqual([
      {
        field: 'baseUrl',
        label: 'Base URL',
        message: 'Custom /v1 gateway requires Base URL.',
      },
      {
        field: 'apiKey',
        label: 'API key',
        message: 'Custom /v1 gateway requires API key.',
      },
    ])

    expect(createProviderSetupRecipeUpsertRequest('lm_studio')).toMatchObject({
      providerId: 'openai_api',
      label: 'LM Studio',
      baseUrl: 'http://127.0.0.1:1234/v1',
      apiKey: null,
    })

    expect(createProviderSetupRecipeUpsertRequest('litellm')).toMatchObject({
      providerId: 'openai_api',
      label: 'LiteLLM',
      baseUrl: 'http://127.0.0.1:4000/v1',
      apiKey: null,
    })
  })
})

describe('provider setup recommendation logic', () => {
  it('recommends a recipe when no profiles are available', () => {
    const recommendation = recommendProviderSetup(null)
    expect(recommendation.primary).toMatchObject({
      kind: 'missing_key_cloud_profile',
      action: 'apply_recipe',
      providerId: 'openai_api',
      recipeId: 'custom_openai_compatible',
    })
  })

  it('recommends ready OpenAI Codex and OpenRouter profiles as fastest paths', () => {
    expect(recommendProviderSetup(profiles([
      profile({
        profileId: 'openai_codex-default',
        providerId: 'openai_codex',
        runtimeKind: 'openai_codex',
        label: 'OpenAI Codex',
        modelId: 'openai_codex',
        presetId: null,
        readiness: readyProof('oauth_session'),
      }),
    ])).primary).toMatchObject({
      kind: 'fastest_ready_profile',
      profileId: 'openai_codex-default',
      action: 'edit_profile',
    })

    expect(recommendProviderSetup(profiles([
      profile({
        readiness: readyProof('stored_secret'),
      }),
    ])).primary).toMatchObject({
      kind: 'fastest_ready_profile',
      profileId: 'openrouter-default',
    })
  })

  it('distinguishes local Ollama and local OpenAI-compatible profiles', () => {
    expect(recommendProviderSetup(profiles([
      profile({
        profileId: 'ollama-default',
        providerId: 'ollama',
        runtimeKind: 'openai_compatible',
        label: 'Ollama',
        modelId: 'llama3.2',
        presetId: 'ollama',
        baseUrl: 'http://127.0.0.1:11434/v1',
        readiness: readyProof('local'),
      }),
    ])).primary).toMatchObject({
      kind: 'best_local_profile',
      profileId: 'ollama-default',
    })

    expect(recommendProviderSetup(profiles([
      profile({
        profileId: 'openai_api-local',
        providerId: 'openai_api',
        runtimeKind: 'openai_compatible',
        label: 'LM Studio',
        modelId: 'local-model',
        presetId: 'openai_api',
        baseUrl: 'http://localhost:1234/v1',
        readiness: readyProof('local'),
      }),
    ])).primary).toMatchObject({
      kind: 'best_local_profile',
      profileId: 'openai_api-local',
    })
  })

  it('handles ambient Bedrock and Vertex profiles plus incomplete profiles', () => {
    expect(recommendProviderSetup(profiles([
      profile({
        profileId: 'bedrock-default',
        providerId: 'bedrock',
        runtimeKind: 'anthropic',
        label: 'Amazon Bedrock',
        modelId: 'anthropic.claude-3-7-sonnet-20250219-v1:0',
        presetId: 'bedrock',
        region: 'us-east-1',
        readiness: readyProof('ambient'),
      }),
    ])).primary).toMatchObject({
      kind: 'fastest_ready_profile',
      profileId: 'bedrock-default',
    })

    expect(recommendProviderSetup(profiles([
      profile({
        profileId: 'vertex-default',
        providerId: 'vertex',
        runtimeKind: 'anthropic',
        label: 'Google Vertex AI',
        modelId: 'claude-3-7-sonnet@20250219',
        presetId: 'vertex',
        region: 'us-central1',
        projectId: 'cadence-project',
        readiness: {
          ready: false,
          status: 'malformed',
          proofUpdatedAt: '2026-04-26T12:00:00Z',
        },
      }),
    ])).primary).toMatchObject({
      kind: 'unsupported_incomplete_profile',
      profileId: 'vertex-default',
    })
  })

  it('keeps the active ready profile primary when several ready profiles compete', () => {
    const recommendation = recommendProviderSetup(
      profiles(
        [
          profile({
            readiness: readyProof('stored_secret'),
          }),
          profile({
            profileId: 'ollama-default',
            providerId: 'ollama',
            runtimeKind: 'openai_compatible',
            label: 'Ollama',
            modelId: 'llama3.2',
            presetId: 'ollama',
            baseUrl: 'http://127.0.0.1:11434/v1',
            readiness: readyProof('local'),
          }),
        ],
        'openrouter-default',
      ),
    )

    expect(recommendation.primary).toMatchObject({
      kind: 'fastest_ready_profile',
      profileId: 'openrouter-default',
    })
    expect(recommendation.alternatives).toEqual([
      expect.objectContaining({
        kind: 'best_local_profile',
        profileId: 'ollama-default',
      }),
    ])
  })
})
