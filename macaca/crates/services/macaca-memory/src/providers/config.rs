use serde::{Deserialize, Serialize};

use super::slot::MemoryProviderComponentSlot;

/// Transport family used by a memory provider entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryProviderTransportKind {
    Builtin,
    Remote,
    Mcp,
}

impl Default for MemoryProviderTransportKind {
    fn default() -> Self {
        Self::Builtin
    }
}

/// Runtime resilience settings for external provider calls.
///
/// These knobs are intentionally small and generic so the fabric can support
/// future providers without hard-coding vendor-specific behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderResilienceConfig {
    #[serde(default = "default_provider_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_provider_payload_limit_bytes")]
    pub payload_limit_bytes: usize,
    #[serde(default = "default_provider_retry_count")]
    pub retry_count: u32,
    #[serde(default = "default_provider_circuit_threshold")]
    pub circuit_failure_threshold: u32,
    #[serde(default = "default_provider_circuit_cooldown_ms")]
    pub circuit_cooldown_ms: u64,
}

impl Default for MemoryProviderResilienceConfig {
    fn default() -> Self {
        Self {
            timeout_ms: default_provider_timeout_ms(),
            payload_limit_bytes: default_provider_payload_limit_bytes(),
            retry_count: default_provider_retry_count(),
            circuit_failure_threshold: default_provider_circuit_threshold(),
            circuit_cooldown_ms: default_provider_circuit_cooldown_ms(),
        }
    }
}

fn default_provider_timeout_ms() -> u64 {
    5_000
}

fn default_provider_payload_limit_bytes() -> usize {
    256 * 1024
}

fn default_provider_retry_count() -> u32 {
    1
}

fn default_provider_circuit_threshold() -> u32 {
    3
}

fn default_provider_circuit_cooldown_ms() -> u64 {
    30_000
}

/// Remote endpoint configuration used by the provider runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderEndpointConfig {
    pub url: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub timeout_ms: u64,
}

/// Configuration for one provider implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderConfig {
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub transport: MemoryProviderTransportKind,
    #[serde(default)]
    pub endpoint: Option<MemoryProviderEndpointConfig>,
    #[serde(default)]
    pub resilience: MemoryProviderResilienceConfig,
    #[serde(default)]
    pub tools: Vec<MemoryProviderToolConfig>,
    #[serde(default)]
    pub components: MemoryProviderComponentSlot,
}

/// Tool registration metadata for adapters that expose runnable operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderToolConfig {
    pub name: String,
    #[serde(default)]
    pub namespaced: bool,
}

/// Profile-level provider selection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderProfileConfig {
    pub agent_private_provider: Option<String>,
    pub session_shared_provider: Option<String>,
    pub embedding_provider: Option<String>,
    pub vector_backend: Option<String>,
    pub active_recall: Option<String>,
    pub knowledge_compiler: Option<String>,
}

/// Named profiles that can be selected at runtime.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderProfilesConfig {
    #[serde(default)]
    pub default_profile: String,
    #[serde(default)]
    pub profiles: std::collections::HashMap<String, MemoryProviderProfileConfig>,
    #[serde(default)]
    pub agents: std::collections::HashMap<String, MemoryProviderProfileConfig>,
    #[serde(default)]
    pub sessions: std::collections::HashMap<String, MemoryProviderProfileConfig>,
}

/// Root provider runtime configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderRuntimeConfig {
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub providers: std::collections::HashMap<String, MemoryProviderConfig>,
    #[serde(default)]
    pub profiles: MemoryProviderProfilesConfig,
    #[serde(default)]
    pub mcp: MemoryProviderMcpConfig,
}

/// MCP provider configuration for adapters that invoke tools instead of HTTP endpoints.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderMcpConfig {
    #[serde(default)]
    pub servers: std::collections::HashMap<String, MemoryProviderMcpServerConfig>,
}

/// A single MCP server entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderMcpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub timeout_ms: u64,
    #[serde(default)]
    pub trust_external: bool,
}

impl Default for MemoryProviderMcpServerConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            env: std::collections::HashMap::new(),
            timeout_ms: default_provider_timeout_ms(),
            trust_external: false,
        }
    }
}

/// Remote provider request envelope used by the `macaca-memory-v1` protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderRemoteRequestConfig {
    pub scope: String,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub timeout_ms: u64,
}

/// Remote provider response envelope used by the `macaca-memory-v1` protocol.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderRemoteResponseConfig {
    pub healthy: bool,
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}
