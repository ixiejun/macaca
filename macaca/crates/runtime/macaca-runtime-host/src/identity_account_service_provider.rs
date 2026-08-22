//! Provider-neutral runtime adapter for the identity-account pack.
//!
//! This deterministic mock is a Strategy implementation for conformance and
//! composition tests. It never stores account payloads, credentials, or vendor
//! state; concrete directory adapters remain replaceable runtime providers.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::identity_account::{
    AccountProviderCapability, IDENTITY_ACCOUNT_COMMANDS, IDENTITY_ACCOUNT_PACK_ID,
    IDENTITY_ACCOUNT_SERVICE_ID,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    KernelServiceId, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
    ServiceType, TraceSchemaRef,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Reference-only account observation emitted after a canonical service call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityAccountRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: IdentityAccountRuntimeEventKind,
    pub outcome: String,
    pub replay_ref: String,
}

/// Bounded account audit categories that contain no directory payload data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityAccountRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    EntitlementChecked,
    ApprovalChecked,
    ResourceReserved,
    ProviderInspected,
    ServiceCall,
    ProviderCallStarted,
    ProviderCallSucceeded,
    CreatePlanned,
    LifecyclePlanned,
    IdentityLinkChanged,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Deterministic mock or Null Object provider behind the canonical service runtime.
pub struct IdentityAccountSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<IdentityAccountRuntimeEvent>,
    handles: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl IdentityAccountSystemServiceProvider {
    /// Create the provider-neutral mock used by conformance tests and composition roots.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Create the explicit unavailable provider used when no account adapter is installed.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: identity_account_service_descriptor(),
            events,
            handles: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }

    /// Return declarative feature facts rather than concrete directory details.
    pub fn capability(&self) -> AccountProviderCapability {
        AccountProviderCapability {
            provider_class: if self.unavailable_reason.is_some() {
                "unavailable"
            } else {
                "mock"
            }
            .into(),
            feature_flags: IDENTITY_ACCOUNT_COMMANDS
                .iter()
                .map(|command| (*command).into())
                .collect::<BTreeSet<_>>(),
            supported_lifecycle_states: BTreeSet::from([
                "active".into(),
                "suspended".into(),
                "disabled".into(),
            ]),
            limits: BTreeMap::from([
                ("max_page_size".into(), 100),
                ("max_snapshot_items".into(), 100),
            ]),
            state: if self.unavailable_reason.is_some() {
                DomainPackProviderCapabilityState::Unavailable
            } else {
                DomainPackProviderCapabilityState::Preview
            },
        }
    }

    /// Subscribe to sanitized account lifecycle observations.
    pub fn subscribe(&self) -> broadcast::Receiver<IdentityAccountRuntimeEvent> {
        self.events.subscribe()
    }

    /// Return reference-only state suitable for snapshot and replay diagnostics.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.handles.read().await.len();
        let _ = self.events.send(event(
            "account.snapshot",
            "snapshot:account-provider",
            IdentityAccountRuntimeEventKind::SnapshotRecorded,
            "ok",
        ));
        BTreeMap::from([
            (
                "descriptor_hash".into(),
                "identity-account:descriptor".into(),
            ),
            ("provider_class".into(), "mock".into()),
            ("active_reference_count".into(), count.min(100).to_string()),
        ])
    }
}

