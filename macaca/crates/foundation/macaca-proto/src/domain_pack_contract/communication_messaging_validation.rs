use super::communication_common::{bounded_communication_token, optional_secret_reference_is_safe};
use super::communication_messaging::{
    MessagingAttachmentRef, MessagingContent, MessagingConversationRef, MessagingCursor,
    MessagingIngestEventCommand, MessagingParticipantRef, MessagingProviderEventRef,
    MessagingSendMessageCommand, MessagingSendTypingCommand, MessagingSenderRef,
};

impl MessagingConversationRef {
    /// Validate conversation references without provider-native channel payloads.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.conversation_id, 160)
            && bounded_communication_token(&self.provider_class, 96)
            && bounded_communication_token(&self.tenant_scope, 160)
            && matches!(self.visibility.as_str(), "private" | "shared" | "public")
    }
}

impl MessagingParticipantRef {
    /// Validate participant handles and consent metadata.
    pub fn is_deliverable_reference(&self) -> bool {
        bounded_communication_token(&self.participant_id, 160)
            && matches!(
                self.participant_kind.as_str(),
                "user" | "agent" | "service" | "external"
            )
            && bounded_communication_token(&self.display_hash, 256)
            && matches!(self.consent_state.as_str(), "granted" | "unknown")
    }
}

impl MessagingSenderRef {
    /// Validate sender identity and ensure credentials are secret-store references only.
    pub fn has_safe_credentials(&self) -> bool {
        bounded_communication_token(&self.sender_id, 160)
            && self.verified
            && bounded_communication_token(&self.provider_class, 96)
            && optional_secret_reference_is_safe(self.secret_ref.as_deref())
    }
}

impl MessagingContent {
    /// Validate message content as references, not raw message text.
    pub fn is_reference_only(&self) -> bool {
        bounded_communication_token(&self.fallback_text_ref, 256)
            && self
                .content_ref
                .as_deref()
                .is_none_or(|reference| bounded_communication_token(reference, 256))
            && matches!(
                self.format.as_str(),
                "text" | "markdown" | "blocks" | "reference"
            )
            && bounded_communication_token(&self.formatting_policy, 160)
    }
}

impl MessagingAttachmentRef {
    /// Validate attachment references and size bounds before provider dispatch.
    pub fn is_within_limit(&self, max_attachment_bytes: u64) -> bool {
        bounded_communication_token(&self.attachment_id, 160)
            && bounded_communication_token(&self.content_ref, 256)
            && bounded_communication_token(&self.content_type, 128)
            && self.size_bytes <= max_attachment_bytes
    }
}

impl MessagingCursor {
    /// Validate pagination cursors as hashes only.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.conversation_id, 160)
            && bounded_communication_token(&self.cursor_hash, 256)
    }
}

impl MessagingProviderEventRef {
    /// Validate provider events as signed handles without raw webhook bodies.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.event_id_hash, 256)
            && bounded_communication_token(&self.provider_class, 96)
            && matches!(
                self.signature_status.as_str(),
                "verified" | "not_configured" | "unavailable"
            )
    }
}

impl MessagingSendMessageCommand {
    /// Validate send requests with approval, idempotency, and bounded content handles.
    pub fn has_admission_preconditions(
        &self,
        max_attachments: usize,
        max_attachment_bytes: u64,
    ) -> bool {
        self.sender.has_safe_credentials()
            && self.conversation.is_safe_reference()
            && self.content.is_reference_only()
            && self.attachments.len() <= max_attachments
            && self
                .attachments
                .iter()
                .all(|attachment| attachment.is_within_limit(max_attachment_bytes))
            && self
                .approval_ref
                .as_deref()
                .is_some_and(|approval| bounded_communication_token(approval, 160))
            && bounded_communication_token(&self.idempotency_key, 128)
    }
}

impl MessagingSendTypingCommand {
    /// Validate typing notifications remain short-lived and conversation-scoped.
    pub fn has_bounded_ttl(&self, max_ttl_ms: u64) -> bool {
        self.conversation.is_safe_reference() && self.ttl_ms > 0 && self.ttl_ms <= max_ttl_ms
    }
}

impl MessagingIngestEventCommand {
    /// Validate event ingestion with idempotency and signature metadata.
    pub fn has_ingest_preconditions(&self) -> bool {
        self.event.is_safe_reference() && bounded_communication_token(&self.idempotency_key, 128)
    }
}
