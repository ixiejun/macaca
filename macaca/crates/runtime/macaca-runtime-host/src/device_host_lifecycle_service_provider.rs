//! Runtime-host Strategy for provider-neutral foreground/background lifecycle calls.
//!
//! The mock supplies synthetic lifecycle references only. It never controls a
//! host process, stores host identifiers, or emits raw presentation/log data.

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::device_foreground_background_host::{
    DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS, DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID,
    DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID,
};
use macaca_proto::{
    admit_host_lifecycle_operation, domain_pack_command_trace, domain_pack_service_result,
    HostLifecyclePreflightFacts, HostLifecyclePreflightFailure, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef,
};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Sanitized event used for lifecycle audit/replay evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostLifecycleRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub replay_ref: String,
    pub outcome: &'static str,
}

/// Aggregate, Memento-friendly state for synthetic sessions and leases.
#[derive(Default)]
struct LifecycleLedger {
    foreground_sessions: RwLock<usize>,
    background_leases: RwLock<usize>,
}

impl LifecycleLedger {
    async fn record(&self, operation: &str) {
        match operation {
            "host_lifecycle.open_foreground_session" => {
                *self.foreground_sessions.write().await += 1
            }
            "host_lifecycle.close_foreground_session" | "host_lifecycle.revoke" => {
                *self.foreground_sessions.write().await = 0
            }
            "host_lifecycle.request_background_lease" => *self.background_leases.write().await += 1,
            "host_lifecycle.release_background_lease" | "host_lifecycle.revoke" => {
                *self.background_leases.write().await = 0
            }
            _ => {}
        }
    }
    async fn clear(&self) {
        *self.foreground_sessions.write().await = 0;
        *self.background_leases.write().await = 0;
    }
    async fn counts(&self) -> (usize, usize) {
        (
            *self.foreground_sessions.read().await,
            *self.background_leases.read().await,
        )
    }
}

/// Mock/Null Object provider selected only by runtime-host composition.
pub struct DeviceHostLifecycleSystemServiceProvider {
    unavailable_reason: Option<String>,
    admission_facts: HostLifecyclePreflightFacts,
    events: tokio::sync::broadcast::Sender<HostLifecycleRuntimeEvent>,
    ledger: Arc<LifecycleLedger>,
}

impl DeviceHostLifecycleSystemServiceProvider {
    /// Build deterministic reference-only behavior for contract/ABI tests.
    pub fn mock() -> Self {
        Self::new(None)
    }
    /// Build a fail-closed Null Object for an absent lifecycle module.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = tokio::sync::broadcast::channel(256);
        Self {
            unavailable_reason,
            admission_facts: HostLifecyclePreflightFacts::permissive(),
            events,
            ledger: Arc::new(LifecycleLedger::default()),
        }
    }
    /// Supply typed host evidence at composition time, never from caller payloads.
    pub fn with_admission_facts(mut self, facts: HostLifecyclePreflightFacts) -> Self {
        self.admission_facts = facts;
        self
    }
    /// Subscribe to bounded audit/replay events.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<HostLifecycleRuntimeEvent> {
        self.events.subscribe()
    }
    /// Return aggregate recovery metadata without identifiers, logs, or provider payloads.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let (sessions, leases) = self.ledger.counts().await;
        let snapshot = BTreeMap::from([
            (
                "provider_class".into(),
                if self.unavailable_reason.is_some() {
                    "unavailable".into()
                } else {
                    "mock".into()
                },
            ),
            (
                "command_count".into(),
                DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS.len().to_string(),
            ),
            (
                "active_foreground_session_count".into(),
                sessions.to_string(),
            ),
            ("active_background_lease_count".into(), leases.to_string()),
            (
                "snapshot_schema".into(),
                "device.host_lifecycle.replay.v1".into(),
            ),
        ]);
        self.emit(
            "host_lifecycle.snapshot",
            "snapshot:provider",
            "snapshot_recorded",
        );
        info!(
            service_id = DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID,
            "device host lifecycle snapshot recorded"
        );
        snapshot
    }
    fn emit(&self, command: &str, trace_id: &str, outcome: &'static str) {
        let _ = self.events.send(HostLifecycleRuntimeEvent {
            command: command.into(),
            trace_id: trace_id.into(),
            replay_ref: format!("replay:host-lifecycle:{trace_id}"),
            outcome,
        });
    }
}

