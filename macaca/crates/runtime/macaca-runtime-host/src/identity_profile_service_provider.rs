//! Provider-neutral runtime adapter for the identity-profile pack.
//!
//! This adapter is a deterministic Strategy used by conformance paths. It is
//! intentionally reference-only: profile fields, avatars, media bytes, and
//! application preference values never enter runtime state or observer events.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::identity_profile::{
    ProfileProviderCapability, IDENTITY_PROFILE_COMMANDS, IDENTITY_PROFILE_PACK_ID,
    IDENTITY_PROFILE_SERVICE_ID,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    KernelServiceId, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
    ServiceType, TraceSchemaRef,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Trace-safe profile fact emitted for observers and replay indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityProfileRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: IdentityProfileRuntimeEventKind,
    pub replay_ref: String,
}

/// Bounded taxonomy for profile lifecycle observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityProfileRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    EntitlementChecked,
    ApprovalChecked,
    ResourceReserved,
    ProviderInspected,
    ServiceCall,
    PatchPlanned,
    PrivacyInspected,
    AvatarReferenceChanged,
    ExportPlanned,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Deterministic mock or fail-closed Null Object profile provider.
pub struct IdentityProfileSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<IdentityProfileRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl IdentityProfileSystemServiceProvider {
    /// Create a provider-neutral mock for composition and contract testing.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Create an explicit unavailable provider without emitting fake results.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: identity_profile_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }

    /// Report capability facts from descriptor commands rather than profile vendor data.
    pub fn capability(&self) -> ProfileProviderCapability {
        ProfileProviderCapability {
            provider_class: "mock".into(),
            feature_flags: IDENTITY_PROFILE_COMMANDS
                .iter()
                .map(|value| (*value).into())
                .collect::<BTreeSet<_>>(),
            supported_value_types: BTreeSet::from(["reference".into(), "hash".into()]),
            limits: BTreeMap::from([
                ("max_page_size".into(), 100),
                ("max_snapshot_items".into(), 100),
            ]),
            state: DomainPackProviderCapabilityState::Preview,
        }
    }

    /// Subscribe to redacted profile events without exposing profile content.
    pub fn subscribe(&self) -> broadcast::Receiver<IdentityProfileRuntimeEvent> {
        self.events.subscribe()
    }

    /// Capture a bounded Memento of reference counts for diagnostics and restart.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(100);
        let _ = self.events.send(event(
            "profile.snapshot",
            "snapshot:profile-provider",
            IdentityProfileRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            (
                "descriptor_hash".into(),
                "identity-profile:descriptor".into(),
            ),
            ("provider_class".into(), "mock".into()),
            ("active_reference_count".into(), count.to_string()),
        ])
    }
}

#[async_trait]
impl SystemService for IdentityProfileSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "profile.declaration",
            "declaration:profile-provider",
            IdentityProfileRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "identity profile provider started");
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
                IdentityProfileRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "identity profile provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !IDENTITY_PROFILE_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let reference = format!("profile:reference:{}", trace.trace_id);
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
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "identity profile provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok", "profile_handle_ref":reference, "provider_class":"mock", "freshness":"current"}),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "identity profile provider stopped");
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(service_id = %self.descriptor.id, "identity profile provider cleanup completed");
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
            "profile.health",
            "health:profile-provider",
            IdentityProfileRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build the profile descriptor solely from protocol constants owned by `macaca-proto`.
pub fn identity_profile_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(IDENTITY_PROFILE_SERVICE_ID),
        ServiceType::new("identity.profile"),
        TraceSchemaRef::new("identity.profile.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), IDENTITY_PROFILE_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        IDENTITY_PROFILE_COMMANDS.len().to_string(),
    );
    descriptor
}

fn common_event_kinds() -> &'static [IdentityProfileRuntimeEventKind] {
    use IdentityProfileRuntimeEventKind::*;
    &[
        AdmissionValidated,
        PolicyDecision,
        EntitlementChecked,
        ApprovalChecked,
        ResourceReserved,
        ServiceCall,
    ]
}

fn event_kind(command: &str) -> IdentityProfileRuntimeEventKind {
    use IdentityProfileRuntimeEventKind::*;
    match command {
        "profile.inspect_provider" | "profile.describe_schema" => ProviderInspected,
        "profile.plan_patch" => PatchPlanned,
        "profile.inspect_privacy_fields" => PrivacyInspected,
        "profile.plan_avatar_update"
        | "profile.set_avatar_reference"
        | "profile.clear_avatar_reference" => AvatarReferenceChanged,
        "profile.plan_export" | "profile.export_profile" => ExportPlanned,
        _ => ServiceCall,
    }
}

fn event(
    command: &str,
    trace_id: &str,
    kind: IdentityProfileRuntimeEventKind,
) -> IdentityProfileRuntimeEvent {
    IdentityProfileRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
