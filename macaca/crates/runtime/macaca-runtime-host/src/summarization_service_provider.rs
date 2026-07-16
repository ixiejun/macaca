//! Provider-neutral runtime adapter for the knowledge-summarization pack.
//!
//! This mock Strategy proves canonical dispatch without retaining source text,
//! prompts, model output, evidence content, credentials, or provider payloads.

use std::collections::{BTreeMap, BTreeSet};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    domain_pack_command_trace, domain_pack_service_result, DomainPackProviderCapabilityState,
    KernelServiceId, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult,
    ServiceType, SummaryProviderCapability, TraceSchemaRef, KNOWLEDGE_SUMMARIZATION_COMMANDS,
    KNOWLEDGE_SUMMARIZATION_PACK_ID, KNOWLEDGE_SUMMARIZATION_SERVICE_ID,
};
use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::summarization_strategy::{
    checkpoint_ref, LongDocumentExecutionPlan, SummarizationStrategyKind,
};

/// Sanitized Observer event containing only command, trace, and replay handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SummarizationRuntimeEvent {
    pub command: String,
    pub trace_id: String,
    pub kind: SummarizationRuntimeEventKind,
    pub replay_ref: String,
}

/// Bounded lifecycle taxonomy for summarization observability.
///
/// Each value identifies an execution phase without recording source text,
/// prompts, model output, provider payloads, or application-owned semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummarizationRuntimeEventKind {
    PackDeclared,
    AdmissionValidated,
    PolicyDecision,
    EntitlementChecked,
    ResourceReserved,
    ApprovalChecked,
    PlanningCompleted,
    RequestValidated,
    SummaryGenerated,
    ConversationSummarized,
    ContextCompressed,
    SummaryRefined,
    SummariesCompared,
    SummaryEvaluated,
    EvidenceInspected,
    ProviderInspected,
    ServiceCall,
    ProviderCallStarted,
    ProviderCallSucceeded,
    HealthReported,
    SnapshotRecorded,
    Unavailable,
    Failure,
}

/// Mock Strategy or explicit fail-closed Null Object selected by runtime-host.
pub struct SummarizationSystemServiceProvider {
    descriptor: ServiceDescriptor,
    events: broadcast::Sender<SummarizationRuntimeEvent>,
    references: RwLock<BTreeMap<String, String>>,
    unavailable_reason: Option<String>,
}

impl SummarizationSystemServiceProvider {
    /// Construct provider-neutral mock behavior for conformance tests.
    pub fn mock() -> Self {
        Self::new(None)
    }
    /// Construct unavailable behavior that never silently substitutes a model.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self::new(Some(reason.into()))
    }
    fn new(unavailable_reason: Option<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            descriptor: summarization_service_descriptor(),
            events,
            references: RwLock::new(BTreeMap::new()),
            unavailable_reason,
        }
    }
    /// Expose generic capability metadata without selecting a model or prompt.
    pub fn capability(&self) -> SummaryProviderCapability {
        self.capability_for_state(if self.unavailable_reason.is_some() {
            DomainPackProviderCapabilityState::Unavailable
        } else {
            DomainPackProviderCapabilityState::Preview
        })
    }

    /// Project a provider-neutral discovery state without selecting a provider implementation.
    ///
    /// Composition roots can report lifecycle and quota transitions through the
    /// common state vocabulary while preserving the same bounded capability shape.
    pub fn capability_for_state(
        &self,
        state: DomainPackProviderCapabilityState,
    ) -> SummaryProviderCapability {
        self.capability_with_diagnostics(state, false, None)
    }

    /// Report a bounded quota or lifecycle diagnostic alongside capability facts.
    pub fn capability_with_diagnostics(
        &self,
        state: DomainPackProviderCapabilityState,
        quota_limited: bool,
        diagnostic_code: Option<String>,
    ) -> SummaryProviderCapability {
        SummaryProviderCapability {
            provider_class: "mock".into(),
            modes: BTreeSet::from([
                "extractive".into(),
                "abstractive".into(),
                "hybrid".into(),
                "context_compression".into(),
            ]),
            source_kinds: BTreeSet::from([
                "document".into(),
                "retrieval".into(),
                "citation".into(),
                "graph".into(),
                "message".into(),
                "transcript".into(),
                "prior_summary".into(),
            ]),
            languages: BTreeSet::from(["und".into()]),
            max_sources: 32,
            max_output_tokens: 4096,
            supports_streaming: false,
            quota_limited,
            diagnostic_code,
            state,
        }
    }
    /// Subscribe to bounded events safe for audit and replay observers.
    pub fn subscribe(&self) -> broadcast::Receiver<SummarizationRuntimeEvent> {
        self.events.subscribe()
    }
    /// Save bounded Memento state for restart diagnostics without summary content.
    pub async fn snapshot(&self) -> BTreeMap<String, String> {
        let count = self.references.read().await.len().min(100);
        let _ = self.events.send(event(
            "summarization.snapshot",
            "snapshot:summarization-provider",
            SummarizationRuntimeEventKind::SnapshotRecorded,
        ));
        BTreeMap::from([
            ("descriptor_hash".into(), "summarization:descriptor".into()),
            ("provider_class".into(), "mock".into()),
            ("active_reference_count".into(), count.to_string()),
        ])
    }
}

