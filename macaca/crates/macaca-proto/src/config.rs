use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::{MacacaError, MacacaResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacacaConfig {
    pub kernel: KernelConfig,
    pub llm: LlmConfig,
    pub memory: MemoryConfig,
    pub ipc: IpcConfig,
    pub persist: PersistConfig,
    pub gateway: GatewayConfig,
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub mcp: McpConfigSection,
}

/// MCP runtime configuration. Currently exposes a key/value `env` map that is
/// re-exported into the backend process environment during `start_server` so
/// every downstream MCP child process inherits the declared values
/// (stdio MCP clients rely on `tokio::process::Command` env inheritance).
///
/// Values follow the same "literal vs env-var-name" convention as LLM keys:
/// if the value looks like an `ALL_CAPS_WITH_UNDERSCORES` identifier it is
/// interpreted as the name of an existing environment variable to forward;
/// otherwise it is treated as a literal value and set verbatim.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfigSection {
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub root_dir: String,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            root_dir: "./data/workspaces".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    pub max_agents: usize,
    pub heartbeat_interval_ms: u64,
    pub agent_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub default_provider: String,
    pub default_model: Option<String>,
    pub max_tokens_per_request: u32,
    pub rate_limit_rpm: u32,
    pub providers: HashMap<String, LlmProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    /// Coding-plan / subscription key (e.g. MiniMax [Token Plan](https://platform.minimaxi.com/docs/token-plan/intro)).
    /// When non-empty after resolution, **takes precedence** over [`Self::api_key`] (pay-as-you-go).
    /// Value: raw key, or `ALL_CAPS` env var name (same rules as `api_key`).
    #[serde(default)]
    pub api_key_plan: Option<String>,
    /// Pay-as-you-go API key: raw `sk-…` string, or env var name if `ALL_CAPS_WITH_UNDERSCORES`
    /// (e.g. `OPENAI_API_KEY`). Used when `api_key_plan` is unset or empty.
    #[serde(default)]
    pub api_key: String,
    pub base_url: String,
    /// Default model for this provider (e.g. "" for DashScope, "gpt-4o" for OpenAI)
    #[serde(default)]
    pub default_model: Option<String>,
}

/// Resolve one key field: empty → `Ok("")`; `ALL_CAPS` → `std::env::var`; else literal.
fn resolve_llm_key_field(raw: &str) -> MacacaResult<String> {
    let v = raw.trim();
    if v.is_empty() {
        return Ok(String::new());
    }
    let is_env_var_name = v
        .chars()
        .all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit());
    if is_env_var_name {
        std::env::var(v).map_err(|_| MacacaError::Config(format!("{v} not set")))
    } else {
        Ok(v.to_string())
    }
}

fn resolve_llm_optional_key(opt: &Option<String>) -> MacacaResult<String> {
    match opt {
        None => Ok(String::new()),
        Some(s) => resolve_llm_key_field(s),
    }
}

impl LlmProviderConfig {
    /// Effective API key: **`api_key_plan` (coding plan) first**, then **`api_key` (按量)**.
    pub fn resolve_api_key(&self) -> MacacaResult<String> {
        let plan = resolve_llm_optional_key(&self.api_key_plan)?;
        if !plan.is_empty() {
            return Ok(plan);
        }
        resolve_llm_key_field(&self.api_key)
    }
}

#[cfg(test)]
mod llm_provider_config_tests {
    use super::*;

    #[test]
    fn resolve_api_key_prefers_api_key_plan() {
        let c = LlmProviderConfig {
            api_key_plan: Some("  sk-plan  ".into()),
            api_key: "SHOULD_NOT_READ".into(),
            base_url: "https://example.com/v1".into(),
            default_model: None,
        };
        assert_eq!(c.resolve_api_key().unwrap(), "sk-plan");
    }

