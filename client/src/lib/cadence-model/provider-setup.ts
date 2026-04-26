import { z } from 'zod'
import {
  type ProviderProfileDto,
  type ProviderProfilesDto,
  type UpsertProviderProfileRequestDto,
  upsertProviderProfileRequestSchema,
} from './provider-profiles'
import { getCloudProviderLabel, isApiKeyCloudProvider, isLocalCloudProvider } from './provider-presets'
import { runtimeProviderIdSchema } from './runtime'

export const providerSetupRecipeIdSchema = z.enum([
  'litellm',
  'lm_studio',
  'mistral',
  'groq',
  'together',
  'deepseek',
  'nvidia_nim',
  'minimax',
  'azure_ai_foundry',
  'atomic_chat',
  'custom_openai_compatible',
])

export const providerSetupRecipeAuthModeSchema = z.enum(['api_key', 'local'])
export const providerSetupRecipeApiKeyModeSchema = z.enum(['required', 'optional', 'none'])
export const providerSetupRecipeRequiredFieldSchema = z.enum(['baseUrl', 'apiKey', 'modelId'])
export const providerSetupRecipeModelCatalogExpectationSchema = z.enum([
  'live',
  'manual',
  'live_or_manual',
])

export const providerSetupRecipeSchema = z
  .object({
    recipeId: providerSetupRecipeIdSchema,
    label: z.string().trim().min(1),
    description: z.string().trim().min(1),
    providerId: z.literal('openai_api'),
    runtimeKind: z.literal('openai_compatible'),
    presetId: z.literal('openai_api'),
    defaultProfileLabel: z.string().trim().min(1),
    defaultModelId: z.string().trim().min(1),
    defaultBaseUrl: z.string().url().nullable(),
    defaultApiVersion: z.string().trim().min(1).nullable(),
    authMode: providerSetupRecipeAuthModeSchema,
    apiKeyMode: providerSetupRecipeApiKeyModeSchema,
    baseUrlRequired: z.boolean(),
    requiredFields: z.array(providerSetupRecipeRequiredFieldSchema),
    modelCatalogExpectation: providerSetupRecipeModelCatalogExpectationSchema,
    baseUrlPlaceholder: z.string().trim().min(1),
    apiKeyPlaceholder: z.string().trim().min(1),
    modelPlaceholder: z.string().trim().min(1),
    guidance: z.string().trim().min(1),
    repairSuggestion: z.string().trim().min(1),
  })
  .strict()
  .superRefine((recipe, ctx) => {
    const requiredFields = new Set(recipe.requiredFields)
    if (requiredFields.size !== recipe.requiredFields.length) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['requiredFields'],
        message: 'Provider setup recipe required fields must be unique.',
      })
    }

    if (recipe.baseUrlRequired && !requiredFields.has('baseUrl')) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['requiredFields'],
        message: 'Provider setup recipes with baseUrlRequired=true must require baseUrl.',
      })
    }

    if (!recipe.baseUrlRequired && requiredFields.has('baseUrl')) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['requiredFields'],
        message: 'Provider setup recipes must not require baseUrl when baseUrlRequired=false.',
      })
    }

    if (recipe.authMode === 'local' && recipe.apiKeyMode !== 'none') {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['apiKeyMode'],
        message: 'Local provider setup recipes must not require or retain API keys.',
      })
    }

    if (recipe.apiKeyMode === 'required' && !requiredFields.has('apiKey')) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['requiredFields'],
        message: 'API-key provider setup recipes must require apiKey.',
      })
    }

    if (recipe.apiKeyMode !== 'required' && requiredFields.has('apiKey')) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['requiredFields'],
        message: 'Provider setup recipes must only require apiKey when apiKeyMode=required.',
      })
    }
  })

export const providerSetupRecipeFieldPromptSchema = z
  .object({
    field: providerSetupRecipeRequiredFieldSchema,
    label: z.string().trim().min(1),
    message: z.string().trim().min(1),
  })
  .strict()

