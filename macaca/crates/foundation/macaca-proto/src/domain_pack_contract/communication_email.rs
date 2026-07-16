use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::communication_common::{
    communication_pack_definition, communication_stable_hash, schema_set,
    CommunicationPackDescriptor, CommunicationProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

/// Stable pack id for provider-neutral email operations.
pub const COMMUNICATION_EMAIL_PACK_ID: &str = "pack.communication.email.v1";
/// Stable service id used by future email providers.
pub const COMMUNICATION_EMAIL_SERVICE_ID: &str = "service.communication.email";

/// Canonical command names described by `pack.communication.email.v1`.
///
/// These commands are descriptor-owned schema identifiers. Concrete SMTP, mailbox, transactional,
/// event, mock, or unavailable providers remain service-runtime implementations.
pub const COMMUNICATION_EMAIL_COMMANDS: &[&str] = &[
    "email.compose",
    "email.validate_recipients",
    "email.save_draft",
    "email.update_draft",
    "email.send",
    "email.schedule_send",
    "email.cancel_scheduled_send",
    "email.sync_mailbox",
    "email.list_threads",
    "email.fetch_message",
    "email.fetch_attachment",
    "email.apply_labels",
    "email.mark_read",
    "email.delivery_status",
    "email.ingest_event",
];

const EMAIL_PERMISSION_SCOPES: &[&str] = &[
    "email.send",
    "email.read",
    "email.draft",
    "email.attachment",
    "email.mailbox.sync",
    "email.mailbox.mutate",
    "email.delivery.read",
    "email.event.ingest",
];

const TRANSACTIONAL_MAIL_METADATA: &[(&str, &str)] = &[
    ("send", "true"),
    ("draft", "false"),
    ("mailbox_sync", "false"),
    ("event_ingest", "true"),
];
const MAILBOX_SYNC_METADATA: &[(&str, &str)] = &[
    ("send", "true"),
    ("draft", "true"),
    ("mailbox_sync", "true"),
    ("event_ingest", "true"),
];
const EMAIL_EVENT_METADATA: &[(&str, &str)] = &[
    ("send", "false"),
    ("draft", "false"),
    ("mailbox_sync", "false"),
    ("event_ingest", "true"),
];
const EMAIL_MOCK_METADATA: &[(&str, &str)] = &[
    ("send", "true"),
    ("draft", "true"),
    ("mailbox_sync", "true"),
    ("event_ingest", "true"),
];
const EMAIL_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("send", "false"),
    ("draft", "false"),
    ("mailbox_sync", "false"),
    ("event_ingest", "false"),
];

const EMAIL_PROVIDER_CLASSES: &[CommunicationProviderClass<'_>] = &[
    CommunicationProviderClass {
        provider_class: "transactional-mail",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TRANSACTIONAL_MAIL_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "mailbox-sync",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MAILBOX_SYNC_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "event-ingest",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: EMAIL_EVENT_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: EMAIL_MOCK_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: EMAIL_UNAVAILABLE_METADATA,
    },
];

/// Build the descriptor-only catalog entry for `pack.communication.email.v1`.
pub fn communication_email_pack_definition() -> DomainPackDefinition {
    communication_pack_definition(CommunicationPackDescriptor {
        slug: "email",
        service_id: COMMUNICATION_EMAIL_SERVICE_ID,
        commands: COMMUNICATION_EMAIL_COMMANDS,
        permission_scopes: EMAIL_PERMISSION_SCOPES,
        provider_classes: EMAIL_PROVIDER_CLASSES,
        health_probe: "email.delivery_status",
        unavailable_reason: "email_provider_not_installed",
        replay_schema: "email.pack.replay.v1",
        data_classification: "communication_message_metadata",
        retention_policy: "content_by_reference_delivery_metadata_only",
        redaction_policy: "credentials_provider_payloads_message_bodies_and_attachments_redacted",
        examples: &[
            "Declare `pack.communication.email.v1` as optional until an email provider is installed.",
            "Use artifact and attachment references instead of raw attachment bytes.",
        ],
        migration_notes: &[
            "Email becomes callable only after an approved communication service provider registers command schemas.",
            "Provider-native SMTP, mailbox, webhook, and transactional payloads must stay behind provider adapters.",
        ],
    })
}

