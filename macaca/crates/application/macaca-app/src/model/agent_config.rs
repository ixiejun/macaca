//! Agent configuration and capability reference types for application manifests.
//!
//! These serde-friendly DTOs describe inline agent declarations and file-path
//! references. They are intentionally data-only: runtime services resolve models,
//! permissions, and tool visibility without embedding application-specific logic.

use serde::{Deserialize, Serialize};

/// Serde placeholder — manifests and `config/default.toml` supply the real model id.
pub(super) fn default_model() -> String {
    String::new()
}

/// Default permission tier when an inline agent omits `permission_level`.
pub(super) fn default_permission() -> String {
    "user".into()
}

/// Default LLM configuration declared by an application.
///
/// **Security note:** This does NOT contain API keys. Apps declare which
/// provider/model they prefer; the kernel resolves the actual credentials
/// from the user's configuration. Apps never see API keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLlmConfig {
    /// LLM provider name (e.g., "openai", "anthropic", "dashscope").
    pub provider: String,
    /// Model name (e.g., "gpt-4", "claude-sonnet-4-20250514").
    pub model: String,
}

/// A capability reference in inline config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRef {
    /// Capability name.
    pub name: String,
    /// Description.
    #[serde(default)]
    pub description: String,
}

/// Per-agent standard AgentSkills visibility policy in app manifests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentSkillsConfig {
    /// If set, only these skill names are visible to this agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow: Option<Vec<String>>,
    /// Skill names hidden from this agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
}

/// An inline agent configuration within an app manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineAgentConfig {
    /// Human-readable agent name.
    pub name: String,
    /// Capability names.
    #[serde(default)]
    pub capabilities: Vec<CapabilityRef>,
    /// System prompt.
    #[serde(default)]
    pub prompt_template: String,
    /// LLM model name.
    #[serde(default = "default_model")]
    pub model: String,
    /// Permission level.
    #[serde(default = "default_permission")]
    pub permission_level: String,
    /// Allowed tools.
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Max tokens.
    pub max_tokens: Option<u32>,
    /// Temperature.
    pub temperature: Option<f32>,
    /// Optional standard AgentSkills visibility policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skills: Option<AgentSkillsConfig>,
    /// Optional context engine override for this agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_engine: Option<String>,
}

/// Application-level context engine configuration block.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppContextConfig {
    /// Optional default context engine for this application.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    /// Optional fallback engine for this application.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_engine: Option<String>,
    /// Optional override for workspace guide files (priorities and per-file byte budgets).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_guides: Option<macaca_proto::config::WorkspaceGuideSourcesConfig>,
    /// Optional inject of Markdown profile candidates (`AGENTS.md`, etc.) through the composer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_profile: Option<macaca_proto::config::AgentProfileContextConfig>,
}

/// Source of agent configuration — either a file path or inline config.
///
/// Uses `#[serde(untagged)]` so YAML manifests can list file paths as plain
/// strings or inline agent objects without an explicit discriminator field.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentSource {
    /// Path to a YAML/TOML agent config file.
    FilePath(String),
    /// Inline agent configuration.
    Inline(InlineAgentConfig),
}