export const providerRecommendationKindSchema = z.enum([
  'fastest_ready_profile',
  'best_local_profile',
  'missing_key_cloud_profile',
  'unsupported_incomplete_profile',
])
export const providerRecommendationActionSchema = z.enum([
  'activate_profile',
  'edit_profile',
  'apply_recipe',
])
export const providerRecommendationSchema = z
  .object({
    kind: providerRecommendationKindSchema,
    title: z.string().trim().min(1),
    message: z.string().trim().min(1),
    action: providerRecommendationActionSchema,
    actionLabel: z.string().trim().min(1),
    profileId: z.string().trim().min(1).nullable(),
    providerId: runtimeProviderIdSchema.nullable(),
    recipeId: providerSetupRecipeIdSchema.nullable(),
    priority: z.number().int(),
  })
  .strict()
  .superRefine((recommendation, ctx) => {
    if (recommendation.action === 'apply_recipe' && !recommendation.recipeId) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['recipeId'],
        message: 'Recipe recommendations must include recipeId.',
      })
    }

    if (recommendation.action !== 'apply_recipe' && !recommendation.profileId) {
      ctx.addIssue({
        code: z.ZodIssueCode.custom,
        path: ['profileId'],
        message: 'Profile recommendations must include profileId.',
      })
    }
  })

export const providerRecommendationSetSchema = z
  .object({
    primary: providerRecommendationSchema.nullable(),
    alternatives: z.array(providerRecommendationSchema),
  })
  .strict()

export type ProviderSetupRecipeIdDto = z.infer<typeof providerSetupRecipeIdSchema>
export type ProviderSetupRecipeAuthModeDto = z.infer<typeof providerSetupRecipeAuthModeSchema>
export type ProviderSetupRecipeApiKeyModeDto = z.infer<typeof providerSetupRecipeApiKeyModeSchema>
export type ProviderSetupRecipeRequiredFieldDto = z.infer<typeof providerSetupRecipeRequiredFieldSchema>
export type ProviderSetupRecipeModelCatalogExpectationDto = z.infer<typeof providerSetupRecipeModelCatalogExpectationSchema>
export type ProviderSetupRecipeDto = z.infer<typeof providerSetupRecipeSchema>
export type ProviderSetupRecipeFieldPromptDto = z.infer<typeof providerSetupRecipeFieldPromptSchema>
export type ProviderRecommendationKindDto = z.infer<typeof providerRecommendationKindSchema>
export type ProviderRecommendationActionDto = z.infer<typeof providerRecommendationActionSchema>
export type ProviderRecommendationDto = z.infer<typeof providerRecommendationSchema>
export type ProviderRecommendationSetDto = z.infer<typeof providerRecommendationSetSchema>

export interface ProviderSetupRecipeDraftDefaults {
  label: string
  modelId: string
  baseUrl: string
  apiVersion: string
}

export interface ProviderSetupRecipeInput {
  profileId?: string | null
  label?: string | null
  modelId?: string | null
  baseUrl?: string | null
  apiVersion?: string | null
  apiKey?: string | null
  activate?: boolean
}

