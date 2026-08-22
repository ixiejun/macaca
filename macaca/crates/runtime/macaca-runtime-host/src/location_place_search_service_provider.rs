//! Runtime service adapter for provider-neutral place-search commands.
//!
//! The deterministic mock stores only opaque result/session references. Query
//! text, coordinates, media references, session tokens, and provider payloads
//! are never echoed into service results, logs, snapshots, or replay events.

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::location_place_search::{
    PlaceSearchProviderCapability, LOCATION_PLACE_SEARCH_COMMANDS, LOCATION_PLACE_SEARCH_PACK_ID,
    LOCATION_PLACE_SEARCH_SERVICE_ID, LOCATION_PLACE_SEARCH_TRACE_EVENTS,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::location_place_search_strategy::{
    ConfiguredPlaceSearchStrategy, PlaceSearchProviderStrategy,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceSearchRuntimeEventKind {
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
    AttributionRecorded,
    SessionPurged,
    SnapshotRecorded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceSearchRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: PlaceSearchRuntimeEventKind,
    pub replay_ref: String,
}

pub struct LocationPlaceSearchSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<PlaceSearchRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn PlaceSearchProviderStrategy>,
}

impl LocationPlaceSearchSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(None)
    }

    pub fn mock_with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut provider = Self::new(None);
        provider.strategy = Arc::new(ConfiguredPlaceSearchStrategy::with_commands(commands));
        provider
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        let strategy: Arc<dyn PlaceSearchProviderStrategy> =
            Arc::new(if unavailable_reason.is_some() {
                ConfiguredPlaceSearchStrategy::unavailable()
            } else {
                ConfiguredPlaceSearchStrategy::mock()
            });
        Self {
            descriptor: location_place_search_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
            strategy,
        }
    }

    pub fn capability(&self) -> PlaceSearchProviderCapability {
        self.strategy.capability()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PlaceSearchRuntimeEvent> {
        self.events.subscribe()
    }

    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(100);
        let _ = self.events.send(event(
            "place_search.snapshot",
            "snapshot:place-search",
            PlaceSearchRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            ("pack_id".into(), LOCATION_PLACE_SEARCH_PACK_ID.into()),
            ("provider_class".into(), self.capability().provider_class),
            ("active_reference_count".into(), count.to_string()),
            (
                "redaction_profile".into(),
                "hashes_and_references_only".into(),
            ),
        ])
    }

    pub async fn shutdown(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(
            service_id = LOCATION_PLACE_SEARCH_SERVICE_ID,
            "place search provider shutdown completed"
        );
        Ok(())
    }
}

#[async_trait]
impl SystemService for LocationPlaceSearchSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = self.descriptor.clone();
        descriptor
            .metadata
            .insert("provider_class".into(), self.capability().provider_class);
        descriptor
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "place_search.declaration",
            "declaration:place-search",
            PlaceSearchRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "place search provider started");
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
                PlaceSearchRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "place search provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !LOCATION_PLACE_SEARCH_COMMANDS.contains(&command.name.as_str()) {
            return Err(normalize_place_error(ServiceError::UnsupportedCommand(
                command.name.to_string(),
            )));
        }
        if let Err(error) = self.strategy.validate_command(command.name.as_str()) {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                PlaceSearchRuntimeEventKind::CommandFailed,
            ));
            return Err(normalize_place_error(error));
        }
        if let Some(reason) = place_admission_denial(&command.payload) {
            let _ = self.events.send(event(
                "place_search.policy_decision",
                &trace.trace_id,
                PlaceSearchRuntimeEventKind::PolicyDecision,
            ));
            return Err(normalize_place_error(ServiceError::DisabledByPolicy(
                reason.into(),
            )));
        }
        let reference = format!("place-search:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), reference.clone());
        let kind = if command.name.as_str() == "place_search.purge_session" {
            PlaceSearchRuntimeEventKind::SessionPurged
        } else if command.name.as_str() == "place_search.inspect_attribution" {
            PlaceSearchRuntimeEventKind::AttributionRecorded
        } else {
            PlaceSearchRuntimeEventKind::CommandSucceeded
        };
        for event_kind in [
            PlaceSearchRuntimeEventKind::AdmissionValidated,
            PlaceSearchRuntimeEventKind::PolicyDecision,
            PlaceSearchRuntimeEventKind::EntitlementChecked,
            PlaceSearchRuntimeEventKind::ResourceReserved,
            PlaceSearchRuntimeEventKind::CommandRequested,
            PlaceSearchRuntimeEventKind::ProviderSelected,
            kind,
        ] {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                event_kind,
            ));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "place search provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({
                "status": "ok",
                "place_reference": reference,
                "provider_class": self.capability().provider_class,
                "freshness": "current",
                "attribution_ref": format!("attribution:{}", trace.trace_id),
                "redaction_profile": "hashes_and_references_only",
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

pub fn location_place_search_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(LOCATION_PLACE_SEARCH_SERVICE_ID),
        ServiceType::new("location.place_search"),
        TraceSchemaRef::new("location.place_search.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), LOCATION_PLACE_SEARCH_PACK_ID.into());
    descriptor.metadata.insert(
        "command_count".into(),
        LOCATION_PLACE_SEARCH_COMMANDS.len().to_string(),
    );
    descriptor
        .metadata
        .insert("field_mask_required".into(), "details".into());
    descriptor
        .metadata
        .insert("session_retention".into(), "ephemeral".into());
    descriptor.metadata.insert(
        "trace_event_count".into(),
        LOCATION_PLACE_SEARCH_TRACE_EVENTS.len().to_string(),
    );
    descriptor
}

fn place_admission_denial(payload: &serde_json::Value) -> Option<&'static str> {
    let blocked = |key: &str, reason: &'static str| {
        (payload.get(key).and_then(serde_json::Value::as_bool) == Some(true)).then_some(reason)
    };
    blocked("policy_denied", "policy_denied")
        .or_else(|| blocked("entitlement_missing", "entitlement_missing"))
        .or_else(|| blocked("approval_required", "approval_required"))
        .or_else(|| blocked("precise_location_denied", "precise_location_denied"))
        .or_else(|| blocked("external_network_denied", "external_network_denied"))
        .or_else(|| blocked("field_unsupported", "field_unsupported"))
        .or_else(|| blocked("field_mask_missing", "field_mask_missing"))
        .or_else(|| blocked("region_denied", "region_denied"))
        .or_else(|| blocked("quota_exceeded", "quota_exceeded"))
        .or_else(|| blocked("stale_reference", "stale_reference"))
}

fn normalize_place_error(error: ServiceError) -> ServiceError {
    match error {
        ServiceError::UnsupportedCommand(_) => {
            ServiceError::UnsupportedCommand("place_search_command_unsupported".into())
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
    kind: PlaceSearchRuntimeEventKind,
) -> PlaceSearchRuntimeEvent {
    PlaceSearchRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:place-search:{trace_id}"),
    }
}
