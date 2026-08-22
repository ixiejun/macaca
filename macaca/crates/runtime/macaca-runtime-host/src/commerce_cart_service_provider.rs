//! Runtime-host adapter for the provider-neutral commerce cart pack.
//!
//! This module implements the Adapter/Strategy boundary required by the cart pack.  The mock
//! provider records only opaque cart references and bounded lifecycle events; it deliberately does
//! not parse storefront payloads, buyer PII, payment data, checkout URLs, or vendor mutation DSLs.
//! A concrete cart implementation can replace this Strategy through the normal service registry
//! without changing SDK, kernel, shell, or application code.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::commerce_cart_strategy::{CommerceCartProviderStrategy, ConfiguredCommerceCartStrategy};
use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::commerce_cart::{
    CartProviderCapability, COMMERCE_CART_COMMANDS, COMMERCE_CART_PACK_ID,
    COMMERCE_CART_SERVICE_ID, COMMERCE_CART_TRACE_EVENTS,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    KernelServiceId, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Sanitized cart lifecycle event used by replay and audit observers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommerceCartRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: CommerceCartRuntimeEventKind,
    pub replay_ref: String,
}

/// Bounded event taxonomy; no event variant carries provider payloads or buyer data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommerceCartRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    EntitlementChecked,
    ApprovalChecked,
    ResourceReserved,
    ProviderInspected,
    ServiceCall,
    MutationPlanned,
    HandoffPlanned,
    ExportPlanned,
    ProviderCallStarted,
    ProviderCallSucceeded,
    ProviderCallFailed,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Deterministic mock or fail-closed Null Object cart provider.
pub struct CommerceCartSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<CommerceCartRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn CommerceCartProviderStrategy>,
}

impl CommerceCartSystemServiceProvider {
    /// Create a metadata-only mock Strategy for conformance and replay tests.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Build a synthetic cart provider with explicit command capability gaps.
    pub fn mock_with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut provider = Self::new(None);
        provider.strategy = Arc::new(ConfiguredCommerceCartStrategy::with_commands(commands));
        provider
    }

    /// Create a fail-closed provider when no cart adapter is installed.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        let strategy: Arc<dyn CommerceCartProviderStrategy> =
            Arc::new(if unavailable_reason.is_some() {
                ConfiguredCommerceCartStrategy::unavailable()
            } else {
                ConfiguredCommerceCartStrategy::mock()
            });
        Self {
            descriptor: commerce_cart_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
            strategy,
        }
    }

    /// Report feature flags and bounded limits without exposing provider implementation details.
    pub fn capability(&self) -> CartProviderCapability {
        self.strategy.capability()
    }

    /// Subscribe to bounded cart lifecycle events for replay and audit tests.
    pub fn subscribe(&self) -> broadcast::Receiver<CommerceCartRuntimeEvent> {
        self.events.subscribe()
    }

    /// Return a bounded Memento containing only descriptor and reference counts.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(100);
        let _ = self.events.send(event(
            "cart.snapshot",
            "snapshot:commerce-cart-provider",
            CommerceCartRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            ("pack_id".into(), COMMERCE_CART_PACK_ID.into()),
            ("descriptor_hash".into(), "commerce-cart:descriptor".into()),
            ("provider_class".into(), self.capability().provider_class),
            ("active_reference_count".into(), count.to_string()),
            (
                "redaction_profile".into(),
                "references_and_hashes_only".into(),
            ),
        ])
    }

    /// Explicit lifecycle alias used by composition roots that expose service shutdown directly.
    pub async fn shutdown(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(
            service_id = COMMERCE_CART_SERVICE_ID,
            "commerce cart provider shutdown completed"
        );
        Ok(())
    }
}

