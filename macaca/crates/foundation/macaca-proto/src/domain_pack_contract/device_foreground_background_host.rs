use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::device_common::{
    define_device_command_wrappers, device_pack_definition, device_stable_hash,
    DevicePackCommandEnvelope, DevicePackDescriptor, DevicePackError, DevicePackPage,
    DeviceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID: &str =
    "pack.device.foreground_background_host.v1";
pub const DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID: &str = "service.device.host_lifecycle";

pub const DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS: &[&str] = &[
    "host_lifecycle.inspect_state",
    "host_lifecycle.subscribe_events",
    "host_lifecycle.open_foreground_session",
    "host_lifecycle.close_foreground_session",
    "host_lifecycle.request_background_lease",
    "host_lifecycle.release_background_lease",
    "host_lifecycle.inspect_policy",
    "host_lifecycle.revoke",
    "host_lifecycle.inspect_host",
];

const HOST_LIFECYCLE_PERMISSION_SCOPES: &[&str] = &[
    "device.host_lifecycle.read",
    "device.host_lifecycle.events",
    "device.host_lifecycle.foreground",
    "device.host_lifecycle.background",
    "device.host_lifecycle.revoke",
];

const HOST_NATIVE_METADATA: &[(&str, &str)] = &[
    ("host_native", "true"),
    ("service_type_names", "not_os_semantics"),
    ("raw_lifecycle_logs", "false"),
];
const BROWSER_METADATA: &[(&str, &str)] = &[
    ("browser", "true"),
    ("visibility", "descriptor_only"),
    ("throttling", "reported"),
];
const REMOTE_METADATA: &[(&str, &str)] = &[
    ("remote_host", "true"),
    ("delegation", "approval_required"),
    ("host_identifiers", "redacted"),
];
const MOCK_METADATA: &[(&str, &str)] = &[("fixtures", "synthetic"), ("callable", "false")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("lifecycle", "false"), ("reason", "provider_not_installed")];

const HOST_LIFECYCLE_PROVIDER_CLASSES: &[DeviceProviderClass<'_>] = &[
    DeviceProviderClass {
        provider_class: "host-native",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: HOST_NATIVE_METADATA,
    },
    DeviceProviderClass {
        provider_class: "browser",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: BROWSER_METADATA,
    },
    DeviceProviderClass {
        provider_class: "remote-host",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: REMOTE_METADATA,
    },
    DeviceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MOCK_METADATA,
    },
    DeviceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: UNAVAILABLE_METADATA,
    },
];

