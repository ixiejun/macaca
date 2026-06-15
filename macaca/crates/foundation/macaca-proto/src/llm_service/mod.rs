//! Provider-neutral LLM service contract.
//!
//! These DTOs are the canonical protocol boundary for `service.llm`. SDK
//! clients can build and inspect LLM commands without importing the concrete
//! LLM provider crate, while runtime-host providers can map the same commands
//! to local, remote, plugin, or unavailable strategies.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ApplicationId, CapabilityId, CleanupPolicy, KernelServiceId, LlmMessage, LlmOptions,
    LlmResponse, MacacaError, MacacaResult, ServiceCapability, ServiceDescriptor, ServiceHealth,
    ServiceScope, ServiceType, TokenUsage, TraceContext, TraceSchemaRef,
};

pub mod hardening;

pub use hardening::*;

/// Stable service id used by runtime-host registration and SDK clients.
pub const LLM_SERVICE_ID: &str = "service.llm";

/// Command names accepted by the LLM service provider adapter.
pub const LLM_CHAT_COMMAND: &str = "llm.chat";
pub const LLM_MODEL_SELECTION_COMMAND: &str = "llm.model.select";
pub const LLM_SNAPSHOT_COMMAND: &str = "llm.snapshot";

/// Provider-neutral request scope shared by all LLM service commands.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmServiceScope {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub agent_name: String,
}

impl LlmServiceScope {
    /// Build a validated scope for one model operation.
    pub fn new(
        application_id: ApplicationId,
        session_id: impl Into<String>,
        agent_name: impl Into<String>,
    ) -> MacacaResult<Self> {
        let session_id = non_empty(session_id.into(), "LLM service requires session_id")?;
        let agent_name = non_empty(agent_name.into(), "LLM service requires agent_name")?;
        Ok(Self {
            application_id,
            session_id,
            agent_name,
        })
    }
}

/// Optional budget and policy hints attached to an LLM command.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LlmPolicyHints {
    pub max_prompt_tokens: Option<u32>,
    pub max_completion_tokens: Option<u32>,
    pub max_cost_micros: Option<u64>,
    pub privacy_tier: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Typed chat command accepted by the LLM service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChatCommand {
    pub scope: LlmServiceScope,
    pub trace: TraceContext,
    pub messages: Vec<LlmMessage>,
    pub options: LlmOptions,
    pub model_hint: Option<String>,
    pub policy: LlmPolicyHints,
}

impl LlmChatCommand {
    /// Construct a chat command and reject untraceable or unscoped calls early.
    pub fn new(
        scope: LlmServiceScope,
        trace: TraceContext,
        messages: Vec<LlmMessage>,
        options: LlmOptions,
    ) -> MacacaResult<Self> {
        if trace.trace_id.trim().is_empty() {
            return Err(MacacaError::Config(
                "LLM chat command requires trace_id".into(),
            ));
        }
        if messages.is_empty() {
            return Err(MacacaError::Llm(
                "LLM chat command requires at least one message".into(),
            ));
        }
        Ok(Self {
            scope,
            trace,
            messages,
            options,
            model_hint: None,
            policy: LlmPolicyHints::default(),
        })
    }

    /// Attach a provider-neutral model hint before the command enters routing.
    pub fn model_hint(mut self, model_hint: impl Into<String>) -> Self {
        self.model_hint = Some(model_hint.into());
        self
    }

    /// Attach policy hints evaluated by runtime decorators or provider strategy.
    pub fn policy(mut self, policy: LlmPolicyHints) -> Self {
        self.policy = policy;
        self
    }
}

/// Typed model-selection command used by status and dry-run callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmModelSelectionCommand {
    pub scope: LlmServiceScope,
    pub trace: TraceContext,
    pub request_model: Option<String>,
    pub agent_model: Option<String>,
    pub app_model: Option<String>,
    pub app_provider: Option<String>,
    pub system_model: Option<String>,
    pub fallbacks: Vec<String>,
    pub policy: LlmPolicyHints,
}

/// Command for deterministic service snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmServiceSnapshotCommand {
    pub scope: LlmServiceScope,
    pub trace: TraceContext,
    pub include_inventory: bool,
}

/// Provider-neutral summary of the selected model route.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmRouteSummary {
    pub provider_id: String,
    pub model: String,
    pub source: String,
    pub fallbacks: Vec<String>,
}

