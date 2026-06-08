//! Runtime metadata and policy DTOs parsed from `SKILL.md` frontmatter.
//!
//! These structures gate skill visibility (env, OS, config) and describe optional
//! install/MCP attachments without encoding any application-specific routing.

use serde::{Deserialize, Serialize};

use super::model::AgentSkill;

/// Invocation policy parsed from standard AgentSkills frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillInvocationPolicy {
    /// Whether the skill can be exposed as a user command.
    pub user_invocable: bool,
    /// Whether the skill should be hidden from model prompt injection.
    pub disable_model_invocation: bool,
}

/// Prompt/command exposure computed for a skill entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillExposure {
    /// Include in runtime status/debug inventory.
    pub include_in_runtime_registry: bool,
    /// Include in `<available_skills>` prompt catalog.
    pub include_in_available_skills_prompt: bool,
    /// Expose as a user-invocable command when command routing exists.
    pub user_invocable: bool,
}

impl Default for SkillExposure {
    fn default() -> Self {
        Self {
            include_in_runtime_registry: true,
            include_in_available_skills_prompt: true,
            user_invocable: true,
        }
    }
}

impl Default for SkillInvocationPolicy {
    fn default() -> Self {
        Self {
            user_invocable: true,
            disable_model_invocation: false,
        }
    }
}

/// Runtime metadata used for gating and UI display.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillMetadata {
    pub always: bool,
    pub skill_key: Option<String>,
    pub primary_env: Option<String>,
    pub emoji: Option<String>,
    pub homepage: Option<String>,
    pub os: Vec<String>,
    pub requires_bins: Vec<String>,
    pub requires_any_bins: Vec<String>,
    pub requires_env: Vec<String>,
    pub requires_config: Vec<String>,
    pub install: Vec<SkillInstallSpec>,
    pub mcp_servers: Vec<SkillMcpServerConfig>,
}

/// Installer metadata from a standard skill manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillInstallSpec {
    pub id: Option<String>,
    pub kind: String,
    pub package: Option<String>,
    pub module: Option<String>,
    pub formula: Option<String>,
    pub bins: Vec<String>,
    pub label: Option<String>,
}

/// Provider-neutral MCP server declaration attached to a knowledge skill.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillMcpServerConfig {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub transport: String,
    pub tool_prefix: Option<String>,
}

/// A parsed skill plus runtime policies (composite for discovery/runtime pipeline).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
    pub skill: AgentSkill,
    pub metadata: SkillMetadata,
    pub invocation: SkillInvocationPolicy,
    pub exposure: SkillExposure,
}

/// Result of parsing the full `SKILL.md` frontmatter and body.
#[derive(Debug, Clone)]
pub struct ParsedSkillMd {
    pub name: String,
    pub description: String,
    pub body: String,
    pub metadata: SkillMetadata,
    pub invocation: SkillInvocationPolicy,
}
