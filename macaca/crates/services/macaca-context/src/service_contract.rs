//! Provider-neutral Context service contract for Route C S5.
//!
//! Context composition is an orchestration capability: it selects providers,
//! applies budget/governance, optionally triggers active recall, and returns a
//! model-ready prompt plus diagnostics.  This contract keeps that orchestration
//! replaceable and prevents Web/CLI/framework code from constructing concrete
//! context engines directly.

use chrono::{DateTime, Utc};
use macaca_proto::{
    config::ContextConfig, ApplicationId, LlmMessage, LlmOptions, MacacaError, MacacaResult,
    TraceContext,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::{
    ActiveRecallCapability, ContextAssembleInput, ContextAssembleResult, ContextBudget,
    ContextEngineInfo, ContextEngineSelection, ContextReport, KnowledgeDigestCapability,
    McpCapabilityCatalog, RuntimeToolCapabilityCatalog, SkillCapabilityCatalog,
};

/// Stable service id used by runtime-host registration and SDK clients.
pub const CONTEXT_SERVICE_ID: &str = "service.context";

/// Command names accepted by the Context service provider adapter.
pub const CONTEXT_ASSEMBLE_COMMAND: &str = "context.assemble";
pub const CONTEXT_ACTIVE_RECALL_COMMAND: &str = "context.active_recall";
pub const CONTEXT_PROVIDER_INVENTORY_COMMAND: &str = "context.provider_inventory";
pub const CONTEXT_ENGINE_INVENTORY_COMMAND: &str = "context.engine_inventory";
pub const CONTEXT_SNAPSHOT_COMMAND: &str = "context.snapshot";

/// Explicit context operation scope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextServiceScope {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub agent_name: String,
}

impl ContextServiceScope {
    /// Build a validated context scope for trace and policy decisions.
    pub fn new(
        application_id: ApplicationId,
        session_id: impl Into<String>,
        agent_name: impl Into<String>,
    ) -> MacacaResult<Self> {
        let session_id = non_empty(session_id.into(), "Context service requires session_id")?;
        let agent_name = non_empty(agent_name.into(), "Context service requires agent_name")?;
        Ok(Self {
            application_id,
            session_id,
            agent_name,
        })
    }
}

/// Policy and budget hints interpreted by context strategies.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ContextPolicyHints {
    pub privacy_tier: Option<String>,
    pub active_recall_enabled: Option<bool>,
    pub provider_chain: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Provider-neutral snapshot used by a Context Service implementation to rebuild
/// the same provider chain a host previously assembled in-process.
///
/// The snapshot intentionally stores only serializable catalogs and filesystem
/// roots. Runtime-only capabilities such as vector recall and knowledge digest
/// remain Strategy objects injected into the service provider, so DTOs do not
/// become hidden handles to Web or memory backends.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextAssemblySnapshot {
    pub context_config: ContextConfig,
    pub agent_profile_root: Option<PathBuf>,
    pub skill_capability_catalog: SkillCapabilityCatalog,
    pub mcp_capability_catalog: McpCapabilityCatalog,
    pub runtime_tool_capability_catalog: RuntimeToolCapabilityCatalog,
    pub ready_mcp_server_ids: Vec<String>,
    pub factory_params: Value,
}

/// Command for model-context assembly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAssembleCommand {
    pub scope: ContextServiceScope,
    pub trace: TraceContext,
    pub model: String,
    pub base_messages: Vec<LlmMessage>,
    pub options: LlmOptions,
    pub budget: ContextBudget,
    pub selection: ContextEngineSelection,
    pub policy: ContextPolicyHints,
    pub assembly: Option<ContextAssemblySnapshot>,
}

impl ContextAssembleCommand {
    /// Build a command that can be passed through service runtime admission.
    pub fn new(
        scope: ContextServiceScope,
        trace: TraceContext,
        model: impl Into<String>,
        base_messages: Vec<LlmMessage>,
        options: LlmOptions,
        selection: ContextEngineSelection,
    ) -> MacacaResult<Self> {
        validate_trace(&trace, "Context assemble command requires trace_id")?;
        if base_messages.is_empty() {
            return Err(MacacaError::Config(
                "Context assemble command requires at least one base message".into(),
            ));
        }
        Ok(Self {
            scope,
            trace,
            model: model.into(),
            base_messages,
            options,
            budget: ContextBudget::default(),
            selection,
            policy: ContextPolicyHints::default(),
            assembly: None,
        })
    }

