//! Agent Skill — `SKILL.md` knowledge skills per the agentskills.io spec.
//!
//! Split from monolithic `agent_skill.rs` (P5 iteration 86) using a Facade module tree.
//! Knowledge skills provide instructions injected into agent context (progressive
//! disclosure), distinct from executable YAML skills in [`SkillDefinition`](crate::SkillDefinition).
//!
//! # Design patterns
//! - **Facade**: `mod.rs` re-exports preserve `macaca_skill::agent_skill::*`.
//! - **Value Object**: `AgentSkill`, metadata DTOs, `SkillSourceScope` precedence.
//! - **Builder/Parser**: `parser` module assembles `ParsedSkillMd` from frontmatter.
//! - **Lazy Loading**: tier-1 metadata vs tier-2 `ActivatedSkill` body activation.

mod metadata;
mod model;
mod parser;
mod scope;

#[cfg(test)]
mod tests;

pub use metadata::{
    ParsedSkillMd, SkillEntry, SkillExposure, SkillInstallSpec, SkillInvocationPolicy,
    SkillMcpServerConfig, SkillMetadata,
};
pub use model::{ActivatedSkill, AgentSkill};
pub use parser::{extract_body, parse_skill_md, parse_skill_md_full};
pub use scope::SkillSourceScope;
