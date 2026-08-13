use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::communication_common::{
    communication_pack_definition, communication_stable_hash, schema_set,
    CommunicationPackDescriptor, CommunicationProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

/// Stable pack id for provider-neutral messaging operations.
pub const COMMUNICATION_MESSAGING_PACK_ID: &str = "pack.communication.messaging.v1";
/// Stable service id used by future messaging providers.
pub const COMMUNICATION_MESSAGING_SERVICE_ID: &str = "service.communication.messaging";

/// Canonical command names described by `pack.communication.messaging.v1`.
pub const COMMUNICATION_MESSAGING_COMMANDS: &[&str] = &[
    "messaging.find_conversation",
    "messaging.create_conversation",
    "messaging.inspect_participants",
    "messaging.send_message",
    "messaging.reply_message",
    "messaging.edit_message",
    "messaging.delete_message",
    "messaging.list_messages",
    "messaging.fetch_message",
    "messaging.add_reaction",
    "messaging.remove_reaction",
    "messaging.mark_read",
    "messaging.attach_handle",
    "messaging.delivery_status",
    "messaging.send_typing",
    "messaging.ingest_event",
];

const MESSAGING_PERMISSION_SCOPES: &[&str] = &[
    "messaging.send",
    "messaging.read",
    "messaging.conversation.manage",
    "messaging.edit",
    "messaging.delete",
    "messaging.reaction",
    "messaging.attachment",
    "messaging.read_receipt",
    "messaging.delivery.read",
    "messaging.typing",
    "messaging.event.ingest",
];

const CONVERSATION_BRIDGE_METADATA: &[(&str, &str)] = &[
    ("send", "true"),
    ("conversation_manage", "true"),
    ("reaction", "true"),
    ("event_ingest", "false"),
];
const DELIVERY_BRIDGE_METADATA: &[(&str, &str)] = &[
    ("send", "true"),
    ("conversation_manage", "false"),
    ("reaction", "false"),
    ("event_ingest", "true"),
];
const MESSAGING_EVENT_METADATA: &[(&str, &str)] = &[
    ("send", "false"),
    ("conversation_manage", "false"),
    ("reaction", "true"),
    ("event_ingest", "true"),
];
const MESSAGING_MOCK_METADATA: &[(&str, &str)] = &[
    ("send", "true"),
    ("conversation_manage", "true"),
    ("reaction", "true"),
    ("event_ingest", "true"),
];
const MESSAGING_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("send", "false"),
    ("conversation_manage", "false"),
    ("reaction", "false"),
    ("event_ingest", "false"),
];

const MESSAGING_PROVIDER_CLASSES: &[CommunicationProviderClass<'_>] = &[
    CommunicationProviderClass {
        provider_class: "conversation-bridge",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CONVERSATION_BRIDGE_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "delivery-bridge",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: DELIVERY_BRIDGE_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "event-ingest",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MESSAGING_EVENT_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MESSAGING_MOCK_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: MESSAGING_UNAVAILABLE_METADATA,
    },
];

