use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::communication_common::{
    communication_pack_definition, communication_stable_hash, schema_set,
    CommunicationPackDescriptor, CommunicationProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

/// Stable pack id for provider-neutral notification operations.
pub const COMMUNICATION_NOTIFICATION_PACK_ID: &str = "pack.communication.notification.v1";
/// Stable service id used by future notification providers.
pub const COMMUNICATION_NOTIFICATION_SERVICE_ID: &str = "service.communication.notification";

/// Canonical command names described by `pack.communication.notification.v1`.
pub const COMMUNICATION_NOTIFICATION_COMMANDS: &[&str] = &[
    "notification.publish",
    "notification.schedule",
    "notification.update",
    "notification.cancel",
    "notification.list_notifications",
    "notification.register_action",
    "notification.unregister_action",
    "notification.acknowledge",
    "notification.dismiss",
    "notification.inspect_delivery",
    "notification.register_subscription",
    "notification.revoke_subscription",
];

const NOTIFICATION_PERMISSION_SCOPES: &[&str] = &[
    "notification.publish",
    "notification.schedule",
    "notification.update",
    "notification.cancel",
    "notification.action.register",
    "notification.action.receive",
    "notification.subscription.manage",
    "notification.delivery.inspect",
    "notification.host.surface",
];

const HOST_NOTIFICATION_METADATA: &[(&str, &str)] = &[
    ("local", "true"),
    ("push", "false"),
    ("actions", "true"),
    ("subscriptions", "false"),
];
const PUSH_BRIDGE_METADATA: &[(&str, &str)] = &[
    ("local", "false"),
    ("push", "true"),
    ("actions", "true"),
    ("subscriptions", "true"),
];
const SUBSCRIPTION_BRIDGE_METADATA: &[(&str, &str)] = &[
    ("local", "false"),
    ("push", "true"),
    ("actions", "false"),
    ("subscriptions", "true"),
];
const NOTIFICATION_MOCK_METADATA: &[(&str, &str)] = &[
    ("local", "true"),
    ("push", "true"),
    ("actions", "true"),
    ("subscriptions", "true"),
];
const NOTIFICATION_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("local", "false"),
    ("push", "false"),
    ("actions", "false"),
    ("subscriptions", "false"),
];

const NOTIFICATION_PROVIDER_CLASSES: &[CommunicationProviderClass<'_>] = &[
    CommunicationProviderClass {
        provider_class: "host-notification",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: HOST_NOTIFICATION_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "push-bridge",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PUSH_BRIDGE_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "subscription-bridge",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: SUBSCRIPTION_BRIDGE_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: NOTIFICATION_MOCK_METADATA,
    },
    CommunicationProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: NOTIFICATION_UNAVAILABLE_METADATA,
    },
];