#[async_trait]
impl SystemService for SummarizationSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }
    async fn start(&self) -> ServiceResult<()> {
        let _ = self.events.send(event(
            "summarization.declaration",
            "declaration:summarization-provider",
            SummarizationRuntimeEventKind::PackDeclared,
        ));
        info!(service_id = %self.descriptor.id, "summarization provider started");
        Ok(())
    }
    async fn call(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<macaca_proto::ServiceCallResult> {
        let trace = domain_pack_command_trace(&command)?;
        if let Some(reason) = &self.unavailable_reason {
            let _ = self.events.send(event(
                command.name.as_str(),
                &trace.trace_id,
                SummarizationRuntimeEventKind::Unavailable,
            ));
            warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason_code = %reason, "summarization provider unavailable");
            return Err(ServiceError::ServiceUnavailable(reason.clone()));
        }
        if !KNOWLEDGE_SUMMARIZATION_COMMANDS.contains(&command.name.as_str()) {
            let _ = self.events.send(event(
                command.name.as_str(),
                &trace.trace_id,
                SummarizationRuntimeEventKind::Failure,
            ));
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        let dependency_unavailable = (command.name.as_str()
            == "summarization.summarize_with_citations"
            && command
                .payload
                .get("citation_service_available")
                .and_then(serde_json::Value::as_bool)
                == Some(false))
            || (command.name.as_str() == "summarization.inspect_summary_evidence"
                && command
                    .payload
                    .get("evidence_service_available")
                    .and_then(serde_json::Value::as_bool)
                    == Some(false));
        if dependency_unavailable {
            let _ = self.events.send(event(
                command.name.as_str(),
                &trace.trace_id,
                SummarizationRuntimeEventKind::Unavailable,
            ));
            return Err(ServiceError::ServiceUnavailable(
                "summarization_declared_dependency_unavailable".into(),
            ));
        }
        let mode = command
            .payload
            .get("mode")
            .and_then(serde_json::Value::as_str);
        let strategy = SummarizationStrategyKind::for_command_and_mode(command.name.as_str(), mode)
            .expect("descriptor command must have a summarization strategy");
        let long_document_plan = (strategy == SummarizationStrategyKind::LongDocumentSynthesis)
            .then(|| LongDocumentExecutionPlan::for_trace(&trace.trace_id, 1));
        let reference = format!("summarization:reference:{}", trace.trace_id);
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
                .send(event(command.name.as_str(), &trace.trace_id, *kind));
        }
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "summarization provider call completed");
        Ok(domain_pack_service_result(
            serde_json::json!({"status":"ok", "summary_handle_ref":reference, "strategy":strategy.label(), "checkpoint_ref":checkpoint_ref(strategy, &trace.trace_id), "long_document_plan":long_document_plan, "provider_class":"mock", "result_metadata":"bounded:provider-owned"}),
            trace,
            "mock",
        ))
    }
    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "summarization provider stopped");
        Ok(())
    }
    async fn cleanup(&self) -> ServiceResult<()> {
        self.references.write().await.clear();
        info!(service_id = %self.descriptor.id, "summarization provider cleanup completed");
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
            "summarization.health",
            "health:summarization-provider",
            SummarizationRuntimeEventKind::HealthReported,
        ));
        Ok(health)
    }
}

/// Build the service descriptor solely from proto-owned contract constants.
pub fn summarization_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(KNOWLEDGE_SUMMARIZATION_SERVICE_ID),
        ServiceType::new("knowledge.summarization"),
        TraceSchemaRef::new("knowledge.summarization.replay.v1"),
    );
    descriptor
        .metadata
        .insert("pack_id".into(), KNOWLEDGE_SUMMARIZATION_PACK_ID.into());
    descriptor
        .metadata
        .insert("provider_class".into(), "mock".into());
    descriptor.metadata.insert(
        "command_count".into(),
        KNOWLEDGE_SUMMARIZATION_COMMANDS.len().to_string(),
    );
    descriptor
}
fn common_event_kinds() -> &'static [SummarizationRuntimeEventKind] {
    use SummarizationRuntimeEventKind::*;
    &[
        AdmissionValidated,
        PolicyDecision,
        EntitlementChecked,
        ResourceReserved,
        ApprovalChecked,
        ServiceCall,
        ProviderCallStarted,
        ProviderCallSucceeded,
    ]
}

fn event_kind(command: &str) -> SummarizationRuntimeEventKind {
    use SummarizationRuntimeEventKind::*;
    match command {
        "summarization.plan" => PlanningCompleted,
        "summarization.validate_request" => RequestValidated,
        "summarization.summarize"
        | "summarization.summarize_with_citations"
        | "summarization.summarize_many" => SummaryGenerated,
        "summarization.summarize_conversation" => ConversationSummarized,
        "summarization.compress_context" => ContextCompressed,
        "summarization.refine_summary" => SummaryRefined,
        "summarization.compare_summaries" => SummariesCompared,
        "summarization.evaluate_summary" => SummaryEvaluated,
        "summarization.inspect_summary_evidence" => EvidenceInspected,
        "summarization.inspect_provider" => ProviderInspected,
        _ => ServiceCall,
    }
}

fn event(
    command: &str,
    trace_id: &str,
    kind: SummarizationRuntimeEventKind,
) -> SummarizationRuntimeEvent {
    SummarizationRuntimeEvent {
        command: command.into(),
        trace_id: trace_id.into(),
        kind,
        replay_ref: format!("replay:{trace_id}"),
    }
}
