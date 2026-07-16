//! Provider-neutral runtime adapter for the knowledge document-parsing pack.
//!
//! The deterministic mock models only opaque parse-job references. It does not
//! inspect document bytes, OCR images, embedded files, page text, tables, forms,
//! signatures, or format-specific payloads, preserving those concerns for
//! replaceable parser adapters behind the service boundary.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DocumentParserCapability,
    DomainPackProviderCapabilityState, KernelServiceId, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
    KNOWLEDGE_DOCUMENT_PARSING_COMMANDS, KNOWLEDGE_DOCUMENT_PARSING_PACK_ID,
    KNOWLEDGE_DOCUMENT_PARSING_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Reference-only parsing event suitable for trace and audit observers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentParsingRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: DocumentParsingRuntimeEventKind,
    pub replay_ref: String,
}

/// Bounded parser lifecycle taxonomy without document content or provider data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentParsingRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    ResourceReserved,
    EntitlementChecked,
    ApprovalChecked,
    FormatDetected,
    DocumentValidated,
    ParseJobStarted,
    ParseJobInspected,
    ParseJobCanceled,
    TextExtracted,
    LayoutExtracted,
    TablesExtracted,
    FormsExtracted,
    MetadataExtracted,
    Canonicalized,
    Chunked,
    ParserInspected,
    ServiceCall,
    ProviderCallStarted,
    ProviderCallSucceeded,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Deterministic mock or explicit unavailable document parsing provider.
pub struct DocumentParsingSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<DocumentParsingRuntimeEvent>,
    jobs: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl DocumentParsingSystemServiceProvider {
    /// Create the provider-neutral mock Strategy for runtime conformance tests.
    pub fn mock() -> Self {
        Self::new(None)
    }
    /// Create the fail-closed Null Object for optional parser absence.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: document_parsing_service_descriptor(),
            events,
            jobs: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }
    /// Return bounded format and feature facts without binding an OCR engine.
    pub fn capability(&self) -> DocumentParserCapability {
        DocumentParserCapability {
            provider_class: "mock".into(),
            supported_formats: BTreeSet::from(["reference".into()]),
            supported_features: BTreeSet::from([
                "ocr".into(),
                "layout".into(),
                "tables".into(),
                "forms".into(),
                "async_jobs".into(),
            ]),
            max_bytes: 65_536,
            max_pages: 100,
            state: DomainPackProviderCapabilityState::Preview,
        }
    }
    /// Subscribe to sanitized parsing progress and lifecycle events.
    pub fn subscribe(&self) -> broadcast::Receiver<DocumentParsingRuntimeEvent> {
        self.events.subscribe()
    }
    /// Capture a bounded Memento of opaque parse-job state for recovery diagnostics.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.jobs.read().await.len().min(100);
        let _ = self.events.send(event(
            "document_parsing.snapshot",
            "snapshot:document-parsing-provider",
            DocumentParsingRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            (
                "descriptor_hash".into(),
                "document-parsing:descriptor".into(),
            ),
            ("provider_class".into(), "mock".into()),
            ("active_job_count".into(), count.to_string()),
        ])
    }
}

#[async_trait]
impl SystemService for DocumentParsingSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }
    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "document_parsing.declaration",
            "declaration:document-parsing-provider",
            DocumentParsingRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "document parsing provider started");
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
                DocumentParsingRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "document parsing provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !KNOWLEDGE_DOCUMENT_PARSING_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let job_ref = format!("document-parse:reference:{}", trace.trace_id);
        self.jobs
            .write()
            .await
            .insert(trace.trace_id.clone(), job_ref.clone());
        for kind in common_event_kinds()
            .iter()
            .chain([event_kind(command.name.as_str())].iter())
        {
            let _ = self
                .events
                .send(event(&command.name.to_string(), &trace.trace_id, *kind));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "document parsing provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok", "parse_job_ref":job_ref, "provider_class":"mock", "result_metadata":"bounded:provider-owned"}),
            trace,
            "mock",
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "document parsing provider stopped");
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.jobs.write().await.clear();
        info!(service_id = %self.descriptor.id, "document parsing provider cleanup completed");
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
            "document_parsing.health",
            "health:document-parsing-provider",
            DocumentParsingRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build a descriptor only from proto-owned document parsing contract constants.
pub fn document_parsing_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(KNOWLEDGE_DOCUMENT_PARSING_SERVICE_ID),
        ServiceType::new("knowledge.document_parsing"),
        TraceSchemaRef::new("knowledge.document_parsing.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), KNOWLEDGE_DOCUMENT_PARSING_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        KNOWLEDGE_DOCUMENT_PARSING_COMMANDS.len().to_string(),
    );
    descriptor
}
fn common_event_kinds() -> &'static [DocumentParsingRuntimeEventKind] {
    use DocumentParsingRuntimeEventKind::*;
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
fn event_kind(command: &str) -> DocumentParsingRuntimeEventKind {
    use DocumentParsingRuntimeEventKind::*;
    match command {
        "document_parsing.detect_format" => FormatDetected,
        "document_parsing.validate_document" => DocumentValidated,
        "document_parsing.parse_document" | "document_parsing.start_parse_job" => ParseJobStarted,
        "document_parsing.get_parse_job" => ParseJobInspected,
        "document_parsing.cancel_parse_job" => ParseJobCanceled,
        "document_parsing.extract_text" => TextExtracted,
        "document_parsing.extract_layout" => LayoutExtracted,
        "document_parsing.extract_tables" => TablesExtracted,
        "document_parsing.extract_forms" => FormsExtracted,
        "document_parsing.extract_metadata" => MetadataExtracted,
        "document_parsing.convert_to_canonical" => Canonicalized,
        "document_parsing.chunk_document" => Chunked,
        "document_parsing.inspect_parser" => ParserInspected,
        _ => ServiceCall,
    }
}
fn event(
    command: &str,
    trace_id: &str,
    kind: DocumentParsingRuntimeEventKind,
) -> DocumentParsingRuntimeEvent {
    DocumentParsingRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