#[async_trait]
impl SystemService for IdentityAccountSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = self.descriptor.clone();
        descriptor
            .metadata
            .insert("provider_class".into(), self.capability().provider_class);
        descriptor
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "account.declaration",
            "declaration:account-provider",
            IdentityAccountRuntimeEventKind::PackDeclared,
            "ok",
        ));
        info!(service_id = %self.descriptor.id, "identity account provider started");
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
                IdentityAccountRuntimeEventKind::Unavailable,
                "unavailable",
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "identity account provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !IDENTITY_ACCOUNT_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        if let Some(reason) = account_admission_denial(&command.payload) {
            let _ = self.events.send(event(
                "account.policy_decision",
                &trace.trace_id,
                IdentityAccountRuntimeEventKind::PolicyDecision,
                "denied",
            ));
            return Err(ServiceError::DisabledByPolicy(reason.into()));
        }
        if self.handles.read().await.len() >= 100 {
            return Err(ServiceError::DisabledByPolicy("quota_exceeded".into()));
        }

        let handle_ref = format!("account:reference:{}", trace.trace_id);
        self.handles
            .write()
            .await
            .insert(trace.trace_id.clone(), handle_ref.clone());
        for kind in common_event_kinds()
            .iter()
            .chain([event_kind(command.name.as_str())].iter())
        {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                *kind,
                "ok",
            ));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "identity account provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({
                "status": "ok",
                "account_handle_ref": handle_ref,
                "provider_class": "mock",
                "freshness": "current",
            }),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "identity account provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.handles.write().await.clear();
        info!(service_id = %self.descriptor.id, "identity account provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        let health = match &self.unavailable_reason {
            Some(reason) => ServiceHealth::Unavailable {
                reason: reason.clone(),
            },
            None => ServiceHealth::Healthy,
        };
        let outcome = if self.unavailable_reason.is_some() {
            "unavailable"
        } else {
            "ok"
        };
        let _ = self.events.send(event(
            "account.health",
            "health:account-provider",
            IdentityAccountRuntimeEventKind::HealthReported,
            outcome,
        ));
        Ok(health)
    }
}

/// Evaluate opaque host-issued policy facts before retaining account references.
fn account_admission_denial(payload: &serde_json::Value) -> Option<&'static str> {
    let blocked = |key: &str, reason: &'static str| {
        (payload.get(key).and_then(serde_json::Value::as_bool) == Some(true)).then_some(reason)
    };
    blocked("policy_denied", "policy_denied")
        .or_else(|| blocked("entitlement_missing", "entitlement_missing"))
        .or_else(|| blocked("approval_required", "approval_required"))
        .or_else(|| blocked("permission_denied", "permission_denied"))
        .or_else(|| blocked("lifecycle_unsupported", "lifecycle_unsupported"))
        .or_else(|| blocked("linked_identity_denied", "linked_identity_denied"))
        .or_else(|| blocked("recovery_denied", "recovery_reference_denied"))
        .or_else(|| blocked("audit_export_denied", "audit_export_denied"))
        .or_else(|| blocked("stale_data", "stale_data"))
        .or_else(|| blocked("quota_exceeded", "quota_exceeded"))
        .or_else(|| blocked("timeout", "timeout"))
        .or_else(|| blocked("cancelled", "cancelled"))
}

/// Build the runtime descriptor entirely from proto-owned contract constants.
pub fn identity_account_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(IDENTITY_ACCOUNT_SERVICE_ID),
        ServiceType::new("identity.account"),
        TraceSchemaRef::new("identity.account.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), IDENTITY_ACCOUNT_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        IDENTITY_ACCOUNT_COMMANDS.len().to_string(),
    );
    descriptor
}

fn common_event_kinds() -> &'static [IdentityAccountRuntimeEventKind] {
    use IdentityAccountRuntimeEventKind::*;
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

fn event_kind(command: &str) -> IdentityAccountRuntimeEventKind {
    match command {
        "account.inspect_provider" | "account.describe_schema" => {
            IdentityAccountRuntimeEventKind::ProviderInspected
        }
        "account.plan_create" | "account.plan_update" | "account.plan_audit_export" => {
            IdentityAccountRuntimeEventKind::CreatePlanned
        }
        "account.plan_lifecycle_transition" => IdentityAccountRuntimeEventKind::LifecyclePlanned,
        "account.link_identity" | "account.unlink_identity" => {
            IdentityAccountRuntimeEventKind::IdentityLinkChanged
        }
        _ => IdentityAccountRuntimeEventKind::ServiceCall,
    }
}

fn event(
    command: &str,
    trace_id: &str,
    kind: IdentityAccountRuntimeEventKind,
    outcome: &str,
) -> IdentityAccountRuntimeEvent {
    IdentityAccountRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        outcome: outcome.into(),
        replay_ref: format!("replay:{trace_id}"),
    }
}