const OPENAI_COMPATIBLE_SETUP_RECIPES: ProviderSetupRecipeDto[] = [
  {
    recipeId: 'litellm',
    label: 'LiteLLM proxy',
    description: 'Connect to a LiteLLM proxy that exposes an OpenAI-compatible /v1 endpoint.',
    providerId: 'openai_api',
    runtimeKind: 'openai_compatible',
    presetId: 'openai_api',
    defaultProfileLabel: 'LiteLLM',
    defaultModelId: 'gpt-4.1-mini',
    defaultBaseUrl: 'http://127.0.0.1:4000/v1',
    defaultApiVersion: null,
    authMode: 'api_key',
    apiKeyMode: 'optional',
    baseUrlRequired: true,
    requiredFields: ['baseUrl', 'modelId'],
    modelCatalogExpectation: 'live_or_manual',
    baseUrlPlaceholder: 'http://127.0.0.1:4000/v1',
    apiKeyPlaceholder: 'Optional LiteLLM virtual key',
    modelPlaceholder: 'gpt-4.1-mini',
    guidance: 'Use the LiteLLM proxy base URL and the model alias configured in your LiteLLM config.',
    repairSuggestion: 'Start the LiteLLM proxy, confirm the /v1 endpoint is reachable, and add the proxy key if your gateway requires one.',
  },
  {
    recipeId: 'lm_studio',
    label: 'LM Studio',
    description: 'Connect to the local LM Studio server without storing a placeholder API key.',
    providerId: 'openai_api',
    runtimeKind: 'openai_compatible',
    presetId: 'openai_api',
    defaultProfileLabel: 'LM Studio',
    defaultModelId: 'local-model',
    defaultBaseUrl: 'http://127.0.0.1:1234/v1',
    defaultApiVersion: null,
    authMode: 'local',
    apiKeyMode: 'none',
    baseUrlRequired: true,
    requiredFields: ['baseUrl', 'modelId'],
    modelCatalogExpectation: 'live_or_manual',
    baseUrlPlaceholder: 'http://127.0.0.1:1234/v1',
    apiKeyPlaceholder: 'No API key',
    modelPlaceholder: 'local-model',
    guidance: 'Start the LM Studio local server and choose the loaded model id.',
    repairSuggestion: 'Start the LM Studio server, load a model, and check that the local /v1 endpoint answers model requests.',
  },
  {
    recipeId: 'mistral',
    label: 'Mistral',
    description: 'Connect Mistral through its OpenAI-compatible /v1 chat-completions surface.',
    providerId: 'openai_api',
    runtimeKind: 'openai_compatible',
    presetId: 'openai_api',
    defaultProfileLabel: 'Mistral',
    defaultModelId: 'mistral-large-latest',
    defaultBaseUrl: 'https://api.mistral.ai/v1',
    defaultApiVersion: null,
    authMode: 'api_key',
    apiKeyMode: 'required',
    baseUrlRequired: true,
    requiredFields: ['baseUrl', 'apiKey', 'modelId'],
    modelCatalogExpectation: 'live_or_manual',
    baseUrlPlaceholder: 'https://api.mistral.ai/v1',
    apiKeyPlaceholder: 'Paste Mistral API key',
    modelPlaceholder: 'mistral-large-latest',
    guidance: 'Use the Mistral API base URL and a Mistral model id such as mistral-large-latest.',
    repairSuggestion: 'Verify the Mistral API key, base URL, and model id, then check the connection again.',
  },
  {
    recipeId: 'groq',
    label: 'Groq',
    description: 'Connect Groq through its OpenAI-compatible endpoint and a saved app-local API key.',
    providerId: 'openai_api',
    runtimeKind: 'openai_compatible',
    presetId: 'openai_api',
    defaultProfileLabel: 'Groq',
    defaultModelId: 'llama-3.3-70b-versatile',
    defaultBaseUrl: 'https://api.groq.com/openai/v1',
    defaultApiVersion: null,
    authMode: 'api_key',
    apiKeyMode: 'required',
    baseUrlRequired: true,
    requiredFields: ['baseUrl', 'apiKey', 'modelId'],
    modelCatalogExpectation: 'live_or_manual',
    baseUrlPlaceholder: 'https://api.groq.com/openai/v1',
    apiKeyPlaceholder: 'Paste Groq API key',
    modelPlaceholder: 'llama-3.3-70b-versatile',
    guidance: 'Use a Groq API key and a Groq model id supported by the OpenAI-compatible endpoint.',
    repairSuggestion: 'Verify the Groq API key, endpoint URL, and selected model id, then check the connection again.',
  },
  {
    recipeId: 'together',
    label: 'Together AI',
    description: 'Connect Together AI through its OpenAI-compatible /v1 gateway.',
    providerId: 'openai_api',
    runtimeKind: 'openai_compatible',
    presetId: 'openai_api',
    defaultProfileLabel: 'Together AI',
    defaultModelId: 'meta-llama/Llama-3.3-70B-Instruct-Turbo',
    defaultBaseUrl: 'https://api.together.xyz/v1',
    defaultApiVersion: null,
    authMode: 'api_key',
    apiKeyMode: 'required',
    baseUrlRequired: true,
    requiredFields: ['baseUrl', 'apiKey', 'modelId'],
    modelCatalogExpectation: 'live_or_manual',
    baseUrlPlaceholder: 'https://api.together.xyz/v1',
    apiKeyPlaceholder: 'Paste Together API key',
    modelPlaceholder: 'meta-llama/Llama-3.3-70B-Instruct-Turbo',
    guidance: 'Use the Together /v1 endpoint with the model id from your Together account.',
    repairSuggestion: 'Confirm the Together API key has inference access and that the selected model id is enabled for your account.',
  },
  {
    recipeId: 'deepseek',
    label: 'DeepSeek',
    description: 'Connect DeepSeek through its OpenAI-compatible API surface.',
    providerId: 'openai_api',
    runtimeKind: 'openai_compatible',
    presetId: 'openai_api',
    defaultProfileLabel: 'DeepSeek',
    defaultModelId: 'deepseek-chat',
    defaultBaseUrl: 'https://api.deepseek.com/v1',
    defaultApiVersion: null,
    authMode: 'api_key',
    apiKeyMode: 'required',
    baseUrlRequired: true,
    requiredFields: ['baseUrl', 'apiKey', 'modelId'],
    modelCatalogExpectation: 'live_or_manual',
    baseUrlPlaceholder: 'https://api.deepseek.com/v1',
    apiKeyPlaceholder: 'Paste DeepSeek API key',
    modelPlaceholder: 'deepseek-chat',
    guidance: 'Use the DeepSeek /v1 endpoint and a DeepSeek model id such as deepseek-chat.',
    repairSuggestion: 'Verify the DeepSeek API key, endpoint URL, and model id, then check the connection again.',
  },
  {
    recipeId: 'nvidia_nim',
    label: 'NVIDIA NIM',
    description: 'Connect NVIDIA-hosted NIM endpoints through the OpenAI-compatible API Catalog route.',
    providerId: 'openai_api',
    runtimeKind: 'openai_compatible',
    presetId: 'openai_api',
    defaultProfileLabel: 'NVIDIA NIM',
    defaultModelId: 'meta/llama-3.1-70b-instruct',
    defaultBaseUrl: 'https://integrate.api.nvidia.com/v1',
    defaultApiVersion: null,
    authMode: 'api_key',
    apiKeyMode: 'required',
    baseUrlRequired: true,
    requiredFields: ['baseUrl', 'apiKey', 'modelId'],
    modelCatalogExpectation: 'live_or_manual',
    baseUrlPlaceholder: 'https://integrate.api.nvidia.com/v1',
    apiKeyPlaceholder: 'Paste NVIDIA API key',
    modelPlaceholder: 'meta/llama-3.1-70b-instruct',
    guidance: 'Use the NVIDIA API Catalog /v1 endpoint with a model id available to your NVIDIA account.',
    repairSuggestion: 'Confirm the NVIDIA API key is valid, the model id is enabled, and the NIM endpoint can answer /models or use the manual model id.',
  },
  {
    recipeId: 'minimax',
    label: 'MiniMax',
    description: 'Connect MiniMax through its Compatible OpenAI API endpoint.',
    providerId: 'openai_api',
    runtimeKind: 'openai_compatible',
    presetId: 'openai_api',
    defaultProfileLabel: 'MiniMax',
    defaultModelId: 'MiniMax-M2.7',
    defaultBaseUrl: 'https://api.minimax.io/v1',
    defaultApiVersion: null,
    authMode: 'api_key',
    apiKeyMode: 'required',
    baseUrlRequired: true,
    requiredFields: ['baseUrl', 'apiKey', 'modelId'],
    modelCatalogExpectation: 'live_or_manual',
    baseUrlPlaceholder: 'https://api.minimax.io/v1',
    apiKeyPlaceholder: 'Paste MiniMax API key',
    modelPlaceholder: 'MiniMax-M2.7',
    guidance: 'Use the MiniMax OpenAI-compatible base URL and the model id shown in your MiniMax account.',
    repairSuggestion: 'Verify the MiniMax API key, endpoint URL, and selected model id, then check the connection again.',
  },
  {
    recipeId: 'azure_ai_foundry',
    label: 'Azure AI Foundry',
    description: 'Connect Azure AI Foundry deployments exposed through the OpenAI-compatible endpoint route.',
    providerId: 'openai_api',
    runtimeKind: 'openai_compatible',
    presetId: 'openai_api',
    defaultProfileLabel: 'Azure AI Foundry',
    defaultModelId: 'deployment-name',
    defaultBaseUrl: null,
    defaultApiVersion: null,
    authMode: 'api_key',
    apiKeyMode: 'required',
    baseUrlRequired: true,
    requiredFields: ['baseUrl', 'apiKey', 'modelId'],
    modelCatalogExpectation: 'manual',
    baseUrlPlaceholder: 'https://<resource>.openai.azure.com/openai/v1',
    apiKeyPlaceholder: 'Paste Azure AI Foundry key',
    modelPlaceholder: 'deployment-name',
    guidance: 'Use the Azure AI Foundry OpenAI-compatible endpoint and set Model to the deployment name.',
    repairSuggestion: 'Confirm the Foundry endpoint route, deployment name, and API key. Use the Azure OpenAI preset instead for deployment URLs that require api-version metadata.',
  },
  {
    recipeId: 'atomic_chat',
    label: 'Atomic Chat local',
    description: 'Connect Atomic Chat as a local OpenAI-compatible model backend without storing a placeholder key.',
    providerId: 'openai_api',
    runtimeKind: 'openai_compatible',
    presetId: 'openai_api',
    defaultProfileLabel: 'Atomic Chat',
    defaultModelId: 'local-model',
    defaultBaseUrl: 'http://127.0.0.1:1337/v1',
    defaultApiVersion: null,
    authMode: 'local',
    apiKeyMode: 'none',
    baseUrlRequired: true,
    requiredFields: ['baseUrl', 'modelId'],
    modelCatalogExpectation: 'live_or_manual',
    baseUrlPlaceholder: 'http://127.0.0.1:1337/v1',
    apiKeyPlaceholder: 'No API key',
    modelPlaceholder: 'local-model',
    guidance: 'Start Atomic Chat with its local server enabled, then choose the loaded model id.',
    repairSuggestion: 'Start Atomic Chat, enable its local OpenAI-compatible server, and update the base URL if your server uses a different port.',
  },
  {
    recipeId: 'custom_openai_compatible',
    label: 'Custom /v1 gateway',
    description: 'Connect any OpenAI-compatible HTTP gateway by supplying the base URL and model id.',
    providerId: 'openai_api',
    runtimeKind: 'openai_compatible',
    presetId: 'openai_api',
    defaultProfileLabel: 'Custom OpenAI-compatible',
    defaultModelId: 'provider/model-id',
    defaultBaseUrl: null,
    defaultApiVersion: null,
    authMode: 'api_key',
    apiKeyMode: 'required',
    baseUrlRequired: true,
    requiredFields: ['baseUrl', 'apiKey', 'modelId'],
    modelCatalogExpectation: 'live_or_manual',
    baseUrlPlaceholder: 'https://gateway.example.com/v1',
    apiKeyPlaceholder: 'Paste gateway API key',
    modelPlaceholder: 'provider/model-id',
    guidance: 'Use the gateway root ending in /v1 and the model id that gateway expects.',
    repairSuggestion: 'Confirm the gateway exposes an OpenAI-compatible /v1 API, then check the saved endpoint and key.',
  },
].map((recipe) => providerSetupRecipeSchema.parse(recipe))

