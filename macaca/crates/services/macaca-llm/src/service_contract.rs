//! Provider-neutral LLM service contract for Route C S5.
//!
//! The types in this module are the canonical DTOs that cross the LLM service
//! boundary.  They deliberately describe *what the caller wants* rather than
//! *which concrete provider should be used*.  Runtime-host adapters can map
//! these commands to `LlmProvider`, `LlmRouter`, remote services, or test
//! doubles without changing Web, CLI, framework, or SDK call sites.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use macaca_proto::{
    ApplicationId, LlmMessage, LlmOptions, LlmResponse, MacacaError, MacacaResult, TokenUsage,
    TraceContext,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Stable service id used by runtime-host registration and SDK clients.
pub const LLM_SERVICE_ID: &str = "service.llm";

/// Command names accepted by the LLM service provider adapter.
pub const LLM_CHAT_COMMAND: &str = "llm.chat";
pub const LLM_MODEL_SELECTION_COMMAND: &str = "llm.model.select";
pub const LLM_SNAPSHOT_COMMAND: &str = "llm.snapshot";

/// Provider-neutral request scope shared by all LLM service commands.
///
/// The scope is explicit so auditing, policy, and model routing never need to
/// infer the application/session/agent owner from arbitrary message metadata.
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
///
/// The service contract keeps these hints generic.  Runtime decorators and
/// provider strategies can interpret them later without adding provider-specific
/// fields to the public command shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LlmPolicyHints {
    pub max_prompt_tokens: Option<u32>,
    pub max_completion_tokens: Option<u32>,
    pub max_cost_micros: Option<u64>,
    pub privacy_tier: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Typed chat command accepted by the LLM service.
///
/// `model_hint` is intentionally a hint, not a provider selection.  The service
/// may pass it to a router strategy, apply policy, or use a configured default.
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
///
/// The snapshot is a Memento: it captures availability and inventory metadata
/// without serializing full prompts, completions, API keys, or provider URLs.
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
///
/// This trait intentionally exposes only the `llm.chat` command surface so
/// `macaca-framework` can depend on the LLM service boundary without pulling
/// in the full SDK facade (`SystemLlmClient`). The **Interface Segregation**
/// pattern prevents `sdk ↔ framework` cyclic Cargo edges while keeping chat
/// dispatch auditable through typed [`LlmChatCommand`] DTOs and
/// [`TraceContext`] propagation.
#[async_trait]
pub trait LlmServiceChatClient: Send + Sync {
    /// Dispatch one provider-neutral chat command through the LLM service.
    ///
    /// Implementations must preserve `command.trace` for downstream audit and
    /// must not mutate provider selection semantics — routing remains owned by
    /// the LLM service runtime, not by framework adapters.
    async fn chat(&self, command: LlmChatCommand) -> MacacaResult<LlmChatResult>;
}

fn non_empty(value: String, message: &str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(trimmed)
}
