use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::device_common::{
    define_device_command_wrappers, device_pack_definition, device_stable_hash,
    DevicePackCommandEnvelope, DevicePackDescriptor, DevicePackError, DevicePackPage,
    DeviceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const DEVICE_CAMERA_PACK_ID: &str = "pack.device.camera.v1";
pub const DEVICE_CAMERA_SERVICE_ID: &str = "service.device.camera";

pub const DEVICE_CAMERA_COMMANDS: &[&str] = &[
    "camera.inspect_authorization",
    "camera.request_authorization",
    "camera.list_devices",
    "camera.inspect_device",
    "camera.open_session",
    "camera.start_preview",
    "camera.stop_preview",
    "camera.capture_photo",
    "camera.start_recording",
    "camera.stop_recording",
    "camera.read_frame",
    "camera.set_controls",
    "camera.inspect_controls",
    "camera.close_session",
    "camera.inspect_host",
];

pub const CAMERA_PERMISSION_SCOPES: &[&str] = &[
    "device.camera.read_status",
    "device.camera.request_permission",
    "device.camera.preview",
    "device.camera.capture_photo",
    "device.camera.record_video",
    "device.camera.read_frame",
    "device.camera.controls",
    "device.camera.session.manage",
];

const CAMERA_HOST_METADATA: &[(&str, &str)] = &[
    ("host_native", "true"),
    ("privacy_indicator", "required_when_available"),
    ("raw_frames_in_trace", "false"),
];
const CAMERA_BROWSER_METADATA: &[(&str, &str)] = &[
    ("browser", "true"),
    ("permission_prompt", "host_owned"),
    ("raw_media_bytes", "false"),
];
const CAMERA_REMOTE_METADATA: &[(&str, &str)] = &[
    ("remote_host", "true"),
    ("capture", "approval_required"),
    ("stable_device_ids", "redacted"),
];
const CAMERA_MOCK_METADATA: &[(&str, &str)] = &[("fixtures", "synthetic"), ("callable", "false")];
const CAMERA_UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("capture", "false"), ("reason", "provider_not_installed")];

const CAMERA_PROVIDER_CLASSES: &[DeviceProviderClass<'_>] = &[
    DeviceProviderClass {
        provider_class: "host-native",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CAMERA_HOST_METADATA,
    },
    DeviceProviderClass {
        provider_class: "browser",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CAMERA_BROWSER_METADATA,
    },
    DeviceProviderClass {
        provider_class: "remote-host",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CAMERA_REMOTE_METADATA,
    },
    DeviceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CAMERA_MOCK_METADATA,
    },
    DeviceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: CAMERA_UNAVAILABLE_METADATA,
    },
];

