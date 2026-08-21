use super::foundation_session_state::{
    SessionStateCreateCheckpointCommand, SessionStateKeyRef, SessionStateManifestDeclaration,
    SessionStateRetentionPolicy, SessionStateSessionRef, SessionStateValueRef,
    FOUNDATION_SESSION_STATE_PACK_ID,
};
use super::foundation_validation::{
    bounded_reference, opaque_artifact_reference, secret_store_reference,
};
use super::model::AppServiceContractConfig;

/// Validate manifest declarations before ABI projection or provider selection.
///
/// Required feature flags are accepted as provider-neutral admission facts;
/// catalog availability remains checked by the existing required/optional pack
/// projection. Duplicate session identities and unbounded retention are rejected
/// before an application obtains a callable service import.
pub fn validate_session_state_declarations(
    declaration: &AppServiceContractConfig,
) -> Result<(), &'static str> {
    let declared = declaration
        .use_packs
        .iter()
        .chain(declaration.required_packs.iter())
        .chain(declaration.optional_packs.iter())
        .any(|pack_id| pack_id == FOUNDATION_SESSION_STATE_PACK_ID);
    if !declaration.session_state_declarations.is_empty() && !declared {
        return Err("session-state declarations require the foundation session-state pack");
    }
    let mut sessions = std::collections::BTreeSet::new();
    for session in &declaration.session_state_declarations {
        if !session.is_admissible() {
            return Err("session-state declaration is invalid or out of bounds");
        }
        if !sessions.insert((
            &session.session.session_id,
            session.session.task_id.as_deref(),
        )) {
            return Err("session-state declarations must be unique per session scope");
        }
    }
    Ok(())
}

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

impl SessionStateManifestDeclaration {
    /// Validate bounded scope and retention without consulting a concrete provider.
    pub fn is_admissible(&self) -> bool {
        self.session.is_bounded_reference() && self.retention.is_bounded(31_536_000, 128, 100_000)
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
