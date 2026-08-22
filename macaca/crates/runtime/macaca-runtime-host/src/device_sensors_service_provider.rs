//! Runtime-host Strategy for provider-neutral device sensor commands.
//!
//! The deterministic provider returns opaque references and aggregate counts.
//! It never reads sample vectors, calibration payloads, hardware identifiers,
//! or host sensor APIs; concrete adapters belong at the runtime composition root.

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::device_sensors::{
    DEVICE_SENSORS_COMMANDS, DEVICE_SENSORS_PACK_ID, DEVICE_SENSORS_SERVICE_ID,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorRuntimeEvent {
    pub command: String,
    /// Stable provider-neutral audit name; sample payloads remain omitted.
    pub event_name: String,
    pub trace_id: String,
    pub replay_ref: String,
    pub outcome: &'static str,
}

/// Provider-neutral stream lease lifecycle; invalid transitions fail closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorLeaseState {
    Requested,
    Active,
    Draining,
    Closed,
    Expired,
    Revoked,
    Failed,
    Unavailable,
}

pub fn transition_sensor_lease(
    state: SensorLeaseState,
    operation: &str,
) -> Option<SensorLeaseState> {
    match (state, operation) {
        (SensorLeaseState::Requested, "open") => Some(SensorLeaseState::Active),
        (SensorLeaseState::Active, "drain") => Some(SensorLeaseState::Draining),
        (SensorLeaseState::Active, "close") | (SensorLeaseState::Draining, "close") => {
            Some(SensorLeaseState::Closed)
        }
        (SensorLeaseState::Active, "expire") => Some(SensorLeaseState::Expired),
        (SensorLeaseState::Active, "revoke") => Some(SensorLeaseState::Revoked),
        _ => None,
    }
}

#[derive(Default)]
struct SensorLedger {
    streams: RwLock<usize>,
    leases: RwLock<usize>,
}

impl SensorLedger {
    async fn record(&self, operation: &str) {
        match operation {
            "sensors.open_stream" | "sensors.acquire_lease" => {
                if operation.ends_with("stream") {
                    *self.streams.write().await += 1;
                } else {
                    *self.leases.write().await += 1;
                }
            }
            "sensors.close_stream" => *self.streams.write().await = 0,
            "sensors.release_lease" => *self.leases.write().await = 0,
            _ => {}
        }
    }
    async fn clear(&self) {
        *self.streams.write().await = 0;
        *self.leases.write().await = 0;
    }
    async fn counts(&self) -> (usize, usize) {
        (*self.streams.read().await, *self.leases.read().await)
    }
}

/// Mock/Null Object provider selected by runtime-host composition.
pub struct DeviceSensorsSystemServiceProvider {
    unavailable_reason: Option<String>,
    events: broadcast::Sender<SensorRuntimeEvent>,
    ledger: Arc<SensorLedger>,
}

impl DeviceSensorsSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(None)
    }
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            unavailable_reason,
            events,
            ledger: Arc::new(SensorLedger::default()),
        }
    }
    pub fn subscribe(&self) -> broadcast::Receiver<SensorRuntimeEvent> {
        self.events.subscribe()
    }
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let (streams, leases) = self.ledger.counts().await;
        let snapshot = BTreeMap::from([
            (
                "provider_class".into(),
                if self.unavailable_reason.is_some() {
                    "unavailable"
                } else {
                    "mock"
                }
                .into(),
            ),
            ("active_stream_count".into(), streams.to_string()),
            ("active_lease_count".into(), leases.to_string()),
            (
                "command_count".into(),
                DEVICE_SENSORS_COMMANDS.len().to_string(),
            ),
            ("snapshot_schema".into(), "device.sensors.replay.v1".into()),
        ]);
        self.emit(
            "sensors.snapshot_recorded",
            "snapshot:provider",
            "snapshot_recorded",
        );
        snapshot
    }
    fn emit(&self, command: &str, trace_id: &str, outcome: &'static str) {
        let _ = self.events.send(SensorRuntimeEvent {
            command: command.into(),
            event_name: command.into(),
            trace_id: trace_id.into(),
            replay_ref: format!("replay:sensors:{trace_id}"),
            outcome,
        });
    }
}

