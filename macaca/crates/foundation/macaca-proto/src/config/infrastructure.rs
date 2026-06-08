//! Cross-cutting infrastructure: IPC, persistence, gateway, and observability.

use serde::{Deserialize, Serialize};

/// NATS IPC connection settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcConfig {
    pub nats_url: String,
    pub nats_auto_start: bool,
    pub reconnect_max_attempts: u32,
    pub reconnect_delay_ms: u64,
}

/// Embedded persistence engine selection and snapshot cadence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistConfig {
    pub engine: String,
    pub data_dir: String,
    pub snapshot_interval_seconds: u64,
}

/// Optional chat gateway integrations (Telegram, Discord).
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

/// Logging, tracing, OTLP export, and rotated file log settings.
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