/// Build the descriptor-only catalog entry for `pack.communication.notification.v1`.
pub fn communication_notification_pack_definition() -> DomainPackDefinition {
    communication_pack_definition(CommunicationPackDescriptor {
        slug: "notification",
        service_id: COMMUNICATION_NOTIFICATION_SERVICE_ID,
        commands: COMMUNICATION_NOTIFICATION_COMMANDS,
        permission_scopes: NOTIFICATION_PERMISSION_SCOPES,
        provider_classes: NOTIFICATION_PROVIDER_CLASSES,
        health_probe: "notification.inspect_delivery",
        unavailable_reason: "notification_provider_not_installed",
        replay_schema: "notification.pack.replay.v1",
        data_classification: "attention_delivery_metadata",
        retention_policy: "delivery_handles_and_status_metadata_only",
        redaction_policy: "tokens_endpoints_keys_credentials_and_unbounded_content_redacted",
        examples: &[
            "Declare `pack.communication.notification.v1` as optional until a notification provider is installed.",
            "Use subscription handles and secret references instead of raw push endpoints or tokens.",
        ],
        migration_notes: &[
            "Notification becomes callable only after an approved communication provider registers command schemas.",
            "Provider-specific push payloads, device tokens, endpoints, and keys must remain provider-private.",
        ],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationDeliveryChannel {
    Local,
    Push,
    InApp,
    Host,
    ProviderRemote,
    AutoPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationMessage {
    pub title_ref: String,
    pub body_ref: String,
    pub locale: Option<String>,
    pub sensitivity: String,
    pub category_id: Option<String>,
    pub collapse_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationTarget {
    pub target_id: String,
    pub target_kind: String,
    pub subscription: Option<NotificationSubscriptionHandle>,
    pub redaction_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationSchedule {
    pub deliver_at_epoch_ms: Option<i128>,
    pub relative_delay_ms: Option<u64>,
    pub timezone_id: Option<String>,
    pub expiry_epoch_ms: Option<i128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationActionDefinition {
    pub action_id: String,
    pub title_ref: String,
    pub semantic_role: String,
    pub destructive: bool,
    pub requires_foreground: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationActionEvent {
    pub delivery: NotificationDeliveryHandle,
    pub action_id: String,
    pub bounded_input_ref: Option<String>,
    pub replay_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationDeliveryStatus {
    Accepted,
    Scheduled,
    Delivered,
    Acknowledged,
    Dismissed,
    Canceled,
    Expired,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationSubscriptionHandle {
    pub subscription_id: String,
    pub target_class: String,
    pub secret_ref: Option<String>,
    pub provider_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationDeliveryHandle {
    pub delivery_id: String,
    pub channel: NotificationDeliveryChannel,
    pub provider_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationProviderCapability {
    pub provider_class: String,
    pub supported_commands: BTreeSet<String>,
    pub channels: BTreeSet<NotificationDeliveryChannel>,
    pub supports_schedule: bool,
    pub supports_update_cancel: bool,
    pub supports_actions: bool,
    pub supports_subscriptions: bool,
    pub max_payload_bytes: u64,
    pub max_actions: u32,
    pub availability: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationProviderSnapshot {
    pub descriptor_hash: String,
    pub provider_class: String,
    pub active_delivery_count: u32,
    pub subscription_count: u32,
    pub quota_hashes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationPublishCommand {
    pub message: NotificationMessage,
    pub target: NotificationTarget,
    pub channel: NotificationDeliveryChannel,
    pub client_request_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationScheduleCommand {
    pub publish: NotificationPublishCommand,
    pub schedule: NotificationSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationUpdateCommand {
    pub delivery: NotificationDeliveryHandle,
    pub message: NotificationMessage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationCancelCommand {
    pub delivery: NotificationDeliveryHandle,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationListNotificationsCommand {
    pub target: NotificationTarget,
    pub status_filter: Vec<NotificationDeliveryStatus>,
    pub page_size: u32,
    pub cursor_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationRegisterActionCommand {
    pub category_id: String,
    pub action: NotificationActionDefinition,
    pub callback_route: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationUnregisterActionCommand {
    pub category_id: String,
    pub action_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationAcknowledgeCommand {
    pub delivery: NotificationDeliveryHandle,
    pub action_event: Option<NotificationActionEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationDismissCommand {
    pub delivery: NotificationDeliveryHandle,
    pub dismissed_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationInspectDeliveryCommand {
    pub delivery: NotificationDeliveryHandle,
    pub include_provider_evidence: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationRegisterSubscriptionCommand {
    pub target: NotificationTarget,
    pub channel: NotificationDeliveryChannel,
    pub secret_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationRevokeSubscriptionCommand {
    pub subscription: NotificationSubscriptionHandle,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationResultStatus {
    Success,
    PartialPage,
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
pub struct NotificationError {
    pub code: NotificationResultStatus,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationResultEnvelope<T> {
    pub status: NotificationResultStatus,
    pub data: Option<T>,
    pub error: Option<NotificationError>,
    pub trace_id: String,
    pub descriptor_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub snapshot_schema_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn communication_notification_descriptor_hashes() -> NotificationDescriptorHashes {
    NotificationDescriptorHashes {
        command_schema_hash: notification_stable_hash(&COMMUNICATION_NOTIFICATION_COMMANDS),
        result_schema_hash: notification_stable_hash(&NotificationResultStatus::Success),
        snapshot_schema_hash: notification_stable_hash(&NotificationProviderSnapshot {
            descriptor_hash: "descriptor".into(),
            provider_class: "unavailable".into(),
            active_delivery_count: 0,
            subscription_count: 0,
            quota_hashes: BTreeMap::new(),
        }),
        provider_capability_schema_hash: notification_stable_hash(
            &NotificationProviderCapability {
                provider_class: "unavailable".into(),
                supported_commands: schema_set(COMMUNICATION_NOTIFICATION_COMMANDS),
                channels: BTreeSet::from([NotificationDeliveryChannel::InApp]),
                supports_schedule: false,
                supports_update_cancel: false,
                supports_actions: false,
                supports_subscriptions: false,
                max_payload_bytes: 0,
                max_actions: 0,
                availability: DomainPackProviderCapabilityState::Unavailable,
            },
        ),
        unavailable_schema_hash: notification_stable_hash(&NotificationError {
            code: NotificationResultStatus::Unavailable,
            message: "notification provider is not installed".into(),
            retryable: false,
        }),
    }
}

pub fn notification_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    communication_stable_hash(value)
}
