//! `aos-skill` — Skill management for Agent OS.
//!
//! This crate handles two distinct skill concepts:
//!
//! ## Executable Skills (YAML)
//! Declared in `.yaml`/`.yml` files, backed by shell commands, MCP servers,
//! or scripts. Managed by [`SkillRegistry`] and wrapped as tools via [`SkillTool`].
//!
//! ## Agent Skills (SKILL.md)
//! Knowledge/instruction skills per the [agentskills.io](https://agentskills.io)
//! specification. Stored in `~/.macaca/skills/` and provisioned to compatible
//! clients (Claude Code, Cursor, etc.). Managed by [`SkillCatalog`] with
//! progressive disclosure (catalog → instructions → resources).
//!
//! Agent OS acts as both a skill store and a skills-aware client.

pub mod agent_skill;
pub mod catalog;
pub mod definition;
pub mod discovery;
pub mod provisioner;
pub mod registry;
pub mod tool;

// Executable skills (YAML).
pub use definition::{SkillDefinition, SkillEntryPoint};
pub use registry::SkillRegistry;
pub use tool::SkillTool;

// Agent Skills (SKILL.md / agentskills.io).
pub use agent_skill::{AgentSkill, ActivatedSkill};
pub use catalog::{CatalogEntry, SkillCatalog};
pub use discovery::{DiscoveredSkill, SkillScope};
pub use provisioner::{ClientConfig, SkillProvisioner};