/// Build the descriptor-only catalog entry for `pack.communication.messaging.v1`.
pub fn communication_messaging_pack_definition() -> DomainPackDefinition {
    communication_pack_definition(CommunicationPackDescriptor {
        slug: "messaging",
        service_id: COMMUNICATION_MESSAGING_SERVICE_ID,
        commands: COMMUNICATION_MESSAGING_COMMANDS,
        permission_scopes: MESSAGING_PERMISSION_SCOPES,
        provider_classes: MESSAGING_PROVIDER_CLASSES,
        health_probe: "messaging.delivery_status",
        unavailable_reason: "messaging_provider_not_installed",
        replay_schema: "messaging.pack.replay.v1",
        data_classification: "communication_conversation_metadata",
        retention_policy: "message_content_by_reference_delivery_metadata_only",
        redaction_policy: "tokens_provider_payloads_message_bodies_and_attachments_redacted",
        examples: &[
            "Declare `pack.communication.messaging.v1` as optional until a messaging provider is installed.",
            "Use fallback text and artifact handles instead of provider-native rich payloads.",
        ],
        migration_notes: &[
            "Messaging becomes callable only after an approved communication provider registers command schemas.",
            "Provider-native channel, bot, webhook, and SMS payloads must stay behind provider adapters.",
        ],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessagingConversationKind {
    Channel,
    Direct,
    Group,
    Bot,
    Webhook,
    Sms,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingConversationRef {
    pub conversation_id: String,
    pub provider_class: String,
    pub kind: MessagingConversationKind,
    pub tenant_scope: String,
    pub visibility: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingParticipantRef {
    pub participant_id: String,
    pub participant_kind: String,
    pub display_hash: String,
    pub consent_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingSenderRef {
    pub sender_id: String,
    pub verified: bool,
    pub provider_class: String,
    pub secret_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingContent {
    pub fallback_text_ref: String,
    pub content_ref: Option<String>,
    pub format: String,
    pub formatting_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingAttachmentRef {
    pub attachment_id: String,
    pub content_ref: String,
    pub content_type: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingMessageRef {
    pub message_id: String,
    pub conversation_id: String,
    pub thread_id: Option<String>,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingReaction {
    pub reaction_key: String,
    pub actor_id: String,
    pub provider_representation_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingCursor {
    pub conversation_id: String,
    pub cursor_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessagingDeliveryState {
    Accepted,
    Queued,
    Sent,
    Delivered,
    Read,
    Failed,
    Deleted,
    Edited,
    RateLimited,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingProviderEventRef {
    pub event_id_hash: String,
    pub provider_class: String,
    pub signature_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingRateLimitStatus {
    pub bucket: String,
    pub remaining: u32,
    pub reset_epoch_ms: Option<i128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    pub supported_conversation_kinds: BTreeSet<MessagingConversationKind>,
    pub supports_reactions: bool,
    pub supports_typing: bool,
    pub supports_event_ingest: bool,
    /// Whether the adapter supports opaque attachment handles rather than bytes.
    #[serde(default)]
    pub supports_attachment_handles: bool,
    /// Whether the adapter can resume conversation reads using cursor hashes.
    #[serde(default)]
    pub supports_cursors: bool,
    /// Whether service lifecycle health is reported by this adapter.
    #[serde(default)]
    pub supports_health: bool,
    /// Bounded formatting classes supported by the adapter's normalized renderer.
    #[serde(default)]
    pub supported_formats: BTreeSet<String>,
    /// Descriptor-owned rate-limit bucket with no provider account identity.
    #[serde(default)]
    pub rate_limit_bucket: String,
    pub max_attachment_bytes: u64,
    pub max_message_bytes: u64,
    pub availability: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingProviderSnapshot {
    pub descriptor_hash: String,
    pub provider_class: String,
    pub active_conversation_count: u32,
    pub rate_limits: BTreeMap<String, MessagingRateLimitStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingFindConversationCommand {
    pub selector: String,
    pub kind: Option<MessagingConversationKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingCreateConversationCommand {
    pub participants: Vec<MessagingParticipantRef>,
    pub topic_ref: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingInspectParticipantsCommand {
    pub conversation: MessagingConversationRef,
    pub page_size: u32,
    pub cursor: Option<MessagingCursor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingSendMessageCommand {
    pub sender: MessagingSenderRef,
    pub conversation: MessagingConversationRef,
    pub content: MessagingContent,
    pub attachments: Vec<MessagingAttachmentRef>,
    pub approval_ref: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingReplyMessageCommand {
    pub parent: MessagingMessageRef,
    pub send: MessagingSendMessageCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingEditMessageCommand {
    pub message: MessagingMessageRef,
    pub content: MessagingContent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingDeleteMessageCommand {
    pub message: MessagingMessageRef,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingListMessagesCommand {
    pub conversation: MessagingConversationRef,
    pub cursor: Option<MessagingCursor>,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingFetchMessageCommand {
    pub message: MessagingMessageRef,
    pub projection: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingReactionCommand {
    pub message: MessagingMessageRef,
    pub reaction: MessagingReaction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingMarkReadCommand {
    pub conversation: MessagingConversationRef,
    pub message: Option<MessagingMessageRef>,
    pub read_position: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingAttachHandleCommand {
    pub message: MessagingMessageRef,
    pub attachment: MessagingAttachmentRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingDeliveryStatusCommand {
    pub message: MessagingMessageRef,
    pub event: Option<MessagingProviderEventRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingSendTypingCommand {
    pub conversation: MessagingConversationRef,
    pub ttl_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingIngestEventCommand {
    pub event: MessagingProviderEventRef,
    pub state: MessagingDeliveryState,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessagingResultStatus {
    Success,
    PartialPage,
    Denied,
    InvalidConversation,
    InvalidRecipient,
    UnsupportedFormat,
    UnsupportedCommand,
    ConsentRequired,
    AttachmentTooLarge,
    RateLimited,
    ProviderRejected,
    Unavailable,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingError {
    pub code: MessagingResultStatus,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingResultEnvelope<T> {
    pub status: MessagingResultStatus,
    pub data: Option<T>,
    pub error: Option<MessagingError>,
    pub trace_id: String,
    pub descriptor_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessagingDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub snapshot_schema_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn communication_messaging_descriptor_hashes() -> MessagingDescriptorHashes {
    MessagingDescriptorHashes {
        command_schema_hash: messaging_stable_hash(&COMMUNICATION_MESSAGING_COMMANDS),
        result_schema_hash: messaging_stable_hash(&MessagingResultStatus::Success),
        snapshot_schema_hash: messaging_stable_hash(&MessagingProviderSnapshot {
            descriptor_hash: "descriptor".into(),
            provider_class: "unavailable".into(),
            active_conversation_count: 0,
            rate_limits: BTreeMap::new(),
        }),
        provider_capability_schema_hash: messaging_stable_hash(&MessagingProviderCapability {
            provider_class: "unavailable".into(),
            supported_commands: schema_set(COMMUNICATION_MESSAGING_COMMANDS),
            supported_conversation_kinds: BTreeSet::from([MessagingConversationKind::Channel]),
            supports_reactions: false,
            supports_typing: false,
            supports_event_ingest: false,
            supports_attachment_handles: false,
            supports_cursors: false,
            supports_health: false,
            supported_formats: BTreeSet::new(),
            rate_limit_bucket: "unavailable".into(),
            max_attachment_bytes: 0,
            max_message_bytes: 0,
            availability: DomainPackProviderCapabilityState::Unavailable,
        }),
        unavailable_schema_hash: messaging_stable_hash(&MessagingError {
            code: MessagingResultStatus::Unavailable,
            message: "messaging provider is not installed".into(),
            retryable: false,
        }),
    }
}

pub fn messaging_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    communication_stable_hash(value)
}
