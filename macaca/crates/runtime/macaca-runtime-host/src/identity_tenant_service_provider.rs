//! Provider-neutral runtime adapter for the identity-tenant pack.
//!
//! The adapter is an in-memory Strategy for conformance paths only. It stores
//! opaque references rather than tenant policy, quota, residency, or config
//! values, keeping concrete cloud and directory adapters replaceable.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::identity_tenant::{
    TenantProviderCapability, IDENTITY_TENANT_COMMANDS, IDENTITY_TENANT_PACK_ID,
    IDENTITY_TENANT_SERVICE_ID,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    KernelServiceId, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
    ServiceType, TraceSchemaRef,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Reference-only event emitted to trace, audit, and replay observers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityTenantRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: IdentityTenantRuntimeEventKind,
    pub replay_ref: String,
}

/// Tenant-boundary event taxonomy containing no provider or configuration payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityTenantRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    ResourceReserved,
    EntitlementChecked,
    ApprovalChecked,
    ProviderInspected,
    ServiceCall,
    TenantLifecycleChanged,
    PolicyAttachmentChanged,
    QuotaReservationChanged,
    UsageSnapshotRecorded,
    ResidencyInspected,
    ConfigReferenceChanged,
    RelationshipInspected,
    AuditExportRequested,
    ArtifactRequested,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Deterministic mock or unavailable implementation behind `SystemService`.
pub struct IdentityTenantSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<IdentityTenantRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl IdentityTenantSystemServiceProvider {
    /// Construct a provider-neutral mock Strategy for runtime conformance tests.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Construct a fail-closed Null Object for an absent tenant provider.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: identity_tenant_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }

    /// Report generic command and lifecycle support without exposing provider internals.
    pub fn capability(&self) -> TenantProviderCapability {
        TenantProviderCapability {
            provider_class: "mock".into(),
            feature_flags: IDENTITY_TENANT_COMMANDS
                .iter()
                .map(|value| (*value).into())
                .collect::<BTreeSet<_>>(),
            supported_lifecycle_states: BTreeSet::from(["active".into(), "archived".into()]),
            quota_dimensions: BTreeSet::from(["reference_count".into()]),
            limits: BTreeMap::from([
                ("max_page_size".into(), 100),
                ("max_snapshot_items".into(), 100),
            ]),
            state: DomainPackProviderCapabilityState::Preview,
        }
    }

    /// Subscribe to sanitized provider-boundary events.
    pub fn subscribe(&self) -> broadcast::Receiver<IdentityTenantRuntimeEvent> {
        self.events.subscribe()
    }

    /// Return a bounded Memento that does not retain tenant data or config values.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(100);
        let _ = self.events.send(event(
            "tenant.snapshot",
            "snapshot:tenant-provider",
            IdentityTenantRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            (
                "descriptor_hash".into(),
                "identity-tenant:descriptor".into(),
            ),
            ("provider_class".into(), "mock".into()),
            ("active_reference_count".into(), count.to_string()),
        ])
    }
}

#[async_trait]
impl SystemService for IdentityTenantSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "tenant.declaration",
            "declaration:tenant-provider",
            IdentityTenantRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "identity tenant provider started");
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
                IdentityTenantRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "identity tenant provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !IDENTITY_TENANT_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let reference = format!("tenant:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), reference.clone());
        for kind in common_event_kinds()
            .iter()
            .chain([event_kind(command.name.as_str())].iter())
        {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, *kind));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "identity tenant provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok", "tenant_handle_ref":reference, "provider_class":"mock", "freshness":"current"}),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "identity tenant provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(service_id = %self.descriptor.id, "identity tenant provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        let health = match &self.unavailable_reason {
            Some(reason) => ServiceHealth::Unavailable {
                reason: reason.clone(),
            },
            None => ServiceHealth::Healthy,
        };
        let _ = self.events.send(event(
            "tenant.health",
            "health:tenant-provider",
            IdentityTenantRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build a descriptor exclusively from protocol-owned pack constants.
pub fn identity_tenant_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(IDENTITY_TENANT_SERVICE_ID),
        ServiceType::new("identity.tenant"),
        TraceSchemaRef::new("identity.tenant.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), IDENTITY_TENANT_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        IDENTITY_TENANT_COMMANDS.len().to_string(),
    );
    descriptor
}

fn common_event_kinds() -> &'static [IdentityTenantRuntimeEventKind] {
    use IdentityTenantRuntimeEventKind::*;
    &[
        AdmissionValidated,
        PolicyDecision,
        ResourceReserved,
        EntitlementChecked,
        ApprovalChecked,
        ServiceCall,
    ]
}

fn event_kind(command: &str) -> IdentityTenantRuntimeEventKind {
    use IdentityTenantRuntimeEventKind::*;
    match command {
        "tenant.inspect_provider" | "tenant.discover_schema" => ProviderInspected,
        "tenant.create"
        | "tenant.update"
        | "tenant.plan_lifecycle_transition"
        | "tenant.request_lifecycle_transition" => TenantLifecycleChanged,
        "tenant.plan_policy_attachment" | "tenant.request_policy_attachment" => {
            PolicyAttachmentChanged
        }
        "tenant.plan_quota_reservation" | "tenant.request_quota_reservation" => {
            QuotaReservationChanged
        }
        "tenant.snapshot_usage" => UsageSnapshotRecorded,
        "tenant.inspect_residency" => ResidencyInspected,
        "tenant.inspect_config" | "tenant.update_config_reference" => ConfigReferenceChanged,
        "tenant.inspect_relationships" => RelationshipInspected,
        "tenant.export_audit" => AuditExportRequested,
        "tenant.get_artifact" => ArtifactRequested,
        _ => ServiceCall,
    }
}

fn event(
    command: &str,
    trace_id: &str,
    kind: IdentityTenantRuntimeEventKind,
) -> IdentityTenantRuntimeEvent {
    IdentityTenantRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
