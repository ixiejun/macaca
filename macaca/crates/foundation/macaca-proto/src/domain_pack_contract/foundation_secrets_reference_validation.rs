use super::foundation_secrets_reference::{
    SecretAccessPolicy, SecretExternalLocator, SecretLeaseReference, SecretPurposeBinding,
    SecretReference, SecretsCreateLeaseCommand, SecretsImportReferenceCommand,
    SecretsResolveForProviderCommand,
};
use super::foundation_validation::bounded_reference;

impl SecretReference {
    /// Validate reference metadata without accepting secret material or provider URLs.
    pub fn is_safe_reference(&self) -> bool {
        bounded_reference(&self.reference_id, 160)
            && bounded_reference(&self.provider_class, 96)
            && self
                .version_hint
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 96))
    }
}

impl SecretExternalLocator {
    /// Import requests may carry only a redacted locator hash, never an external URL or value.
    pub fn is_safe_reference(&self) -> bool {
        bounded_reference(&self.provider_class, 96)
            && bounded_reference(&self.redacted_locator_hash, 256)
    }
}

impl SecretPurposeBinding {
    /// Bind a secret reference to a bounded generic service purpose.
    pub fn is_bounded_binding(&self) -> bool {
        bounded_reference(&self.purpose, 160)
            && self.service_id.starts_with("service.")
            && bounded_reference(&self.service_id, 160)
            && self.expires_at_epoch_millis.is_none_or(|value| value > 0)
    }
}

impl SecretAccessPolicy {
    /// Validate an allow-list and lease limit before provider-side injection is considered.
    pub fn is_bounded_policy(&self, max_lease_ttl_seconds: u64) -> bool {
        !self.allowed_service_ids.is_empty()
            && self.allowed_service_ids.len() <= 64
            && self.allowed_service_ids.iter().all(|service_id| {
                service_id.starts_with("service.") && bounded_reference(service_id, 160)
            })
            && self.max_lease_ttl_seconds > 0
            && self.max_lease_ttl_seconds <= max_lease_ttl_seconds
    }
}

impl SecretsImportReferenceCommand {
    /// Validate reference-only imports before approval and provider dispatch.
    pub fn has_safe_preconditions(&self, max_lease_ttl_seconds: u64) -> bool {
        self.locator.is_safe_reference()
            && self.purpose.is_bounded_binding()
            && self.policy.is_bounded_policy(max_lease_ttl_seconds)
    }
}

impl SecretsResolveForProviderCommand {
    /// Resolution addresses an approved service only; it cannot return an app-facing value.
    pub fn has_safe_preconditions(&self) -> bool {
        self.reference.is_safe_reference()
            && bounded_reference(&self.purpose, 160)
            && self.service_id.starts_with("service.")
            && bounded_reference(&self.service_id, 160)
    }
}

impl SecretsCreateLeaseCommand {
    /// Check lease duration before runtime policy issues a provider-scoped lease.
    pub fn is_bounded_request(&self, max_ttl_seconds: u64) -> bool {
        self.reference.is_safe_reference()
            && bounded_reference(&self.purpose, 160)
            && self.ttl_seconds > 0
            && self.ttl_seconds <= max_ttl_seconds
    }
}

impl SecretLeaseReference {
    /// Validate lease evidence without exposing a resolved secret value.
    pub fn is_safe_reference(&self) -> bool {
        bounded_reference(&self.lease_id, 160)
            && bounded_reference(&self.reference_id, 160)
            && self.expires_at_epoch_millis > 0
    }
}