    #[test]
    fn resolve_api_key_falls_back_to_api_key_paygo() {
        let c = LlmProviderConfig {
            api_key_plan: None,
            api_key: "sk-paygo-inline".into(),
            base_url: "https://example.com/v1".into(),
            default_model: None,
        };
        assert_eq!(c.resolve_api_key().unwrap(), "sk-paygo-inline");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub session_ttl_seconds: u64,
    pub file_store_path: String,
    pub auto_retrieve_on: String,
    pub vector: VectorConfig,
    pub embedding: EmbeddingConfig,
    pub compression: CompressionConfig,
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
    /// Raw API key or `ALL_CAPS` env var name (same rules as [`LlmProviderConfig::api_key`]).
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcConfig {
    pub nats_url: String,
    pub nats_auto_start: bool,
    pub reconnect_max_attempts: u32,
    pub reconnect_delay_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistConfig {
    pub engine: String,
    pub data_dir: String,
    pub snapshot_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    pub enabled: bool,
    pub telegram: Option<TelegramConfig>,
    pub discord: Option<DiscordConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token_env: String,
    pub allowed_user_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub enabled: bool,
    pub bot_token_env: String,
    pub command_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    pub log_level: String,
    pub tracing_enabled: bool,
    pub otlp_endpoint: String,
    /// File logging configuration
    #[serde(default)]
    pub log_file: LogFileConfig,
}

/// Configuration for file-based logging with rotation and compression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFileConfig {
    /// Enable file logging
    #[serde(default = "default_log_file_enabled")]
    pub enabled: bool,
    /// Directory to store log files
    #[serde(default = "default_log_dir")]
    pub dir: String,
    /// Log file name prefix
    #[serde(default = "default_log_prefix")]
    pub prefix: String,
    /// Log format: "json" or "text"
    #[serde(default = "default_log_format")]
    pub format: String,
    /// Maximum number of days to retain log files
    #[serde(default = "default_log_retention_days")]
    pub retention_days: u64,
    /// Compress old log files
    #[serde(default = "default_log_compress")]
    pub compress: bool,
}

fn default_log_file_enabled() -> bool {
    true
}
fn default_log_dir() -> String {
    "./logs".into()
}
fn default_log_prefix() -> String {
    "macaca".into()
}
fn default_log_format() -> String {
    "json".into()
}
fn default_log_retention_days() -> u64 {
    10
}
fn default_log_compress() -> bool {
    true
}

impl Default for LogFileConfig {
    fn default() -> Self {
        Self {
            enabled: default_log_file_enabled(),
            dir: default_log_dir(),
            prefix: default_log_prefix(),
            format: default_log_format(),
            retention_days: default_log_retention_days(),
            compress: default_log_compress(),
        }
    }
}

impl Default for MacacaConfig {
    fn default() -> Self {
        Self {
            kernel: KernelConfig {
                max_agents: 16,
                heartbeat_interval_ms: 5000,
                agent_timeout_ms: 30000,
            },
            llm: LlmConfig {
                default_provider: "anthropic".into(),
                default_model: None,
                max_tokens_per_request: 8192,
                rate_limit_rpm: 60,
                providers: HashMap::new(),
            },
            memory: MemoryConfig {
                session_ttl_seconds: 3600,
                file_store_path: "./data/memory/files".into(),
                auto_retrieve_on: "task_start".into(),
                vector: VectorConfig {
                    backend: "milvus".into(),
                    milvus_url: "http://localhost:19530".into(),
                    collection_name: "agent_memory".into(),
                },
                embedding: EmbeddingConfig {
                    provider: "dashscope".into(),
                    model: "text-embedding-v4".into(),
                    api_key: "DASHSCOPE_API_KEY".into(),
                    dimensions: 1024,
                    base_url: "https://dashscope.aliyuncs.com/api/v1/services/embeddings/text-embedding/text-embedding".into(),
                },
                compression: CompressionConfig {
                    enabled: true,
                    threshold_entries: 100,
                    strategy: "llm_summarize".into(),
                },
            },
            ipc: IpcConfig {
                nats_url: "nats://localhost:4222".into(),
                nats_auto_start: true,
                reconnect_max_attempts: 10,
                reconnect_delay_ms: 1000,
            },
            persist: PersistConfig {
                engine: "redb".into(),
                data_dir: "./data/persist".into(),
                snapshot_interval_seconds: 300,
            },
            gateway: GatewayConfig {
                enabled: true,
                telegram: Some(TelegramConfig {
                    enabled: true,
                    bot_token_env: "TELEGRAM_BOT_TOKEN".into(),
                    allowed_user_ids: Vec::new(),
                }),
                discord: Some(DiscordConfig {
                    enabled: true,
                    bot_token_env: "DISCORD_BOT_TOKEN".into(),
                    command_prefix: "!".into(),
                }),
            },
            observability: ObservabilityConfig {
                log_level: "info".into(),
                tracing_enabled: true,
                otlp_endpoint: String::new(),
                log_file: LogFileConfig::default(),
            },
            workspace: WorkspaceConfig::default(),
            mcp: McpConfigSection::default(),
        }
    }
}

impl MacacaConfig {
    /// Load config from a TOML file, with environment variable overrides.
    /// Env vars use the format: AOS_SECTION__KEY (double underscore for nesting).
    pub fn load(path: impl AsRef<Path>) -> MacacaResult<Self> {
        let builder = config::Config::builder()
            .add_source(config::File::from(path.as_ref()).required(false))
            .add_source(
                config::Environment::with_prefix("AOS")
                    .separator("__")
                    .try_parsing(true),
            )
            .build()
            .map_err(|e| MacacaError::Config(e.to_string()))?;

        builder
            .try_deserialize()
            .map_err(|e| MacacaError::Config(e.to_string()))
    }

    /// Load from default path (config/default.toml) or fall back to defaults.
    pub fn load_default() -> Self {
        Self::load("config/default.toml").unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let cfg = MacacaConfig::default();
        assert_eq!(cfg.kernel.max_agents, 16);
        assert_eq!(cfg.memory.embedding.model, "text-embedding-v4");
        assert_eq!(cfg.memory.vector.backend, "milvus");
        assert_eq!(cfg.memory.embedding.dimensions, 1024);
        assert!(cfg.gateway.telegram.unwrap().enabled);
    }

    #[test]
    fn load_nonexistent_falls_back_to_default() {
        let cfg = MacacaConfig::load_default();
        assert_eq!(cfg.persist.engine, "redb");
    }
}
