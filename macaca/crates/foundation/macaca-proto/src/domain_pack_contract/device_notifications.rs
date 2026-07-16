use serde::{Deserialize, Serialize};

use super::device_common::{
    define_device_command_wrappers, device_pack_definition, device_stable_hash,
    DevicePackCommandEnvelope, DevicePackDescriptor, DevicePackError, DevicePackPage,
    DeviceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const DEVICE_NOTIFICATIONS_PACK_ID: &str = "pack.device.notifications.v1";
pub const DEVICE_NOTIFICATIONS_SERVICE_ID: &str = "service.device.notifications";

pub const DEVICE_NOTIFICATIONS_COMMANDS: &[&str] = &[
    "notifications.inspect_authorization",
    "notifications.request_authorization",
    "notifications.register_channel",
    "notifications.register_category",
    "notifications.post",
    "notifications.schedule",
    "notifications.cancel",
    "notifications.list_pending",
    "notifications.inspect_history",
    "notifications.set_badge",
    "notifications.clear_badge",
    "notifications.subscribe_interactions",
    "notifications.inspect_push_support",
    "notifications.inspect_host",
];

const NOTIFICATIONS_PERMISSION_SCOPES: &[&str] = &[
    "device.notifications.read_status",
    "device.notifications.request_permission",
    "device.notifications.post",
    "device.notifications.schedule",
    "device.notifications.manage",
    "device.notifications.interactions",
];

const NOTIFY_HOST_METADATA: &[(&str, &str)] = &[
    ("host_native", "true"),
    ("authorization", "host_owned"),
    ("raw_bodies_in_trace", "false"),
];
const NOTIFY_BROWSER_METADATA: &[(&str, &str)] = &[
    ("browser", "true"),
    ("push_support", "inspect_only"),
    ("push_tokens_exposed", "false"),
];
const NOTIFY_REMOTE_METADATA: &[(&str, &str)] = &[
    ("remote_host", "true"),
    ("delivery", "policy_bound"),
    ("background_actions", "approval_required"),
];
const NOTIFY_MOCK_METADATA: &[(&str, &str)] = &[("fixtures", "synthetic"), ("callable", "false")];
const NOTIFY_UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("post", "false"), ("reason", "provider_not_installed")];

const NOTIFICATIONS_PROVIDER_CLASSES: &[DeviceProviderClass<'_>] = &[
    DeviceProviderClass {
        provider_class: "host-native",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: NOTIFY_HOST_METADATA,
    },
    DeviceProviderClass {
        provider_class: "browser",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: NOTIFY_BROWSER_METADATA,
    },
    DeviceProviderClass {
        provider_class: "remote-host",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: NOTIFY_REMOTE_METADATA,
    },
    DeviceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: NOTIFY_MOCK_METADATA,
    },
    DeviceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: NOTIFY_UNAVAILABLE_METADATA,
    },
];