const OPENAI_COMPATIBLE_SETUP_RECIPE_BY_ID = new Map(
  OPENAI_COMPATIBLE_SETUP_RECIPES.map((recipe) => [recipe.recipeId, recipe]),
)

export function listProviderSetupRecipes(): ProviderSetupRecipeDto[] {
  return OPENAI_COMPATIBLE_SETUP_RECIPES
}

export function getProviderSetupRecipe(
  recipeId: ProviderSetupRecipeIdDto | string | null | undefined,
): ProviderSetupRecipeDto | null {
  if (typeof recipeId !== 'string') {
    return null
  }

  return OPENAI_COMPATIBLE_SETUP_RECIPE_BY_ID.get(recipeId as ProviderSetupRecipeIdDto) ?? null
}

export function getProviderSetupRecipeDraftDefaults(
  recipeId: ProviderSetupRecipeIdDto | string,
): ProviderSetupRecipeDraftDefaults {
  const recipe = requireProviderSetupRecipe(recipeId)
  return {
    label: recipe.defaultProfileLabel,
    modelId: recipe.defaultModelId,
    baseUrl: recipe.defaultBaseUrl ?? '',
    apiVersion: recipe.defaultApiVersion ?? '',
  }
}

export function getProviderSetupRecipeMissingFields(
  recipeId: ProviderSetupRecipeIdDto | string,
  input: ProviderSetupRecipeInput,
): ProviderSetupRecipeFieldPromptDto[] {
  const recipe = requireProviderSetupRecipe(recipeId)
  const prompts: ProviderSetupRecipeFieldPromptDto[] = []

  if (recipe.requiredFields.includes('baseUrl') && !normalizeOptionalText(input.baseUrl ?? recipe.defaultBaseUrl)) {
    prompts.push(recipeFieldPrompt('baseUrl', recipe.label, 'Base URL'))
  }

  if (recipe.requiredFields.includes('apiKey') && !normalizeOptionalText(input.apiKey)) {
    prompts.push(recipeFieldPrompt('apiKey', recipe.label, 'API key'))
  }

  if (recipe.requiredFields.includes('modelId') && !normalizeOptionalText(input.modelId ?? recipe.defaultModelId)) {
    prompts.push(recipeFieldPrompt('modelId', recipe.label, 'Model ID'))
  }

  return prompts.map((prompt) => providerSetupRecipeFieldPromptSchema.parse(prompt))
}

