use super::communication_common::{bounded_communication_token, optional_secret_reference_is_safe};
use super::communication_inbox::{
    InboxAttachmentHandle, InboxClaim, InboxClaimItemCommand, InboxCursor, InboxEvent,
    InboxFetchAttachmentCommand, InboxFetchBodyCommand, InboxItem, InboxSource,
    InboxSummarizeItemCommand, InboxSyncCheckpoint,
};

impl InboxSource {
    /// Validate source metadata and require credentials through secret references only.
    pub fn has_safe_credentials(&self) -> bool {
        bounded_communication_token(&self.source_id, 160)
            && matches!(
                self.source_kind.as_str(),
                "mailbox" | "conversation" | "ticket" | "notification"
            )
            && bounded_communication_token(&self.provider_class, 96)
            && optional_secret_reference_is_safe(self.credential_secret_ref.as_deref())
            && matches!(
                self.health.as_str(),
                "available" | "degraded" | "unavailable"
            )
    }
}

impl InboxCursor {
    /// Validate inbox cursors as hashes and watermarks only.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.source_id, 160)
            && bounded_communication_token(&self.cursor_hash, 256)
            && self
                .watermark_hash
                .as_deref()
                .is_none_or(|hash| bounded_communication_token(hash, 256))
            && self.expires_epoch_ms.is_none_or(|expiry| expiry > 0)
    }
}

impl InboxItem {
    /// Validate item metadata without exposing raw bodies or attachment bytes.
    pub fn is_safe_projection(&self, max_labels: usize) -> bool {
        bounded_communication_token(&self.item_id, 160)
            && bounded_communication_token(&self.source_id, 160)
            && bounded_communication_token(&self.sender_hash, 256)
            && self
                .subject_ref
                .as_deref()
                .is_none_or(|reference| bounded_communication_token(reference, 256))
            && self
                .preview_ref
                .as_deref()
                .is_none_or(|reference| bounded_communication_token(reference, 256))
            && self.label_ids.len() <= max_labels
            && self
                .label_ids
                .iter()
                .all(|label| bounded_communication_token(label, 160))
    }
}

impl InboxAttachmentHandle {
    /// Validate attachment handles before a source adapter can fetch content.
    pub fn is_within_limit(&self, max_bytes: u64) -> bool {
        bounded_communication_token(&self.item_id, 160)
            && bounded_communication_token(&self.part_id, 160)
            && bounded_communication_token(&self.filename_hash, 256)
            && bounded_communication_token(&self.mime_type, 128)
            && self.size_bytes <= max_bytes
            && self
                .content_ref
                .as_deref()
                .is_none_or(|reference| bounded_communication_token(reference, 256))
    }
}

impl InboxEvent {
    /// Validate source events as idempotent handles, not raw webhook bodies.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.source_id, 160)
            && bounded_communication_token(&self.event_id_hash, 256)
            && matches!(
                self.mutation_type.as_str(),
                "created" | "updated" | "deleted" | "moved"
            )
            && bounded_communication_token(&self.idempotency_key, 128)
    }
}

impl InboxClaim {
    /// Validate claim leases and ownership references.
    pub fn is_active_lease(&self) -> bool {
        bounded_communication_token(&self.item_id, 160)
            && bounded_communication_token(&self.owner_ref, 160)
            && self.lease_expires_epoch_ms > 0
            && matches!(
                self.claim_state.as_str(),
                "claimed" | "released" | "expired"
            )
    }
}

impl InboxSyncCheckpoint {
    /// Validate resumable sync checkpoints as cursor and checkpoint hashes.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.source_id, 160)
            && self.cursor.is_safe_reference()
            && bounded_communication_token(&self.checkpoint_hash, 256)
    }
}

impl InboxFetchBodyCommand {
    /// Validate body fetches stay within bounded byte windows.
    pub fn has_bounded_fetch(&self, max_bytes: u64) -> bool {
        bounded_communication_token(&self.item_id, 160)
            && bounded_communication_token(&self.body_part, 160)
            && self.max_bytes <= max_bytes
    }
}

impl InboxFetchAttachmentCommand {
    /// Validate attachment fetches stay within bounded byte windows.
    pub fn has_bounded_fetch(&self, max_bytes: u64) -> bool {
        self.max_bytes <= max_bytes && self.attachment.is_within_limit(max_bytes)
    }
}

impl InboxClaimItemCommand {
    /// Validate claim requests before creating a processing lease.
    pub fn has_claim_preconditions(&self, max_lease_ms: u64) -> bool {
        bounded_communication_token(&self.item_id, 160)
            && bounded_communication_token(&self.owner_ref, 160)
            && self.lease_ms > 0
            && self.lease_ms <= max_lease_ms
    }
}

impl InboxSummarizeItemCommand {
    /// Validate delegated summarization uses pack references and redaction profiles only.
    pub fn has_delegation_preconditions(&self) -> bool {
        bounded_communication_token(&self.item_id, 160)
            && bounded_communication_token(&self.redaction_profile, 160)
            && self.delegated_pack_id.starts_with("pack.")
            && bounded_communication_token(&self.delegated_pack_id, 160)
    }
}
