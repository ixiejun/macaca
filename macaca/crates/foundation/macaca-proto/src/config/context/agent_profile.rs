//! Agent profile Markdown resolution tunables for context assembly.

use serde::{Deserialize, Serialize};

/// Where to resolve agent profile Markdown files (`AGENTS.md`, `SOUL.md`, etc.).
///
/// The OS layer stays application-agnostic: resolution uses only configured paths and
/// well-known filenames — never hard-coded business agent names.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentProfileRootKind {
    /// Application install: `{app_dir}/personas/{agent_name}` (same tree as `IDENTITY.md` / `TOOLS.md`).
    #[default]
    PersonaDirectory,
    /// Data workspace private sandbox: `{data_dir}/workspaces/{app_id}/agents/{agent_name}`.
    AgentPrivateWorkspace,
}

fn default_agent_profile_max_file_bytes() -> u64 {
    2 * 1024 * 1024
}

fn default_agent_profile_inject_heartbeat() -> bool {
    true
}

fn default_agent_profile_include_memory_seed() -> bool {
    true
}

/// Tunables for [`super::ContextConfig::agent_profile`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentProfileContextConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub root_kind: AgentProfileRootKind,
    /// Per-file byte budget before UTF-8 truncation/skip diagnostics.
    #[serde(default = "default_agent_profile_max_file_bytes")]
    pub max_file_bytes: u64,
    /// When false, `HEARTBEAT.md` is never read by the profile provider (operators may gate cadence elsewhere).
    #[serde(default = "default_agent_profile_inject_heartbeat")]
    pub inject_heartbeat: bool,
    /// When false, `MEMORY.md` is omitted from candidates (seed/audit injection disabled for this agent).
    #[serde(default = "default_agent_profile_include_memory_seed")]
    pub include_memory_seed: bool,
    /// When > 0, reject profile bodies exceeding this **line** count after frontmatter stripping.
    /// `0` disables the check (default).
    #[serde(default)]
    pub profile_max_content_lines: u32,
}

impl Default for AgentProfileContextConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            root_kind: AgentProfileRootKind::default(),
            max_file_bytes: default_agent_profile_max_file_bytes(),
            inject_heartbeat: default_agent_profile_inject_heartbeat(),
            include_memory_seed: default_agent_profile_include_memory_seed(),
            profile_max_content_lines: 0,
        }
    }
}
