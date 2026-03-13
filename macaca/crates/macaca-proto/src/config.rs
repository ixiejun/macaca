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
    pub max_tokens_per_request: u32,
    pub rate_limit_rpm: u32,
    pub providers: HashMap<String, LlmProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    /// API key — can be a raw key (e.g. `sk-xxx`) or an env var name (e.g. `OPENAI_API_KEY`).
    pub api_key_env: String,
    pub base_url: String,
}

impl LlmProviderConfig {
    /// Resolve the API key: if the value looks like a raw key (contains `-` or
    /// starts with `sk`), use it directly; otherwise treat as an env var name.
    pub fn resolve_api_key(&self) -> MacacaResult<String> {
        let v = &self.api_key_env;
        if v.is_empty() {
            return Ok(String::new());
        }
        // Heuristic: raw keys typically contain lowercase, dashes, or start with sk/key prefix.
        // Env var names are ALL_UPPER_WITH_UNDERSCORES.
        let is_env_var_name = !v.is_empty()
            && v.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit());
        if is_env_var_name {
            std::env::var(v).map_err(|_| MacacaError::Config(format!("{v} not set")))
        } else {
            Ok(v.clone())
        }
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
    pub api_key_env: String,
    pub dimensions: usize,
    /// Base URL for the embedding API endpoint.
    #[serde(default)]
    pub base_url: String,
}

impl EmbeddingConfig {
    /// Resolve the API key (same heuristic as LlmProviderConfig).
    pub fn resolve_api_key(&self) -> MacacaResult<String> {
        let v = &self.api_key_env;
        if v.is_empty() {
            return Ok(String::new());
        }
        let is_env_var_name = !v.is_empty()
            && v.chars().all(|c| c.is_ascii_uppercase() || c == '_' || c.is_ascii_digit());
        if is_env_var_name {
            std::env::var(v).map_err(|_| MacacaError::Config(format!("{v} not set")))
        } else {
            Ok(v.clone())
        }
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
                    api_key_env: "DASHSCOPE_API_KEY".into(),
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
            },
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
