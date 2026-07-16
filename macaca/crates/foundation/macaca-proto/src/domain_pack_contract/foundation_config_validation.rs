use super::foundation_config::{
    ConfigKeyReference, ConfigListKeysCommand, ConfigSchemaReference, ConfigSelector,
    ConfigTypedValueRef, ConfigValueKind, ConfigWatchCommand,
};
use super::foundation_validation::{
    bounded_page_size, bounded_reference, opaque_artifact_reference, secret_store_reference,
};

impl ConfigKeyReference {
    /// Check that a key remains an app-scoped, trace-safe logical reference.
    pub fn is_bounded_reference(&self) -> bool {
        bounded_reference(&self.namespace, 160) && bounded_reference(&self.key, 256)
    }
}

impl ConfigSchemaReference {
    /// Check schema identifiers before a provider receives a validation request.
    pub fn is_bounded_reference(&self) -> bool {
        bounded_reference(&self.schema_id, 160) && bounded_reference(&self.version, 64)
    }
}

impl ConfigSelector {
    /// Check selectors without interpreting tenant or environment provider data.
    pub fn is_bounded_reference(&self) -> bool {
        bounded_reference(&self.profile, 96)
            && self
                .tenant_ref
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 160))
            && self
                .environment_ref
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 160))
    }
}

impl ConfigTypedValueRef {
    /// Require secret-classified configuration to use a secret-store reference.
    pub fn is_admissible_reference(&self) -> bool {
        let value_is_secret = secret_store_reference(&self.value_ref);
        opaque_artifact_reference(&self.value_ref)
            && self
                .schema
                .as_ref()
                .is_none_or(ConfigSchemaReference::is_bounded_reference)
            && match self.kind {
                ConfigValueKind::SecretReference => value_is_secret,
                _ => !self.secret_reference_required && !value_is_secret,
            }
    }
}

impl ConfigListKeysCommand {
    /// Bound a listing request so diagnostics cannot become an unbounded export.
    pub fn is_bounded_request(&self, max_page_size: u32) -> bool {
        bounded_reference(&self.namespace, 160)
            && self
                .prefix
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 256))
            && bounded_page_size(self.page_size, max_page_size)
            && self
                .cursor
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 256))
    }
}

impl ConfigWatchCommand {
    /// Validate bounded watch state before runtime-level watch budgeting.
    pub fn is_bounded_request(&self) -> bool {
        bounded_reference(&self.namespace, 160)
            && self.selector.is_bounded_reference()
            && self
                .start_cursor
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 256))
    }
}
