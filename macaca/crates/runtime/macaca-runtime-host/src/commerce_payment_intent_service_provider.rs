//! Runtime-host service provider for tokenized, provider-neutral payment intents.
//!
//! This boundary accepts references and hashes only. It never stores or emits
//! credentials, client secrets, webhook bodies, wallet cryptograms, or gateway DSL.
use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::commerce_payment_intent::{
    PaymentIntentProviderCapability, COMMERCE_PAYMENT_INTENT_COMMANDS,
    COMMERCE_PAYMENT_INTENT_PACK_ID, COMMERCE_PAYMENT_INTENT_SERVICE_ID,
    COMMERCE_PAYMENT_INTENT_TRACE_EVENTS,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCallResult,
    ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType,
    TraceSchemaRef,
};
use tokio::sync::{broadcast, RwLock};
use tracing::warn;

use crate::commerce_payment_intent_strategy::{
    CommercePaymentIntentProviderStrategy, ConfiguredCommercePaymentIntentStrategy,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommercePaymentIntentRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    EntitlementChecked,
    ApprovalChecked,
    ResourceReserved,
    ProviderInspected,
    ServiceCall,
    StateTransitionPlanned,
    SensitiveInputRejected,
    Unavailable,
    ProviderCallSucceeded,
    ProviderCallFailed,
    HealthReported,
    SnapshotRecorded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommercePaymentIntentRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: CommercePaymentIntentRuntimeEventKind,
    pub replay_ref: String,
}

pub struct CommercePaymentIntentSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<CommercePaymentIntentRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn CommercePaymentIntentProviderStrategy>,
}

impl CommercePaymentIntentSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(None)
    }

    pub fn mock_with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut provider = Self::new(None);
        provider.strategy = Arc::new(ConfiguredCommercePaymentIntentStrategy::with_commands(
            commands,
        ));
        provider
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        let strategy: Arc<dyn CommercePaymentIntentProviderStrategy> =
            Arc::new(if unavailable_reason.is_some() {
                ConfiguredCommercePaymentIntentStrategy::unavailable()
            } else {
                ConfiguredCommercePaymentIntentStrategy::mock()
            });
        Self {
            descriptor: commerce_payment_intent_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
            strategy,
        }
    }

    pub fn capability(&self) -> PaymentIntentProviderCapability {
        self.strategy.capability()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CommercePaymentIntentRuntimeEvent> {
        self.events.subscribe()
    }

    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(256);
        let _ = self.events.send(event(
            "payment_intent.snapshot",
            "snapshot:payment-intent",
            CommercePaymentIntentRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            ("pack_id".into(), COMMERCE_PAYMENT_INTENT_PACK_ID.into()),
            ("provider_class".into(), self.capability().provider_class),
            ("active_reference_count".into(), count.to_string()),
            (
                "redaction_profile".into(),
                "token_hashes_and_state_metadata_only".into(),
            ),
        ])
    }

    pub async fn shutdown(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        Ok(())
    }
}

#[async_trait]
impl SystemService for CommercePaymentIntentSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = self.descriptor.clone();
        descriptor
            .metadata
            .insert("provider_class".into(), self.capability().provider_class);
        descriptor
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "payment_intent.declaration",
            "declaration:payment-intent",
            CommercePaymentIntentRuntimeEventKind::PackDeclared,
        ));
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                CommercePaymentIntentRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = COMMERCE_PAYMENT_INTENT_SERVICE_ID, command = %command.name, "payment intent provider unavailable");
            return Err(ServiceError::ServiceUnavailable(sanitize(reason)));
        }
        if !COMMERCE_PAYMENT_INTENT_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(
                "payment_intent_command_unsupported".into(),
            ));
        }
        if raw_credential_present(&command.payload) {
            let _ = self.events.send(event(
                "payment_intent.sensitive_input_rejected",
                &trace.trace_id,
                CommercePaymentIntentRuntimeEventKind::SensitiveInputRejected,
            ));
            return Err(ServiceError::InvalidArgument(
                "raw_credential_rejected".into(),
            ));
        }
        self.strategy.validate_command(command.name.as_str())?;
        if let Some(reason) = admission_denial(&command.payload) {
            let _ = self.events.send(event(
                "payment_intent.policy_decision",
                &trace.trace_id,
                CommercePaymentIntentRuntimeEventKind::PolicyDecision,
            ));
            return Err(ServiceError::DisabledByPolicy(reason.into()));
        }
        if self.references.read().await.len() >= 256 {
            return Err(ServiceError::DisabledByPolicy("quota_exceeded".into()));
        }
        let intent_ref = format!("payment-intent:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), intent_ref.clone());
        for kind in [
            CommercePaymentIntentRuntimeEventKind::AdmissionValidated,
            CommercePaymentIntentRuntimeEventKind::EntitlementChecked,
            CommercePaymentIntentRuntimeEventKind::ResourceReserved,
            command_event(command.name.as_str()),
            CommercePaymentIntentRuntimeEventKind::ProviderCallSucceeded,
        ] {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, kind));
        }
        let state = if command.name.as_str() == "payment_intent.confirm" {
            "requires_action"
        } else {
            "requires_confirmation"
        };
        Ok(domain_pack_service_result(
            serde_json::json!({
                "status": if state == "requires_action" { "action_required" } else { "ok" },
                "payment_intent_ref": intent_ref,
                "state": state,
                "provider_class": "mock",
                "idempotency_key_hash": format!("idempotency:{}", trace.trace_id),
                "freshness": "current",
                "client_secret": null,
                "raw_credentials": false,
                "refund_receipt_settlement_execution": false
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
            "payment_intent.health",
            "health:payment-intent",
            CommercePaymentIntentRuntimeEventKind::HealthReported,
        ));
        health
    }
}

fn raw_credential_present(payload: &serde_json::Value) -> bool {
    [
        "card_number",
        "cvc",
        "cvv",
        "pan",
        "client_secret",
        "webhook_body",
    ]
    .iter()
    .any(|key| payload.get(*key).is_some_and(|value| !value.is_null()))
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
        "capture_unsupported",
        "cancel_unsupported",
        "audit_export_denied",
    ]
    .into_iter()
    .find(|key| payload.get(*key).and_then(serde_json::Value::as_bool) == Some(true))
}

fn command_event(command: &str) -> CommercePaymentIntentRuntimeEventKind {
    match command {
        "payment_intent.inspect_provider" | "payment_intent.describe_schema" => {
            CommercePaymentIntentRuntimeEventKind::ProviderInspected
        }
        command if command.contains("plan_") || command == "payment_intent.confirm" => {
            CommercePaymentIntentRuntimeEventKind::StateTransitionPlanned
        }
        _ => CommercePaymentIntentRuntimeEventKind::ServiceCall,
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
    kind: CommercePaymentIntentRuntimeEventKind,
) -> CommercePaymentIntentRuntimeEvent {
    CommercePaymentIntentRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}

pub fn commerce_payment_intent_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(COMMERCE_PAYMENT_INTENT_SERVICE_ID),
        ServiceType::new("commerce.payment_intent"),
        TraceSchemaRef::new("commerce.payment_intent.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), COMMERCE_PAYMENT_INTENT_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        COMMERCE_PAYMENT_INTENT_COMMANDS.len().to_string(),
    );
    descriptor.metadata.insert(
        "trace_event_count".into(),
        COMMERCE_PAYMENT_INTENT_TRACE_EVENTS.len().to_string(),
    );
    descriptor
}
