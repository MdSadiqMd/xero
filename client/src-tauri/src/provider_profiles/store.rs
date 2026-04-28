use serde::{Deserialize, Serialize};

use crate::runtime::{
    normalize_openai_codex_model_id, ANTHROPIC_PROVIDER_ID, AZURE_OPENAI_PROVIDER_ID,
    BEDROCK_PROVIDER_ID, GEMINI_AI_STUDIO_PROVIDER_ID, GEMINI_RUNTIME_KIND,
    GITHUB_MODELS_PROVIDER_ID, OLLAMA_PROVIDER_ID, OPENAI_API_PROVIDER_ID,
    OPENAI_CODEX_PROVIDER_ID, OPENAI_COMPATIBLE_RUNTIME_KIND, OPENROUTER_PROVIDER_ID,
    VERTEX_PROVIDER_ID,
};

pub const OPENAI_CODEX_DEFAULT_PROFILE_ID: &str = "openai_codex-default";
pub const OPENROUTER_DEFAULT_PROFILE_ID: &str = "openrouter-default";
pub const ANTHROPIC_DEFAULT_PROFILE_ID: &str = "anthropic-default";
pub const GITHUB_MODELS_DEFAULT_PROFILE_ID: &str = "github_models-default";
pub const OPENROUTER_FALLBACK_MODEL_ID: &str = "openai/gpt-4.1-mini";
const PROVIDER_PROFILES_SCHEMA_VERSION: u32 = 3;
const OPENAI_CODEX_DEFAULT_PROFILE_LABEL: &str = "OpenAI Codex";
const OPENROUTER_DEFAULT_PROFILE_LABEL: &str = "OpenRouter";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderProfilesMetadataFile {
    #[serde(default = "provider_profiles_schema_version")]
    pub version: u32,
    pub active_profile_id: String,
    #[serde(default)]
    pub profiles: Vec<ProviderProfileRecord>,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub migration: Option<ProviderProfilesMigrationState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderProfileRecord {
    pub profile_id: String,
    pub provider_id: String,
    #[serde(default)]
    pub runtime_kind: String,
    pub label: String,
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_link: Option<ProviderProfileCredentialLink>,
    #[serde(default)]
    pub migrated_from_legacy: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub migrated_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderProfileCredentialLink {
    #[serde(rename = "openai_codex")]
    OpenAiCodex {
        account_id: String,
        session_id: String,
        updated_at: String,
    },
    #[serde(rename = "api_key", alias = "openrouter", alias = "anthropic")]
    ApiKey { updated_at: String },
    #[serde(rename = "local")]
    Local { updated_at: String },
    #[serde(rename = "ambient")]
    Ambient { updated_at: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderProfilesMigrationState {
    pub source: String,
    pub migrated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_settings_updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openrouter_credentials_updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openai_auth_updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openrouter_model_inferred: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderApiKeyCredentialEntry {
    pub profile_id: String,
    pub api_key: String,
    pub updated_at: String,
}

pub type OpenRouterProfileCredentialEntry = ProviderApiKeyCredentialEntry;
pub type AnthropicProfileCredentialEntry = ProviderApiKeyCredentialEntry;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderProfileCredentialsFile {
    #[serde(default)]
    pub api_keys: Vec<ProviderApiKeyCredentialEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfilesSnapshot {
    pub metadata: ProviderProfilesMetadataFile,
    pub credentials: ProviderProfileCredentialsFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderProfileReadinessStatus {
    Ready,
    Missing,
    Malformed,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ProviderProfileReadinessProof {
    OAuthSession,
    StoredSecret,
    Local,
    Ambient,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProfileReadinessProjection {
    pub ready: bool,
    pub status: ProviderProfileReadinessStatus,
    pub proof: Option<ProviderProfileReadinessProof>,
    pub proof_updated_at: Option<String>,
}

impl ProviderProfilesSnapshot {
    pub fn active_profile(&self) -> Option<&ProviderProfileRecord> {
        self.metadata
            .profiles
            .iter()
            .find(|profile| profile.profile_id == self.metadata.active_profile_id)
    }

    pub fn profile(&self, profile_id: &str) -> Option<&ProviderProfileRecord> {
        self.metadata
            .profiles
            .iter()
            .find(|profile| profile.profile_id == profile_id)
    }

    pub fn any_openrouter_api_key_configured(&self) -> bool {
        self.metadata.profiles.iter().any(|profile| {
            profile.provider_id == OPENROUTER_PROVIDER_ID
                && profile.readiness(&self.credentials).ready
        })
    }

    pub fn any_anthropic_api_key_configured(&self) -> bool {
        self.metadata.profiles.iter().any(|profile| {
            profile.provider_id == ANTHROPIC_PROVIDER_ID
                && profile.readiness(&self.credentials).ready
        })
    }

    pub fn preferred_openrouter_credential(&self) -> Option<&OpenRouterProfileCredentialEntry> {
        self.preferred_api_key_credential_for_provider(OPENROUTER_PROVIDER_ID)
    }

    pub fn preferred_anthropic_credential(&self) -> Option<&AnthropicProfileCredentialEntry> {
        self.preferred_api_key_credential_for_provider(ANTHROPIC_PROVIDER_ID)
    }

    pub fn api_key_credential(&self, profile_id: &str) -> Option<&ProviderApiKeyCredentialEntry> {
        self.credentials
            .api_keys
            .iter()
            .find(|entry| entry.profile_id == profile_id)
    }

    pub fn openrouter_credential(
        &self,
        profile_id: &str,
    ) -> Option<&OpenRouterProfileCredentialEntry> {
        self.api_key_credential(profile_id)
    }

    pub fn anthropic_credential(
        &self,
        profile_id: &str,
    ) -> Option<&AnthropicProfileCredentialEntry> {
        self.api_key_credential(profile_id)
    }

    pub fn matched_api_key_credential_for_profile(
        &self,
        profile_id: &str,
    ) -> Option<&ProviderApiKeyCredentialEntry> {
        self.profile(profile_id)
            .and_then(|profile| self.matched_api_key_credential(profile))
    }

    fn preferred_api_key_credential_for_provider(
        &self,
        provider_id: &str,
    ) -> Option<&ProviderApiKeyCredentialEntry> {
        self.active_profile()
            .filter(|profile| profile.provider_id == provider_id)
            .and_then(|profile| self.matched_api_key_credential(profile))
            .or_else(|| {
                self.metadata
                    .profiles
                    .iter()
                    .filter(|profile| profile.provider_id == provider_id)
                    .find_map(|profile| self.matched_api_key_credential(profile))
            })
    }

    fn matched_api_key_credential(
        &self,
        profile: &ProviderProfileRecord,
    ) -> Option<&ProviderApiKeyCredentialEntry> {
        let ProviderProfileCredentialLink::ApiKey { updated_at } =
            profile.credential_link.as_ref()?
        else {
            return None;
        };

        self.api_key_credential(&profile.profile_id)
            .filter(|entry| entry.updated_at == *updated_at)
    }
}

impl ProviderProfileRecord {
    pub fn readiness(
        &self,
        credentials: &ProviderProfileCredentialsFile,
    ) -> ProviderProfileReadinessProjection {
        match &self.credential_link {
            Some(ProviderProfileCredentialLink::OpenAiCodex { updated_at, .. }) => {
                ProviderProfileReadinessProjection {
                    ready: true,
                    status: ProviderProfileReadinessStatus::Ready,
                    proof: Some(ProviderProfileReadinessProof::OAuthSession),
                    proof_updated_at: Some(updated_at.clone()),
                }
            }
            Some(ProviderProfileCredentialLink::ApiKey { updated_at }) => {
                let matched_secret = credentials.api_keys.iter().any(|entry| {
                    entry.profile_id == self.profile_id && entry.updated_at == *updated_at
                });
                if matched_secret {
                    ProviderProfileReadinessProjection {
                        ready: true,
                        status: ProviderProfileReadinessStatus::Ready,
                        proof: Some(ProviderProfileReadinessProof::StoredSecret),
                        proof_updated_at: Some(updated_at.clone()),
                    }
                } else {
                    ProviderProfileReadinessProjection {
                        ready: false,
                        status: ProviderProfileReadinessStatus::Malformed,
                        proof: None,
                        proof_updated_at: Some(updated_at.clone()),
                    }
                }
            }
            Some(ProviderProfileCredentialLink::Local { updated_at }) => {
                ProviderProfileReadinessProjection {
                    ready: true,
                    status: ProviderProfileReadinessStatus::Ready,
                    proof: Some(ProviderProfileReadinessProof::Local),
                    proof_updated_at: Some(updated_at.clone()),
                }
            }
            Some(ProviderProfileCredentialLink::Ambient { updated_at }) => {
                ProviderProfileReadinessProjection {
                    ready: true,
                    status: ProviderProfileReadinessStatus::Ready,
                    proof: Some(ProviderProfileReadinessProof::Ambient),
                    proof_updated_at: Some(updated_at.clone()),
                }
            }
            None => ProviderProfileReadinessProjection {
                ready: false,
                status: ProviderProfileReadinessStatus::Missing,
                proof: None,
                proof_updated_at: None,
            },
        }
    }
}

pub fn default_provider_profiles_snapshot() -> ProviderProfilesSnapshot {
    let timestamp = crate::auth::now_timestamp();
    ProviderProfilesSnapshot {
        metadata: ProviderProfilesMetadataFile {
            version: PROVIDER_PROFILES_SCHEMA_VERSION,
            active_profile_id: OPENAI_CODEX_DEFAULT_PROFILE_ID.into(),
            profiles: vec![build_openai_default_profile(None, None, &timestamp)],
            updated_at: timestamp,
            migration: None,
        },
        credentials: ProviderProfileCredentialsFile::default(),
    }
}

/// Project a `ProviderProfilesSnapshot` from the flat `provider_credentials`
/// rows. The legacy module no longer owns its own SQL tables — this
/// synthesis is the read path so the seven legacy consumers keep working
/// against the snapshot shape they were written against.
pub fn synthesize_provider_profiles_snapshot_from_credentials(
    records: &[crate::provider_credentials::ProviderCredentialRecord],
) -> ProviderProfilesSnapshot {
    let timestamp = crate::auth::now_timestamp();
    let mut profiles: Vec<ProviderProfileRecord> = Vec::new();
    let mut credentials = ProviderProfileCredentialsFile::default();

    for record in records {
        let Some(synthesized) = synthesize_profile_from_credential(record) else {
            continue;
        };
        if let Some(api_key_entry) = synthesized.api_key_entry {
            credentials.api_keys.push(api_key_entry);
        }
        profiles.push(synthesized.profile);
    }

    if profiles.is_empty() {
        profiles.push(build_openai_default_profile(None, None, &timestamp));
    }

    let active_profile_id = profiles
        .iter()
        .find(|profile| profile.provider_id == OPENAI_CODEX_PROVIDER_ID)
        .map(|profile| profile.profile_id.clone())
        .or_else(|| profiles.first().map(|profile| profile.profile_id.clone()))
        .unwrap_or_else(|| OPENAI_CODEX_DEFAULT_PROFILE_ID.into());

    ProviderProfilesSnapshot {
        metadata: ProviderProfilesMetadataFile {
            version: PROVIDER_PROFILES_SCHEMA_VERSION,
            active_profile_id,
            profiles,
            updated_at: timestamp,
            migration: None,
        },
        credentials,
    }
}

struct SynthesizedProfile {
    profile: ProviderProfileRecord,
    api_key_entry: Option<ProviderApiKeyCredentialEntry>,
}

fn synthesize_profile_from_credential(
    record: &crate::provider_credentials::ProviderCredentialRecord,
) -> Option<SynthesizedProfile> {
    use crate::provider_credentials::ProviderCredentialKind;

    let provider_id = record.provider_id.as_str();
    let (profile_id, label, runtime_kind, preset_id) =
        synthesized_profile_metadata(provider_id);

    let credential_link = match record.kind {
        ProviderCredentialKind::OAuthSession => {
            let account_id = record.oauth_account_id.clone()?;
            let session_id = record.oauth_session_id.clone()?;
            Some(ProviderProfileCredentialLink::OpenAiCodex {
                account_id,
                session_id,
                updated_at: record.updated_at.clone(),
            })
        }
        ProviderCredentialKind::ApiKey => Some(ProviderProfileCredentialLink::ApiKey {
            updated_at: record.updated_at.clone(),
        }),
        ProviderCredentialKind::Local => Some(ProviderProfileCredentialLink::Local {
            updated_at: record.updated_at.clone(),
        }),
        ProviderCredentialKind::Ambient => Some(ProviderProfileCredentialLink::Ambient {
            updated_at: record.updated_at.clone(),
        }),
    };

    let model_id = record
        .default_model_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            if provider_id == OPENAI_CODEX_PROVIDER_ID {
                normalize_openai_codex_model_id(OPENAI_CODEX_PROVIDER_ID)
            } else if provider_id == OPENROUTER_PROVIDER_ID {
                OPENROUTER_FALLBACK_MODEL_ID.into()
            } else {
                provider_id.to_owned()
            }
        });

    let api_key_entry = match (record.kind, record.api_key.as_ref()) {
        (ProviderCredentialKind::ApiKey, Some(api_key)) => Some(ProviderApiKeyCredentialEntry {
            profile_id: profile_id.clone(),
            api_key: api_key.clone(),
            updated_at: record.updated_at.clone(),
        }),
        _ => None,
    };

    let profile = ProviderProfileRecord {
        profile_id,
        provider_id: provider_id.to_owned(),
        runtime_kind: runtime_kind.to_owned(),
        label,
        model_id,
        preset_id,
        base_url: record.base_url.clone(),
        api_version: record.api_version.clone(),
        region: record.region.clone(),
        project_id: record.project_id.clone(),
        credential_link,
        migrated_from_legacy: false,
        migrated_at: None,
        updated_at: record.updated_at.clone(),
    };

    Some(SynthesizedProfile {
        profile,
        api_key_entry,
    })
}

fn synthesized_profile_metadata(provider_id: &str) -> (String, String, &'static str, Option<String>) {
    match provider_id {
        OPENAI_CODEX_PROVIDER_ID => (
            OPENAI_CODEX_DEFAULT_PROFILE_ID.into(),
            OPENAI_CODEX_DEFAULT_PROFILE_LABEL.into(),
            OPENAI_CODEX_PROVIDER_ID,
            None,
        ),
        OPENROUTER_PROVIDER_ID => (
            OPENROUTER_DEFAULT_PROFILE_ID.into(),
            OPENROUTER_DEFAULT_PROFILE_LABEL.into(),
            OPENROUTER_PROVIDER_ID,
            Some(OPENROUTER_PROVIDER_ID.into()),
        ),
        ANTHROPIC_PROVIDER_ID => (
            ANTHROPIC_DEFAULT_PROFILE_ID.into(),
            "Anthropic".into(),
            ANTHROPIC_PROVIDER_ID,
            Some(ANTHROPIC_PROVIDER_ID.into()),
        ),
        GITHUB_MODELS_PROVIDER_ID => (
            GITHUB_MODELS_DEFAULT_PROFILE_ID.into(),
            "GitHub Models".into(),
            OPENAI_COMPATIBLE_RUNTIME_KIND,
            Some(GITHUB_MODELS_PROVIDER_ID.into()),
        ),
        OPENAI_API_PROVIDER_ID => (
            format!("{}-default", OPENAI_API_PROVIDER_ID),
            "OpenAI API".into(),
            OPENAI_COMPATIBLE_RUNTIME_KIND,
            Some(OPENAI_API_PROVIDER_ID.into()),
        ),
        OLLAMA_PROVIDER_ID => (
            format!("{}-default", OLLAMA_PROVIDER_ID),
            "Ollama".into(),
            OPENAI_COMPATIBLE_RUNTIME_KIND,
            Some(OLLAMA_PROVIDER_ID.into()),
        ),
        AZURE_OPENAI_PROVIDER_ID => (
            format!("{}-default", AZURE_OPENAI_PROVIDER_ID),
            "Azure OpenAI".into(),
            OPENAI_COMPATIBLE_RUNTIME_KIND,
            Some(AZURE_OPENAI_PROVIDER_ID.into()),
        ),
        GEMINI_AI_STUDIO_PROVIDER_ID => (
            format!("{}-default", GEMINI_AI_STUDIO_PROVIDER_ID),
            "Gemini".into(),
            GEMINI_RUNTIME_KIND,
            Some(GEMINI_AI_STUDIO_PROVIDER_ID.into()),
        ),
        BEDROCK_PROVIDER_ID => (
            format!("{}-default", BEDROCK_PROVIDER_ID),
            "Amazon Bedrock".into(),
            ANTHROPIC_PROVIDER_ID,
            Some(BEDROCK_PROVIDER_ID.into()),
        ),
        VERTEX_PROVIDER_ID => (
            format!("{}-default", VERTEX_PROVIDER_ID),
            "Vertex AI".into(),
            ANTHROPIC_PROVIDER_ID,
            Some(VERTEX_PROVIDER_ID.into()),
        ),
        other => (
            format!("{}-default", other),
            other.to_owned(),
            OPENAI_COMPATIBLE_RUNTIME_KIND,
            None,
        ),
    }
}

pub(crate) fn build_openai_default_profile(
    credential_link: Option<ProviderProfileCredentialLink>,
    migrated_at: Option<&str>,
    updated_at: &str,
) -> ProviderProfileRecord {
    ProviderProfileRecord {
        profile_id: OPENAI_CODEX_DEFAULT_PROFILE_ID.into(),
        provider_id: OPENAI_CODEX_PROVIDER_ID.into(),
        runtime_kind: OPENAI_CODEX_PROVIDER_ID.into(),
        label: OPENAI_CODEX_DEFAULT_PROFILE_LABEL.into(),
        model_id: normalize_openai_codex_model_id(OPENAI_CODEX_PROVIDER_ID),
        preset_id: None,
        base_url: None,
        api_version: None,
        region: None,
        project_id: None,
        credential_link,
        migrated_from_legacy: migrated_at.is_some(),
        migrated_at: migrated_at.map(str::to_owned),
        updated_at: updated_at.to_owned(),
    }
}

const fn provider_profiles_schema_version() -> u32 {
    PROVIDER_PROFILES_SCHEMA_VERSION
}
