use super::communication_common::{bounded_communication_token, optional_secret_reference_is_safe};
use super::communication_email::{
    EmailAttachmentRef, EmailBodyPart, EmailComposeCommand, EmailConsentStatus,
    EmailFetchAttachmentCommand, EmailProviderEventRef, EmailRecipient, EmailSaveDraftCommand,
    EmailScheduleSendCommand, EmailSendCommand, EmailSenderRef, EmailSyncMailboxCommand,
};

impl EmailSenderRef {
    /// Validate sender identity and ensure credentials are secret-store references only.
    pub fn has_safe_credentials(&self) -> bool {
        bounded_communication_token(&self.sender_id, 160)
            && bounded_communication_token(&self.address_hash, 256)
            && self.verified
            && bounded_communication_token(&self.provider_class, 96)
            && optional_secret_reference_is_safe(self.secret_ref.as_deref())
    }
}

impl EmailRecipient {
    /// Validate recipients using address hashes and consent/domain policy metadata.
    pub fn is_deliverable_reference(&self) -> bool {
        bounded_communication_token(&self.address_hash, 256)
            && !matches!(self.consent, EmailConsentStatus::Denied)
            && bounded_communication_token(&self.domain_policy, 96)
    }
}

impl EmailBodyPart {
    /// Validate message bodies as content references, not raw text or HTML payloads.
    pub fn is_reference_only(&self) -> bool {
        bounded_communication_token(&self.content_ref, 256)
            && bounded_communication_token(&self.redaction_policy, 160)
            && self
                .language
                .as_deref()
                .is_none_or(|language| bounded_communication_token(language, 32))
    }
}

impl EmailAttachmentRef {
    /// Validate attachment handles before fetch/send operations.
    pub fn is_within_limit(&self, max_attachment_bytes: u64) -> bool {
        bounded_communication_token(&self.attachment_id, 160)
            && bounded_communication_token(&self.content_ref, 256)
            && bounded_communication_token(&self.content_type, 128)
            && self.size_bytes <= max_attachment_bytes
            && self
                .checksum
                .as_deref()
                .is_none_or(|checksum| bounded_communication_token(checksum, 256))
    }
}

impl EmailComposeCommand {
    /// Validate compose input using sender, recipient, body, and attachment references only.
    pub fn has_admission_preconditions(
        &self,
        max_recipients: usize,
        max_attachment_bytes: u64,
    ) -> bool {
        self.sender.has_safe_credentials()
            && !self.recipients.is_empty()
            && self.recipients.len() <= max_recipients
            && self
                .recipients
                .iter()
                .all(EmailRecipient::is_deliverable_reference)
            && bounded_communication_token(&self.subject_ref, 256)
            && !self.body_parts.is_empty()
            && self.body_parts.iter().all(EmailBodyPart::is_reference_only)
            && self
                .attachments
                .iter()
                .all(|attachment| attachment.is_within_limit(max_attachment_bytes))
    }
}

impl EmailSaveDraftCommand {
    /// Validate idempotent draft creation without invoking a provider.
    pub fn has_admission_preconditions(
        &self,
        max_recipients: usize,
        max_attachment_bytes: u64,
    ) -> bool {
        self.compose
            .has_admission_preconditions(max_recipients, max_attachment_bytes)
            && bounded_communication_token(&self.idempotency_key, 128)
    }
}

impl EmailSendCommand {
    /// Validate send requests require approval and idempotency.
    pub fn has_send_preconditions(&self) -> bool {
        self.approval_ref
            .as_deref()
            .is_some_and(|approval| bounded_communication_token(approval, 160))
            && bounded_communication_token(&self.idempotency_key, 128)
            && (self.message.is_some() ^ self.draft.is_some())
    }
}

impl EmailScheduleSendCommand {
    /// Validate scheduled sends as approved, timezone-bound commands.
    pub fn has_schedule_preconditions(&self) -> bool {
        self.send.has_send_preconditions()
            && self.send_at_epoch_ms > 0
            && bounded_communication_token(&self.timezone_id, 96)
    }
}

impl EmailSyncMailboxCommand {
    /// Validate mailbox sync pagination and cursor references.
    pub fn has_bounded_pagination(&self, max_page_size: u32) -> bool {
        bounded_communication_token(&self.mailbox_id, 160)
            && self.page_size > 0
            && self.page_size <= max_page_size
            && self.cursor.as_ref().is_none_or(|cursor| {
                bounded_communication_token(&cursor.mailbox_id, 160)
                    && bounded_communication_token(&cursor.cursor_hash, 256)
                    && bounded_communication_token(&cursor.provider_class, 96)
            })
    }
}

impl EmailFetchAttachmentCommand {
    /// Validate attachment reads do not exceed the caller's bounded byte window.
    pub fn has_bounded_fetch(&self, max_attachment_bytes: u64) -> bool {
        self.max_bytes <= max_attachment_bytes
            && self.attachment.is_within_limit(max_attachment_bytes)
    }
}

impl EmailProviderEventRef {
    /// Validate delivery events as signed provider event handles only.
    pub fn is_safe_reference(&self) -> bool {
        bounded_communication_token(&self.event_id_hash, 256)
            && bounded_communication_token(&self.provider_class, 96)
            && matches!(
                self.signature_status.as_str(),
                "verified" | "not_configured" | "unavailable"
            )
    }
}
