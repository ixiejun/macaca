use std::collections::BTreeSet;

use super::super::device_camera::{CameraConstraints, CameraPreviewLease, CameraSession};
use super::super::device_foreground_background_host::{
    BackgroundLease, ForegroundSession, HostPresentationRequirement,
};
use super::super::device_local_files::LocalFileWritePlan;
use super::super::device_notifications::{NotificationContent, NotificationDeliveryPolicy};
use super::super::device_sensors::SensorStreamLease;

#[test]
fn camera_preflight_requires_explicit_session_constraints_and_cleanup() {
    let constraints = CameraConstraints {
        constraint_ref: "constraints-ref".into(),
        max_width: 1280,
        max_height: 720,
        max_fps: 30,
        output_intent: "photo".into(),
    };
    assert!(constraints.has_output_intent(2_000_000, 60));

    let session = CameraSession {
        session_ref: "session-ref".into(),
        camera_ref: "camera-ref".into(),
        state: "active".into(),
        constraints_ref: "constraints-ref".into(),
        resource_reservation_ref: "resource-ref".into(),
        cancellation_behavior: "stop_and_release".into(),
        revocation_behavior: "close_session".into(),
        expires_at_epoch_ms: 10,
    };
    assert!(session.has_explicit_lifecycle());

    let invalid_session = CameraSession {
        cancellation_behavior: String::new(),
        ..session
    };
    assert!(!invalid_session.has_explicit_lifecycle());

    let preview = CameraPreviewLease {
        lease_ref: "preview-ref".into(),
        session_ref: "session-ref".into(),
        state: "active".into(),
        max_duration_ms: 1_000,
    };
    assert!(preview.has_bounded_duration(2_000));
}

#[test]
fn local_file_preflight_requires_explicit_write_plan_for_mutations() {
    let plan = LocalFileWritePlan {
        plan_ref: "plan-ref".into(),
        handle_ref: "handle-ref".into(),
        mode: "overwrite".into(),
        destructive: true,
        approval_ref: Some("approval-ref".into()),
    };
    assert!(plan.is_explicit_for_write_operation());

    let missing_approval = LocalFileWritePlan {
        approval_ref: None,
        ..plan
    };
    assert!(!missing_approval.is_explicit_for_write_operation());
}

#[test]
fn notifications_preflight_requires_delivery_policy_and_redaction_class() {
    let policy = NotificationDeliveryPolicy {
        policy_ref: "policy-ref".into(),
        interruption_class: "active".into(),
        lock_screen_redaction: "hide_body".into(),
    };
    assert!(policy.is_explicit());

    let content = NotificationContent {
        content_ref: "content-ref".into(),
        title_hash: "title-hash".into(),
        body_hash: "body-hash".into(),
        redaction_class: "notification-redacted".into(),
    };
    assert!(content.has_delivery_redaction(&policy));

    let raw_content = NotificationContent {
        body_hash: String::new(),
        ..content
    };
    assert!(!raw_content.has_delivery_redaction(&policy));
}

#[test]
fn sensors_preflight_requires_full_stream_contract() {
    let lease = SensorStreamLease {
        lease_ref: "lease-ref".into(),
        sensor_ref: "sensor-ref".into(),
        state: "active".into(),
        max_duration_ms: 1_000,
        max_sample_count: 60,
        frequency_hz: 30,
        delivery_mode: "batch_ref".into(),
        cancellation_behavior: "stop_stream".into(),
        revocation_behavior: "release_lease".into(),
    };
    assert!(lease.has_explicit_stream_contract(2_000, 120, 60));

    let too_fast = SensorStreamLease {
        frequency_hz: 120,
        ..lease
    };
    assert!(!too_fast.has_explicit_stream_contract(2_000, 120, 60));
}

#[test]
fn host_lifecycle_preflight_requires_purpose_budget_dependencies_and_cleanup() {
    let foreground = ForegroundSession {
        session_ref: "session-ref".into(),
        purpose_ref: "purpose-ref".into(),
        presentation_requirement: HostPresentationRequirement {
            presentation_ref: "presentation-ref".into(),
            presentation_class: "visible".into(),
            user_visible: true,
        },
        state: "active".into(),
        max_duration_ms: 1_000,
        resource_budget_ref: "resource-ref".into(),
        cleanup_behavior: "release_on_close".into(),
    };
    assert!(foreground.has_explicit_contract(2_000));

    let background = BackgroundLease {
        lease_ref: "lease-ref".into(),
        purpose_ref: "purpose-ref".into(),
        lease_class: "short_task".into(),
        state: "active".into(),
        max_duration_ms: 1_000,
        resource_budget_ref: "resource-ref".into(),
        dependent_capabilities: BTreeSet::from(["device.sensors".into()]),
        cleanup_behavior: "release_on_expiry".into(),
    };
    assert!(background.has_explicit_contract(2_000));

    let missing_dependency = BackgroundLease {
        dependent_capabilities: BTreeSet::new(),
        ..background
    };
    assert!(!missing_dependency.has_explicit_contract(2_000));
}
