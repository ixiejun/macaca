//! Runtime-host Strategy for provider-neutral device-camera commands.
//!
//! The mock produces synthetic references only. It never opens a host camera,
//! reads raw frames, retains media bytes, or exposes hardware identifiers.

use std::collections::BTreeMap;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::device_camera::{
    DEVICE_CAMERA_COMMANDS, DEVICE_CAMERA_PACK_ID, DEVICE_CAMERA_SERVICE_ID,
};
use macaca_proto::{
    admit_camera_operation, domain_pack_command_trace, domain_pack_service_result,
    CameraPreflightFacts, CameraPreflightFailure, CameraSessionAction, CameraSessionState,
    KernelServiceId, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
};
use tracing::{info, warn};

/// Bounded audit/replay event with no frame, media, or device payload fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub replay_ref: String,
    pub outcome: &'static str,
}

/// Mock/Null Object provider chosen exclusively from runtime-host composition.
pub struct DeviceCameraSystemServiceProvider {
    unavailable_reason: Option<String>,
    admission_facts: CameraPreflightFacts,
    events: tokio::sync::broadcast::Sender<CameraRuntimeEvent>,
}

impl DeviceCameraSystemServiceProvider {
    /// Build deterministic synthetic-reference behavior for contract tests.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Build a fail-closed provider when no camera adapter is installed.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = tokio::sync::broadcast::channel(256);
        Self {
            unavailable_reason,
            admission_facts: CameraPreflightFacts::permissive(),
            events,
        }
    }

    /// Attach host-issued evidence without reading caller command metadata.
    pub fn with_admission_facts(mut self, admission_facts: CameraPreflightFacts) -> Self {
        self.admission_facts = admission_facts;
        self
    }

    /// Subscribe to sanitized events suitable for audit and replay tests.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<CameraRuntimeEvent> {
        self.events.subscribe()
    }

    /// Produce a bounded Memento for host recovery diagnostics.
    pub fn snapshot(&self) -> BTreeMap<String, String> {
        let state = if self.unavailable_reason.is_some() {
            "unavailable"
        } else {
            "preview"
        };
        let snapshot = BTreeMap::from([
            (
                "provider_class".into(),
                if self.unavailable_reason.is_some() {
                    "unavailable".into()
                } else {
                    "mock".into()
                },
            ),
            ("capability_state".into(), state.into()),
            (
                "command_count".into(),
                DEVICE_CAMERA_COMMANDS.len().to_string(),
            ),
            ("snapshot_schema".into(), "device.camera.replay.v1".into()),
        ]);
        self.emit("camera.snapshot", "snapshot:provider", "snapshot_recorded");
        info!(
            service_id = DEVICE_CAMERA_SERVICE_ID,
            "device camera provider snapshot recorded"
        );
        snapshot
    }

    fn emit(&self, command: &str, trace_id: &str, outcome: &'static str) {
        let _ = self.events.send(CameraRuntimeEvent {
            command: command.into(),
            trace_id: trace_id.into(),
            replay_ref: format!("replay:camera:{trace_id}"),
            outcome,
        });
    }
}

#[async_trait]
impl SystemService for DeviceCameraSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(DEVICE_CAMERA_SERVICE_ID),
            ServiceType::new("device.camera"),
            TraceSchemaRef::new("device.camera.replay.v1"),
        );
        descriptor
            .metadata
            .insert("pack_id".into(), DEVICE_CAMERA_PACK_ID.into());
        descriptor.metadata.insert(
            "provider_class".into(),
            if self.unavailable_reason.is_some() {
                "unavailable"
            } else {
                "mock"
            }
            .into(),
        );
        descriptor.metadata.insert(
            "command_count".into(),
            DEVICE_CAMERA_COMMANDS.len().to_string(),
        );
        descriptor
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = DEVICE_CAMERA_SERVICE_ID,
            "device camera provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        let operation = command.name.as_str();
        if !DEVICE_CAMERA_COMMANDS.contains(&operation) {
            self.emit(operation, &trace.trace_id, "unsupported");
            return Err(ServiceError::UnsupportedCommand(operation.into()));
        }
        if let Some(reason) = &self.unavailable_reason {
            self.emit(operation, &trace.trace_id, "unavailable");
            warn!(service_id = DEVICE_CAMERA_SERVICE_ID, command = operation, trace_id = %trace.trace_id, reason_code = %reason, "device camera provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if let Err(rejection) = admit_camera_operation(self.admission_facts) {
            self.emit(operation, &trace.trace_id, "preflight_rejected");
            warn!(service_id = DEVICE_CAMERA_SERVICE_ID, command = operation, trace_id = %trace.trace_id, rejection = ?rejection, "device camera command rejected before adapter dispatch");
            return Err(preflight_error(rejection));
        }
        self.emit(operation, &trace.trace_id, "completed");
        info!(service_id = DEVICE_CAMERA_SERVICE_ID, command = operation, trace_id = %trace.trace_id, "device camera command completed with synthetic references");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"reference_only","operation":operation,"result_ref":format!("camera-reference:{}", trace.trace_id)}),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        Ok(())
    }
    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self
            .unavailable_reason
            .as_ref()
            .map_or(ServiceHealth::Healthy, |reason| {
                ServiceHealth::Unavailable {
                    reason: reason.clone(),
                }
            }))
    }
}

fn preflight_error(rejection: CameraPreflightFailure) -> ServiceError {
    match rejection {
        CameraPreflightFailure::Unavailable => {
            ServiceError::ServiceUnavailable("camera_provider_unavailable".into())
        }
        CameraPreflightFailure::QuotaExceeded
        | CameraPreflightFailure::Timeout
        | CameraPreflightFailure::Cancellation => {
            ServiceError::AdapterFailure(format!("camera_{rejection:?}").to_lowercase())
        }
        _ => ServiceError::DisabledByPolicy(format!("camera_{rejection:?}").to_lowercase()),
    }
}

/// Keep the State pattern visible at the provider boundary for adapter authors.
pub fn camera_session_transition_for_adapter(
    state: CameraSessionState,
    action: CameraSessionAction,
) -> Option<CameraSessionState> {
    macaca_proto::transition_camera_session(state, action)
}
