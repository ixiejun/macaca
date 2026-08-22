//! Conformance tests for the provider-neutral device-camera Strategy.

use macaca_kernel::SystemService;
use macaca_proto::device_camera::DEVICE_CAMERA_COMMANDS;
use macaca_proto::{
    CameraPreflightFacts, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth,
    TraceContext,
};

use super::device_camera_service_provider::DeviceCameraSystemServiceProvider;

#[tokio::test]
async fn camera_commands_are_traceable_and_do_not_echo_frames_or_media() {
    let provider = DeviceCameraSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    for command in DEVICE_CAMERA_COMMANDS {
        let marker = "private-camera-frame-media-marker";
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({"raw_frame":marker,"media_bytes":marker,"device_id":marker}),
                TraceContext::new(format!("camera-{command}")),
            ))
            .await
            .unwrap();
        assert_eq!(result.output["status"], "reference_only");
        assert!(!result.output.to_string().contains(marker));
        let event = receive_outcome(&mut events, "completed").await;
        assert_eq!(event.outcome, "completed");
        assert!(event.command.starts_with("camera."));
        assert!(!event.replay_ref.contains(marker));
    }
}

#[tokio::test]
async fn unavailable_camera_fails_closed_and_snapshot_is_bounded() {
    let provider = DeviceCameraSystemServiceProvider::unavailable("module_absent");
    let mut events = provider.subscribe();
    assert!(matches!(
        provider.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    for command in DEVICE_CAMERA_COMMANDS {
        assert!(matches!(
            provider
                .call(ServiceCommand::with_trace(
                    ServiceCommandName::new(*command),
                    serde_json::json!({"raw_frame":"must-not-open"}),
                    TraceContext::new(format!("unavailable-{command}"))
                ))
                .await,
            Err(ServiceError::ServiceUnavailable(_))
        ));
        assert_eq!(events.recv().await.unwrap().outcome, "unavailable");
    }
    let snapshot = provider.snapshot().await;
    assert_eq!(snapshot["snapshot_schema"], "device.camera.replay.v1");
    let event = events.recv().await.unwrap();
    assert_eq!(event.command, "camera.snapshot_recorded");
    assert_eq!(event.outcome, "snapshot_recorded");
}

#[tokio::test]
async fn cleanup_releases_synthetic_camera_session_and_output_resources() {
    let provider = DeviceCameraSystemServiceProvider::mock();
    for command in ["camera.open_session", "camera.start_preview"] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(command),
                serde_json::json!({}),
                TraceContext::new(command),
            ))
            .await
            .unwrap();
    }
    let active = provider.snapshot().await;
    assert_eq!(active["active_session_count"], "1");
    assert_eq!(active["active_output_count"], "1");
    provider.cleanup().await.unwrap();
    let released = provider.snapshot().await;
    assert_eq!(released["active_session_count"], "0");
    assert_eq!(released["active_output_count"], "0");
}

#[tokio::test]
async fn host_rejections_do_not_complete_camera_work() {
    for (index, facts) in [
        CameraPreflightFacts {
            foreground_active: false,
            ..CameraPreflightFacts::permissive()
        },
        CameraPreflightFacts {
            privacy_indicator_available: false,
            ..CameraPreflightFacts::permissive()
        },
        CameraPreflightFacts {
            constraints_valid: false,
            ..CameraPreflightFacts::permissive()
        },
        CameraPreflightFacts {
            requested_units: 2,
            reserved_units: 1,
            ..CameraPreflightFacts::permissive()
        },
        CameraPreflightFacts {
            approval_required: true,
            approval_granted: false,
            ..CameraPreflightFacts::permissive()
        },
    ]
    .into_iter()
    .enumerate()
    {
        let provider = DeviceCameraSystemServiceProvider::mock().with_admission_facts(facts);
        let mut events = provider.subscribe();
        assert!(provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("camera.capture_photo"),
                serde_json::json!({"raw_frame":"must-not-observe"}),
                TraceContext::new(format!("camera-rejected-{index}"))
            ))
            .await
            .is_err());
        let event = receive_command(&mut events, "camera.policy_decision").await;
        assert_eq!(event.command, "camera.policy_decision");
        assert_eq!(event.outcome, "preflight_rejected");
    }
}

#[tokio::test]
async fn canonical_camera_operations_emit_sanitized_audit_taxonomy() {
    let provider = DeviceCameraSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    for (operation, expected_event) in [
        ("camera.open_session", "camera.session_opened"),
        ("camera.start_preview", "camera.preview_started"),
        ("camera.capture_photo", "camera.photo_captured"),
        ("camera.start_recording", "camera.recording_started"),
        ("camera.stop_recording", "camera.recording_stopped"),
        ("camera.read_frame", "camera.frame_reference_created"),
        ("camera.set_controls", "camera.controls_changed"),
        ("camera.close_session", "camera.session_closed"),
    ] {
        provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(operation),
                serde_json::json!({"media_bytes":"must-not-appear"}),
                TraceContext::new(operation),
            ))
            .await
            .unwrap();
        let event = receive_command(&mut events, expected_event).await;
        assert_eq!(event.command, expected_event);
        assert!(!event.replay_ref.contains("must-not-appear"));
    }
}

