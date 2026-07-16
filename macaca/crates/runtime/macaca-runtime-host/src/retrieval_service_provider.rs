//! Provider-neutral runtime adapter for the knowledge-retrieval pack.
//!
//! The deterministic mock is a Strategy used for conformance. It persists only
//! opaque collection and evidence references, not vectors, chunks, documents,
//! prompts, filters, query text, scores, or private corpus content. Real vector
//! stores and rerank engines remain replaceable providers behind this boundary.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, CleanupPolicy, DomainPackProviderCapabilityState, KernelServiceId,
    RetrievalProviderCapability, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
    KNOWLEDGE_RETRIEVAL_COMMANDS, KNOWLEDGE_RETRIEVAL_PACK_ID, KNOWLEDGE_RETRIEVAL_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Trace-safe retrieval fact emitted after descriptor-owned service dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: RetrievalRuntimeEventKind,
    pub replay_ref: String,
}

/// Bounded retrieval event categories that omit corpus and query contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    ResourceReserved,
    EntitlementChecked,
    ApprovalChecked,
    CollectionChanged,
    RecordsChanged,
    QueryExecuted,
    BulkQueryExecuted,
    RangeQueryExecuted,
    RerankExecuted,
    ContextExpanded,
    EvidencePackaged,
    CollectionInspected,
    RecordInspected,
    CollectionRefreshed,
    DiagnosticsInspected,
    ServiceCall,
    ProviderCallStarted,
    ProviderCallSucceeded,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
    Failure,
}

/// Deterministic mock or explicit unavailable retrieval provider.
pub struct RetrievalSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<RetrievalRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl RetrievalSystemServiceProvider {
    /// Create the provider-neutral mock Strategy for canonical runtime tests.
    pub fn mock() -> Self {
        Self::new(None)
    }
    /// Create the fail-closed Null Object for an absent retrieval adapter.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: retrieval_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }
    /// Return descriptor-derived capability facts without selecting a vector backend.
    pub fn capability(&self) -> RetrievalProviderCapability {
        RetrievalProviderCapability {
            provider_class: "mock".into(),
            vector_features: BTreeSet::from([
                "dense".into(),
                "sparse".into(),
                "hybrid".into(),
                "multivector".into(),
                "named_vector_spaces".into(),
            ]),
            namespace_features: BTreeSet::from(["namespace".into(), "partition".into()]),
            query_features: BTreeSet::from([
                "metadata_filter".into(),
                "bulk_query".into(),
                "range_search".into(),
                "parent_window_expansion".into(),
            ]),
            max_top_k: 100,
            max_filters: 32,
            supports_rerank: true,
            supports_evidence: true,
            rate_limited: false,
            consistency_mode: "bounded_eventual".into(),
            state: DomainPackProviderCapabilityState::Preview,
        }
    }
    /// Subscribe to sanitized retrieval events for audit and replay observers.
    pub fn subscribe(&self) -> broadcast::Receiver<RetrievalRuntimeEvent> {
        self.events.subscribe()
    }
    /// Capture bounded opaque-reference state, never collection records or vectors.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(100);
        let _ = self.events.send(event(
            "retrieval.snapshot",
            "snapshot:retrieval-provider",
            RetrievalRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            ("descriptor_hash".into(), "retrieval:descriptor".into()),
            ("provider_class".into(), "mock".into()),
            ("active_reference_count".into(), count.to_string()),
        ])
    }
}

#[async_trait]
impl SystemService for RetrievalSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }
    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "retrieval.declaration",
            "declaration:retrieval-provider",
            RetrievalRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "retrieval provider started");
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
                RetrievalRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "retrieval provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !KNOWLEDGE_RETRIEVAL_COMMANDS.contains(&command.name.as_str()) {
            let _ = self.events.send(event(
                command.name.as_str(),
                &trace.trace_id,
                RetrievalRuntimeEventKind::Failure,
            ));
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let reference = format!("retrieval:reference:{}", trace.trace_id);
        let result_state = bounded_result_state(&command);
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
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "retrieval provider call completed");
        Ok(result(
            serde_json::json!({"status":result_state, "retrieval_handle_ref":reference, "next_cursor_ref":(result_state == "paged").then(|| format!("retrieval:cursor:{}", trace.trace_id)), "partial_result_ref":(result_state == "partial").then(|| format!("retrieval:partial:{}", trace.trace_id)), "async_handle_ref":(result_state == "async").then(|| format!("retrieval:async:{}", trace.trace_id)), "provider_class":"mock", "evidence_metadata":"bounded:provider-owned"}),
            trace,
            result_state,
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "retrieval provider stopped");
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(service_id = %self.descriptor.id, "retrieval provider cleanup completed");
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
            "retrieval.health",
            "health:retrieval-provider",
            RetrievalRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}
/// Select only supported mock result states without preserving provider data.
fn bounded_result_state(command: &ServiceCommand) -> &'static str {
    match command
        .payload
        .get("result_state")
        .and_then(serde_json::Value::as_str)
    {
        Some("paged") => "paged",
        Some("partial") => "partial",
        Some("async") => "async",
        _ => "ok",
    }
}

/// Build a sanitized service result while preserving a bounded lifecycle state.
fn result(
    output: serde_json::Value,
    trace: macaca_proto::TraceContext,
    status: &str,
) -> ServiceCallResult {
    ServiceCallResult {
        output,
        trace,
        status: status.into(),
        metadata: BTreeMap::from([("provider_class".into(), "mock".into())]),
        cleanup_hint: Some(CleanupPolicy::None),
    }
}

/// Build a descriptor only from proto-owned retrieval contract constants.
pub fn retrieval_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(KNOWLEDGE_RETRIEVAL_SERVICE_ID),
        ServiceType::new("knowledge.retrieval"),
        TraceSchemaRef::new("knowledge.retrieval.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), KNOWLEDGE_RETRIEVAL_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        KNOWLEDGE_RETRIEVAL_COMMANDS.len().to_string(),
    );
    descriptor
}
fn common_event_kinds() -> &'static [RetrievalRuntimeEventKind] {
    use RetrievalRuntimeEventKind::*;
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
fn event_kind(command: &str) -> RetrievalRuntimeEventKind {
    use RetrievalRuntimeEventKind::*;
    match command {
        "retrieval.register_collection" => CollectionChanged,
        "retrieval.upsert_records" | "retrieval.delete_records" => RecordsChanged,
        "retrieval.retrieve" | "retrieval.retrieve_by_id" => QueryExecuted,
        "retrieval.bulk_retrieve" => BulkQueryExecuted,
        "retrieval.range_retrieve" => RangeQueryExecuted,
        "retrieval.rerank_context" => RerankExecuted,
        "retrieval.expand_context" => ContextExpanded,
        "retrieval.package_evidence" => EvidencePackaged,
        "retrieval.inspect_collection" => CollectionInspected,
        "retrieval.inspect_record" => RecordInspected,
        "retrieval.refresh_collection" => CollectionRefreshed,
        "retrieval.query_diagnostics" => DiagnosticsInspected,
        _ => ServiceCall,
    }
}
fn event(command: &str, trace_id: &str, kind: RetrievalRuntimeEventKind) -> RetrievalRuntimeEvent {
    RetrievalRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