export function createProviderSetupRecipeUpsertRequest(
  recipeId: ProviderSetupRecipeIdDto | string,
  input: ProviderSetupRecipeInput = {},
): UpsertProviderProfileRequestDto {
  const recipe = requireProviderSetupRecipe(recipeId)
  const missing = getProviderSetupRecipeMissingFields(recipe.recipeId, input)
  if (missing.length > 0) {
    throw new Error(missing.map((field) => field.message).join(' '))
  }

  const baseUrl = normalizeOptionalText(input.baseUrl ?? recipe.defaultBaseUrl)
  const apiVersion = normalizeOptionalText(input.apiVersion ?? recipe.defaultApiVersion)
  const apiKey =
    recipe.apiKeyMode === 'none'
      ? null
      : normalizeOptionalText(input.apiKey)

  return upsertProviderProfileRequestSchema.parse({
    profileId: normalizeOptionalText(input.profileId) ?? `openai_api-${recipe.recipeId}`,
    providerId: recipe.providerId,
    runtimeKind: recipe.runtimeKind,
    label: normalizeOptionalText(input.label) ?? recipe.defaultProfileLabel,
    modelId: normalizeOptionalText(input.modelId) ?? recipe.defaultModelId,
    presetId: recipe.presetId,
    baseUrl,
    apiVersion,
    region: null,
    projectId: null,
    apiKey,
    activate: input.activate ?? false,
  })
}