#[async_trait]
impl SystemService for DeviceHostLifecycleSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID),
            ServiceType::new("device.host_lifecycle"),
            TraceSchemaRef::new("device.host_lifecycle.replay.v1"),
        );
        descriptor.metadata.insert(
            "pack_id".into(),
            DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID.into(),
        );
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
            DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS.len().to_string(),
        );
        descriptor
    }
    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID,
            "device host lifecycle provider started"
        );
        Ok(())
    }
    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        let operation = command.name.as_str();
        if !DEVICE_FOREGROUND_BACKGROUND_HOST_COMMANDS.contains(&operation) {
            self.emit(
                "host_lifecycle.command_failed",
                &trace.trace_id,
                "unsupported",
            );
            return Err(ServiceError::UnsupportedCommand(operation.into()));
        }
        if let Some(reason) = &self.unavailable_reason {
            self.emit("host_lifecycle.unavailable", &trace.trace_id, "unavailable");
            warn!(service_id = DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID, command = operation, trace_id = %trace.trace_id, reason_code = %reason, "device host lifecycle provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if let Err(rejection) = admit_host_lifecycle_operation(self.admission_facts) {
            self.emit(
                "host_lifecycle.policy_decision",
                &trace.trace_id,
                "preflight_rejected",
            );
            warn!(service_id = DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID, command = operation, trace_id = %trace.trace_id, rejection = ?rejection, "device host lifecycle command rejected before adapter dispatch");
            return Err(preflight_error(rejection));
        }
        self.ledger.record(operation).await;
        self.emit(
            host_lifecycle_success_event(operation),
            &trace.trace_id,
            "completed",
        );
        info!(service_id = DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID, command = operation, trace_id = %trace.trace_id, "device host lifecycle command completed with synthetic reference");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"reference_only","operation":operation,"lifecycle_ref":format!("host-lifecycle-reference:{}", trace.trace_id)}),
            trace,
            "mock",
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        self.ledger.clear().await;
        info!(
            service_id = DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID,
            "device host lifecycle provider stopped and resources released"
        );
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

/// Map each lifecycle command to a stable audit event without exposing payloads.
fn host_lifecycle_success_event(operation: &str) -> &'static str {
    match operation {
        "host_lifecycle.inspect_state" => "host_lifecycle.state_inspected",
        "host_lifecycle.subscribe_events" => "host_lifecycle.events_subscribed",
        "host_lifecycle.open_foreground_session" => "host_lifecycle.foreground_session_opened",
        "host_lifecycle.close_foreground_session" => "host_lifecycle.foreground_session_closed",
        "host_lifecycle.request_background_lease" => "host_lifecycle.background_lease_requested",
        "host_lifecycle.release_background_lease" => "host_lifecycle.background_lease_released",
        "host_lifecycle.inspect_policy" => "host_lifecycle.policy_inspected",
        "host_lifecycle.revoke" => "host_lifecycle.session_or_lease_revoked",
        "host_lifecycle.inspect_host" => "host_lifecycle.host_inspected",
        _ => "host_lifecycle.command_completed",
    }
}

fn preflight_error(rejection: HostLifecyclePreflightFailure) -> ServiceError {
    match rejection {
        HostLifecyclePreflightFailure::Unavailable => {
            ServiceError::ServiceUnavailable("host_lifecycle_provider_unavailable".into())
        }
        HostLifecyclePreflightFailure::QuotaExceeded
        | HostLifecyclePreflightFailure::Timeout
        | HostLifecyclePreflightFailure::Cancellation => {
            ServiceError::AdapterFailure(format!("host_lifecycle_{rejection:?}").to_lowercase())
        }
        _ => ServiceError::DisabledByPolicy(format!("host_lifecycle_{rejection:?}").to_lowercase()),
    }
}
