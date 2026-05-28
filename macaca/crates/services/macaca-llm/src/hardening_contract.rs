//! Hardened `service.llm` command contracts.
//!
//! This module contains provider-neutral DTOs for catalog reads, route
//! diagnostics, budget visibility, degradation explanations, and continuation
//! protocol validation.  The contracts intentionally describe capabilities and
//! requirements instead of naming concrete providers, models, or application
//! workflows, so runtime-host providers can be swapped without changing SDK,
//! shell, or application code.

use chrono::{DateTime, Utc};
use macaca_proto::{LlmMessage, TraceContext};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::service_contract::{LlmPolicyHints, LlmRouteSummary, LlmServiceScope};

/// Command names added by the Codex-class LLM hardening work.
pub const LLM_MODEL_LIST_COMMAND: &str = "model.list";
pub const LLM_PROVIDER_CAPABILITIES_READ_COMMAND: &str = "model.provider.capabilities.read";
pub const LLM_ROUTE_RESOLVE_COMMAND: &str = "model.route.resolve";
pub const LLM_CONTINUATION_VALIDATE_COMMAND: &str = "llm.continuation.validate";
pub const LLM_BUDGET_STATUS_COMMAND: &str = "llm.budget.status";
pub const LLM_DEGRADATION_EXPLAIN_COMMAND: &str = "llm.degradation.explain";

/// Redacted provider protocol metadata used by validation and diagnostics.
///
/// The flags form an executable Specification: callers can ask the service to
/// validate a transcript before dispatch, and the service can reject provider
/// protocol mismatches with structured diagnostics instead of leaking an opaque
/// provider error back into an agent loop.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmProviderProtocolMetadata {
    pub supports_reasoning: bool,
    pub requires_reasoning_continuation: bool,
    pub supports_tool_calls: bool,
    pub supports_tool_results: bool,
    pub supports_service_tiers: bool,
    pub retry_policy: LlmRetryPolicy,
}

impl Default for LlmProviderProtocolMetadata {
    fn default() -> Self {
        Self {
            supports_reasoning: false,
            requires_reasoning_continuation: false,
            supports_tool_calls: true,
            supports_tool_results: true,
            supports_service_tiers: false,
            retry_policy: LlmRetryPolicy::default(),
        }
    }
}

/// Provider-neutral retry policy summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmRetryPolicy {
    pub max_attempts: u8,
    pub retryable_failure_codes: Vec<String>,
    pub backoff_base_ms: u64,
}

impl Default for LlmRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            retryable_failure_codes: Vec::new(),
            backoff_base_ms: 0,
        }
    }
}

/// One model row visible through the service catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmModelCatalogEntry {
    pub provider_id: String,
    pub model: String,
    pub display_name: String,
    pub service_tiers: Vec<String>,
    pub reasoning_efforts: Vec<String>,
    pub protocol: LlmProviderProtocolMetadata,
}

/// Provider capability row returned to policy and diagnostics callers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmProviderCapabilityRecord {
    pub provider_id: String,
    pub healthy: bool,
    pub default_model: Option<String>,
    pub capabilities: Vec<String>,
    pub protocol: LlmProviderProtocolMetadata,
}

/// Generic request for catalog-style reads.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCatalogReadCommand {
    pub scope: LlmServiceScope,
    pub trace: TraceContext,
    pub include_disabled: bool,
}

/// Generic model catalog response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmModelCatalogResult {
    pub models: Vec<LlmModelCatalogEntry>,
    pub captured_at: DateTime<Utc>,
}

/// Provider capability response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmProviderCapabilitiesResult {
    pub providers: Vec<LlmProviderCapabilityRecord>,
    pub captured_at: DateTime<Utc>,
}

/// Route resolution command that supersedes the older model-selection dry run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRouteResolveCommand {
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

/// Route resolution response with bounded diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmRouteResolveResult {
    pub selected: LlmRouteSummary,
    pub diagnostics: Vec<LlmDiagnostic>,
}

/// Continuation validation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmContinuationValidateCommand {
    pub scope: LlmServiceScope,
    pub trace: TraceContext,
    pub route: Option<LlmRouteSummary>,
    pub protocol: LlmProviderProtocolMetadata,
    pub messages: Vec<LlmMessage>,
}

/// Continuation validation result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmContinuationValidateResult {
    pub accepted: bool,
    pub diagnostics: Vec<LlmDiagnostic>,
}

/// Budget status command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmBudgetStatusCommand {
    pub scope: LlmServiceScope,
    pub trace: TraceContext,
    pub policy: LlmPolicyHints,
}

/// Budget status response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmBudgetStatusResult {
    pub status: String,
    pub max_prompt_tokens: Option<u32>,
    pub max_completion_tokens: Option<u32>,
    pub max_cost_micros: Option<u64>,
    pub remaining_cost_micros: Option<u64>,
    pub diagnostics: Vec<LlmDiagnostic>,
}

/// Degradation explanation command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmDegradationExplainCommand {
    pub scope: LlmServiceScope,
    pub trace: TraceContext,
    pub route: Option<LlmRouteSummary>,
    pub failure_code: Option<String>,
    pub policy: LlmPolicyHints,
}

/// Degradation explanation response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmDegradationExplainResult {
    pub degraded: bool,
    pub reason_code: String,
    pub retryable: bool,
    pub diagnostics: Vec<LlmDiagnostic>,
}

/// Structured diagnostic emitted by hardened LLM commands.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmDiagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub metadata: BTreeMap<String, String>,
}

impl LlmDiagnostic {
    /// Build a bounded diagnostic with no raw prompt or provider payload.
    pub fn new(
        code: impl Into<String>,
        severity: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity: severity.into(),
            message: message.into(),
            metadata: BTreeMap::new(),
        }
    }

    /// Attach sanitized key/value metadata for audit correlation.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Sanitized service response used by generic command handlers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmHardenedServiceResponse {
    pub status: String,
    pub diagnostics: Vec<LlmDiagnostic>,
    pub payload: Value,
}