/// Sender identity bound to a provider-owned credential or mailbox account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailSenderRef {
    pub sender_id: String,
    pub address_hash: String,
    pub verified: bool,
    pub provider_class: String,
    pub secret_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailRecipientKind {
    To,
    Cc,
    Bcc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailConsentStatus {
    Unknown,
    Granted,
    Required,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailRecipient {
    pub kind: EmailRecipientKind,
    pub address_hash: String,
    pub display_name: Option<String>,
    pub consent: EmailConsentStatus,
    pub domain_policy: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailBodyKind {
    Text,
    Html,
    Markdown,
    Reference,
}

/// Body content reference that keeps full message text out of traces and snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailBodyPart {
    pub kind: EmailBodyKind,
    pub content_ref: String,
    pub language: Option<String>,
    pub redaction_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailAttachmentRef {
    pub attachment_id: String,
    pub content_ref: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub checksum: Option<String>,
    pub inline_content_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailMessageRef {
    pub message_id: String,
    pub thread_id: Option<String>,
    pub folder_id: Option<String>,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailDraftRef {
    pub draft_id: String,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailSyncCursor {
    pub mailbox_id: String,
    pub cursor_hash: String,
    pub provider_class: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailDeliveryState {
    Accepted,
    Queued,
    Scheduled,
    Sent,
    Delivered,
    Bounced,
    Complained,
    Deferred,
    Failed,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailProviderEventRef {
    pub event_id_hash: String,
    pub provider_class: String,
    pub signature_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailRateLimitStatus {
    pub bucket: String,
    pub remaining: u32,
    pub reset_epoch_ms: Option<i128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    pub supports_drafts: bool,
    pub supports_scheduled_send: bool,
    pub supports_mailbox_sync: bool,
    pub supports_event_ingest: bool,
    pub max_attachment_bytes: u64,
    pub max_recipients: u32,
    pub availability: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailProviderSnapshot {
    pub descriptor_hash: String,
    pub provider_class: String,
    pub sender_identity_count: u32,
    pub rate_limits: BTreeMap<String, EmailRateLimitStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailComposeCommand {
    pub sender: EmailSenderRef,
    pub recipients: Vec<EmailRecipient>,
    pub subject_ref: String,
    pub body_parts: Vec<EmailBodyPart>,
    pub attachments: Vec<EmailAttachmentRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailValidateRecipientsCommand {
    pub sender: EmailSenderRef,
    pub recipients: Vec<EmailRecipient>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailSaveDraftCommand {
    pub compose: EmailComposeCommand,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailUpdateDraftCommand {
    pub draft: EmailDraftRef,
    pub compose: EmailComposeCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailSendCommand {
    pub message: Option<EmailMessageRef>,
    pub draft: Option<EmailDraftRef>,
    pub approval_ref: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailScheduleSendCommand {
    pub send: EmailSendCommand,
    pub send_at_epoch_ms: i128,
    pub timezone_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailCancelScheduledSendCommand {
    pub scheduled_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailSyncMailboxCommand {
    pub mailbox_id: String,
    pub cursor: Option<EmailSyncCursor>,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailListThreadsCommand {
    pub mailbox_id: String,
    pub label_ids: Vec<String>,
    pub cursor: Option<EmailSyncCursor>,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailFetchMessageCommand {
    pub message: EmailMessageRef,
    pub body_projection: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailFetchAttachmentCommand {
    pub attachment: EmailAttachmentRef,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailApplyLabelsCommand {
    pub messages: Vec<EmailMessageRef>,
    pub add_labels: Vec<String>,
    pub remove_labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailMarkReadCommand {
    pub messages: Vec<EmailMessageRef>,
    pub read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailDeliveryStatusCommand {
    pub message: EmailMessageRef,
    pub event: Option<EmailProviderEventRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailIngestEventCommand {
    pub event: EmailProviderEventRef,
    pub normalized_state: EmailDeliveryState,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmailResultStatus {
    Success,
    PartialPage,
    Denied,
    InvalidSender,
    InvalidRecipient,
    ConsentRequired,
    AttachmentTooLarge,
    Unsupported,
    RateLimited,
    ProviderRejected,
    Unavailable,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailError {
    pub code: EmailResultStatus,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailResultEnvelope<T> {
    pub status: EmailResultStatus,
    pub data: Option<T>,
    pub error: Option<EmailError>,
    pub trace_id: String,
    pub descriptor_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub snapshot_schema_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

/// Return deterministic hashes for the email contract schema surface.
pub fn communication_email_descriptor_hashes() -> EmailDescriptorHashes {
    EmailDescriptorHashes {
        command_schema_hash: email_stable_hash(&COMMUNICATION_EMAIL_COMMANDS),
        result_schema_hash: email_stable_hash(&EmailResultStatus::Success),
        snapshot_schema_hash: email_stable_hash(&EmailProviderSnapshot {
            descriptor_hash: "descriptor".into(),
            provider_class: "unavailable".into(),
            sender_identity_count: 0,
            rate_limits: BTreeMap::new(),
        }),
        provider_capability_schema_hash: email_stable_hash(&EmailProviderCapability {
            provider_class: "unavailable".into(),
            supported_commands: schema_set(COMMUNICATION_EMAIL_COMMANDS),
            supports_drafts: false,
            supports_scheduled_send: false,
            supports_mailbox_sync: false,
            supports_event_ingest: false,
            max_attachment_bytes: 0,
            max_recipients: 0,
            availability: DomainPackProviderCapabilityState::Unavailable,
        }),
        unavailable_schema_hash: email_stable_hash(&EmailError {
            code: EmailResultStatus::Unavailable,
            message: "email provider is not installed".into(),
            retryable: false,
        }),
    }
}

pub fn email_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    communication_stable_hash(value)
}
