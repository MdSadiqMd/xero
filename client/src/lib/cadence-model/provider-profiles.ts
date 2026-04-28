// Phase 4: legacy provider-profile types are retained as opaque shapes so
// the still-skipped legacy tests stay typeable. The Zod schemas, helpers
// (`getActiveProviderProfile`, `projectRuntimeSettingsFromProviderProfiles`),
// and runtime use of these types is gone — anything reaching for these
// types now is by definition exercising a deleted code path.
//
// Once the skipped tests are rewritten or deleted, this file can be removed
// entirely and the cadence-model barrel pruned.

import { z } from 'zod'
import type { RuntimeProviderIdDto } from './runtime'

export type ProviderProfileReadinessStatusDto = 'ready' | 'missing' | 'malformed'
export type ProviderProfileReadinessProofDto =
  | 'oauth_session'
  | 'stored_secret'
  | 'local'
  | 'ambient'

export interface ProviderProfileReadinessDto {
  ready: boolean
  status: ProviderProfileReadinessStatusDto
  proof?: ProviderProfileReadinessProofDto | null
  proofUpdatedAt?: string | null
}

export interface ProviderProfileDto {
  profileId: string
  providerId: RuntimeProviderIdDto
  runtimeKind: 'openai_codex' | 'openrouter' | 'anthropic' | 'openai_compatible' | 'gemini'
  label: string
  modelId: string
  presetId?:
    | 'openrouter'
    | 'anthropic'
    | 'github_models'
    | 'openai_api'
    | 'ollama'
    | 'azure_openai'
    | 'gemini_ai_studio'
    | 'bedrock'
    | 'vertex'
    | null
  baseUrl?: string | null
  apiVersion?: string | null
  region?: string | null
  projectId?: string | null
  active: boolean
  readiness: ProviderProfileReadinessDto
  migratedFromLegacy: boolean
  migratedAt?: string | null
}

export interface ProviderProfilesMigrationDto {
  source: string
  migratedAt: string
  runtimeSettingsUpdatedAt?: string | null
  openrouterCredentialsUpdatedAt?: string | null
  openaiAuthUpdatedAt?: string | null
  openrouterModelInferred?: boolean | null
}

export interface ProviderProfilesDto {
  activeProfileId: string
  profiles: ProviderProfileDto[]
  migration?: ProviderProfilesMigrationDto | null
}

export interface UpsertProviderProfileRequestDto {
  profileId: string
  providerId: RuntimeProviderIdDto
  runtimeKind: ProviderProfileDto['runtimeKind']
  label: string
  modelId: string
  presetId?: ProviderProfileDto['presetId']
  baseUrl?: string | null
  apiVersion?: string | null
  region?: string | null
  projectId?: string | null
  apiKey?: string | null
  activate?: boolean
}

// Tests sometimes call these schemas to construct fixtures or assert
// validation. We keep loose schemas that just `passthrough` so they remain
// callable but do not enforce the legacy contract. They are unused by
// production code.
export const providerProfileReadinessSchema = z.unknown() as unknown as z.ZodType<
  ProviderProfileReadinessDto
>
export const providerProfileSchema = z.unknown() as unknown as z.ZodType<ProviderProfileDto>
export const providerProfilesSchema = z.unknown() as unknown as z.ZodType<ProviderProfilesDto>
export const upsertProviderProfileRequestSchema = z.unknown() as unknown as z.ZodType<
  UpsertProviderProfileRequestDto
>
export const setActiveProviderProfileRequestSchema = z.unknown() as unknown as z.ZodType<{
  profileId: string
}>
export const logoutProviderProfileRequestSchema = z.unknown() as unknown as z.ZodType<{
  profileId: string
}>
export type SetActiveProviderProfileRequestDto = { profileId: string }
export type LogoutProviderProfileRequestDto = { profileId: string }

export function getActiveProviderProfile(
  providerProfiles: ProviderProfilesDto | null | undefined,
): ProviderProfileDto | null {
  if (!providerProfiles) return null
  return (
    providerProfiles.profiles.find(
      (profile) => profile.profileId === providerProfiles.activeProfileId,
    ) ?? null
  )
}

export function projectRuntimeSettingsFromProviderProfiles(
  _providerProfiles: ProviderProfilesDto | null | undefined,
): null {
  // Legacy projection — the runtime-settings slice is gone.
  return null
}
