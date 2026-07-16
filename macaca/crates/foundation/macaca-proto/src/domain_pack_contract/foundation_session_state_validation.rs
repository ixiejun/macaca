use super::foundation_session_state::{
    SessionStateCreateCheckpointCommand, SessionStateKeyRef, SessionStateRetentionPolicy,
    SessionStateSessionRef, SessionStateValueRef,
};
use super::foundation_validation::{
    bounded_reference, opaque_artifact_reference, secret_store_reference,
};

impl SessionStateSessionRef {
    /// Validate session/task identities as bounded logical references.
    pub fn is_bounded_reference(&self) -> bool {
        bounded_reference(&self.session_id, 160)
            && self
                .task_id
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 160))
    }
}

impl SessionStateKeyRef {
    /// Bound state-key admission before session-store dispatch.
    pub fn is_bounded_reference(&self) -> bool {
        self.session.is_bounded_reference() && bounded_reference(&self.key, 256)
    }
}

impl SessionStateValueRef {
    /// Preserve redaction by requiring opaque artifacts or secret-store references.
    pub fn is_admissible_reference(&self) -> bool {
        let is_secret = secret_store_reference(&self.value_ref);
        opaque_artifact_reference(&self.value_ref)
            && self
                .schema_id
                .as_deref()
                .is_none_or(|value| bounded_reference(value, 160))
            && self.secret_reference_required == is_secret
    }
}

impl SessionStateRetentionPolicy {
    /// Bound retention before checkpoint reservation and cleanup scheduling.
    pub fn is_bounded(
        &self,
        max_ttl_seconds: u64,
        max_checkpoints: u32,
        max_revisions: u32,
    ) -> bool {
        self.ttl_seconds
            .is_none_or(|ttl| ttl > 0 && ttl <= max_ttl_seconds)
            && self.max_checkpoints > 0
            && self.max_checkpoints <= max_checkpoints
            && self.compact_after_revisions > 0
            && self.compact_after_revisions <= max_revisions
    }
}

impl SessionStateCreateCheckpointCommand {
    /// Validate checkpoint identity and bounded cleanup policy before side effects.
    pub fn has_safe_preconditions(
        &self,
        max_ttl_seconds: u64,
        max_checkpoints: u32,
        max_revisions: u32,
    ) -> bool {
        self.session.is_bounded_reference()
            && self
                .retention
                .is_bounded(max_ttl_seconds, max_checkpoints, max_revisions)
    }
}
