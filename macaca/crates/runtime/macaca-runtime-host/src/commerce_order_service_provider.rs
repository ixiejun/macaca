//! Runtime-host adapter for provider-neutral order lifecycle operations.
//!
//! The service stores only opaque order and replay references. Payment, receipt,
//! invoice, inventory, and carrier side effects remain outside this boundary.
use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::commerce_order::{
    OrderProviderCapability, COMMERCE_ORDER_COMMANDS, COMMERCE_ORDER_PACK_ID,
    COMMERCE_ORDER_SERVICE_ID, COMMERCE_ORDER_TRACE_EVENTS,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef,
};
use tokio::sync::{broadcast, RwLock};
use tracing::warn;

use crate::commerce_order_strategy::{
    CommerceOrderProviderStrategy, ConfiguredCommerceOrderStrategy,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommerceOrderRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    EntitlementChecked,
    ApprovalChecked,
    ResourceReserved,
    ProviderInspected,
    ServiceCall,
    LifecyclePlanned,
    FulfillmentIntentPlanned,
    Unavailable,
    ProviderCallSucceeded,
    ProviderCallFailed,
    HealthReported,
    SnapshotRecorded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommerceOrderRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: CommerceOrderRuntimeEventKind,
    pub replay_ref: String,
}

pub struct CommerceOrderSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<CommerceOrderRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn CommerceOrderProviderStrategy>,
}

impl CommerceOrderSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(None)
    }

    pub fn mock_with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut provider = Self::new(None);
        provider.strategy = Arc::new(ConfiguredCommerceOrderStrategy::with_commands(commands));
        provider
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        let strategy: Arc<dyn CommerceOrderProviderStrategy> =
            Arc::new(if unavailable_reason.is_some() {
                ConfiguredCommerceOrderStrategy::unavailable()
            } else {
                ConfiguredCommerceOrderStrategy::mock()
            });
        Self {
            descriptor: commerce_order_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
            strategy,
        }
    }

    pub fn capability(&self) -> OrderProviderCapability {
        self.strategy.capability()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CommerceOrderRuntimeEvent> {
        self.events.subscribe()
    }

    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(256);
        let _ = self.events.send(event(
            "order.snapshot",
            "snapshot:commerce-order",
            CommerceOrderRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            ("pack_id".into(), COMMERCE_ORDER_PACK_ID.into()),
            ("provider_class".into(), self.capability().provider_class),
            ("active_reference_count".into(), count.to_string()),
            ("freshness".into(), "current_or_explicit_stale".into()),
            (
                "redaction_profile".into(),
                "references_hashes_and_state_metadata_only".into(),
            ),
        ])
    }

    pub async fn shutdown(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        Ok(())
    }
}

#[async_trait]
impl SystemService for CommerceOrderSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = self.descriptor.clone();
        descriptor
            .metadata
            .insert("provider_class".into(), self.capability().provider_class);
        descriptor
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "order.declaration",
            "declaration:commerce-order",
            CommerceOrderRuntimeEventKind::PackDeclared,
        ));
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                CommerceOrderRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = COMMERCE_ORDER_SERVICE_ID, command = %command.name, "commerce order provider unavailable");
            return Err(ServiceError::ServiceUnavailable(sanitize(reason)));
        }
        if !COMMERCE_ORDER_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(
                "order_command_unsupported".into(),
            ));
        }
        self.strategy.validate_command(command.name.as_str())?;
        if let Some(reason) = admission_denial(&command.payload) {
            let _ = self.events.send(event(
                "order.policy_decision",
                &trace.trace_id,
                CommerceOrderRuntimeEventKind::PolicyDecision,
            ));
            return Err(ServiceError::DisabledByPolicy(reason.into()));
        }
        if self.references.read().await.len() >= 256 {
            return Err(ServiceError::DisabledByPolicy("quota_exceeded".into()));
        }
        let order_ref = format!("order:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), order_ref.clone());
        for kind in [
            CommerceOrderRuntimeEventKind::AdmissionValidated,
            CommerceOrderRuntimeEventKind::EntitlementChecked,
            CommerceOrderRuntimeEventKind::ResourceReserved,
            command_event(command.name.as_str()),
            CommerceOrderRuntimeEventKind::ProviderCallSucceeded,
        ] {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, kind));
        }
        Ok(domain_pack_service_result(
            serde_json::json!({
                "status": "ok",
                "order_ref": order_ref,
                "provider_class": "mock",
                "lifecycle_state": "planned",
                "freshness": "current",
                "version_token_hash": format!("version:{}", trace.trace_id),
                "payment_receipt_inventory_execution": false,
                "redaction": "references_only"
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
        let health =
            self.unavailable_reason
                .as_ref()
                .map_or(Ok(ServiceHealth::Healthy), |reason| {
                    Ok(ServiceHealth::Unavailable {
                        reason: sanitize(reason),
                    })
                });
        let _ = self.events.send(event(
            "order.health",
            "health:commerce-order",
            CommerceOrderRuntimeEventKind::HealthReported,
        ));
        health
    }
}

fn admission_denial(payload: &serde_json::Value) -> Option<&'static str> {
    [
        "policy_denied",
        "entitlement_missing",
        "approval_required",
        "conflict",
        "quota_exceeded",
        "stale_data",
        "timeout",
        "cancelled",
        "fulfillment_unsupported",
        "cancel_unsupported",
        "audit_export_denied",
    ]
    .into_iter()
    .find(|key| payload.get(*key).and_then(serde_json::Value::as_bool) == Some(true))
}

fn command_event(command: &str) -> CommerceOrderRuntimeEventKind {
    match command {
        "order.inspect_provider" | "order.describe_schema" => {
            CommerceOrderRuntimeEventKind::ProviderInspected
        }
        command if command.contains("fulfillment_intent") => {
            CommerceOrderRuntimeEventKind::FulfillmentIntentPlanned
        }
        command if command.contains("transition") || command.contains("cancellation") => {
            CommerceOrderRuntimeEventKind::LifecyclePlanned
        }
        _ => CommerceOrderRuntimeEventKind::ServiceCall,
    }
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .take(64)
        .collect()
}

fn event(
    command: &str,
    trace_id: &str,
    kind: CommerceOrderRuntimeEventKind,
) -> CommerceOrderRuntimeEvent {
    CommerceOrderRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}

pub fn commerce_order_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(COMMERCE_ORDER_SERVICE_ID),
        ServiceType::new("commerce.order"),
        TraceSchemaRef::new("commerce.order.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), COMMERCE_ORDER_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        COMMERCE_ORDER_COMMANDS.len().to_string(),
    );
    descriptor.metadata.insert(
        "trace_event_count".into(),
        COMMERCE_ORDER_TRACE_EVENTS.len().to_string(),
    );
    descriptor
}
