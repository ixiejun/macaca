//! Provider-neutral runtime adapter for the identity auth-handoff pack.
//!
//! The adapter is a deterministic Strategy used to prove the canonical service
//! path. It accepts only opaque correlation references and consumes callback
//! references once, which provides a fail-closed replay guard without owning an
//! IdP, browser, credentials, tokens, session store, or application login flow.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::identity_auth_handoff::{
    AuthHandoffProviderCapability, IDENTITY_AUTH_HANDOFF_COMMANDS, IDENTITY_AUTH_HANDOFF_PACK_ID,
    IDENTITY_AUTH_HANDOFF_SERVICE_ID,
};
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    KernelServiceId, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
    ServiceType, TraceSchemaRef,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Sanitized auth-handoff event for observers, audits, and replay indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityAuthHandoffRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: IdentityAuthHandoffRuntimeEventKind,
    pub replay_ref: String,
}

/// Stable event categories that never include callback or credential content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityAuthHandoffRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    EntitlementChecked,
    ApprovalChecked,
    ResourceReserved,
    ProviderInspected,
    ServiceCall,
    HandoffPlanned,
    CallbackVerified,
    TokenReferenceExchanged,
    SessionBindingPlanned,
    ReplayRejected,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Deterministic mock and unavailable provider behind the `SystemService` interface.
pub struct IdentityAuthHandoffSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<IdentityAuthHandoffRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    consumed_callback_refs: RwLock<BTreeSet<String>>,
    unavailable_reason: Option<String>,
}

impl IdentityAuthHandoffSystemServiceProvider {
    /// Create a provider-neutral mock for service-runtime conformance tests.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Create the explicit Null Object used when no auth adapter is installed.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: identity_auth_handoff_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            consumed_callback_refs: RwLock::new(BTreeSet::new()),
            unavailable_reason,
        }
    }

    /// Report protocol support as generic contract facts, not vendor routing.
    pub fn capability(&self) -> AuthHandoffProviderCapability {
        AuthHandoffProviderCapability {
            provider_class: "mock".into(),
            feature_flags: IDENTITY_AUTH_HANDOFF_COMMANDS
                .iter()
                .map(|value| (*value).into())
                .collect::<BTreeSet<_>>(),
            protocol_profiles: BTreeSet::from([
                "oauth2_reference".into(),
                "oidc_reference".into(),
                "saml_reference".into(),
                "webauthn_reference".into(),
            ]),
            limits: BTreeMap::from([
                ("max_pending_handoffs".into(), 100),
                ("max_snapshot_items".into(), 100),
            ]),
            state: DomainPackProviderCapabilityState::Preview,
        }
    }

    /// Subscribe to reference-only handoff events.
    pub fn subscribe(&self) -> broadcast::Receiver<IdentityAuthHandoffRuntimeEvent> {
        self.events.subscribe()
    }

    /// Return bounded replay metadata without retaining handoff payloads.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let references = self.references.read().await.len().min(100);
        let consumed = self.consumed_callback_refs.read().await.len().min(100);
        let _ = self.events.send(event(
            "auth_handoff.snapshot",
            "snapshot:auth-handoff-provider",
            IdentityAuthHandoffRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            (
                "descriptor_hash".into(),
                "identity-auth-handoff:descriptor".into(),
            ),
            ("provider_class".into(), "mock".into()),
            ("active_reference_count".into(), references.to_string()),
            ("consumed_callback_count".into(), consumed.to_string()),
        ])
    }

    async fn reject_replay_if_consumed(
        &self,
        command: &ServiceCommand,
        trace_id: &str,
    ) -> ServiceResult<()> {
        if command.name.as_str() != "auth_handoff.verify_callback" {
            return Ok(());
        }
        let callback_ref = command
            .metadata
            .get("callback_ref_hash")
            .cloned()
            .unwrap_or_else(|| trace_id.into());
        let mut consumed = self.consumed_callback_refs.write().await;
        if !consumed.insert(callback_ref) {
            let _ = self.events.send(event(
                &command.name.to_string(),
                trace_id,
                IdentityAuthHandoffRuntimeEventKind::ReplayRejected,
            ));
            return Err(ServiceError::AdapterFailure("replay_rejected".into()));
        }
        Ok(())
    }
}

#[async_trait]
impl SystemService for IdentityAuthHandoffSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }
    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "auth_handoff.declaration",
            "declaration:auth-handoff-provider",
            IdentityAuthHandoffRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "identity auth handoff provider started");
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
                IdentityAuthHandoffRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "identity auth handoff provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !IDENTITY_AUTH_HANDOFF_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        self.reject_replay_if_consumed(&command, &trace.trace_id)
            .await?;
        let reference = format!("auth-handoff:reference:{}", trace.trace_id);
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
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "identity auth handoff provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok", "handoff_handle_ref":reference, "provider_class":"mock", "freshness":"current"}),
            trace,
            "mock",
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "identity auth handoff provider stopped");
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        self.consumed_callback_refs.write().await.clear();
        info!(service_id = %self.descriptor.id, "identity auth handoff provider cleanup completed");
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
            "auth_handoff.health",
            "health:auth-handoff-provider",
            IdentityAuthHandoffRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build a descriptor solely from proto-owned auth-handoff contract constants.
pub fn identity_auth_handoff_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(IDENTITY_AUTH_HANDOFF_SERVICE_ID),
        ServiceType::new("identity.auth_handoff"),
        TraceSchemaRef::new("identity.auth_handoff.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), IDENTITY_AUTH_HANDOFF_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        IDENTITY_AUTH_HANDOFF_COMMANDS.len().to_string(),
    );
    descriptor
}

fn common_event_kinds() -> &'static [IdentityAuthHandoffRuntimeEventKind] {
    use IdentityAuthHandoffRuntimeEventKind::*;
    &[
        AdmissionValidated,
        PolicyDecision,
        EntitlementChecked,
        ApprovalChecked,
        ResourceReserved,
        ServiceCall,
    ]
}
fn event_kind(command: &str) -> IdentityAuthHandoffRuntimeEventKind {
    use IdentityAuthHandoffRuntimeEventKind::*;
    match command {
        "auth_handoff.inspect_provider" | "auth_handoff.describe_schema" => ProviderInspected,
        "auth_handoff.plan_handoff" | "auth_handoff.start_handoff" => HandoffPlanned,
        "auth_handoff.verify_callback" => CallbackVerified,
        "auth_handoff.exchange_token_reference" => TokenReferenceExchanged,
        "auth_handoff.plan_session_binding" | "auth_handoff.bind_session" => SessionBindingPlanned,
        _ => ServiceCall,
    }
}
fn event(
    command: &str,
    trace_id: &str,
    kind: IdentityAuthHandoffRuntimeEventKind,
) -> IdentityAuthHandoffRuntimeEvent {
    IdentityAuthHandoffRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
