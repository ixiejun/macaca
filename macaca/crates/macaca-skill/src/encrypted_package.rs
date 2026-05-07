//! Encrypted skill package hook for Route C Phase 08.
//!
//! This module does not claim to provide DRM or local tamper resistance. Its
//! purpose is narrower and auditable: detect encrypted package metadata,
//! require an entitlement decision before decrypt/load, and delegate actual
//! decryption to a replaceable strategy supplied by the host or Store service.

use async_trait::async_trait;
use macaca_proto::{CommerceError, EntitlementDecision, PackageManifest, PackageRuntimeKind};
use tracing::{info, warn};

/// Metadata extracted from a package that requests encrypted loading.
///
/// The fields are intentionally provider-neutral. A future Store adapter may
/// map them to concrete envelope formats, key systems, or remote execution
/// strategies without changing skill consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedPackageMetadata {
    pub envelope: String,
    pub key_id: Option<String>,
}

/// Result of an encrypted package load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecryptedPackage {
    pub bytes: Vec<u8>,
    pub metadata: EncryptedPackageMetadata,
}

/// Strategy boundary for decrypting package bytes.
///
/// Implementations must avoid logging secrets or plaintext payloads. The hook
/// receives a successful entitlement decision so decryptors can include
/// entitlement id or policy metadata in their own audit trail.
#[async_trait]
pub trait EncryptedPackageDecryptor: Send + Sync {
    async fn decrypt(
        &self,
        manifest: &PackageManifest,
        encrypted_bytes: &[u8],
        metadata: &EncryptedPackageMetadata,
        decision: &EntitlementDecision,
    ) -> Result<Vec<u8>, CommerceError>;
}

/// Authorization strategy used before a decryptor is invoked.
#[async_trait]
pub trait EncryptedPackageAuthorizer: Send + Sync {
    async fn authorize_decrypt(
        &self,
        manifest: &PackageManifest,
    ) -> Result<EntitlementDecision, CommerceError>;
}

/// Proxy that gates decrypt/load behind entitlement authorization.
pub struct EncryptedPackageLoader<A, D> {
    authorizer: A,
    decryptor: D,
}

impl<A, D> EncryptedPackageLoader<A, D>
where
    A: EncryptedPackageAuthorizer,
    D: EncryptedPackageDecryptor,
{
    /// Build a loader from explicit authorization and decrypt strategies.
    pub fn new(authorizer: A, decryptor: D) -> Self {
        Self {
            authorizer,
            decryptor,
        }
    }

    /// Load encrypted bytes after entitlement authorization.
    pub async fn load(
        &self,
        manifest: &PackageManifest,
        encrypted_bytes: &[u8],
    ) -> Result<Option<DecryptedPackage>, CommerceError> {
        let Some(metadata) = encrypted_package_metadata(manifest) else {
            info!(
                package_id = %manifest.id,
                "encrypted package loader skipped non-encrypted package"
            );
            return Ok(None);
        };

        info!(
            package_id = %manifest.id,
            developer_id = %manifest.developer,
            envelope = %metadata.envelope,
            "encrypted package authorization started"
        );
        let decision = self.authorizer.authorize_decrypt(manifest).await?;
        if !decision.allowed {
            warn!(
                package_id = %manifest.id,
                state = %decision.state,
                reason = %decision.reason,
                "encrypted package decrypt denied by entitlement decision"
            );
            return Err(CommerceError::EncryptedPackageDenied(decision.reason));
        }

        let bytes = self
            .decryptor
            .decrypt(manifest, encrypted_bytes, &metadata, &decision)
            .await?;
        info!(
            package_id = %manifest.id,
            byte_len = bytes.len(),
            "encrypted package decrypt hook completed"
        );
        Ok(Some(DecryptedPackage { bytes, metadata }))
    }
}

/// Detect whether a package asks for encrypted loading.
pub fn encrypted_package_metadata(manifest: &PackageManifest) -> Option<EncryptedPackageMetadata> {
    let runtime_encrypted = matches!(
        manifest.runtime.kind,
        Some(PackageRuntimeKind::EncryptedTextBundle)
    );
    let envelope = manifest
        .metadata
        .get("encrypted.envelope")
        .cloned()
        .or_else(|| runtime_encrypted.then(|| "runtime_encrypted_text_bundle".into()))?;
    Some(EncryptedPackageMetadata {
        envelope,
        key_id: manifest.metadata.get("encrypted.key_id").cloned(),
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use macaca_proto::{
        DeveloperId, EntitlementDecision, EntitlementState, LicenseType, PackageId,
        PackageManifest, PackageRuntime, PackageRuntimeKind, PackageType,
    };

    use super::*;

    struct StaticAuthorizer {
        decision: EntitlementDecision,
    }

    #[async_trait]
    impl EncryptedPackageAuthorizer for StaticAuthorizer {
        async fn authorize_decrypt(
            &self,
            _manifest: &PackageManifest,
        ) -> Result<EntitlementDecision, CommerceError> {
            Ok(self.decision.clone())
        }
    }

    struct CountingDecryptor {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl EncryptedPackageDecryptor for CountingDecryptor {
        async fn decrypt(
            &self,
            _manifest: &PackageManifest,
            encrypted_bytes: &[u8],
            _metadata: &EncryptedPackageMetadata,
            _decision: &EntitlementDecision,
        ) -> Result<Vec<u8>, CommerceError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(encrypted_bytes.iter().rev().copied().collect())
        }
    }

    fn manifest() -> PackageManifest {
        let mut manifest = PackageManifest::new(
            PackageId::new("skill.encrypted"),
            PackageType::Skill,
            "1.0.0",
            DeveloperId::new("developer.skill"),
            PackageRuntime::new(PackageRuntimeKind::EncryptedTextBundle, "1"),
        );
        manifest.commerce.license_type = LicenseType::paid();
        manifest.commerce.store_required = true;
        manifest
    }

    #[tokio::test]
    async fn encrypted_skill_without_entitlement_does_not_call_decrypt_hook() {
        let calls = Arc::new(AtomicUsize::new(0));
        let loader = EncryptedPackageLoader::new(
            StaticAuthorizer {
                decision: EntitlementDecision::deny(
                    PackageId::new("skill.encrypted"),
                    DeveloperId::new("developer.skill"),
                    "decrypt",
                    EntitlementState::missing(),
                    "missing entitlement",
                ),
            },
            CountingDecryptor {
                calls: Arc::clone(&calls),
            },
        );

        let err = loader.load(&manifest(), b"abc").await.unwrap_err();

        assert!(matches!(err, CommerceError::EncryptedPackageDenied(_)));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn encrypted_skill_with_entitlement_enters_decrypt_hook() {
        let calls = Arc::new(AtomicUsize::new(0));
        let loader = EncryptedPackageLoader::new(
            StaticAuthorizer {
                decision: EntitlementDecision::allow(
                    PackageId::new("skill.encrypted"),
                    DeveloperId::new("developer.skill"),
                    "decrypt",
                    EntitlementState::valid(),
                ),
            },
            CountingDecryptor {
                calls: Arc::clone(&calls),
            },
        );

        let decrypted = loader.load(&manifest(), b"abc").await.unwrap().unwrap();

        assert_eq!(decrypted.bytes, b"cba");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
