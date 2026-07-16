use super::identity_profile::{
    AvatarReference, ProfileArtifactHandle, ProfileExportPlan, ProfileField, ProfileFreshness,
    ProfileMetadataNamespace, ProfilePreference, ProfileRecord, ProfileSchemaDescriptor,
};
use super::identity_validation::{bounded_identity_reference, opaque_identity_artifact};

impl ProfileField {
    /// Check minimized field evidence, privacy labels, and value references before dispatch.
    pub fn is_safe_projection(&self) -> bool {
        bounded_identity_reference(&self.field_key, 96)
            && bounded_identity_reference(&self.value_type, 64)
            && opaque_identity_artifact(&self.value_ref)
            && bounded_identity_reference(&self.source_ref, 160)
            && matches!(
                self.visibility.as_str(),
                "private" | "restricted" | "public"
            )
            && matches!(
                self.privacy_class.as_str(),
                "public" | "private" | "sensitive"
            )
            && bounded_identity_reference(&self.retention_class, 96)
            && self
                .locale
                .as_deref()
                .is_none_or(|locale| bounded_identity_reference(locale, 64))
            && bounded_identity_reference(&self.redaction_class, 96)
    }
}

impl ProfileSchemaDescriptor {
    /// Verify a compact schema declaration before a provider-specific schema
    /// adapter is selected.  The DTO validates shape only, not application data.
    pub fn is_valid_schema(&self, max_fields: usize) -> bool {
        bounded_identity_reference(&self.schema_ref, 160)
            && !self.field_definitions.is_empty()
            && self.field_definitions.len() <= max_fields
            && self.field_definitions.iter().all(|(field, kind)| {
                bounded_identity_reference(field, 96)
                    && matches!(
                        kind.as_str(),
                        "string" | "number" | "boolean" | "reference" | "date"
                    )
            })
            && self
                .extension_refs
                .iter()
                .all(|value| bounded_identity_reference(value, 160))
            && self
                .validation_rule_refs
                .iter()
                .all(|value| bounded_identity_reference(value, 160))
            && self
                .default_field_mask
                .iter()
                .all(|field| self.field_definitions.contains_key(field))
            && bounded_identity_reference(&self.compatibility_hash, 256)
    }
}

impl ProfileFreshness {
    /// Carry current, stale, or unknown freshness explicitly for typed outcomes.
    pub fn is_valid_evidence(&self) -> bool {
        self.source_timestamp_epoch_ms > 0
            && self
                .cache_timestamp_epoch_ms
                .is_none_or(|cached| cached >= self.source_timestamp_epoch_ms)
            && matches!(
                self.freshness_class.as_str(),
                "current" | "stale" | "unknown"
            )
    }
}

impl ProfileMetadataNamespace {
    /// Permit only explicit profile namespace classes and bounded access-policy handles.
    pub fn is_safe_namespace(&self) -> bool {
        bounded_identity_reference(&self.namespace_ref, 160)
            && matches!(
                self.namespace_kind.as_str(),
                "public"
                    | "private"
                    | "app_scoped"
                    | "provider_scoped"
                    | "directory_scoped"
                    | "custom"
            )
            && bounded_identity_reference(&self.access_policy_ref, 160)
            && matches!(
                self.visibility.as_str(),
                "private" | "restricted" | "public"
            )
    }
}

impl ProfilePreference {
    /// Reject application-business preference payloads from the generic profile boundary.
    pub fn is_profile_owned(&self) -> bool {
        !self.application_business_boundary
            && bounded_identity_reference(&self.preference_ref, 160)
            && bounded_identity_reference(&self.key, 96)
            && opaque_identity_artifact(&self.value_ref)
            && bounded_identity_reference(&self.scope_ref, 160)
            && bounded_identity_reference(&self.source_ref, 160)
    }
}

impl AvatarReference {
    /// Validate retained avatar metadata as handles and hashes, not photo bytes or URLs.
    pub fn is_bounded_reference(&self, now_epoch_ms: u64) -> bool {
        bounded_identity_reference(&self.avatar_ref, 160)
            && self
                .hosted_url_ref
                .as_deref()
                .is_none_or(|value| bounded_identity_reference(value, 256))
            && self
                .media_artifact_handle
                .as_deref()
                .is_none_or(|value| opaque_identity_artifact(value))
            && self.hosted_url_ref.is_some() != self.media_artifact_handle.is_some()
            && bounded_identity_reference(&self.checksum, 256)
            && bounded_identity_reference(&self.dimensions, 64)
            && bounded_identity_reference(&self.content_type, 96)
            && self
                .expires_at_epoch_ms
                .is_none_or(|expiry| expiry > now_epoch_ms)
            && bounded_identity_reference(&self.retention_class, 96)
            && bounded_identity_reference(&self.redaction_profile, 96)
    }
}

impl ProfileExportPlan {
    /// Bound export field masks and require a redaction profile before export approval.
    pub fn is_bounded_export(&self, max_fields: usize) -> bool {
        bounded_identity_reference(&self.export_ref, 160)
            && bounded_identity_reference(&self.scope_ref, 160)
            && !self.field_mask.is_empty()
            && self.field_mask.len() <= max_fields
            && self
                .field_mask
                .iter()
                .all(|field| bounded_identity_reference(field, 96))
            && matches!(self.format.as_str(), "json" | "csv" | "artifact")
            && bounded_identity_reference(&self.redaction_profile, 96)
    }
}

impl ProfileArtifactHandle {
    /// Keep profile exports bounded to expiring artifact metadata and policy handles.
    pub fn is_safe_async_result(&self, now_epoch_ms: u64) -> bool {
        opaque_identity_artifact(&self.artifact_id)
            && bounded_identity_reference(&self.content_class, 96)
            && bounded_identity_reference(&self.checksum, 256)
            && self.expires_at_epoch_ms > now_epoch_ms
            && bounded_identity_reference(&self.retention_class, 96)
            && bounded_identity_reference(&self.access_policy_ref, 160)
    }
}

impl ProfileRecord {
    /// Validate a bounded profile projection before trace, audit, or SDK exposure.
    pub fn has_safe_projection(&self, max_fields: usize, max_preferences: usize) -> bool {
        self.is_bounded(max_fields, max_preferences)
            && bounded_identity_reference(&self.profile_ref, 160)
            && bounded_identity_reference(&self.account_ref, 160)
            && bounded_identity_reference(&self.subject_ref, 160)
            && self.fields.iter().all(ProfileField::is_safe_projection)
            && self
                .metadata_namespaces
                .iter()
                .all(ProfileMetadataNamespace::is_safe_namespace)
            && self
                .preferences
                .iter()
                .all(ProfilePreference::is_profile_owned)
            && self
                .avatar_ref
                .as_ref()
                .is_none_or(|avatar| avatar.is_bounded_reference(0))
            && self.privacy_map.iter().all(|(field, privacy)| {
                self.fields
                    .iter()
                    .any(|candidate| candidate.field_key == *field)
                    && matches!(privacy.as_str(), "public" | "private" | "sensitive")
            })
            && self.freshness.is_valid_evidence()
            && bounded_identity_reference(&self.version_token_hash, 256)
            && bounded_identity_reference(&self.redaction_class, 96)
    }
}