#[async_trait]
impl SystemService for DeviceSensorsSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(DEVICE_SENSORS_SERVICE_ID),
            ServiceType::new("device.sensors"),
            TraceSchemaRef::new("device.sensors.replay.v1"),
        );
        descriptor
            .metadata
            .insert("pack_id".into(), DEVICE_SENSORS_PACK_ID.into());
        descriptor.metadata.insert(
            "provider_class".into(),
            if self.unavailable_reason.is_some() {
                "unavailable"
            } else {
                "mock"
            }
            .into(),
        );
        descriptor
    }
    async fn start(&self) -> ServiceResult<()> {
        self.emit(
            "sensors.pack_declared",
            "lifecycle:sensors",
            "pack_declared",
        );
        info!(
            service_id = DEVICE_SENSORS_SERVICE_ID,
            "device sensors provider started"
        );
        Ok(())
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        let operation = command.name.as_str();
        if !DEVICE_SENSORS_COMMANDS.contains(&operation) {
            self.emit("sensors.command_failed", &trace.trace_id, "unsupported");
            return Err(ServiceError::UnsupportedCommand(operation.into()));
        }
        if let Some(reason) = &self.unavailable_reason {
            self.emit("sensors.unavailable", &trace.trace_id, "unavailable");
            warn!(service_id = DEVICE_SENSORS_SERVICE_ID, command = operation, reason_code = %reason, "device sensors provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if let Some(reason) = sensor_admission_denial(&command.payload) {
            self.emit("sensors.policy_decision", &trace.trace_id, "denied");
            return Err(ServiceError::DisabledByPolicy(reason.into()));
        }
        let (streams, leases) = self.ledger.counts().await;
        if (operation == "sensors.open_stream" && streams >= 32)
            || (operation == "sensors.acquire_lease" && leases >= 32)
        {
            self.emit("sensors.policy_decision", &trace.trace_id, "quota_exceeded");
            return Err(ServiceError::DisabledByPolicy("quota_exceeded".into()));
        }
        for event_name in [
            "sensors.admission_validated",
            "sensors.policy_decision",
            "sensors.entitlement_checked",
            "sensors.resource_reserved",
            "sensors.command_requested",
            "sensors.provider_selected",
        ] {
            self.emit(event_name, &trace.trace_id, "validated");
        }
        self.ledger.record(operation).await;
        self.emit(
            sensor_success_event(operation),
            &trace.trace_id,
            "completed",
        );
        if operation == "sensors.open_stream" {
            self.emit("sensors.stream_opened", &trace.trace_id, "opened");
        }
        if operation == "sensors.close_stream" {
            self.emit("sensors.stream_closed", &trace.trace_id, "closed");
        }
        if operation == "sensors.release_lease" {
            self.emit("sensors.lease_revoked", &trace.trace_id, "revoked");
        }
        self.emit("sensors.command_succeeded", &trace.trace_id, "succeeded");
        info!(service_id = DEVICE_SENSORS_SERVICE_ID, command = operation, trace_id = %trace.trace_id, "device sensor command completed with opaque reference");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"reference_only","operation":operation,"result_ref":format!("sensor-reference:{}", trace.trace_id)}),
            trace,
            "mock",
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        self.ledger.clear().await;
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.ledger.clear().await;
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

fn sensor_admission_denial(payload: &serde_json::Value) -> Option<&'static str> {
    let blocked = |key: &str, reason: &'static str| {
        (payload.get(key).and_then(serde_json::Value::as_bool) == Some(true)).then_some(reason)
    };
    blocked("permission_denied", "permission_denied")
        .or_else(|| blocked("disabled", "disabled"))
        .or_else(|| blocked("foreground_required", "foreground_required"))
        .or_else(|| blocked("sensitive_sensor", "sensitive_sensor_approval_required"))
        .or_else(|| blocked("background_denied", "background_denied"))
        .or_else(|| blocked("lease_revoked", "lease_revoked"))
        .or_else(|| blocked("quota_exceeded", "quota_exceeded"))
        .or_else(|| blocked("cancelled", "cancelled"))
        .or_else(|| {
            (payload
                .get("frequency_hz")
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|frequency| frequency > 100))
            .then_some("frequency_limit_exceeded")
        })
        .or_else(|| {
            (payload
                .get("sample_count")
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|count| count > 1024))
            .then_some("sample_quota_exceeded")
        })
}

fn sensor_success_event(operation: &str) -> &'static str {
    match operation {
        "sensors.list" => "sensors.listed",
        "sensors.inspect" => "sensors.inspected",
        "sensors.read" => "sensors.sample_read",
        "sensors.open_stream" => "sensors.stream_opened",
        "sensors.read_stream" => "sensors.stream_chunk_delivered",
        "sensors.close_stream" => "sensors.stream_closed",
        "sensors.read_batch" => "sensors.batch_read",
        "sensors.inspect_calibration" => "sensors.calibration_inspected",
        "sensors.acquire_lease" => "sensors.lease_acquired",
        "sensors.release_lease" => "sensors.lease_released",
        "sensors.inspect_host" => "sensors.host_inspected",
        _ => "sensors.command_succeeded",
    }
}