/// Build the camera descriptor without binding host camera APIs or media providers.
pub fn device_camera_pack_definition() -> DomainPackDefinition {
    device_pack_definition(DevicePackDescriptor {
        pack_id: DEVICE_CAMERA_PACK_ID,
        child_change_id: "openspec:add-pack-device-camera",
        docs_slug: "camera",
        sdk_slug: "camera",
        service_id: DEVICE_CAMERA_SERVICE_ID,
        commands: DEVICE_CAMERA_COMMANDS,
        permission_scopes: CAMERA_PERMISSION_SCOPES,
        provider_classes: CAMERA_PROVIDER_CLASSES,
        health_probe: "camera.inspect_host",
        unavailable_reason: "device_camera_provider_not_installed",
        replay_schema: "device.camera.replay.v1",
        data_classification: "device_camera_reference_metadata",
        retention_policy: "authorization_device_session_preview_frame_media_control_and_host_status_metadata_by_reference",
        redaction_policy: "raw_frames_raw_media_stable_hardware_identifiers_faces_documents_provider_payloads_session_ids_and_credentials_redacted",
        timeout_ms: 120_000,
        budget_units: 6,
        examples: &[
            "Declare `pack.device.camera.v1` as optional until a camera provider is installed.",
            "Use authorization descriptors, session references, frame references, and media handles instead of raw host camera APIs.",
        ],
        migration_notes: &[
            "Camera commands become callable only after an approved camera service provider registers matching schemas.",
            "Media processing, AI vision, OCR, local file persistence, sensors, and application capture UI remain separate capability owners.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraAuthorization {
    pub authorization_ref: String,
    pub state: String,
    pub prompt_allowed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraDescriptor {
    pub camera_ref: String,
    pub facing_mode: String,
    pub privacy_class: String,
    pub supported_outputs: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraConstraints {
    pub constraint_ref: String,
    pub max_width: u32,
    pub max_height: u32,
    pub max_fps: u32,
    pub output_intent: String,
}

impl CameraConstraints {
    /// Keep capture requests bounded before provider dispatch.
    pub fn is_bounded(&self, max_pixels: u64, max_fps: u32) -> bool {
        u64::from(self.max_width) * u64::from(self.max_height) <= max_pixels
            && self.max_fps <= max_fps
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraSession {
    pub session_ref: String,
    pub camera_ref: String,
    pub state: String,
    pub constraints_ref: String,
    pub resource_reservation_ref: String,
    pub cancellation_behavior: String,
    pub revocation_behavior: String,
    pub expires_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraPreviewLease {
    pub lease_ref: String,
    pub session_ref: String,
    pub state: String,
    pub max_duration_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraFrameReference {
    pub frame_ref: String,
    pub session_ref: String,
    pub media_ref: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraMediaReference {
    pub media_ref: String,
    pub media_kind: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraControls {
    pub controls_ref: String,
    pub supported_controls: BTreeSet<String>,
    pub requested_values: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraHostStatus {
    pub host_ref: String,
    pub enabled: bool,
    pub foreground_required: bool,
    pub privacy_indicator_available: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraError {
    pub code: String,
    pub trace_safe_detail: String,
    pub retryable: bool,
}

define_device_command_wrappers!(
    CameraInspectAuthorizationCommand,
    CameraRequestAuthorizationCommand,
    CameraListDevicesCommand,
    CameraInspectDeviceCommand,
    CameraOpenSessionCommand,
    CameraStartPreviewCommand,
    CameraStopPreviewCommand,
    CameraCapturePhotoCommand,
    CameraStartRecordingCommand,
    CameraStopRecordingCommand,
    CameraReadFrameCommand,
    CameraSetControlsCommand,
    CameraInspectControlsCommand,
    CameraCloseSessionCommand,
    CameraInspectHostCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    PromptNotAllowed,
    ForegroundRequired,
    DeviceUnavailable,
    ConstraintUnsatisfied,
    SessionExpired,
    SessionRevoked,
    PrivacyIndicatorUnavailable,
    CaptureInterrupted,
    MediaTooLarge,
    QuotaExceeded,
    ProviderFailure,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraResultEnvelope<T> {
    pub status: CameraResultStatus,
    pub data: Option<T>,
    pub page: Option<DevicePackPage<T>>,
    pub error: Option<DevicePackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub authorization_hash: String,
    pub device_descriptor_hash: String,
    pub session_hash: String,
    pub media_reference_hash: String,
    pub redaction_profile_hash: String,
}

pub fn device_camera_descriptor_hashes() -> CameraDescriptorHashes {
    CameraDescriptorHashes {
        command_schema_hash: camera_stable_hash(&DEVICE_CAMERA_COMMANDS),
        result_schema_hash: camera_stable_hash(&CameraResultStatus::Success),
        descriptor_hash: camera_stable_hash(&device_camera_pack_definition()),
        authorization_hash: camera_stable_hash(&CameraAuthorization {
            authorization_ref: "authorization".into(),
            state: "promptable".into(),
            prompt_allowed: true,
        }),
        device_descriptor_hash: camera_stable_hash(&CameraDescriptor {
            camera_ref: "camera".into(),
            facing_mode: "environment".into(),
            privacy_class: "camera".into(),
            supported_outputs: BTreeSet::from(["photo".into(), "video".into()]),
        }),
        session_hash: camera_stable_hash(&CameraSession {
            session_ref: "session".into(),
            camera_ref: "camera".into(),
            state: "active".into(),
            constraints_ref: "constraints".into(),
            resource_reservation_ref: "resource".into(),
            cancellation_behavior: "stop_and_release".into(),
            revocation_behavior: "close_session".into(),
            expires_at_epoch_ms: 10,
        }),
        media_reference_hash: camera_stable_hash(&CameraMediaReference {
            media_ref: "media".into(),
            media_kind: "photo".into(),
            checksum: "checksum".into(),
            expires_at_epoch_ms: 10,
        }),
        redaction_profile_hash: camera_stable_hash("camera-redaction-v1"),
    }
}

pub fn camera_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    device_stable_hash(value)
}