#[test]
fn every_camera_command_has_a_specific_audit_event() {
    let expected = [
        (
            "camera.inspect_authorization",
            "camera.authorization_inspected",
        ),
        (
            "camera.request_authorization",
            "camera.authorization_requested",
        ),
        ("camera.list_devices", "camera.devices_listed"),
        ("camera.inspect_device", "camera.device_inspected"),
        ("camera.open_session", "camera.session_opened"),
        ("camera.start_preview", "camera.preview_started"),
        ("camera.stop_preview", "camera.preview_stopped"),
        ("camera.capture_photo", "camera.photo_captured"),
        ("camera.start_recording", "camera.recording_started"),
        ("camera.stop_recording", "camera.recording_stopped"),
        ("camera.read_frame", "camera.frame_reference_created"),
        ("camera.set_controls", "camera.controls_changed"),
        ("camera.inspect_controls", "camera.controls_inspected"),
        ("camera.close_session", "camera.session_closed"),
        ("camera.inspect_host", "camera.host_inspected"),
    ];
    assert_eq!(expected.len(), DEVICE_CAMERA_COMMANDS.len());
    for command in DEVICE_CAMERA_COMMANDS {
        assert!(expected.iter().any(|(known, _)| known == command));
    }
}

#[tokio::test]
async fn camera_results_events_and_snapshots_redact_sensitive_provider_data() {
    let provider = DeviceCameraSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    let sensitive = serde_json::json!({
        "raw_frame": "raw-frame-marker",
        "media_bytes": "media-bytes-marker",
        "hardware_id": "hardware-marker",
        "detected_face": "face-marker",
        "document": "document-marker",
        "provider_payload": "provider-marker",
        "credential": "credential-marker",
        "session_id": "session-marker",
        "media_reference": "media-reference-marker",
        "snapshot": "snapshot-marker",
        "diagnostics": "diagnostics-marker"
    });
    let result = provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("camera.capture_photo"),
            sensitive,
            TraceContext::new("camera-redaction"),
        ))
        .await
        .unwrap();
    let event = events.recv().await.unwrap();
    let snapshot = provider.snapshot().await;
    let snapshot_event = events.recv().await.unwrap();

    let observable = format!("{result:?}{event:?}{snapshot:?}{snapshot_event:?}");
    for marker in [
        "raw-frame-marker",
        "media-bytes-marker",
        "hardware-marker",
        "face-marker",
        "document-marker",
        "provider-marker",
        "credential-marker",
        "session-marker",
        "media-reference-marker",
        "snapshot-marker",
        "diagnostics-marker",
    ] {
        assert!(
            !observable.contains(marker),
            "sensitive marker leaked: {marker}"
        );
    }
}

#[tokio::test]
async fn camera_replay_reference_is_stable_after_provider_restart() {
    let trace_id = "camera-restart-trace";
    let first = DeviceCameraSystemServiceProvider::mock();
    first
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("camera.inspect_host"),
            serde_json::json!({"raw_frame":"must-not-replay"}),
            TraceContext::new(trace_id),
        ))
        .await
        .unwrap();
    first.cleanup().await.unwrap();

    // Recreating the Strategy models a host refresh while preserving only the
    // deterministic, trace-derived replay address.
    let restarted = DeviceCameraSystemServiceProvider::mock();
    let mut events = restarted.subscribe();
    restarted
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("camera.inspect_host"),
            serde_json::json!({"media_bytes":"must-not-replay"}),
            TraceContext::new(trace_id),
        ))
        .await
        .unwrap();
    let event = events.recv().await.unwrap();
    assert_eq!(event.trace_id, trace_id);
    assert_eq!(event.replay_ref, format!("replay:camera:{trace_id}"));
}

async fn receive_outcome(
    events: &mut tokio::sync::broadcast::Receiver<
        super::device_camera_service_provider::CameraRuntimeEvent,
    >,
    expected: &str,
) -> super::device_camera_service_provider::CameraRuntimeEvent {
    loop {
        let event = events.recv().await.unwrap();
        if event.outcome == expected {
            return event;
        }
    }
}

async fn receive_command(
    events: &mut tokio::sync::broadcast::Receiver<
        super::device_camera_service_provider::CameraRuntimeEvent,
    >,
    expected: &str,
) -> super::device_camera_service_provider::CameraRuntimeEvent {
    loop {
        let event = events.recv().await.unwrap();
        if event.command == expected {
            return event;
        }
    }
}

#[tokio::test]
async fn camera_emits_stable_pack_admission_and_policy_events() {
    let provider = DeviceCameraSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider.start().await.unwrap();
    provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("camera.request_authorization"),
            serde_json::json!({"raw_frame":"redacted"}),
            TraceContext::new("camera-audit"),
        ))
        .await
        .unwrap();
    let mut names = Vec::new();
    while let Ok(event) = events.try_recv() {
        names.push(event.event_name);
    }
    for expected in [
        "camera.pack_declared",
        "camera.admission_validated",
        "camera.policy_decision",
        "camera.authorization_requested",
    ] {
        assert!(
            names.iter().any(|name| name == expected),
            "missing {expected}"
        );
    }
}
