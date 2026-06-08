//! Root [`MacacaConfig`] aggregate, defaults, and TOML/environment loading.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{MacacaError, MacacaResult};

use super::{
    autonomy::AutonomyConfig, context::ContextConfig, drivers::DriversConfig,
    infrastructure::{
        DiscordConfig, GatewayConfig, IpcConfig, LogFileConfig, ObservabilityConfig, PersistConfig,
        TelegramConfig,
    },
    kernel::KernelConfig, llm::LlmConfig, memory::{
        CompressionConfig, EmbeddingConfig, MemoryConfig, MemoryProviderRuntimeConfig, VectorConfig,
    },
    mcp::McpConfigSection, workspace::WorkspaceConfig,
};

/// Top-level Macaca OS configuration deserialized from TOML and environment overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacacaConfig {
    pub kernel: KernelConfig,
    pub llm: LlmConfig,
    #[serde(default)]
    pub autonomy: AutonomyConfig,
    #[serde(default)]
    pub context: ContextConfig,
    pub memory: MemoryConfig,
    pub ipc: IpcConfig,
    pub persist: PersistConfig,
    pub gateway: GatewayConfig,
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub mcp: McpConfigSection,
    #[serde(default)]
    pub drivers: DriversConfig,
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
                // Provider/model routing resolves from config/default.toml, not struct defaults.
                default_provider: String::new(),
                default_model: None,
                max_tokens_per_request: 8192,
                rate_limit_rpm: 60,
                providers: HashMap::new(),
            },
            autonomy: AutonomyConfig::default(),
            context: ContextConfig::default(),
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
                    provider: String::new(),
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
                provider_runtime: MemoryProviderRuntimeConfig::default(),
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
            drivers: DriversConfig::default(),
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