/// Build the notifications descriptor without binding host notification APIs.
pub fn device_notifications_pack_definition() -> DomainPackDefinition {
    device_pack_definition(DevicePackDescriptor {
        pack_id: DEVICE_NOTIFICATIONS_PACK_ID,
        child_change_id: "openspec:add-pack-device-notifications",
        docs_slug: "notifications",
        sdk_slug: "notifications",
        service_id: DEVICE_NOTIFICATIONS_SERVICE_ID,
        commands: DEVICE_NOTIFICATIONS_COMMANDS,
        permission_scopes: NOTIFICATIONS_PERMISSION_SCOPES,
        provider_classes: NOTIFICATIONS_PROVIDER_CLASSES,
        health_probe: "notifications.inspect_host",
        unavailable_reason: "device_notifications_provider_not_installed",
        replay_schema: "device.notifications.replay.v1",
        data_classification: "device_notification_reference_metadata",
        retention_policy: "authorization_channel_category_action_content_trigger_delivery_record_interaction_and_host_status_metadata_by_reference",
        redaction_policy: "notification_title_body_action_input_push_tokens_provider_payloads_history_interactions_and_credentials_redacted",
        timeout_ms: 90_000,
        budget_units: 4,
        examples: &[
            "Declare `pack.device.notifications.v1` as optional until a notification provider is installed.",
            "Use authorization state, channels, categories, redacted content handles, records, interactions, and host status instead of host notification APIs.",
        ],
        migration_notes: &[
            "Notification commands become callable only after an approved notification service provider registers matching schemas.",
            "Communication notifications, messaging, inbox, gateways, workflow schedule, and application reminder logic remain separate capability owners.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationAuthorization {
    pub authorization_ref: String,
    pub state: String,
    pub prompt_allowed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub channel_ref: String,
    pub importance_class: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationCategory {
    pub category_ref: String,
    pub action_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationAction {
    pub action_ref: String,
    pub action_kind: String,
    pub requires_foreground: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationContent {
    pub content_ref: String,
    pub title_hash: String,
    pub body_hash: String,
    pub redaction_class: String,
}

impl NotificationContent {
    /// Require hashed content handles instead of raw notification text in DTO evidence.
    pub fn is_redacted(&self) -> bool {
        !self.title_hash.is_empty() && !self.body_hash.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationTrigger {
    pub trigger_ref: String,
    pub trigger_kind: String,
    pub scheduled_epoch_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationDeliveryPolicy {
    pub policy_ref: String,
    pub interruption_class: String,
    pub lock_screen_redaction: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationRecord {
    pub notification_ref: String,
    pub channel_ref: String,
    pub category_ref: Option<String>,
    pub delivery_state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationInteraction {
    pub interaction_ref: String,
    pub notification_ref: String,
    pub action_ref: String,
    pub input_hash: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationError {
    pub code: String,
    pub trace_safe_detail: String,
    pub retryable: bool,
}

define_device_command_wrappers!(
    NotificationsInspectAuthorizationCommand,
    NotificationsRequestAuthorizationCommand,
    NotificationsRegisterChannelCommand,
    NotificationsRegisterCategoryCommand,
    NotificationsPostCommand,
    NotificationsScheduleCommand,
    NotificationsCancelCommand,
    NotificationsListPendingCommand,
    NotificationsInspectHistoryCommand,
    NotificationsSetBadgeCommand,
    NotificationsClearBadgeCommand,
    NotificationsSubscribeInteractionsCommand,
    NotificationsInspectPushSupportCommand,
    NotificationsInspectHostCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationsResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    PromptNotAllowed,
    ChannelMissing,
    CategoryMissing,
    ContentTooLarge,
    SensitiveContentBlocked,
    QuotaExceeded,
    ScheduleTooFar,
    BackgroundActionDenied,
    InteractionExpired,
    HostDisabled,
    ProviderFailure,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationsResultEnvelope<T> {
    pub status: NotificationsResultStatus,
    pub data: Option<T>,
    pub page: Option<DevicePackPage<T>>,
    pub error: Option<DevicePackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationsDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub authorization_hash: String,
    pub channel_hash: String,
    pub schedule_hash: String,
    pub interaction_hash: String,
    pub redaction_profile_hash: String,
}

pub fn device_notifications_descriptor_hashes() -> NotificationsDescriptorHashes {
    NotificationsDescriptorHashes {
        command_schema_hash: notifications_stable_hash(&DEVICE_NOTIFICATIONS_COMMANDS),
        result_schema_hash: notifications_stable_hash(&NotificationsResultStatus::Success),
        descriptor_hash: notifications_stable_hash(&device_notifications_pack_definition()),
        authorization_hash: notifications_stable_hash(&NotificationAuthorization {
            authorization_ref: "authorization".into(),
            state: "promptable".into(),
            prompt_allowed: true,
        }),
        channel_hash: notifications_stable_hash(&NotificationChannel {
            channel_ref: "channel".into(),
            importance_class: "default".into(),
            redaction_class: "hash_only".into(),
        }),
        schedule_hash: notifications_stable_hash(&NotificationTrigger {
            trigger_ref: "trigger".into(),
            trigger_kind: "time".into(),
            scheduled_epoch_ms: Some(10),
        }),
        interaction_hash: notifications_stable_hash(&NotificationInteraction {
            interaction_ref: "interaction".into(),
            notification_ref: "notification".into(),
            action_ref: "open".into(),
            input_hash: Some("input".into()),
        }),
        redaction_profile_hash: notifications_stable_hash("notifications-redaction-v1"),
    }
}

pub fn notifications_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    device_stable_hash(value)
}
