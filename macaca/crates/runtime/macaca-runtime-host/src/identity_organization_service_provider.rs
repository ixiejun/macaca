//! Provider-neutral runtime adapter for the identity-organization pack.
//!
//! This module implements the Adapter and Strategy boundaries required by the
//! service runtime. It deliberately retains only opaque references, so a
//! concrete organization directory can be replaced without exposing provider
//! payloads, invitation tokens, member lists, or application workflows.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::identity_organization::{
    OrganizationProviderCapability, IDENTITY_ORGANIZATION_COMMANDS, IDENTITY_ORGANIZATION_PACK_ID,
    IDENTITY_ORGANIZATION_SERVICE_ID,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    KernelServiceId, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
    ServiceType, TraceSchemaRef,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// A trace-safe observation generated at an organization service boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityOrganizationRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: IdentityOrganizationRuntimeEventKind,
    pub replay_ref: String,
}

/// Stable event taxonomy for the Observer-based audit integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityOrganizationRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    ResourceReserved,
    EntitlementChecked,
    ApprovalChecked,
    ProviderInspected,
    ServiceCall,
    OrganizationLifecycleChanged,
    MembershipLifecycleChanged,
    InvitationLifecycleChanged,
    RoleBindingChanged,
    DirectoryLinkInspected,
    AuditExportRequested,
    ArtifactRequested,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Deterministic mock and explicit unavailable implementation of `SystemService`.
pub struct IdentityOrganizationSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<IdentityOrganizationRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl IdentityOrganizationSystemServiceProvider {
    /// Construct the in-memory conformance Strategy without binding a directory product.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Construct the Null Object Strategy for an absent optional provider.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: identity_organization_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }

    /// Describe supported behavior with generic feature references and bounded limits.
    pub fn capability(&self) -> OrganizationProviderCapability {
        OrganizationProviderCapability {
            provider_class: "mock".into(),
            feature_flags: IDENTITY_ORGANIZATION_COMMANDS
                .iter()
                .map(|value| (*value).into())
                .collect::<BTreeSet<_>>(),
            supported_states: BTreeSet::from(["active".into(), "archived".into()]),
            limits: BTreeMap::from([
                ("max_page_size".into(), 100),
                ("max_snapshot_items".into(), 100),
            ]),
            state: DomainPackProviderCapabilityState::Preview,
        }
    }

    /// Subscribe to sanitized observation records rather than provider-native events.
    pub fn subscribe(&self) -> broadcast::Receiver<IdentityOrganizationRuntimeEvent> {
        self.events.subscribe()
    }

    /// Capture bounded Memento metadata for restart diagnostics without payload retention.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(100);
        let _ = self.events.send(event(
            "organization.snapshot",
            "snapshot:organization-provider",
            IdentityOrganizationRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            (
                "descriptor_hash".into(),
                "identity-organization:descriptor".into(),
            ),
            ("provider_class".into(), "mock".into()),
            ("active_reference_count".into(), count.to_string()),
        ])
    }
}

#[async_trait]
impl SystemService for IdentityOrganizationSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "organization.declaration",
            "declaration:organization-provider",
            IdentityOrganizationRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "identity organization provider started");
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
                IdentityOrganizationRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "identity organization provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !IDENTITY_ORGANIZATION_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let reference = format!("organization:reference:{}", trace.trace_id);
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
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "identity organization provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({
                "status": "ok", "organization_handle_ref": reference,
                "provider_class": "mock", "freshness": "current"
            }),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "identity organization provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(service_id = %self.descriptor.id, "identity organization provider cleanup completed");
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
            "organization.health",
            "health:organization-provider",
            IdentityOrganizationRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build a descriptor from the provider-neutral protocol constants.
pub fn identity_organization_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(IDENTITY_ORGANIZATION_SERVICE_ID),
        ServiceType::new("identity.organization"),
        TraceSchemaRef::new("identity.organization.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), IDENTITY_ORGANIZATION_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        IDENTITY_ORGANIZATION_COMMANDS.len().to_string(),
    );
    descriptor
}

fn common_event_kinds() -> &'static [IdentityOrganizationRuntimeEventKind] {
    use IdentityOrganizationRuntimeEventKind::*;
    &[
        AdmissionValidated,
        PolicyDecision,
        ResourceReserved,
        EntitlementChecked,
        ApprovalChecked,
        ServiceCall,
    ]
}

fn event_kind(command: &str) -> IdentityOrganizationRuntimeEventKind {
    use IdentityOrganizationRuntimeEventKind::*;
    match command {
        "organization.inspect_provider" | "organization.discover_schema" => ProviderInspected,
        "organization.create"
        | "organization.update"
        | "organization.archive"
        | "organization.restore" => OrganizationLifecycleChanged,
        "organization.plan_membership_change" | "organization.request_membership_change" => {
            MembershipLifecycleChanged
        }
        "organization.create_invitation"
        | "organization.resend_invitation"
        | "organization.revoke_invitation"
        | "organization.inspect_invitation" => InvitationLifecycleChanged,
        "organization.plan_role_binding"
        | "organization.request_role_binding"
        | "organization.list_role_bindings" => RoleBindingChanged,
        "organization.inspect_directory_links" => DirectoryLinkInspected,
        "organization.export_audit" => AuditExportRequested,
        "organization.get_artifact" => ArtifactRequested,
        _ => ServiceCall,
    }
}

fn event(
    command: &str,
    trace_id: &str,
    kind: IdentityOrganizationRuntimeEventKind,
) -> IdentityOrganizationRuntimeEvent {
    IdentityOrganizationRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
