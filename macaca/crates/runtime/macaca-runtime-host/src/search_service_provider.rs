//! Provider-neutral runtime adapter for the knowledge-search pack.
//!
//! The deterministic mock is a Strategy used for contract conformance. It
//! retains opaque corpus and replay references only. Query text, provider
//! payloads, indexed documents, snippets, credentials, and ranking details
//! remain behind replaceable provider boundaries.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    KernelServiceId, SearchProviderCapability, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef, KNOWLEDGE_SEARCH_COMMANDS,
    KNOWLEDGE_SEARCH_PACK_ID, KNOWLEDGE_SEARCH_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Trace-safe search observation retained for audit and replay lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: SearchRuntimeEventKind,
    pub replay_ref: String,
}

/// Bounded event taxonomy that deliberately omits corpus and query contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    ResourceReserved,
    EntitlementChecked,
    ApprovalChecked,
    CorpusRegistered,
    IndexInspected,
    SearchExecuted,
    SuggestionsRequested,
    AutocompleteRequested,
    FacetsRequested,
    RankingExplained,
    IndexRefreshed,
    StatsInspected,
    DiagnosticsInspected,
    ServiceCall,
    ProviderCallStarted,
    ProviderCallSucceeded,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
}

/// Deterministic mock or explicit unavailable search provider.
pub struct SearchSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<SearchRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl SearchSystemServiceProvider {
    /// Create the provider-neutral mock Strategy for canonical runtime tests.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Create the fail-closed Null Object for an absent search adapter.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: search_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }

    /// Report generic capability facts without selecting an index engine.
    pub fn capability(&self) -> SearchProviderCapability {
        SearchProviderCapability {
            provider_class: "mock".into(),
            query_features: BTreeSet::from([
                "query_ast".into(),
                "filters".into(),
                "facets".into(),
                "sort".into(),
                "suggest".into(),
                "autocomplete".into(),
            ]),
            max_page_size: 100,
            supports_semantic: true,
            supports_hybrid: true,
            state: DomainPackProviderCapabilityState::Preview,
        }
    }

    /// Subscribe to sanitized search lifecycle events.
    pub fn subscribe(&self) -> broadcast::Receiver<SearchRuntimeEvent> {
        self.events.subscribe()
    }

    /// Capture bounded opaque-reference Memento data for diagnostic recovery.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(100);
        let _ = self.events.send(event(
            "search.snapshot",
            "snapshot:search-provider",
            SearchRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            ("descriptor_hash".into(), "search:descriptor".into()),
            ("provider_class".into(), "mock".into()),
            ("active_reference_count".into(), count.to_string()),
        ])
    }
}

#[async_trait]
impl SystemService for SearchSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "search.declaration",
            "declaration:search-provider",
            SearchRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "search provider started");
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
                SearchRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "search provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !KNOWLEDGE_SEARCH_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }

        let reference = format!("search:reference:{}", trace.trace_id);
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
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "search provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok", "search_handle_ref":reference, "provider_class":"mock", "result_metadata":"bounded:provider-owned"}),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "search provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(service_id = %self.descriptor.id, "search provider cleanup completed");
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
            "search.health",
            "health:search-provider",
            SearchRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build a descriptor only from proto-owned search contract constants.
pub fn search_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(KNOWLEDGE_SEARCH_SERVICE_ID),
        ServiceType::new("knowledge.search"),
        TraceSchemaRef::new("knowledge.search.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), KNOWLEDGE_SEARCH_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        KNOWLEDGE_SEARCH_COMMANDS.len().to_string(),
    );
    descriptor
}

fn common_event_kinds() -> &'static [SearchRuntimeEventKind] {
    use SearchRuntimeEventKind::*;
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

fn event_kind(command: &str) -> SearchRuntimeEventKind {
    use SearchRuntimeEventKind::*;
    match command {
        "search.register_corpus" => CorpusRegistered,
        "search.inspect_index" => IndexInspected,
        "search.search" => SearchExecuted,
        "search.suggest" => SuggestionsRequested,
        "search.autocomplete" => AutocompleteRequested,
        "search.facets" => FacetsRequested,
        "search.explain_ranking" => RankingExplained,
        "search.refresh_index" => IndexRefreshed,
        "search.index_stats" => StatsInspected,
        "search.query_diagnostics" => DiagnosticsInspected,
        _ => ServiceCall,
    }
}

fn event(command: &str, trace_id: &str, kind: SearchRuntimeEventKind) -> SearchRuntimeEvent {
    SearchRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
