//! Provider-neutral runtime adapter for the knowledge-graph pack.
//!
//! The deterministic mock Strategy exists for canonical service conformance.
//! It stores opaque graph and replay references only; graph values, RDF terms,
//! query text, source documents, credentials, execution plans, and database
//! payloads stay behind replaceable runtime-host provider adapters.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    GraphProviderCapability, KernelServiceId, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef, KNOWLEDGE_GRAPH_COMMANDS,
    KNOWLEDGE_GRAPH_PACK_ID, KNOWLEDGE_GRAPH_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

/// Sanitized observation emitted for audit and replay consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub replay_ref: String,
}

/// Mock Strategy or fail-closed Null Object selected only by runtime composition.
pub struct GraphSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<GraphRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl GraphSystemServiceProvider {
    /// Build the provider-neutral mock used by canonical path tests.
    pub fn mock() -> Self {
        Self::new(None)
    }

    /// Build an explicit unavailable provider that never falls back silently.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }

    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: graph_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }

    /// Return bounded capability metadata without selecting a graph database.
    pub fn capability(&self) -> GraphProviderCapability {
        GraphProviderCapability {
            provider_class: "mock".into(),
            graph_models: BTreeSet::from(["property_graph".into(), "rdf".into()]),
            query_dialects: BTreeSet::from(["portable".into()]),
            import_export_formats: BTreeSet::from(["graph_bundle".into()]),
            max_depth: 5,
            state: DomainPackProviderCapabilityState::Preview,
        }
    }

    /// Subscribe to bounded events that carry no private graph content.
    pub fn subscribe(&self) -> broadcast::Receiver<GraphRuntimeEvent> {
        self.events.subscribe()
    }

    /// Capture Memento state as bounded reference counts for recovery diagnostics.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(100);
        let _ = self
            .events
            .send(event("graph.snapshot", "snapshot:graph-provider"));
        BTreeMap::from([
            ("descriptor_hash".into(), "graph:descriptor".into()),
            ("provider_class".into(), "mock".into()),
            ("active_reference_count".into(), count.to_string()),
        ])
    }
}

#[async_trait]
impl SystemService for GraphSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        let _ = self
            .events
            .send(event("graph.declaration", "declaration:graph-provider"));
        info!(service_id = %self.descriptor.id, "graph provider started");
        Ok(())
    }

    async fn call(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            let _ = self
                .events
                .send(event(command.name.as_str(), &trace.trace_id));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "graph provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !KNOWLEDGE_GRAPH_COMMANDS.contains(&command.name.as_str()) {
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let reference = format!("graph:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), reference.clone());
        let _ = self
            .events
            .send(event(command.name.as_str(), &trace.trace_id));
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "graph provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok", "graph_handle_ref":reference, "provider_class":"mock", "result_metadata":"bounded:provider-owned"}),
            trace,
            "mock",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "graph provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(service_id = %self.descriptor.id, "graph provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(match &self.unavailable_reason {
            Some(reason) => ServiceHealth::Unavailable {
                reason: reason.clone(),
            },
            None => ServiceHealth::Healthy,
        })
    }
}

/// Build the descriptor from proto-owned graph contract constants only.
pub fn graph_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(KNOWLEDGE_GRAPH_SERVICE_ID),
        ServiceType::new("knowledge.graph"),
        TraceSchemaRef::new("knowledge.graph.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), KNOWLEDGE_GRAPH_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        KNOWLEDGE_GRAPH_COMMANDS.len().to_string(),
    );
    descriptor
}

fn event(command: &str, trace_id: &str) -> GraphRuntimeEvent {
    GraphRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        replay_ref: format!("replay:{trace_id}"),
    }
}
