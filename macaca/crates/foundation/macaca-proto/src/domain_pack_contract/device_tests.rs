use std::collections::{BTreeMap, BTreeSet};

use super::device_camera::*;
use super::device_common::*;
use super::device_foreground_background_host::*;
use super::device_local_files::*;
use super::device_notifications::*;
use super::device_sensors::*;
use super::*;

// Device tests validate provider-neutral contract shape only. They do not
// contact host sensor, camera, filesystem, notification, lifecycle, browser, or
// remote-host APIs. Fixtures use synthetic references and hashes instead of raw
// samples, frames, media bytes, host paths, file contents, notification bodies,
// push tokens, host identifiers, session ids, provider payloads, or credentials.

#[test]
fn device_descriptors_are_discoverable_and_not_callable() {
    let cases = [
        (
            device_sensors_pack_definition(),
            DEVICE_SENSORS_PACK_ID,
            DEVICE_SENSORS_SERVICE_ID,
            DEVICE_SENSORS_COMMANDS,
            "device_sensors_provider_not_installed",
            "host-native",
            "sensors.open_stream",
        ),
        (
            device_camera_pack_definition(),
            DEVICE_CAMERA_PACK_ID,
            DEVICE_CAMERA_SERVICE_ID,
            DEVICE_CAMERA_COMMANDS,
            "device_camera_provider_not_installed",
            "browser",
            "camera.open_session",
        ),
        (
            device_local_files_pack_definition(),
            DEVICE_LOCAL_FILES_PACK_ID,
            DEVICE_LOCAL_FILES_SERVICE_ID,
            DEVICE_LOCAL_FILES_COMMANDS,
            "device_local_files_provider_not_installed",
            "remote-host",
            "local_files.request_directory_handle",
        ),
        (
            device_notifications_pack_definition(),
            DEVICE_NOTIFICATIONS_PACK_ID,
            DEVICE_NOTIFICATIONS_SERVICE_ID,
            DEVICE_NOTIFICATIONS_COMMANDS,
            "device_notifications_provider_not_installed",
            "host-native",
            "notifications.subscribe_interactions",
        ),
        (
            device_foreground_background_host_pack_definition(),
            DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID,
            DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID,
            DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS,
            "device_foreground_background_host_provider_not_installed",
            "browser",
            "host_lifecycle.request_background_lease",
        ),
    ];

    for (definition, pack_id, service_id, commands, unavailable_reason, provider_class, command) in
        cases
    {
        assert_eq!(definition.pack_id, pack_id);
        assert!(!definition.is_callable());
        assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
        assert_eq!(
            definition.metadata.parent_pack_id.as_deref(),
            Some("pack.device.v1")
        );
        assert_eq!(
            definition.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(definition
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/device"));
        assert!(definition
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|schemas| schemas.contains(command)));

        let descriptor_commands = definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .expect("device descriptor exposes command schemas");
        for expected in commands {
            assert!(
                descriptor_commands.contains(*expected),
                "missing command {expected}"
            );
        }
    }
}

#[test]
fn industrial_catalog_uses_specialized_device_descriptors() {
    let definitions = industrial_reference_domain_pack_definitions();
    let sensors = find_pack(&definitions, DEVICE_SENSORS_PACK_ID);
    let camera = find_pack(&definitions, DEVICE_CAMERA_PACK_ID);
    let local_files = find_pack(&definitions, DEVICE_LOCAL_FILES_PACK_ID);
    let notifications = find_pack(&definitions, DEVICE_NOTIFICATIONS_PACK_ID);
    let host = find_pack(&definitions, DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID);

    assert_eq!(
        sensors
            .metadata
            .provider_descriptors
            .get("host-native")
            .and_then(|descriptor| descriptor.metadata.get("raw_samples_in_trace"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        camera
            .metadata
            .provider_descriptors
            .get("browser")
            .and_then(|descriptor| descriptor.metadata.get("raw_media_bytes"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        local_files
            .metadata
            .provider_descriptors
            .get("host-native")
            .and_then(|descriptor| descriptor.metadata.get("raw_paths"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        notifications
            .metadata
            .provider_descriptors
            .get("browser")
            .and_then(|descriptor| descriptor.metadata.get("push_tokens_exposed"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        host.metadata
            .provider_descriptors
            .get("host-native")
            .and_then(|descriptor| descriptor.metadata.get("service_type_names"))
            .map(String::as_str),
        Some("not_os_semantics")
    );
}

#[test]
fn device_command_and_result_dtos_are_serde_compatible() {
    let envelope = DevicePackCommandEnvelope {
        subject_ref: "device:subject".into(),
        parameters: BTreeMap::from([("mode".into(), "synthetic".into())]),
        cursor: None,
        page_size: Some(20),
        idempotency_key: Some("idem-device".into()),
    };

    let values = [
        serde_json::to_value(SensorsOpenStreamCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(CameraOpenSessionCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(LocalFilesRequestDirectoryHandleCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(NotificationsSubscribeInteractionsCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(HostLifecycleRequestBackgroundLeaseCommand { request: envelope })
            .unwrap(),
        serde_json::to_value(SensorsResultEnvelope::<SensorDescriptor> {
            status: SensorsResultStatus::ForegroundRequired,
            data: None,
            page: None,
            error: Some(DevicePackError {
                code: "foreground_required".into(),
                message: "synthetic foreground requirement".into(),
                retryable: false,
                trace_safe_detail: Some("host_policy".into()),
            }),
        })
        .unwrap(),
        serde_json::to_value(CameraResultEnvelope::<CameraSession> {
            status: CameraResultStatus::SessionRevoked,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(LocalFilesResultEnvelope::<LocalFileHandle> {
            status: LocalFilesResultStatus::HandleRevoked,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(NotificationsResultEnvelope::<NotificationRecord> {
            status: NotificationsResultStatus::SensitiveContentBlocked,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(HostLifecycleResultEnvelope::<HostLifecycleState> {
            status: HostLifecycleResultStatus::Throttled,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
    ];

    assert!(values.iter().all(|value| value.is_object()));
}

#[test]
fn device_descriptor_hashes_are_stable_and_distinct() {
    let hash_groups = [
        hash_values(&device_sensors_descriptor_hashes()),
        hash_values(&device_camera_descriptor_hashes()),
        hash_values(&device_local_files_descriptor_hashes()),
        hash_values(&device_notifications_descriptor_hashes()),
        hash_values(&device_foreground_background_host_descriptor_hashes()),
    ];

    for hashes in hash_groups {
        let unique = hashes.into_iter().collect::<BTreeSet<_>>();
        assert!(unique.len() >= 7);
        assert!(unique.iter().all(|hash| !hash.is_empty()));
    }
}

#[test]
fn device_validation_helpers_are_provider_neutral() {
    let lease = SensorStreamLease {
        max_duration_ms: 1_000,
        max_sample_count: 60,
        ..Default::default()
    };
    let constraints = CameraConstraints {
        max_width: 1280,
        max_height: 720,
        max_fps: 30,
        ..Default::default()
    };
    let transfer = LocalFileTransfer {
        bytes_total: 1024,
        ..Default::default()
    };
    let content = NotificationContent {
        title_hash: "title".into(),
        body_hash: "body".into(),
        ..Default::default()
    };
    let session = ForegroundSession {
        presentation_requirement: HostPresentationRequirement {
            presentation_class: "visible".into(),
            ..Default::default()
        },
        max_duration_ms: 1_000,
        ..Default::default()
    };

    assert!(lease.is_bounded());
    assert!(constraints.is_bounded(1_000_000, 60));
    assert!(transfer.is_bounded(2048));
    assert!(content.is_redacted());
    assert!(session.is_bounded());
}

#[test]
fn invalid_device_descriptor_is_rejected() {
    let mut invalid = device_sensors_pack_definition();
    invalid.pack_id = "device.sensors.v1".into();

    assert!(DomainPackDefinitionSpec.validate(&invalid).is_err());
}

fn hash_values<T: serde::Serialize>(value: &T) -> Vec<String> {
    let json = serde_json::to_value(value).expect("descriptor hash fixture serializes");
    json.as_object()
        .expect("descriptor hashes serialize as object")
        .values()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect()
}

fn find_pack<'a>(
    definitions: &'a [DomainPackDefinition],
    pack_id: &str,
) -> &'a DomainPackDefinition {
    definitions
        .iter()
        .find(|definition| definition.pack_id == pack_id)
        .expect("industrial catalog includes specialized device descriptor")
}
