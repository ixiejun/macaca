//! Runtime service adapter for provider-neutral timezone commands.
//!
//! The mock retains only opaque references and dataset version evidence. Exact
//! coordinates, host paths, boundary geometry, and provider payloads never
//! cross the trace, snapshot, or service-result boundary.

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::location_timezone::{
    TimezoneProviderCapability, LOCATION_TIMEZONE_COMMANDS, LOCATION_TIMEZONE_PACK_ID,
    LOCATION_TIMEZONE_SERVICE_ID, LOCATION_TIMEZONE_TRACE_EVENTS,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::location_timezone_strategy::{
    ConfiguredLocationTimezoneStrategy, LocationTimezoneProviderStrategy,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationTimezoneRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    EntitlementChecked,
    ResourceReserved,
    CommandRequested,
    ProviderSelected,
    CommandSucceeded,
    CommandFailed,
    Unavailable,
    DatabaseStale,
    SnapshotRecorded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationTimezoneRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: LocationTimezoneRuntimeEventKind,
    pub replay_ref: String,
    pub dataset_version: String,
}

pub struct LocationTimezoneSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<LocationTimezoneRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn LocationTimezoneProviderStrategy>,
}

impl LocationTimezoneSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(None)
    }

    pub fn mock_with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut provider = Self::new(None);
        provider.strategy = Arc::new(ConfiguredLocationTimezoneStrategy::with_commands(commands));
        provider
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        let strategy: Arc<dyn LocationTimezoneProviderStrategy> =
            Arc::new(if unavailable_reason.is_some() {
                ConfiguredLocationTimezoneStrategy::unavailable()
            } else {
                ConfiguredLocationTimezoneStrategy::mock()
            });
        Self {
            descriptor: location_timezone_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
            strategy,
        }
    }

    pub fn capability(&self) -> TimezoneProviderCapability {
        self.strategy.capability()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LocationTimezoneRuntimeEvent> {
        self.events.subscribe()
    }

    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(100);
        let _ = self.events.send(event(
            "timezone.snapshot",
            "snapshot:timezone",
            LocationTimezoneRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            ("pack_id".into(), LOCATION_TIMEZONE_PACK_ID.into()),
            ("provider_class".into(), self.capability().provider_class),
            ("active_reference_count".into(), count.to_string()),
            ("dataset_version".into(), "synthetic-2026a".into()),
            (
                "redaction_profile".into(),
                "references_and_versions_only".into(),
            ),
        ])
    }

    pub async fn shutdown(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(
            service_id = LOCATION_TIMEZONE_SERVICE_ID,
            "timezone provider shutdown completed"
        );
        Ok(())
    }
}

#[async_trait]
impl SystemService for LocationTimezoneSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = self.descriptor.clone();
        descriptor
            .metadata
            .insert("provider_class".into(), self.capability().provider_class);
        descriptor
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "timezone.declaration",
            "declaration:timezone",
            LocationTimezoneRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "timezone provider started");
        Ok(())
    }

    async fn call(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                LocationTimezoneRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "timezone provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !LOCATION_TIMEZONE_COMMANDS.contains(&command.name.as_str()) {
            return Err(normalize_timezone_error(ServiceError::UnsupportedCommand(
                command.name.to_string(),
            )));
        }
        if let Err(error) = self.strategy.validate_command(command.name.as_str()) {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                LocationTimezoneRuntimeEventKind::CommandFailed,
            ));
            return Err(normalize_timezone_error(error));
        }
        if let Some(reason) = timezone_admission_denial(&command.payload) {
            let kind = if reason == "stale_database" {
                LocationTimezoneRuntimeEventKind::DatabaseStale
            } else {
                LocationTimezoneRuntimeEventKind::PolicyDecision
            };
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, kind));
            return Err(normalize_timezone_error(ServiceError::DisabledByPolicy(
                reason.into(),
            )));
        }
        let reference = format!("timezone:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), reference.clone());
        for kind in [
            LocationTimezoneRuntimeEventKind::AdmissionValidated,
            LocationTimezoneRuntimeEventKind::PolicyDecision,
            LocationTimezoneRuntimeEventKind::EntitlementChecked,
            LocationTimezoneRuntimeEventKind::ResourceReserved,
            LocationTimezoneRuntimeEventKind::CommandRequested,
            LocationTimezoneRuntimeEventKind::ProviderSelected,
            LocationTimezoneRuntimeEventKind::CommandSucceeded,
        ] {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, kind));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "timezone provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({
                "status": "ok",
                "timezone_reference": reference,
                "provider_class": self.capability().provider_class,
                "dataset_version": "synthetic-2026a",
                "redaction_profile": "references_and_versions_only",
            }),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        self.shutdown().await
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.shutdown().await
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self
            .unavailable_reason
            .as_ref()
            .map(|reason| ServiceHealth::Unavailable {
                reason: reason.clone(),
            })
            .unwrap_or(ServiceHealth::Healthy))
    }
}

pub fn location_timezone_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(LOCATION_TIMEZONE_SERVICE_ID),
        ServiceType::new("location.timezone"),
        TraceSchemaRef::new("location.timezone.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), LOCATION_TIMEZONE_PACK_ID.into());
    descriptor.metadata.insert(
        "command_count".into(),
        LOCATION_TIMEZONE_COMMANDS.len().to_string(),
    );
    descriptor
        .metadata
        .insert("dataset_version".into(), "reported_and_redacted".into());
    descriptor.metadata.insert(
        "trace_event_count".into(),
        LOCATION_TIMEZONE_TRACE_EVENTS.len().to_string(),
    );
    descriptor
}

fn timezone_admission_denial(payload: &serde_json::Value) -> Option<&'static str> {
    let blocked = |key: &str, reason: &'static str| {
        (payload.get(key).and_then(serde_json::Value::as_bool) == Some(true)).then_some(reason)
    };
    blocked("policy_denied", "policy_denied")
        .or_else(|| blocked("precise_coordinate_denied", "precise_coordinate_denied"))
        .or_else(|| blocked("external_network_denied", "external_network_denied"))
        .or_else(|| blocked("stale_database", "stale_database"))
        .or_else(|| blocked("quota_exceeded", "quota_exceeded"))
        .or_else(|| blocked("invalid_coordinate", "invalid_coordinate"))
        .or_else(|| blocked("resolver_strategy_missing", "resolver_strategy_missing"))
        .or_else(|| blocked("timeout", "timeout"))
        .or_else(|| blocked("cancelled", "cancelled"))
}

fn normalize_timezone_error(error: ServiceError) -> ServiceError {
    match error {
        ServiceError::UnsupportedCommand(_) => {
            ServiceError::UnsupportedCommand("timezone_command_unsupported".into())
        }
        ServiceError::DisabledByPolicy(reason) => ServiceError::DisabledByPolicy(
            reason
                .chars()
                .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
                .take(64)
                .collect(),
        ),
        other => other,
    }
}

fn event(
    command: &str,
    trace_id: &str,
    kind: LocationTimezoneRuntimeEventKind,
) -> LocationTimezoneRuntimeEvent {
    LocationTimezoneRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:timezone:{trace_id}"),
        dataset_version: "synthetic-2026a".into(),
    }
}
