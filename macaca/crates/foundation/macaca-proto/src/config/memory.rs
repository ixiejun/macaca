//! Memory fabric configuration: vector store, embedding, compression, and provider runtime.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::MacacaResult;

use super::llm::resolve_llm_key_field;

/// Session TTL, file store, vector backend, embedding, and provider-runtime wiring.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub session_ttl_seconds: u64,
    pub file_store_path: String,
    pub auto_retrieve_on: String,
    pub vector: VectorConfig,
    pub embedding: EmbeddingConfig,
    pub compression: CompressionConfig,
    #[serde(default)]
    pub provider_runtime: MemoryProviderRuntimeConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderEndpointConfig {
    pub url: String,
    #[serde(default)]
    pub api_key_env: Option<String>,
    #[serde(default)]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryProviderTransportKind {
    #[default]
    Builtin,
    Remote,
    Mcp,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderResilienceConfig {
    #[serde(default = "default_memory_provider_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_memory_provider_payload_limit_bytes")]
    pub payload_limit_bytes: usize,
    #[serde(default = "default_memory_provider_retry_count")]
    pub retry_count: u32,
    #[serde(default = "default_memory_provider_circuit_threshold")]
    pub circuit_failure_threshold: u32,
    #[serde(default = "default_memory_provider_circuit_cooldown_ms")]
    pub circuit_cooldown_ms: u64,
}

fn default_memory_provider_timeout_ms() -> u64 {
    5_000
}

fn default_memory_provider_payload_limit_bytes() -> usize {
    256 * 1024
}

fn default_memory_provider_retry_count() -> u32 {
    1
}

fn default_memory_provider_circuit_threshold() -> u32 {
    3
}

fn default_memory_provider_circuit_cooldown_ms() -> u64 {
    30_000
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderToolConfig {
    pub name: String,
    #[serde(default)]
    pub namespaced: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderComponentSlotConfig {
    #[serde(default)]
    pub agent_private_provider: Option<String>,
    #[serde(default)]
    pub session_shared_provider: Option<String>,
    #[serde(default)]
    pub embedding_provider: Option<String>,
    #[serde(default)]
    pub vector_backend: Option<String>,
    #[serde(default)]
    pub active_recall: Option<String>,
    #[serde(default)]
    pub knowledge_compiler: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub components: MemoryProviderComponentSlotConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderProfileConfig {
    #[serde(default)]
    pub agent_private_provider: Option<String>,
    #[serde(default)]
    pub session_shared_provider: Option<String>,
    #[serde(default)]
    pub embedding_provider: Option<String>,
    #[serde(default)]
    pub vector_backend: Option<String>,
    #[serde(default)]
    pub active_recall: Option<String>,
    #[serde(default)]
    pub knowledge_compiler: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderProfilesConfig {
    #[serde(default)]
    pub default_profile: String,
    #[serde(default)]
    pub profiles: HashMap<String, MemoryProviderProfileConfig>,
    #[serde(default)]
    pub agents: HashMap<String, MemoryProviderProfileConfig>,
    #[serde(default)]
    pub sessions: HashMap<String, MemoryProviderProfileConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderRuntimeConfig {
    #[serde(default)]
    pub profile: String,
    #[serde(default)]
    pub providers: HashMap<String, MemoryProviderConfig>,
    #[serde(default)]
    pub profiles: MemoryProviderProfilesConfig,
    #[serde(default)]
    pub mcp: MemoryProviderMcpConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderMcpConfig {
    #[serde(default)]
    pub servers: HashMap<String, MemoryProviderMcpServerConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryProviderMcpServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub timeout_ms: u64,
    #[serde(default)]
    pub trust_external: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorConfig {
    pub backend: String,
    pub milvus_url: String,
    pub collection_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: String,
    pub model: String,
    /// Raw API key or `ALL_CAPS` env var name (same rules as [`super::LlmProviderConfig::api_key`]).
    pub api_key: String,
    pub dimensions: usize,
    /// Base URL for the embedding API endpoint.
    #[serde(default)]
    pub base_url: String,
}

impl EmbeddingConfig {
    pub fn resolve_api_key(&self) -> MacacaResult<String> {
        resolve_llm_key_field(&self.api_key)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub threshold_entries: usize,
    pub strategy: String,
}
