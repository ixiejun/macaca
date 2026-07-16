use super::foundation_key_value_state::{
    KeyValueBatchPutCommand, KeyValueKeyRef, KeyValueNamespaceRef, KeyValuePutCommand,
    KeyValueTtlPolicy, KeyValueTypedValueRef,
};
use super::foundation_validation::{
    bounded_reference, opaque_artifact_reference, secret_store_reference,
};

impl KeyValueNamespaceRef {
    /// Keep namespaces tenant-scoped logical identifiers rather than provider handles.
    pub fn is_bounded_reference(&self) -> bool {
        bounded_reference(&self.namespace, 160)
            && self
                .tenant_ref
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 160))
    }
}

impl KeyValueKeyRef {
    /// Check key bounds before state-provider admission.
    pub fn is_bounded_reference(&self) -> bool {
        self.namespace.is_bounded_reference() && bounded_reference(&self.key, 256)
    }
}

impl KeyValueTypedValueRef {
    /// Keep state values as opaque artifacts, requiring secret-store references when classified.
    pub fn is_admissible_reference(&self) -> bool {
        let is_secret = secret_store_reference(&self.value_ref);
        opaque_artifact_reference(&self.value_ref)
            && bounded_reference(&self.value_kind, 96)
            && self
                .schema_id
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 160))
            && (self.secret_reference_required == is_secret)
    }
}

impl KeyValueTtlPolicy {
    /// Require exactly one bounded expiration mode to avoid ambiguous TTL behavior.
    pub fn is_bounded(&self, max_ttl_seconds: u64, now_epoch_millis: u64) -> bool {
        match (self.ttl_seconds, self.expire_at_epoch_millis) {
            (Some(ttl), None) => ttl > 0 && ttl <= max_ttl_seconds,
            (None, Some(expiry)) => expiry > now_epoch_millis,
            _ => false,
        }
    }
}

impl KeyValuePutCommand {
    /// Validate a single mutation before runtime policy and revision checks.
    pub fn has_safe_preconditions(&self, max_ttl_seconds: u64, now_epoch_millis: u64) -> bool {
        self.key.is_bounded_reference()
            && self.value.is_admissible_reference()
            && self
                .ttl
                .as_ref()
                .is_none_or(|ttl| ttl.is_bounded(max_ttl_seconds, now_epoch_millis))
    }
}

impl KeyValueBatchPutCommand {
    /// Bound batch fan-out before resource reservation and provider dispatch.
    pub fn is_bounded_request(
        &self,
        max_entries: usize,
        max_ttl_seconds: u64,
        now_epoch_millis: u64,
    ) -> bool {
        !self.entries.is_empty()
            && self.entries.len() <= max_entries
            && self
                .entries
                .iter()
                .all(|entry| entry.has_safe_preconditions(max_ttl_seconds, now_epoch_millis))
    }
}
