use super::device_camera::{CameraConstraints, CameraPreviewLease, CameraSession};
use super::device_common::bounded_device_token;
use super::device_foreground_background_host::{
    BackgroundLease, ForegroundSession, HostLifecyclePolicy,
};
use super::device_local_files::LocalFileWritePlan;
use super::device_notifications::{NotificationContent, NotificationDeliveryPolicy};
use super::device_sensors::SensorStreamLease;

impl CameraConstraints {
    /// Validate capture constraints include a provider-neutral output intent.
    pub fn has_output_intent(&self, max_pixels: u64, max_fps: u32) -> bool {
        self.is_bounded(max_pixels, max_fps)
            && bounded_device_token(&self.constraint_ref, 160)
            && matches!(
                self.output_intent.as_str(),
                "preview" | "photo" | "video" | "frame_analysis"
            )
    }
}

impl CameraSession {
    /// Validate camera sessions declare constraints, resource reservation, and cleanup behavior.
    pub fn has_explicit_lifecycle(&self) -> bool {
        bounded_device_token(&self.session_ref, 160)
            && bounded_device_token(&self.camera_ref, 160)
            && matches!(
                self.state.as_str(),
                "opening" | "active" | "closing" | "closed"
            )
            && bounded_device_token(&self.constraints_ref, 160)
            && bounded_device_token(&self.resource_reservation_ref, 160)
            && matches!(
                self.cancellation_behavior.as_str(),
                "stop_and_release" | "retain_until_close"
            )
            && matches!(
                self.revocation_behavior.as_str(),
                "close_session" | "drop_outputs"
            )
            && self.expires_at_epoch_ms > 0
    }
}

impl CameraPreviewLease {
    /// Validate preview leases declare bounded duration and revocable state.
    pub fn has_bounded_duration(&self, max_duration_ms: u64) -> bool {
        bounded_device_token(&self.lease_ref, 160)
            && bounded_device_token(&self.session_ref, 160)
            && matches!(self.state.as_str(), "active" | "revoked" | "expired")
            && self.max_duration_ms > 0
            && self.max_duration_ms <= max_duration_ms
    }
}

impl LocalFileWritePlan {
    /// Validate every mutating local-file operation has an explicit plan and approval shape.
    pub fn is_explicit_for_write_operation(&self) -> bool {
        bounded_device_token(&self.plan_ref, 160)
            && bounded_device_token(&self.handle_ref, 160)
            && matches!(
                self.mode.as_str(),
                "write" | "append" | "truncate" | "export" | "overwrite"
            )
            && (!self.destructive
                || self
                    .approval_ref
                    .as_deref()
                    .is_some_and(|approval| bounded_device_token(approval, 160)))
    }
}

impl NotificationDeliveryPolicy {
    /// Validate notification delivery policy is explicit before post or schedule.
    pub fn is_explicit(&self) -> bool {
        bounded_device_token(&self.policy_ref, 160)
            && matches!(
                self.interruption_class.as_str(),
                "passive" | "active" | "time_sensitive" | "critical"
            )
            && matches!(
                self.lock_screen_redaction.as_str(),
                "hide_body" | "hide_all" | "show_redacted"
            )
    }
}

impl NotificationContent {
    /// Validate notification content carries redaction metadata and hash-only text evidence.
    pub fn has_delivery_redaction(&self, policy: &NotificationDeliveryPolicy) -> bool {
        self.is_redacted()
            && bounded_device_token(&self.content_ref, 160)
            && bounded_device_token(&self.redaction_class, 96)
            && policy.is_explicit()
    }
}

impl SensorStreamLease {
    /// Validate sensor streams declare duration, sample count, frequency, delivery, and cleanup.
    pub fn has_explicit_stream_contract(
        &self,
        max_duration_ms: u64,
        max_sample_count: u64,
        max_hz: u32,
    ) -> bool {
        self.is_bounded()
            && bounded_device_token(&self.lease_ref, 160)
            && bounded_device_token(&self.sensor_ref, 160)
            && matches!(self.state.as_str(), "active" | "revoked" | "expired")
            && self.max_duration_ms <= max_duration_ms
            && self.max_sample_count <= max_sample_count
            && self.frequency_hz > 0
            && self.frequency_hz <= max_hz
            && matches!(self.delivery_mode.as_str(), "batch_ref" | "event_ref")
            && matches!(
                self.cancellation_behavior.as_str(),
                "stop_stream" | "flush_then_stop"
            )
            && matches!(
                self.revocation_behavior.as_str(),
                "release_lease" | "drop_buffer"
            )
    }
}

impl ForegroundSession {
    /// Validate foreground sessions declare purpose, presentation, budget, and cleanup.
    pub fn has_explicit_contract(&self, max_duration_ms: u64) -> bool {
        self.is_bounded()
            && bounded_device_token(&self.session_ref, 160)
            && bounded_device_token(&self.purpose_ref, 160)
            && self.max_duration_ms <= max_duration_ms
            && bounded_device_token(&self.resource_budget_ref, 160)
            && matches!(
                self.cleanup_behavior.as_str(),
                "release_on_close" | "release_on_expiry"
            )
    }
}

impl BackgroundLease {
    /// Validate background leases declare purpose, budget, dependent capabilities, and cleanup.
    pub fn has_explicit_contract(&self, max_duration_ms: u64) -> bool {
        bounded_device_token(&self.lease_ref, 160)
            && bounded_device_token(&self.purpose_ref, 160)
            && matches!(
                self.lease_class.as_str(),
                "short_task" | "event_delivery" | "sync"
            )
            && matches!(self.state.as_str(), "active" | "revoked" | "expired")
            && self.max_duration_ms > 0
            && self.max_duration_ms <= max_duration_ms
            && bounded_device_token(&self.resource_budget_ref, 160)
            && !self.dependent_capabilities.is_empty()
            && self.dependent_capabilities.iter().all(|capability| {
                capability.starts_with("device.") && bounded_device_token(capability, 160)
            })
            && matches!(
                self.cleanup_behavior.as_str(),
                "release_on_close" | "release_on_expiry"
            )
    }
}

impl HostLifecyclePolicy {
    /// Validate lifecycle policy carries dependent capability evidence.
    pub fn has_dependent_capabilities(&self) -> bool {
        bounded_device_token(&self.policy_ref, 160)
            && self.dependent_capabilities.iter().all(|capability| {
                capability.starts_with("device.") && bounded_device_token(capability, 160)
            })
    }
}
