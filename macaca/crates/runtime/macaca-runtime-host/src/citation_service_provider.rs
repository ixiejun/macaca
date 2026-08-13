//! Provider-neutral runtime adapter for the knowledge-citations pack.
//!
//! This deterministic mock Strategy proves canonical service dispatch while it
//! stores only opaque citation references. Identifier resolver payloads, source
//! documents, quotations, style files, formatted output, and private corpora
//! remain outside runtime-host and behind replaceable provider boundaries.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, CitationProviderCapability,
    DomainPackProviderCapabilityState, KernelServiceId, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
    KNOWLEDGE_CITATIONS_COMMANDS, KNOWLEDGE_CITATIONS_PACK_ID, KNOWLEDGE_CITATIONS_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Trace-safe citation observation retained for audit and replay lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: CitationRuntimeEventKind,
    pub replay_ref: String,
}

/// Bounded event taxonomy without source content or resolver data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CitationRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    ResourceReserved,
    EntitlementChecked,
    ApprovalChecked,
    CitationCreated,
    IdentifierResolved,
    SourceAnchorLinked,
    CitationVerified,
    CitationFormatted,
    BibliographyFormatted,
    CitationImportedExported,
    AnchorInspected,
    ProviderInspected,
    ServiceCall,
    ProviderCallStarted,
    ProviderCallSucceeded,
    ProviderCallFailed,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Deterministic mock or explicit unavailable citation provider behind `SystemService`.
pub struct CitationSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<CitationRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl CitationSystemServiceProvider {
    /// Create the provider-neutral mock Strategy used by conformance tests.
    pub fn mock() -> Self {
        Self::new(None)
    }
    /// Create the fail-closed Null Object for a missing optional citation adapter.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: citation_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }
    /// Report generic capabilities without selecting identifier or style engines.
    pub fn capability(&self) -> CitationProviderCapability {
        CitationProviderCapability {
            provider_class: "mock".into(),
            identifier_schemes: BTreeSet::from(["reference".into()]),
            style_formats: BTreeSet::from(["reference".into()]),
            selector_support: BTreeSet::from(["reference".into()]),
            verification_depth: "bounded".into(),
            max_items: 100,
            rate_limit_bucket: "runtime_host_default".into(),
            supports_health: true,
            state: DomainPackProviderCapabilityState::Preview,
        }
    }
    /// Subscribe to sanitized citation lifecycle events.
    pub fn subscribe(&self) -> broadcast::Receiver<CitationRuntimeEvent> {
        self.events.subscribe()
    }
    /// Capture bounded reference-count Memento data for diagnostic recovery.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(100);
        let _ = self.events.send(event(
            "citations.snapshot",
            "snapshot:citations-provider",
            CitationRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            ("descriptor_hash".into(), "citations:descriptor".into()),
            ("provider_class".into(), "mock".into()),
            ("redaction_profile".into(), "references_only".into()),
            ("active_reference_count".into(), count.to_string()),
        ])
    }
}

#[async_trait]
impl SystemService for CitationSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }
    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "citations.declaration",
            "declaration:citations-provider",
            CitationRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "citation provider started");
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
                CitationRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "citation provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !KNOWLEDGE_CITATIONS_COMMANDS.contains(&command.name.as_str()) {
            let _ = self.events.send(event(
                &command.name.to_string(),
                &trace.trace_id,
                CitationRuntimeEventKind::ProviderCallFailed,
            ));
            warn!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "citation provider rejected unsupported command");
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let reference = format!("citation:reference:{}", trace.trace_id);
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
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "citation provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok", "citation_handle_ref":reference, "provider_class":"mock", "output_metadata":"bounded:provider-owned"}),
            trace,
            "mock",
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "citation provider stopped");
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(service_id = %self.descriptor.id, "citation provider cleanup completed");
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
            "citations.health",
            "health:citations-provider",
            CitationRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build a descriptor only from proto-owned citations contract constants.
pub fn citation_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(KNOWLEDGE_CITATIONS_SERVICE_ID),
        ServiceType::new("knowledge.citations"),
        TraceSchemaRef::new("citations.pack.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), KNOWLEDGE_CITATIONS_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        KNOWLEDGE_CITATIONS_COMMANDS.len().to_string(),
    );
    descriptor
}
fn common_event_kinds() -> &'static [CitationRuntimeEventKind] {
    use CitationRuntimeEventKind::*;
    &[
        AdmissionValidated,
        PolicyDecision,
        ResourceReserved,
        EntitlementChecked,
        ApprovalChecked,
        ServiceCall,
        ProviderCallStarted,
        ProviderCallSucceeded,
    ]
}
fn event_kind(command: &str) -> CitationRuntimeEventKind {
    use CitationRuntimeEventKind::*;
    match command {
        "citations.create_citation" | "citations.update_citation" => CitationCreated,
        "citations.resolve_identifier" => IdentifierResolved,
        "citations.link_source_span" => SourceAnchorLinked,
        "citations.verify_citation" => CitationVerified,
        "citations.format_citation" => CitationFormatted,
        "citations.format_bibliography" => BibliographyFormatted,
        "citations.import_citations" | "citations.export_citations" => CitationImportedExported,
        "citations.inspect_source_anchor" => AnchorInspected,
        "citations.inspect_provider" => ProviderInspected,
        _ => ServiceCall,
    }
}
fn event(command: &str, trace_id: &str, kind: CitationRuntimeEventKind) -> CitationRuntimeEvent {
    CitationRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
