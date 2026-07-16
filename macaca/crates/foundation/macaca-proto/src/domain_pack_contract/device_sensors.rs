use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::device_common::{
    define_device_command_wrappers, device_pack_definition, device_stable_hash,
    DevicePackCommandEnvelope, DevicePackDescriptor, DevicePackError, DevicePackPage,
    DeviceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const DEVICE_SENSORS_PACK_ID: &str = "pack.device.sensors.v1";
pub const DEVICE_SENSORS_SERVICE_ID: &str = "service.device.sensors";

pub const DEVICE_SENSORS_COMMANDS: &[&str] = &[
    "sensors.list",
    "sensors.inspect",
    "sensors.read",
    "sensors.open_stream",
    "sensors.read_stream",
    "sensors.close_stream",
    "sensors.read_batch",
    "sensors.inspect_calibration",
    "sensors.acquire_lease",
    "sensors.release_lease",
    "sensors.inspect_host",
];

const SENSORS_PERMISSION_SCOPES: &[&str] = &[
    "device.sensors.read",
    "device.sensors.stream",
    "device.sensors.calibration.read",
    "device.sensors.lease.manage",
];

const HOST_NATIVE_METADATA: &[(&str, &str)] = &[
    ("host_native", "true"),
    ("foreground_policy", "required_when_sensitive"),
    ("raw_samples_in_trace", "false"),
];
const BROWSER_METADATA: &[(&str, &str)] = &[
    ("browser", "true"),
    ("permission_prompt", "host_owned"),
    ("secure_context_required", "descriptor_only"),
];
const REMOTE_METADATA: &[(&str, &str)] = &[
    ("remote_host", "true"),
    ("forwarding", "policy_bound"),
    ("stable_hardware_ids", "redacted"),
];
const MOCK_METADATA: &[(&str, &str)] = &[("fixtures", "synthetic"), ("callable", "false")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("streams", "false"), ("reason", "provider_not_installed")];

const SENSORS_PROVIDER_CLASSES: &[DeviceProviderClass<'_>] = &[
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

/// Build the sensors descriptor without binding host sensor APIs or stream providers.
pub fn device_sensors_pack_definition() -> DomainPackDefinition {
    device_pack_definition(DevicePackDescriptor {
        pack_id: DEVICE_SENSORS_PACK_ID,
        child_change_id: "openspec:add-pack-device-sensors",
        docs_slug: "sensors",
        sdk_slug: "sensors",
        service_id: DEVICE_SENSORS_SERVICE_ID,
        commands: DEVICE_SENSORS_COMMANDS,
        permission_scopes: SENSORS_PERMISSION_SCOPES,
        provider_classes: SENSORS_PROVIDER_CLASSES,
        health_probe: "sensors.inspect_host",
        unavailable_reason: "device_sensors_provider_not_installed",
        replay_schema: "device.sensors.replay.v1",
        data_classification: "device_sensor_reference_metadata",
        retention_policy: "sensor_descriptor_accuracy_calibration_stream_lease_batch_and_host_status_metadata_by_reference",
        redaction_policy: "raw_sample_vectors_stable_hardware_identifiers_host_payloads_calibration_details_stream_chunks_and_credentials_redacted",
        timeout_ms: 90_000,
        budget_units: 4,
        examples: &[
            "Declare `pack.device.sensors.v1` as optional until a sensor provider is installed.",
            "Use sensor descriptors, leases, calibration references, and bounded batches instead of raw host sensor payloads.",
        ],
        migration_notes: &[
            "Sensor commands become callable only after an approved sensor service provider registers matching schemas.",
            "Camera, local files, notifications, foreground/background host lifecycle, location, and application lifecycle remain separate capability owners.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorDescriptor {
    pub sensor_ref: String,
    pub sensor_type: SensorType,
    pub privacy_class: String,
    pub coordinate_frame: SensorCoordinateFrame,
    pub max_sample_hz: u32,
    pub host_status_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorType {
    pub type_ref: String,
    pub family: String,
    pub unit: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorReading {
    pub reading_ref: String,
    pub sensor_ref: String,
    pub vector: SensorVector,
    pub accuracy: SensorAccuracy,
    pub timestamp_epoch_ms: u64,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorVector {
    pub axes: BTreeMap<String, String>,
    pub unit: String,
    pub quantization_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorCoordinateFrame {
    pub frame_ref: String,
    pub orientation_ref: String,
    pub host_frame_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorAccuracy {
    pub accuracy_class: String,
    pub confidence_basis_points: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorStreamLease {
    pub lease_ref: String,
    pub sensor_ref: String,
    pub state: String,
    pub max_duration_ms: u64,
    pub max_sample_count: u64,
    pub frequency_hz: u32,
    pub delivery_mode: String,
    pub cancellation_behavior: String,
    pub revocation_behavior: String,
}

impl SensorStreamLease {
    /// Ensure streams are explicitly bounded before a provider can be asked to open one.
    pub fn is_bounded(&self) -> bool {
        self.max_duration_ms > 0 && self.max_sample_count > 0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorBatch {
    pub batch_ref: String,
    pub sensor_ref: String,
    pub sample_count: u32,
    pub chunk_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorCalibration {
    pub calibration_ref: String,
    pub sensor_ref: String,
    pub state: String,
    pub metadata_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorHostStatus {
    pub host_ref: String,
    pub enabled: bool,
    pub foreground_required: bool,
    pub permission_state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorError {
    pub code: String,
    pub trace_safe_detail: String,
    pub retryable: bool,
}

define_device_command_wrappers!(
    SensorsListCommand,
    SensorsInspectCommand,
    SensorsReadCommand,
    SensorsOpenStreamCommand,
    SensorsReadStreamCommand,
    SensorsCloseStreamCommand,
    SensorsReadBatchCommand,
    SensorsInspectCalibrationCommand,
    SensorsAcquireLeaseCommand,
    SensorsReleaseLeaseCommand,
    SensorsInspectHostCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensorsResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Disabled,
    PermissionPromptRequired,
    ForegroundRequired,
    LeaseExpired,
    LeaseRevoked,
    SampleRateTooHigh,
    StreamOverflow,
    Timeout,
    QuotaExceeded,
    CalibrationUnavailable,
    ProviderFailure,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorsResultEnvelope<T> {
    pub status: SensorsResultStatus,
    pub data: Option<T>,
    pub page: Option<DevicePackPage<T>>,
    pub error: Option<DevicePackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorsDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub sensor_descriptor_hash: String,
    pub stream_lease_hash: String,
    pub calibration_hash: String,
    pub redaction_profile_hash: String,
}

pub fn device_sensors_descriptor_hashes() -> SensorsDescriptorHashes {
    SensorsDescriptorHashes {
        command_schema_hash: sensors_stable_hash(&DEVICE_SENSORS_COMMANDS),
        result_schema_hash: sensors_stable_hash(&SensorsResultStatus::Success),
        descriptor_hash: sensors_stable_hash(&device_sensors_pack_definition()),
        provider_capability_hash: sensors_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        sensor_descriptor_hash: sensors_stable_hash(&SensorDescriptor {
            sensor_ref: "sensor".into(),
            sensor_type: SensorType {
                type_ref: "accelerometer".into(),
                family: "motion".into(),
                unit: "m_s2".into(),
            },
            max_sample_hz: 60,
            ..Default::default()
        }),
        stream_lease_hash: sensors_stable_hash(&SensorStreamLease {
            lease_ref: "lease".into(),
            sensor_ref: "sensor".into(),
            state: "active".into(),
            max_duration_ms: 1000,
            max_sample_count: 60,
            frequency_hz: 60,
            delivery_mode: "batch_ref".into(),
            cancellation_behavior: "stop_stream".into(),
            revocation_behavior: "release_lease".into(),
        }),
        calibration_hash: sensors_stable_hash(&SensorCalibration {
            calibration_ref: "calibration".into(),
            sensor_ref: "sensor".into(),
            state: "available".into(),
            metadata_ref: "calibration-ref".into(),
        }),
        redaction_profile_hash: sensors_stable_hash("sensors-redaction-v1"),
    }
}

pub fn sensors_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    device_stable_hash(value)
}