/// Build the foreground/background host descriptor without binding lifecycle APIs.
pub fn device_foreground_background_host_pack_definition() -> DomainPackDefinition {
    device_pack_definition(DevicePackDescriptor {
        pack_id: DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID,
        child_change_id: "openspec:add-pack-device-foreground-background-host",
        docs_slug: "foreground-background-host",
        sdk_slug: "foreground_background_host",
        service_id: DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID,
        commands: DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS,
        permission_scopes: HOST_LIFECYCLE_PERMISSION_SCOPES,
        provider_classes: HOST_LIFECYCLE_PROVIDER_CLASSES,
        health_probe: "host_lifecycle.inspect_host",
        unavailable_reason: "device_foreground_background_host_provider_not_installed",
        replay_schema: "device.host_lifecycle.replay.v1",
        data_classification: "device_host_lifecycle_reference_metadata",
        retention_policy: "lifecycle_state_foreground_session_background_lease_policy_event_snapshot_and_host_status_metadata_by_reference",
        redaction_policy: "provider_payloads_host_identifiers_presentation_metadata_session_lease_ids_lifecycle_logs_and_credentials_redacted",
        timeout_ms: 90_000,
        budget_units: 4,
        examples: &[
            "Declare `pack.device.foreground_background_host.v1` as optional until a host lifecycle provider is installed.",
            "Use lifecycle state, foreground session, background lease, throttling, and policy descriptors instead of host-specific lifecycle APIs.",
        ],
        migration_notes: &[
            "Host lifecycle commands become callable only after an approved service provider registers matching schemas.",
            "Workflow scheduling, task execution, process supervision, camera, sensors, files, notifications, and application background logic remain separate capability owners.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostLifecycleState {
    pub state_ref: String,
    pub visibility: String,
    pub execution_state: String,
    pub throttle_state_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForegroundSession {
    pub session_ref: String,
    pub purpose_ref: String,
    pub presentation_requirement: HostPresentationRequirement,
    pub state: String,
    pub max_duration_ms: u64,
    pub resource_budget_ref: String,
    pub cleanup_behavior: String,
}

impl ForegroundSession {
    /// Foreground sessions must declare bounded duration and presentation evidence.
    pub fn is_bounded(&self) -> bool {
        self.max_duration_ms > 0 && !self.presentation_requirement.presentation_class.is_empty()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundLease {
    pub lease_ref: String,
    pub purpose_ref: String,
    pub lease_class: String,
    pub state: String,
    pub max_duration_ms: u64,
    pub resource_budget_ref: String,
    pub dependent_capabilities: BTreeSet<String>,
    pub cleanup_behavior: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostLifecycleEvent {
    pub event_ref: String,
    pub event_kind: String,
    pub state_ref: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostLifecyclePolicy {
    pub policy_ref: String,
    pub foreground_required: bool,
    pub background_allowed: bool,
    pub dependent_capabilities: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostPresentationRequirement {
    pub presentation_ref: String,
    pub presentation_class: String,
    pub user_visible: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostThrottleState {
    pub throttle_ref: String,
    pub state: String,
    pub budget_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostLifecycleSnapshot {
    pub snapshot_ref: String,
    pub state: HostLifecycleState,
    pub active_session_count: u32,
    pub active_lease_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostLifecycleError {
    pub code: String,
    pub trace_safe_detail: String,
    pub retryable: bool,
}

define_device_command_wrappers!(
    HostLifecycleInspectStateCommand,
    HostLifecycleSubscribeEventsCommand,
    HostLifecycleOpenForegroundSessionCommand,
    HostLifecycleCloseForegroundSessionCommand,
    HostLifecycleRequestBackgroundLeaseCommand,
    HostLifecycleReleaseBackgroundLeaseCommand,
    HostLifecycleInspectPolicyCommand,
    HostLifecycleRevokeCommand,
    HostLifecycleInspectHostCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostLifecycleResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    ForegroundRequired,
    BackgroundDenied,
    EntitlementRequired,
    PresentationRequired,
    LeaseExpired,
    LeaseRevoked,
    Throttled,
    Suspended,
    QuotaExceeded,
    ProviderFailure,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostLifecycleResultEnvelope<T> {
    pub status: HostLifecycleResultStatus,
    pub data: Option<T>,
    pub page: Option<DevicePackPage<T>>,
    pub error: Option<DevicePackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostLifecycleDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub state_hash: String,
    pub session_hash: String,
    pub lease_hash: String,
    pub policy_hash: String,
    pub redaction_profile_hash: String,
}

pub fn device_foreground_background_host_descriptor_hashes() -> HostLifecycleDescriptorHashes {
    HostLifecycleDescriptorHashes {
        command_schema_hash: host_lifecycle_stable_hash(
            &DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS,
        ),
        result_schema_hash: host_lifecycle_stable_hash(&HostLifecycleResultStatus::Success),
        descriptor_hash: host_lifecycle_stable_hash(
            &device_foreground_background_host_pack_definition(),
        ),
        state_hash: host_lifecycle_stable_hash(&HostLifecycleState {
            state_ref: "state".into(),
            visibility: "foreground".into(),
            execution_state: "active".into(),
            throttle_state_ref: "throttle".into(),
        }),
        session_hash: host_lifecycle_stable_hash(&ForegroundSession {
            session_ref: "session".into(),
            purpose_ref: "purpose".into(),
            presentation_requirement: HostPresentationRequirement {
                presentation_ref: "presentation".into(),
                presentation_class: "visible".into(),
                user_visible: true,
            },
            state: "active".into(),
            max_duration_ms: 1000,
            resource_budget_ref: "resource".into(),
            cleanup_behavior: "release_on_close".into(),
        }),
        lease_hash: host_lifecycle_stable_hash(&BackgroundLease {
            lease_ref: "lease".into(),
            purpose_ref: "purpose".into(),
            lease_class: "short_task".into(),
            state: "active".into(),
            max_duration_ms: 1000,
            resource_budget_ref: "resource".into(),
            dependent_capabilities: BTreeSet::from(["device.sensors".into()]),
            cleanup_behavior: "release_on_expiry".into(),
        }),
        policy_hash: host_lifecycle_stable_hash(&HostLifecyclePolicy {
            policy_ref: "policy".into(),
            foreground_required: false,
            background_allowed: true,
            dependent_capabilities: BTreeSet::from(["device.sensors".into()]),
        }),
        redaction_profile_hash: host_lifecycle_stable_hash("host-lifecycle-redaction-v1"),
    }
}

pub fn host_lifecycle_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    device_stable_hash(value)
}
