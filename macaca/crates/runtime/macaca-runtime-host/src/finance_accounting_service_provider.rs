//! Runtime service adapter for the provider-neutral finance-accounting pack.
//!
//! The built-in provider is intentionally synthetic: it retains only opaque
//! references and bounded counters, while policy/admission facts are checked
//! before any reference is retained. Real providers can replace this Strategy
//! through the service registry without changing SDK, kernel, or shell code.

use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::finance_accounting::{
    AccountingProviderCapability, FINANCE_ACCOUNTING_COMMANDS, FINANCE_ACCOUNTING_PACK_ID,
    FINANCE_ACCOUNTING_SERVICE_ID, FINANCE_ACCOUNTING_TRACE_EVENTS,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, KernelServiceId, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::finance_accounting_strategy::{
    ConfiguredFinanceAccountingStrategy, FinanceAccountingProviderStrategy,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinanceAccountingRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    EntitlementChecked,
    ApprovalChecked,
    ResourceReserved,
    ProviderInspected,
    ServiceCallRequested,
    ServiceCallSucceeded,
    ServiceCallFailed,
    SideEffectPlanned,
    SideEffectApproved,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinanceAccountingRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: FinanceAccountingRuntimeEventKind,
    pub replay_ref: String,
}

pub struct FinanceAccountingSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<FinanceAccountingRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
    strategy: Arc<dyn FinanceAccountingProviderStrategy>,
}

impl FinanceAccountingSystemServiceProvider {
    pub fn mock() -> Self {
        Self::new(None)
    }

    pub fn mock_with_commands<I, S>(commands: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut provider = Self::new(None);
        provider.strategy = Arc::new(ConfiguredFinanceAccountingStrategy::with_commands(commands));
        provider
    }

    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        let strategy: Arc<dyn FinanceAccountingProviderStrategy> =
            Arc::new(if unavailable_reason.is_some() {
                ConfiguredFinanceAccountingStrategy::unavailable()
            } else {
                ConfiguredFinanceAccountingStrategy::mock()
            });
        Self {
            descriptor: finance_accounting_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
            strategy,
        }
    }

    pub fn capability(&self) -> AccountingProviderCapability {
        self.strategy.capability()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<FinanceAccountingRuntimeEvent> {
        self.events.subscribe()
    }

    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(100);
        let _ = self.events.send(event(
            "accounting.snapshot",
            "snapshot:finance-accounting",
            FinanceAccountingRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            ("pack_id".into(), FINANCE_ACCOUNTING_PACK_ID.into()),
            ("provider_class".into(), self.capability().provider_class),
            ("active_reference_count".into(), count.to_string()),
            (
                "redaction_profile".into(),
                "references_and_hashes_only".into(),
            ),
        ])
    }

    pub async fn shutdown(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(
            service_id = FINANCE_ACCOUNTING_SERVICE_ID,
            "finance accounting provider shutdown completed"
        );
        Ok(())
    }
}