export function recommendProviderSetup(
  providerProfiles: ProviderProfilesDto | null | undefined,
): ProviderRecommendationSetDto {
  const profiles = providerProfiles?.profiles ?? []
  const activeProfileId = providerProfiles?.activeProfileId ?? null
  const recommendations: ProviderRecommendationDto[] = []

  for (const profile of profiles) {
    const recommendation = recommendationForProfile(profile, activeProfileId)
    if (recommendation) {
      recommendations.push(recommendation)
    }
  }

  if (profiles.length === 0 || recommendations.length === 0) {
    recommendations.push(defaultOpenAiCompatibleRecommendation())
  }

  const sorted = recommendations
    .sort((left, right) => right.priority - left.priority || left.title.localeCompare(right.title))
    .filter(dedupeRecommendationKinds())
  const [primary, ...alternatives] = sorted

  return providerRecommendationSetSchema.parse({
    primary: primary ?? null,
    alternatives,
  })
}

export function isLocalOpenAiCompatibleBaseUrl(baseUrl: string | null | undefined): boolean {
  if (typeof baseUrl !== 'string' || baseUrl.trim().length === 0) {
    return false
  }

  try {
    const parsed = new URL(baseUrl.trim())
    return ['localhost', '127.0.0.1', '::1'].includes(parsed.hostname.toLowerCase())
  } catch {
    return false
  }
}

