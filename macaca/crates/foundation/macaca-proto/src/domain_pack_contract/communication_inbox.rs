use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::communication_common::{
    communication_pack_definition, communication_stable_hash, schema_set,
    CommunicationPackDescriptor, CommunicationProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

/// Stable pack id for provider-neutral inbox aggregation operations.
pub const COMMUNICATION_INBOX_PACK_ID: &str = "pack.communication.inbox.v1";
/// Stable service id used by future inbox aggregation providers.
pub const COMMUNICATION_INBOX_SERVICE_ID: &str = "service.communication.inbox";

/// Canonical command names described by `pack.communication.inbox.v1`.
pub const COMMUNICATION_INBOX_COMMANDS: &[&str] = &[
    "inbox.register_source",
    "inbox.update_source",
    "inbox.revoke_source",
    "inbox.sync_sources",
    "inbox.resume_sync",
    "inbox.ingest_event",
    "inbox.list_items",
    "inbox.search_items",
    "inbox.get_item",
    "inbox.fetch_body",
    "inbox.fetch_attachment",
    "inbox.list_threads",
    "inbox.label_item",
    "inbox.move_item",
    "inbox.archive_item",
    "inbox.mark_read",
    "inbox.claim_item",
    "inbox.release_item",
    "inbox.summarize_item",
];

const INBOX_PERMISSION_SCOPES: &[&str] = &[
    "inbox.source.manage",
    "inbox.sync",
    "inbox.event.ingest",
    "inbox.read.metadata",
    "inbox.read.body",
    "inbox.read.attachment",
    "inbox.search",
    "inbox.write.triage",
    "inbox.claim",
    "inbox.summarize",
];

const SOURCE_SYNC_METADATA: &[(&str, &str)] = &[
    ("sync", "true"),
    ("event_ingest", "false"),
    ("mutation", "true"),
    ("claim", "true"),
];
const INBOX_EVENT_METADATA: &[(&str, &str)] = &[
    ("sync", "false"),
    ("event_ingest", "true"),
    ("mutation", "false"),
    ("claim", "false"),
];
const AGGREGATION_STORE_METADATA: &[(&str, &str)] = &[
    ("sync", "true"),
    ("event_ingest", "true"),
    ("mutation", "true"),
    ("claim", "true"),
];
const INBOX_MOCK_METADATA: &[(&str, &str)] = &[
    ("sync", "true"),
    ("event_ingest", "true"),
    ("mutation", "true"),
    ("claim", "true"),
];
const INBOX_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("sync", "false"),
    ("event_ingest", "false"),
    ("mutation", "false"),
    ("claim", "false"),
];

const INBOX_PROVIDER_CLASSES: &[CommunicationProviderClass<'_>] = &[
    CommunicationProviderClass {
        provider_class: "source-sync",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: SOURCE_SYNC_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "event-ingest",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: INBOX_EVENT_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "aggregation-store",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: AGGREGATION_STORE_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: INBOX_MOCK_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: INBOX_UNAVAILABLE_METADATA,
    },
];