    /// Convert the service command into the existing engine input envelope.
    pub fn into_engine_input(self) -> ContextAssembleInput {
        ContextAssembleInput {
            app_id: Some(self.scope.application_id),
            session_id: Some(self.scope.session_id),
            agent_name: self.scope.agent_name,
            model: self.model,
            base_messages: self.base_messages,
            options: self.options,
            budget: self.budget,
        }
    }
}

/// Runtime-only context capabilities injected into the Context Service.
///
/// Keeping these handles outside [`ContextAssemblySnapshot`] preserves the
/// Strategy pattern: the same command DTO can drive different recall/digest
/// backends while the service provider chooses the implementation at boot.
#[derive(Clone, Default)]
pub struct ContextServiceRuntimeCapabilities {
    pub memory_recall: Option<std::sync::Arc<dyn ActiveRecallCapability>>,
    pub knowledge_digest: Option<std::sync::Arc<dyn KnowledgeDigestCapability>>,
}

/// Command for explicit active recall diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextActiveRecallCommand {
    pub scope: ContextServiceScope,
    pub trace: TraceContext,
    pub query: String,
    pub limit: usize,
    pub policy: ContextPolicyHints,
}

/// Command for provider inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextProviderInventoryCommand {
    pub scope: ContextServiceScope,
    pub trace: TraceContext,
}

/// Command for engine inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEngineInventoryCommand {
    pub scope: ContextServiceScope,
    pub trace: TraceContext,
}

/// Command for deterministic service snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextServiceSnapshotCommand {
    pub scope: ContextServiceScope,
    pub trace: TraceContext,
    pub include_provider_inventory: bool,
    pub include_engine_inventory: bool,
}

/// Result returned by context assembly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAssembleServiceResult {
    pub assembled: ContextAssembleResult,
    pub completed_at: DateTime<Utc>,
}

impl ContextAssembleServiceResult {
    /// Wrap an engine result with service timing metadata.
    pub fn new(assembled: ContextAssembleResult) -> Self {
        Self {
            assembled,
            completed_at: Utc::now(),
        }
    }
}

/// Bounded diagnostics for active recall orchestration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextActiveRecallServiceResult {
    pub status: String,
    pub selected_candidates: usize,
    pub diagnostics: Vec<String>,
}

/// Provider inventory row.  This records metadata, not provider payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextProviderInventoryItem {
    pub provider_id: String,
    pub family: String,
    pub healthy: bool,
}

/// Deterministic Context Service snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextServiceSnapshot {
    pub service_id: String,
    pub healthy: bool,
    pub providers: Vec<ContextProviderInventoryItem>,
    pub engines: Vec<ContextEngineInfo>,
    pub active_recall_available: bool,
    pub digest_available: bool,
    pub last_report_id: Option<String>,
    pub last_audit_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl ContextServiceSnapshot {
    /// Build a healthy snapshot from sanitized inventory data.
    pub fn healthy(engines: Vec<ContextEngineInfo>) -> Self {
        Self {
            service_id: CONTEXT_SERVICE_ID.into(),
            healthy: true,
            providers: Vec::new(),
            engines,
            active_recall_available: false,
            digest_available: false,
            last_report_id: None,
            last_audit_ids: Vec::new(),
            captured_at: Utc::now(),
        }
    }
}

/// Sanitized event emitted by Context Service adapters and clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextServiceEvent {
    pub event_name: String,
    pub service_id: String,
    pub scope: ContextServiceScope,
    pub trace_id: String,
    pub status: String,
    pub occurred_at: DateTime<Utc>,
    pub metadata: Value,
}

/// Result helper used by adapters that need to expose a report-only response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextReportResult {
    pub report: ContextReport,
    pub emitted_at: DateTime<Utc>,
}

fn validate_trace(trace: &TraceContext, message: &str) -> MacacaResult<()> {
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(())
}

fn non_empty(value: String, message: &str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(trimmed)
}