function recommendationForProfile(
  profile: ProviderProfileDto,
  activeProfileId: string | null,
): ProviderRecommendationDto | null {
  const isActive = profile.profileId === activeProfileId
  const label = profile.label.trim() || getCloudProviderLabel(profile.providerId)

  if (profile.readiness.ready) {
    if (isLocalProviderProfile(profile)) {
      return buildRecommendation({
        kind: 'best_local_profile',
        title: `Use local ${label}`,
        message: `${label} is configured with local readiness, so Cadence can run without a hosted API key.`,
        action: isActive ? 'edit_profile' : 'activate_profile',
        actionLabel: isActive ? 'Review profile' : 'Use local profile',
        profile,
        priority: isActive ? 140 : 125,
      })
    }

    return buildRecommendation({
      kind: 'fastest_ready_profile',
      title: `Use ${label}`,
      message: `${label} already has a valid readiness proof and is the quickest available provider path.`,
      action: isActive ? 'edit_profile' : 'activate_profile',
      actionLabel: isActive ? 'Review profile' : 'Use ready profile',
      profile,
      priority: (isActive ? 130 : 105) + providerPriority(profile.providerId),
    })
  }

  if (
    profile.readiness.status === 'missing' &&
    isApiKeyCloudProvider(profile.providerId) &&
    !isLocalProviderProfile(profile)
  ) {
    return buildRecommendation({
      kind: 'missing_key_cloud_profile',
      title: `Add a key for ${label}`,
      message: `${label} is saved but still needs an app-local API key before Cadence can use it.`,
      action: 'edit_profile',
      actionLabel: 'Add key',
      profile,
      priority: 70 + providerPriority(profile.providerId),
    })
  }

  return buildRecommendation({
    kind: 'unsupported_incomplete_profile',
    title: `Repair ${label}`,
    message: `${label} is present but incomplete. Review its endpoint, credential, or ambient setup before using it.`,
    action: 'edit_profile',
    actionLabel: 'Review setup',
    profile,
    priority: 35 + providerPriority(profile.providerId),
  })
}

function defaultOpenAiCompatibleRecommendation(): ProviderRecommendationDto {
  return providerRecommendationSchema.parse({
    kind: 'missing_key_cloud_profile',
    title: 'Set up an OpenAI-compatible endpoint',
    message: 'No usable provider profile is ready yet. Start with a custom /v1 gateway or one of the OpenAI-compatible recipes.',
    action: 'apply_recipe',
    actionLabel: 'Choose recipe',
    profileId: null,
    providerId: 'openai_api',
    recipeId: 'custom_openai_compatible',
    priority: 60,
  })
}

function buildRecommendation(input: {
  kind: ProviderRecommendationKindDto
  title: string
  message: string
  action: ProviderRecommendationActionDto
  actionLabel: string
  profile: ProviderProfileDto
  priority: number
}): ProviderRecommendationDto {
  return providerRecommendationSchema.parse({
    kind: input.kind,
    title: input.title,
    message: input.message,
    action: input.action,
    actionLabel: input.actionLabel,
    profileId: input.profile.profileId,
    providerId: input.profile.providerId,
    recipeId: null,
    priority: input.priority,
  })
}

function isLocalProviderProfile(profile: ProviderProfileDto): boolean {
  return (
    isLocalCloudProvider(profile.providerId) ||
    profile.readiness.proof === 'local' ||
    (profile.providerId === 'openai_api' && isLocalOpenAiCompatibleBaseUrl(profile.baseUrl))
  )
}

function providerPriority(providerId: ProviderProfileDto['providerId']): number {
  switch (providerId) {
    case 'openai_codex':
      return 11
    case 'openrouter':
      return 10
    case 'anthropic':
      return 9
    case 'github_models':
      return 8
    case 'openai_api':
      return 7
    case 'ollama':
      return 6
    case 'gemini_ai_studio':
      return 5
    case 'bedrock':
      return 4
    case 'vertex':
      return 3
    case 'azure_openai':
      return 2
  }
}

function dedupeRecommendationKinds() {
  const seen = new Set<ProviderRecommendationKindDto>()
  return (recommendation: ProviderRecommendationDto): boolean => {
    if (seen.has(recommendation.kind)) {
      return false
    }
    seen.add(recommendation.kind)
    return true
  }
}

function requireProviderSetupRecipe(recipeId: ProviderSetupRecipeIdDto | string): ProviderSetupRecipeDto {
  const recipe = getProviderSetupRecipe(recipeId)
  if (!recipe) {
    throw new Error(`Unknown provider setup recipe \`${recipeId}\`.`)
  }
  return recipe
}

function recipeFieldPrompt(
  field: ProviderSetupRecipeRequiredFieldDto,
  recipeLabel: string,
  label: string,
): ProviderSetupRecipeFieldPromptDto {
  return {
    field,
    label,
    message: `${recipeLabel} requires ${label}.`,
  }
}

function normalizeOptionalText(value: string | null | undefined): string | null {
  if (typeof value !== 'string') {
    return null
  }
  const trimmed = value.trim()
  return trimmed.length > 0 ? trimmed : null
}
