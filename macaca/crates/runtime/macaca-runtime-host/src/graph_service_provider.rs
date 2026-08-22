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
    GraphImportPlan, GraphProviderCapability, GraphQuery, KernelServiceId, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceSchemaRef,
    KNOWLEDGE_GRAPH_COMMANDS, KNOWLEDGE_GRAPH_PACK_ID, KNOWLEDGE_GRAPH_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::graph_strategies::{
    default_graph_import_export_strategies, default_graph_query_strategies,
    DeterministicGraphMergeStrategy, GraphImportExportStrategy, GraphMergeRequest,
    GraphMergeStrategy, GraphQueryValidationStrategy,
};

/// Sanitized observation emitted for audit and replay consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRuntimeEvent {
    pub command: String,
    /// Stable provider-neutral audit name; graph values and query text are omitted.
    pub event_name: String,
    pub trace_id: String,
    pub replay_ref: String,
}

/// Mock Strategy or fail-closed Null Object selected only by runtime composition.
pub struct GraphSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<GraphRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    query_strategies: Vec<Box<dyn GraphQueryValidationStrategy>>,
    import_export_strategies: Vec<Box<dyn GraphImportExportStrategy>>,
    merge_strategy: DeterministicGraphMergeStrategy,
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
            query_strategies: default_graph_query_strategies(),
            import_export_strategies: default_graph_import_export_strategies(),
            merge_strategy: DeterministicGraphMergeStrategy,
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
                .send(event("graph.unavailable", &trace.trace_id));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "graph provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !KNOWLEDGE_GRAPH_COMMANDS.contains(&command.name.as_str()) {
            let _ = self
                .events
                .send(event("graph.command_failed", &trace.trace_id));
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        if let Some(reason) = graph_admission_denial(command.name.as_str(), &command.payload) {
            let _ = self
                .events
                .send(event("graph.policy_decision", &trace.trace_id));
            return Err(ServiceError::DisabledByPolicy(reason.into()));
        }
        if let Some(reason) = self.strategy_admission_denial(&command) {
            let _ = self
                .events
                .send(event("graph.policy_decision", &trace.trace_id));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = reason, "graph Strategy rejected request");
            return Err(ServiceError::DisabledByPolicy(reason.into()));
        }
        if self.references.read().await.len() >= 256 {
            return Err(ServiceError::DisabledByPolicy("quota_exceeded".into()));
        }
        let reference = format!("graph:reference:{}", trace.trace_id);
        self.references
            .write()
            .await
            .insert(trace.trace_id.clone(), reference.clone());
        let _ = self
            .events
            .send(event(command.name.as_str(), &trace.trace_id));
        for audit in [
            "graph.admission_validated",
            "graph.policy_decision",
            "graph.entitlement_checked",
            "graph.resource_reserved",
            "graph.approval_checked",
            "graph.service_call",
        ] {
            let _ = self.events.send(event(audit, &trace.trace_id));
        }
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
        let _ = self
            .events
            .send(event("graph.health", "health:graph-provider"));
        Ok(match &self.unavailable_reason {
            Some(reason) => ServiceHealth::Unavailable {
                reason: reason.clone(),
            },
            None => ServiceHealth::Healthy,
        })
    }
}

