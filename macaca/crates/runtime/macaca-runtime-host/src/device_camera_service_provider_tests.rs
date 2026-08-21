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
        assert_eq!(events.recv().await.unwrap().outcome, "completed");
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
    let snapshot = provider.snapshot();
    assert_eq!(snapshot["snapshot_schema"], "device.camera.replay.v1");
    assert_eq!(events.recv().await.unwrap().outcome, "snapshot_recorded");
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
        assert_eq!(events.recv().await.unwrap().outcome, "preflight_rejected");
    }
}
