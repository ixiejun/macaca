//! Source scope ordering for duplicate skill name resolution.
//!
//! Lower numeric discriminant means higher precedence when the same skill name
//! appears in multiple discovery roots (workspace overrides application, etc.).

use serde::{Deserialize, Serialize};

/// Source scope for an AgentSkills-format knowledge skill.
///
/// Lower priority value means higher precedence when duplicate names exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillSourceScope {
    /// Session/application workspace `skills/`.
    Workspace = 0,
    /// Project/application `.agents/skills/`.
    ProjectAgents = 1,
    /// Application `skills/`.
    Application = 2,
    /// Macaca central store `~/.macaca/skills/`.
    MacacaCentral = 3,
    /// Generic user skill store `~/.agent/skills/`.
    UserAgentGeneric = 4,
    /// Client-specific user stores such as `~/.claude/skills/`.
    UserClient = 5,
    /// Bundled skills shipped with Macaca.
    Bundled = 6,
    /// Extra configured directories.
    Extra = 7,
}

impl SkillSourceScope {
    /// Stable source label used in diagnostics and trace payloads.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::ProjectAgents => "project_agents",
            Self::Application => "application",
            Self::MacacaCentral => "macaca_central",
            Self::UserAgentGeneric => "user_agent_generic",
            Self::UserClient => "user_client",
            Self::Bundled => "bundled",
            Self::Extra => "extra",
        }
    }
}

impl Default for SkillSourceScope {
    fn default() -> Self {
        Self::Extra
    }
}
