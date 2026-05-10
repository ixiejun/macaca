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

pub mod adapter;
pub mod agent_skill;
pub mod catalog;
pub mod definition;
pub mod discovery;
pub mod encrypted_package;
pub mod facade;
pub mod handle;
pub mod package;
pub mod policy;
pub mod provisioner;
pub mod registry;
pub mod request;
pub mod runtime;
pub mod service_adapter;
pub mod service_contract;
pub mod snapshot;
pub mod source;
pub mod tool;

// Executable skills (YAML).
pub use adapter::{LocalSkillRuntimeProxy, SkillRuntimeProxy, SkillToolAdapter};
pub use definition::{SkillDefinition, SkillEntryPoint};
pub use facade::{
    load_executable_skill_definitions, ExecutableSkillToolSet, SkillCatalogSourceView,
    SkillRuntimeFacade,
};
pub use handle::{SkillRuntimeHandle, SkillRuntimeState};
pub use package::{agent_skill_package_descriptor, skill_entry_package_descriptor};
pub use policy::{PolicyDecision, SkillExposureContext, SkillExposurePolicy, SkillPolicyChain};
pub use registry::SkillRegistry;
pub use request::{SkillSnapshotRequest, SkillSnapshotRequestBuilder};
pub use snapshot::SkillRegistrySnapshot;
pub use source::{SkillSource, SkillSourceSet};
pub use tool::SkillTool;

// Agent Skills (SKILL.md / agentskills.io).
pub use agent_skill::{
    ActivatedSkill, AgentSkill, SkillEntry, SkillExposure, SkillInstallSpec, SkillInvocationPolicy,
    SkillMcpServerConfig, SkillMetadata, SkillSourceScope,
};
pub use catalog::{CatalogEntry, SkillCatalog};
pub use discovery::{DiscoveredSkill, SkillScope};
pub use encrypted_package::{
    encrypted_package_metadata, DecryptedPackage, EncryptedPackageAuthorizer,
    EncryptedPackageDecryptor, EncryptedPackageLoader, EncryptedPackageMetadata,
};
pub use provisioner::{ClientConfig, SkillProvisioner};
pub use runtime::{
    path_belongs_to_snapshot_skill, FilteredSkill, SkillPolicy, SkillRuntime, SkillRuntimeLimits,
    SkillRuntimeOptions, SkillSnapshot, SkillSnapshotEntry,
};
pub use service_adapter::skill_service_descriptor;
pub use service_contract::*;