#[async_trait]
impl SystemService for CommerceCartSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = self.descriptor.clone();
        descriptor
            .metadata
            .insert("provider_class".into(), self.capability().provider_class);
        descriptor
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "cart.declaration",
            "declaration:commerce-cart-provider",
            CommerceCartRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "commerce cart provider started");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                CommerceCartRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, reason_code = %reason, "commerce cart provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !COMMERCE_CART_COMMANDS.contains(&command.name.as_str()) {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                CommerceCartRuntimeEventKind::ProviderCallFailed,
            ));
            return Err(normalize_cart_error(ServiceError::UnsupportedCommand(
                command.name.to_string(),
            )));
        }
        self.strategy.validate_command(command.name.as_str())?;
        if let Some(reason) = cart_admission_denial(&command.payload) {
            let _ = self.events.send(event(
                "cart.policy_decision",
                &trace.trace_id,
                CommerceCartRuntimeEventKind::PolicyDecision,
            ));
            return Err(normalize_cart_error(ServiceError::DisabledByPolicy(
                reason.into(),
            )));
        }
        if self.references.read().await.len() >= 100 {
            return Err(normalize_cart_error(ServiceError::DisabledByPolicy(
                "quota_exceeded".into(),
            )));
        }

        let cart_ref = format!("cart:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), cart_ref.clone());
        for kind in common_event_kinds()
            .iter()
            .chain([event_kind(command.name.as_str())].iter())
        {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, *kind));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "commerce cart provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({
                "status": "ok",
                "cart_ref": cart_ref,
                "version_token_hash": format!("version:{}", trace.trace_id),
                "freshness": "current",
                "provider_class": "mock",
                "no_order_or_payment_execution": true,
            }),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        self.shutdown().await?;
        info!(service_id = %self.descriptor.id, "commerce cart provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.shutdown().await
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        let health = match &self.unavailable_reason {
            Some(reason) => ServiceHealth::Unavailable {
                reason: reason.clone(),
            },
            None => ServiceHealth::Healthy,
        };
        let _ = self.events.send(event(
            "cart.health",
            "health:commerce-cart-provider",
            CommerceCartRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Evaluate provider-neutral cart policy facts before retaining cart references.
fn cart_admission_denial(payload: &serde_json::Value) -> Option<&'static str> {
    let blocked = |key: &str, reason: &'static str| {
        (payload.get(key).and_then(serde_json::Value::as_bool) == Some(true)).then_some(reason)
    };
    blocked("policy_denied", "policy_denied")
        .or_else(|| blocked("entitlement_missing", "entitlement_missing"))
        .or_else(|| blocked("approval_required", "approval_required"))
        .or_else(|| blocked("line_mutation_denied", "line_mutation_denied"))
        .or_else(|| blocked("discount_unsupported", "discount_unsupported"))
        .or_else(|| blocked("estimate_unsupported", "estimate_unsupported"))
        .or_else(|| blocked("handoff_denied", "handoff_denied"))
        .or_else(|| blocked("export_denied", "export_denied"))
        .or_else(|| blocked("stale_data", "stale_data"))
        .or_else(|| blocked("quota_exceeded", "quota_exceeded"))
        .or_else(|| blocked("timeout", "timeout"))
        .or_else(|| blocked("cancelled", "cancelled"))
        .or_else(|| {
            (payload
                .get("line_count")
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|count| count > 100))
            .then_some("line_quota_exceeded")
        })
}

/// Normalize provider errors to bounded cart-owned result classes.
fn normalize_cart_error(error: ServiceError) -> ServiceError {
    match error {
        ServiceError::UnsupportedCommand(_) => {
            ServiceError::UnsupportedCommand("cart_command_unsupported".into())
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

/// Build a descriptor from protocol-owned cart constants only.
pub fn commerce_cart_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(COMMERCE_CART_SERVICE_ID),
        ServiceType::new("commerce.cart"),
        TraceSchemaRef::new("commerce.cart.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), COMMERCE_CART_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        COMMERCE_CART_COMMANDS.len().to_string(),
    );
    descriptor.metadata.insert(
        "trace_event_count".into(),
        COMMERCE_CART_TRACE_EVENTS.len().to_string(),
    );
    descriptor
}

fn common_event_kinds() -> &'static [CommerceCartRuntimeEventKind] {
    use CommerceCartRuntimeEventKind::*;
    &[
        AdmissionValidated,
        PolicyDecision,
        EntitlementChecked,
        ApprovalChecked,
        ResourceReserved,
        ServiceCall,
        ProviderCallStarted,
        ProviderCallSucceeded,
    ]
}

fn event_kind(command: &str) -> CommerceCartRuntimeEventKind {
    use CommerceCartRuntimeEventKind::*;
    match command {
        "cart.inspect_provider" => ProviderInspected,
        "cart.plan_context_update"
        | "cart.plan_line_mutation"
        | "cart.plan_discount"
        | "cart.plan_export" => MutationPlanned,
        "cart.plan_handoff" | "cart.handoff_request" => HandoffPlanned,
        "cart.export_cart" | "cart.get_artifact_handle" => ExportPlanned,
        _ => ServiceCall,
    }
}

fn event(
    command: &str,
    trace_id: &str,
    kind: CommerceCartRuntimeEventKind,
) -> CommerceCartRuntimeEvent {
    CommerceCartRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
