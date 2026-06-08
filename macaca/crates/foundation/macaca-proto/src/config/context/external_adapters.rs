//! Declarative external context adapter installation rows.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

fn default_context_external_adapter_enabled() -> bool {
    true
}

fn default_external_adapter_transport() -> ContextExternalAdapterTransportKind {
    ContextExternalAdapterTransportKind::HttpJson
}

/// Transport kinds supported by the Phase 9 external adapter installer.
///
/// Additional transports can be added without changing the surrounding config shape: each row keeps
/// neutral engine metadata plus transport-specific endpoint settings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContextExternalAdapterTransportKind {
    /// POST [`macaca_context::ContextAssembleInput`] as JSON and expect
    /// [`macaca_context::ContextAssembleResult`] JSON in response.
    #[default]
    HttpJson,
}

/// Endpoint settings for the `http_json` external adapter transport.
///
/// Header values follow the same convention used elsewhere in config:
/// - literal strings are sent verbatim
/// - `ALL_CAPS_WITH_UNDERSCORES` values are treated as environment variable names
///   and resolved at startup
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextExternalAdapterHttpJsonConfig {
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

fn default_external_adapter_timeout_ms() -> u64 {
    2_000
}

fn default_external_adapter_max_payload_bytes() -> usize {
    256 * 1024
}

fn default_external_adapter_require_schema_validation() -> bool {
    true
}

fn default_external_adapter_require_budget_validation() -> bool {
    true
}

fn default_external_adapter_circuit_breaker_failures() -> u32 {
    3
}

/// Runtime safety guardrails mapped into `macaca-context` external adapter validation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextExternalAdapterSafetyConfig {
    #[serde(default = "default_external_adapter_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_external_adapter_max_payload_bytes")]
    pub max_payload_bytes: usize,
    #[serde(default = "default_external_adapter_require_schema_validation")]
    pub require_schema_validation: bool,
    #[serde(default = "default_external_adapter_require_budget_validation")]
    pub require_budget_validation: bool,
    #[serde(default = "default_external_adapter_circuit_breaker_failures")]
    pub circuit_breaker_failures: u32,
}

impl Default for ContextExternalAdapterSafetyConfig {
    fn default() -> Self {
        Self {
            timeout_ms: default_external_adapter_timeout_ms(),
            max_payload_bytes: default_external_adapter_max_payload_bytes(),
            require_schema_validation: default_external_adapter_require_schema_validation(),
            require_budget_validation: default_external_adapter_require_budget_validation(),
            circuit_breaker_failures: default_external_adapter_circuit_breaker_failures(),
        }
    }
}

fn default_external_adapter_fallback_engine_id() -> String {
    "legacy".into()
}

fn default_external_adapter_empty_contribution_fallback() -> bool {
    true
}

/// Fallback behavior used when an external adapter times out, errors, or emits an invalid result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextExternalAdapterFallbackConfig {
    #[serde(default = "default_external_adapter_fallback_engine_id")]
    pub fallback_engine_id: String,
    #[serde(default = "default_external_adapter_empty_contribution_fallback")]
    pub empty_external_contribution: bool,
}

impl Default for ContextExternalAdapterFallbackConfig {
    fn default() -> Self {
        Self {
            fallback_engine_id: default_external_adapter_fallback_engine_id(),
            empty_external_contribution: default_external_adapter_empty_contribution_fallback(),
        }
    }
}

/// One declarative external adapter engine installation row.
///
/// The `id` becomes the selectable `ContextEngineInfo.id`. Operators can choose it through
/// `context.default_engine`, app manifest context, or agent-level overrides once the runtime
/// installs the row successfully.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextExternalAdapterConfig {
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default = "default_context_external_adapter_enabled")]
    pub enabled: bool,
    #[serde(default = "default_external_adapter_transport")]
    pub transport: ContextExternalAdapterTransportKind,
    #[serde(default)]
    pub http_json: Option<ContextExternalAdapterHttpJsonConfig>,
    #[serde(default)]
    pub safety: ContextExternalAdapterSafetyConfig,
    #[serde(default)]
    pub fallback: ContextExternalAdapterFallbackConfig,
}