#[async_trait]
impl SystemService for FinanceAccountingSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = self.descriptor.clone();
        descriptor
            .metadata
            .insert("provider_class".into(), self.capability().provider_class);
        descriptor
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "accounting.declaration",
            "declaration:finance-accounting",
            FinanceAccountingRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "finance accounting provider started");
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
                FinanceAccountingRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "finance accounting provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !FINANCE_ACCOUNTING_COMMANDS.contains(&command.name.as_str()) {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                FinanceAccountingRuntimeEventKind::ServiceCallFailed,
            ));
            return Err(normalize_accounting_error(
                ServiceError::UnsupportedCommand(command.name.to_string()),
            ));
        }
        if let Err(error) = self.strategy.validate_command(command.name.as_str()) {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                FinanceAccountingRuntimeEventKind::ServiceCallFailed,
            ));
            return Err(normalize_accounting_error(error));
        }
        if let Some(reason) = accounting_admission_denial(&command.payload) {
            let _ = self.events.send(event(
                "accounting.policy_decision",
                &trace.trace_id,
                FinanceAccountingRuntimeEventKind::PolicyDecision,
            ));
            return Err(normalize_accounting_error(ServiceError::DisabledByPolicy(
                reason.into(),
            )));
        }
        if self.references.read().await.len() >= 100 {
            return Err(normalize_accounting_error(ServiceError::DisabledByPolicy(
                "quota_exceeded".into(),
            )));
        }
        let reference = format!("accounting:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), reference.clone());
        for kind in common_events(command.name.as_str()) {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, *kind));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "finance accounting provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({
                "status": "ok",
                "accounting_reference": reference,
                "provider_class": self.capability().provider_class,
                "freshness": "current",
                "redaction_profile": "references_and_hashes_only",
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
        let health = self
            .unavailable_reason
            .as_ref()
            .map(|reason| ServiceHealth::Unavailable {
                reason: reason.clone(),
            })
            .unwrap_or(ServiceHealth::Healthy);
        let _ = self.events.send(event(
            "accounting.health",
            "health:finance-accounting",
            FinanceAccountingRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

pub fn finance_accounting_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(FINANCE_ACCOUNTING_SERVICE_ID),
        ServiceType::new("finance.accounting"),
        TraceSchemaRef::new("finance.accounting.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), FINANCE_ACCOUNTING_PACK_ID.into());
    descriptor.metadata.insert(
        "command_count".into(),
        FINANCE_ACCOUNTING_COMMANDS.len().to_string(),
    );
    descriptor.metadata.insert(
        "trace_event_count".into(),
        FINANCE_ACCOUNTING_TRACE_EVENTS.len().to_string(),
    );
    // Capability metadata is descriptor-owned so plugins and remote adapters
    // can advertise the same accounting features without OS-layer routing.
    for (key, value) in [
        ("period_locks", "supported"),
        ("write_support", "approval_gated"),
        (
            "report_support",
            "trial_balance,balance_sheet,profit_loss,cash_flow",
        ),
        ("export_support", "bounded_artifact"),
        ("attachments", "reference_only"),
        ("dimensions", "reference_only"),
        ("tax_references", "reference_only"),
        ("async_operations", "replayable"),
    ] {
        descriptor.metadata.insert(key.into(), value.into());
    }
    descriptor
}

fn accounting_admission_denial(payload: &serde_json::Value) -> Option<&'static str> {
    let blocked = |key: &str, reason: &'static str| {
        (payload.get(key).and_then(serde_json::Value::as_bool) == Some(true)).then_some(reason)
    };
    blocked("policy_denied", "policy_denied")
        .or_else(|| blocked("entitlement_missing", "entitlement_missing"))
        .or_else(|| blocked("approval_required", "approval_required"))
        .or_else(|| blocked("permission_denied", "permission_denied"))
        .or_else(|| blocked("period_locked", "period_locked"))
        .or_else(|| blocked("stale_data", "stale_data"))
        .or_else(|| blocked("quota_exceeded", "quota_exceeded"))
        .or_else(|| blocked("timeout", "timeout"))
        .or_else(|| blocked("cancelled", "cancelled"))
}

fn normalize_accounting_error(error: ServiceError) -> ServiceError {
    match error {
        ServiceError::UnsupportedCommand(_) => {
            ServiceError::UnsupportedCommand("accounting_command_unsupported".into())
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

fn common_events(command: &str) -> &'static [FinanceAccountingRuntimeEventKind] {
    use FinanceAccountingRuntimeEventKind::*;
    if command == "accounting.inspect_provider" {
        return &[
            AdmissionValidated,
            PolicyDecision,
            EntitlementChecked,
            ResourceReserved,
            ProviderInspected,
            ServiceCallRequested,
            ServiceCallSucceeded,
        ];
    }
    if matches!(
        command,
        "accounting.account_request"
            | "accounting.post_journal"
            | "accounting.import_statement_lines"
            | "accounting.reconciliation_request"
            | "accounting.audit_export_request"
    ) {
        &[
            AdmissionValidated,
            PolicyDecision,
            EntitlementChecked,
            ApprovalChecked,
            ResourceReserved,
            ServiceCallRequested,
            SideEffectPlanned,
            SideEffectApproved,
            ServiceCallSucceeded,
        ]
    } else {
        &[
            AdmissionValidated,
            PolicyDecision,
            EntitlementChecked,
            ResourceReserved,
            ServiceCallRequested,
            ServiceCallSucceeded,
        ]
    }
}

fn event(
    command: &str,
    trace_id: &str,
    kind: FinanceAccountingRuntimeEventKind,
) -> FinanceAccountingRuntimeEvent {
    FinanceAccountingRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:finance-accounting:{trace_id}"),
    }
}
