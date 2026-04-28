use super::{ProviderCredentialKind, ProviderCredentialRecord};

/// Why a credential row is considered ready. Mirrors the legacy
/// `ProviderProfileReadinessProof` enum but drops the `Malformed` case — under
/// the new schema a row either exists (always ready) or it doesn't.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderCredentialReadinessProof {
    OAuthSession,
    StoredSecret,
    Local,
    Ambient,
}

pub fn readiness_proof(record: &ProviderCredentialRecord) -> ProviderCredentialReadinessProof {
    match record.kind {
        ProviderCredentialKind::ApiKey => ProviderCredentialReadinessProof::StoredSecret,
        ProviderCredentialKind::OAuthSession => ProviderCredentialReadinessProof::OAuthSession,
        ProviderCredentialKind::Local => ProviderCredentialReadinessProof::Local,
        ProviderCredentialKind::Ambient => ProviderCredentialReadinessProof::Ambient,
    }
}