impl GraphSystemServiceProvider {
    /// Run replaceable validation and merge Strategies before reference
    /// allocation.  Empty payloads retain compatibility with descriptor-only
    /// conformance calls; typed callers can opt into strict validation by
    /// supplying the relevant provider-neutral fields.
    fn strategy_admission_denial(&self, command: &ServiceCommand) -> Option<&'static str> {
        let payload = &command.payload;
        if (command.name.as_str() == "graph.query"
            || command.name.as_str() == "graph.validate_query")
            && (payload.get("dialect").is_some() || payload.get("query_ref").is_some())
        {
            let query = GraphQuery {
                query_ref: payload
                    .get("query_ref")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("opaque-query")
                    .into(),
                dialect: payload
                    .get("dialect")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("portable")
                    .into(),
                max_rows: payload
                    .get("max_rows")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(100)
                    .min(u32::MAX as u64) as u32,
                redaction_profile: payload
                    .get("redaction_profile")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("bounded")
                    .into(),
            };
            let Some(strategy) = self
                .query_strategies
                .iter()
                .find(|strategy| strategy.mode() == query.dialect)
            else {
                return Some("query_dialect_not_supported");
            };
            return (!strategy.validate(&query).accepted).then_some("query_strategy_rejected");
        }
        if command.name.as_str() == "graph.import_subgraph" && payload.get("format").is_some() {
            let plan = GraphImportPlan {
                import_ref: payload
                    .get("import_ref")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("opaque-import")
                    .into(),
                format: payload
                    .get("format")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .into(),
                dry_run: payload
                    .get("dry_run")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true),
                batch_size: payload
                    .get("batch_size")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(1)
                    .min(u32::MAX as u64) as u32,
            };
            let Some(strategy) = self
                .import_export_strategies
                .iter()
                .find(|strategy| strategy.format() == plan.format)
            else {
                return Some("import_format_not_supported");
            };
            return (!strategy.validate_import(&plan).accepted)
                .then_some("import_strategy_rejected");
        }
        if command.name.as_str() == "graph.export_subgraph" && payload.get("format").is_some() {
            let format = payload
                .get("format")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            let max_items = payload
                .get("max_items")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0)
                .min(u32::MAX as u64) as u32;
            let Some(strategy) = self
                .import_export_strategies
                .iter()
                .find(|strategy| strategy.format() == format)
            else {
                return Some("export_format_not_supported");
            };
            return (!strategy.validate_export(format, max_items).accepted)
                .then_some("export_strategy_rejected");
        }
        if command.name.as_str() == "graph.merge_entities" && payload.get("source_ref").is_some() {
            let request = GraphMergeRequest {
                source_ref: payload
                    .get("source_ref")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .into(),
                target_ref: payload
                    .get("target_ref")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .into(),
                conflict_policy: payload
                    .get("conflict_policy")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .into(),
                reversible: payload
                    .get("reversible")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            };
            return (!self.merge_strategy.evaluate(&request).accepted)
                .then_some("merge_strategy_rejected");
        }
        None
    }
}

/// Check bounded provider-neutral policy facts before allocating graph references.
fn graph_admission_denial(command: &str, payload: &serde_json::Value) -> Option<&'static str> {
    let blocked = |key: &str, reason: &'static str| {
        (payload.get(key).and_then(serde_json::Value::as_bool) == Some(true)).then_some(reason)
    };
    blocked("source_denied", "source_access_denied")
        .or_else(|| blocked("schema_incompatible", "schema_incompatible"))
        .or_else(|| blocked("query_sensitive", "query_sensitivity_denied"))
        .or_else(|| blocked("delete_approval_required", "delete_approval_required"))
        .or_else(|| blocked("merge_approval_required", "merge_approval_required"))
        .or_else(|| blocked("redaction_required", "import_export_redaction_required"))
        .or_else(|| blocked("provider_unavailable", "provider_unavailable"))
        .or_else(|| blocked("quota_exceeded", "quota_exceeded"))
        .or_else(|| blocked("timeout", "timeout"))
        .or_else(|| blocked("cancelled", "cancelled"))
        .or_else(|| {
            (command.contains("query")
                && payload
                    .get("max_depth")
                    .and_then(serde_json::Value::as_u64)
                    .is_some_and(|depth| depth > 5))
            .then_some("max_depth_exceeded")
        })
        .or_else(|| {
            (payload
                .get("max_rows")
                .and_then(serde_json::Value::as_u64)
                .is_some_and(|rows| rows > 10_000))
            .then_some("max_rows_exceeded")
        })
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
        event_name: graph_audit_event(command).into(),
        trace_id: trace_id.into(),
        replay_ref: format!("replay:{trace_id}"),
    }
}

/// Map graph commands and lifecycle markers to stable audit vocabulary.
fn graph_audit_event(command: &str) -> &'static str {
    match command {
        "graph.declaration" => "graph.pack_declared",
        "graph.snapshot" => "graph.snapshot_recorded",
        "graph.health" => "graph.health",
        "graph.admission_validated" => "graph.admission_validated",
        "graph.policy_decision" => "graph.policy_decision",
        "graph.entitlement_checked" => "graph.entitlement_checked",
        "graph.resource_reserved" => "graph.resource_reserved",
        "graph.approval_checked" => "graph.approval_checked",
        "graph.service_call" => "graph.service_call",
        "graph.unavailable" => "graph.unavailable",
        command if command.contains("query") => "graph.query",
        command if command.contains("traverse") => "graph.traversal",
        command if command.contains("path") => "graph.path",
        command if command.contains("import") || command.contains("export") => {
            "graph.import_export"
        }
        command if command.contains("merge") => "graph.merge",
        command if command.contains("provenance") => "graph.provenance",
        command if command.contains("upsert") || command.contains("delete") => "graph.mutation",
        _ => "graph.command",
    }
}