/// Result returned by a successful chat operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChatResult {
    pub response: LlmResponse,
    pub route: Option<LlmRouteSummary>,
    pub usage: TokenUsage,
    pub completed_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl LlmChatResult {
    /// Wrap a provider response with bounded routing and timing metadata.
    pub fn new(response: LlmResponse, route: Option<LlmRouteSummary>) -> Self {
        Self {
            usage: response.usage,
            response,
            route,
            completed_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Result returned by a dry-run model-selection request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmModelSelectionResult {
    pub selected: LlmRouteSummary,
}

/// Inventory row for one provider exposed by a service snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmProviderInventoryItem {
    pub provider_id: String,
    pub healthy: bool,
    pub default_model: Option<String>,
    pub capabilities: Vec<String>,
}

/// Deterministic LLM service snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmServiceSnapshot {
    pub service_id: String,
    pub healthy: bool,
    pub providers: Vec<LlmProviderInventoryItem>,
    pub default_model: Option<String>,
    pub last_audit_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl LlmServiceSnapshot {
    /// Build a minimal healthy snapshot when only a generic provider is known.
    pub fn healthy(provider_id: impl Into<String>, default_model: Option<String>) -> Self {
        Self {
            service_id: LLM_SERVICE_ID.into(),
            healthy: true,
            providers: vec![LlmProviderInventoryItem {
                provider_id: provider_id.into(),
                healthy: true,
                default_model: default_model.clone(),
                capabilities: vec!["chat".into(), "model_selection".into()],
            }],
            default_model,
            last_audit_ids: Vec::new(),
            captured_at: Utc::now(),
        }
    }

    /// Build a structured unavailable snapshot for unconfigured runtimes.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            service_id: LLM_SERVICE_ID.into(),
            healthy: false,
            providers: vec![LlmProviderInventoryItem {
                provider_id: "unavailable".into(),
                healthy: false,
                default_model: None,
                capabilities: vec![reason.into()],
            }],
            default_model: None,
            last_audit_ids: Vec::new(),
            captured_at: Utc::now(),
        }
    }
}

/// Structured event emitted by LLM service adapters and clients.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmServiceEvent {
    pub event_name: String,
    pub service_id: String,
    pub scope: LlmServiceScope,
    pub trace_id: String,
    pub status: String,
    pub occurred_at: DateTime<Utc>,
    pub metadata: Value,
}

impl LlmServiceEvent {
    /// Create a sanitized event that can be stored in trace/audit logs.
    pub fn new(
        event_name: impl Into<String>,
        scope: LlmServiceScope,
        trace: &TraceContext,
        status: impl Into<String>,
        metadata: Value,
    ) -> Self {
        Self {
            event_name: event_name.into(),
            service_id: LLM_SERVICE_ID.into(),
            scope,
            trace_id: trace.trace_id.clone(),
            status: status.into(),
            occurred_at: Utc::now(),
            metadata,
        }
    }
}

/// Narrow chat port shared across framework adapters and service clients.
#[async_trait]
pub trait LlmServiceChatClient: Send + Sync {
    /// Dispatch one provider-neutral chat command through the LLM service.
    async fn chat(&self, command: LlmChatCommand) -> MacacaResult<LlmChatResult>;
}

/// Build the provider-neutral descriptor for the LLM system service.
pub fn llm_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(LLM_SERVICE_ID),
        ServiceType::new("llm"),
        TraceSchemaRef::new("trace.system_service.llm.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            CapabilityId::new("capability.llm.chat"),
            "Runs model-backed conversational completion through the LLM service boundary.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.llm.model_selection"),
            "Resolves provider-neutral model routing metadata without exposing provider secrets.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.llm.snapshot"),
            "Emits sanitized LLM service inventory and health snapshots.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.llm.hardening"),
            "Exposes model catalog, provider protocol, continuation validation, budget, and degradation diagnostics.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![ServiceScope::Global, ServiceScope::Application("*".into())];
    descriptor.required_permissions = vec!["llm.call".into()];
    descriptor
        .metadata
        .insert("command.chat".into(), LLM_CHAT_COMMAND.into());
    descriptor.metadata.insert(
        "command.model_selection".into(),
        LLM_MODEL_SELECTION_COMMAND.into(),
    );
    descriptor
        .metadata
        .insert("command.snapshot".into(), LLM_SNAPSHOT_COMMAND.into());
    descriptor
        .metadata
        .insert("command.model_list".into(), LLM_MODEL_LIST_COMMAND.into());
    descriptor.metadata.insert(
        "command.provider_capabilities_read".into(),
        LLM_PROVIDER_CAPABILITIES_READ_COMMAND.into(),
    );
    descriptor.metadata.insert(
        "command.route_resolve".into(),
        LLM_ROUTE_RESOLVE_COMMAND.into(),
    );
    descriptor.metadata.insert(
        "command.continuation_validate".into(),
        LLM_CONTINUATION_VALIDATE_COMMAND.into(),
    );
    descriptor.metadata.insert(
        "command.budget_status".into(),
        LLM_BUDGET_STATUS_COMMAND.into(),
    );
    descriptor.metadata.insert(
        "command.degradation_explain".into(),
        LLM_DEGRADATION_EXPLAIN_COMMAND.into(),
    );
    descriptor.cleanup_policy = CleanupPolicy::None;
    descriptor
}

fn non_empty(value: String, message: &str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(trimmed)
}
