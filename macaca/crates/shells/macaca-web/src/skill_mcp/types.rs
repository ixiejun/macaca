//! Serializable DTOs for skill-backed MCP probe and registration status.

use serde::Serialize;

/// Resolved stdio launch plan for one skill-declared MCP server.
#[derive(Debug, Clone)]
pub(super) struct SkillMcpServerLaunch {
    pub skill_name: String,
    pub server_id: String,
    pub command: String,
    pub args: Vec<String>,
}

/// Operator-facing status row for one skill-backed MCP server.
#[derive(Debug, Clone, Serialize)]
pub struct SkillMcpStatus {
    pub skill: String,
    pub server_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub state: SkillMcpStatusState,
    pub exposed_tools: Vec<String>,
    pub failure_reason: Option<String>,
}

/// Lifecycle state returned by dependency probes (no tool bodies).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillMcpStatusState {
    Ready,
    Failed,
    DependencyMissing,
}