/// Build the descriptor-only catalog entry for `pack.communication.inbox.v1`.
pub fn communication_inbox_pack_definition() -> DomainPackDefinition {
    communication_pack_definition(CommunicationPackDescriptor {
        slug: "inbox",
        service_id: COMMUNICATION_INBOX_SERVICE_ID,
        commands: COMMUNICATION_INBOX_COMMANDS,
        permission_scopes: INBOX_PERMISSION_SCOPES,
        provider_classes: INBOX_PROVIDER_CLASSES,
        health_probe: "inbox.sync_sources",
        unavailable_reason: "inbox_provider_not_installed",
        replay_schema: "inbox.pack.replay.v1",
        data_classification: "communication_inbox_metadata",
        retention_policy: "source_cursor_item_metadata_with_content_by_reference",
        redaction_policy: "credentials_tokens_provider_payloads_bodies_attachments_redacted",
        examples: &[
            "Declare `pack.communication.inbox.v1` as optional until an inbox provider is installed.",
            "Use cursors, item handles, and body references instead of raw provider exports.",
        ],
        migration_notes: &[
            "Inbox becomes callable only after an approved communication provider registers command schemas.",
            "Provider-native source payloads, raw bodies, and attachments must remain behind source adapters.",
        ],
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxSource {
    pub source_id: String,
    pub source_kind: String,
    pub provider_class: String,
    pub credential_secret_ref: Option<String>,
    pub health: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxCursor {
    pub source_id: String,
    pub cursor_hash: String,
    pub watermark_hash: Option<String>,
    pub expires_epoch_ms: Option<i128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxItem {
    pub item_id: String,
    pub source_id: String,
    pub thread_id: Option<String>,
    pub sender_hash: String,
    pub subject_ref: Option<String>,
    pub preview_ref: Option<String>,
    pub read: bool,
    pub label_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxThread {
    pub thread_id: String,
    pub source_ids: Vec<String>,
    pub item_count: u32,
    pub unread_count: u32,
    pub summary_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxLabel {
    pub label_id: String,
    pub display_name: String,
    pub provider_mutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxAttachmentHandle {
    pub item_id: String,
    pub part_id: String,
    pub filename_hash: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub content_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxEvent {
    pub source_id: String,
    pub event_id_hash: String,
    pub mutation_type: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxClaim {
    pub item_id: String,
    pub owner_ref: String,
    pub lease_expires_epoch_ms: i128,
    pub claim_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxSyncCheckpoint {
    pub source_id: String,
    pub cursor: InboxCursor,
    pub checkpoint_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    pub source_kinds: BTreeSet<String>,
    pub supports_query: bool,
    pub supports_mutation: bool,
    pub supports_claims: bool,
    pub page_limit: u32,
    pub availability: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxProviderSnapshot {
    pub descriptor_hash: String,
    pub provider_class: String,
    pub source_count: u32,
    pub item_count: u64,
    pub cursor_hashes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxRegisterSourceCommand {
    pub source: InboxSource,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxUpdateSourceCommand {
    pub source: InboxSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxRevokeSourceCommand {
    pub source_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxSyncSourcesCommand {
    pub source_ids: Vec<String>,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxResumeSyncCommand {
    pub checkpoint: InboxSyncCheckpoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxIngestEventCommand {
    pub event: InboxEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxListItemsCommand {
    pub source_id: String,
    pub cursor: Option<InboxCursor>,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxSearchItemsCommand {
    pub source_id: String,
    pub query_ref: String,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxGetItemCommand {
    pub item_id: String,
    pub projection: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxFetchBodyCommand {
    pub item_id: String,
    pub body_part: String,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxFetchAttachmentCommand {
    pub attachment: InboxAttachmentHandle,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxListThreadsCommand {
    pub source_id: String,
    pub cursor: Option<InboxCursor>,
    pub page_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxLabelItemCommand {
    pub item_id: String,
    pub add_labels: Vec<InboxLabel>,
    pub remove_label_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxMoveItemCommand {
    pub item_id: String,
    pub target_folder_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxArchiveItemCommand {
    pub item_id: String,
    pub archive_policy: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxMarkReadCommand {
    pub item_ids: Vec<String>,
    pub read: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxClaimItemCommand {
    pub item_id: String,
    pub owner_ref: String,
    pub lease_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxReleaseItemCommand {
    pub claim: InboxClaim,
    pub outcome: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxSummarizeItemCommand {
    pub item_id: String,
    pub redaction_profile: String,
    pub delegated_pack_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxResultStatus {
    Success,
    Page,
    PartialSync,
    ResetRequired,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    Timeout,
    Canceled,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxError {
    pub code: InboxResultStatus,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxResultEnvelope<T> {
    pub status: InboxResultStatus,
    pub data: Option<T>,
    pub error: Option<InboxError>,
    pub trace_id: String,
    pub descriptor_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub snapshot_schema_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn communication_inbox_descriptor_hashes() -> InboxDescriptorHashes {
    InboxDescriptorHashes {
        command_schema_hash: inbox_stable_hash(&COMMUNICATION_INBOX_COMMANDS),
        result_schema_hash: inbox_stable_hash(&InboxResultStatus::Success),
        snapshot_schema_hash: inbox_stable_hash(&InboxProviderSnapshot {
            descriptor_hash: "descriptor".into(),
            provider_class: "unavailable".into(),
            source_count: 0,
            item_count: 0,
            cursor_hashes: BTreeMap::new(),
        }),
        provider_capability_schema_hash: inbox_stable_hash(&InboxProviderCapability {
            provider_class: "unavailable".into(),
            supported_commands: schema_set(COMMUNICATION_INBOX_COMMANDS),
            source_kinds: BTreeSet::from(["mailbox".into(), "conversation".into()]),
            supports_query: false,
            supports_mutation: false,
            supports_claims: false,
            page_limit: 0,
            availability: DomainPackProviderCapabilityState::Unavailable,
        }),
        unavailable_schema_hash: inbox_stable_hash(&InboxError {
            code: InboxResultStatus::Unavailable,
            message: "inbox provider is not installed".into(),
            retryable: false,
        }),
    }
}

pub fn inbox_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    communication_stable_hash(value)
}
