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
//! specification. Stored in `~/.macaca/skills/` and provisioned to matching
//! clients (Claude Code, Cursor, etc.). Managed by [`SkillCatalog`] with
//! progressive disclosure (catalog → instructions → resources).
//!
//! Agent OS acts as both a skill store and a skills-aware client.

pub mod adapter;
pub mod agent_skill;
pub mod alias;
pub mod catalog;
pub mod curation;
pub(crate) mod curation_policy;
pub mod definition;
pub mod discovery;
pub mod encrypted_package;
pub mod evaluation;
pub mod evolution;
pub mod facade;
pub mod governance;
pub mod governance_store;
pub mod handle;
pub mod lifecycle;
pub mod lifecycle_state_machine;
pub mod merge;
pub mod merge_eligibility;
pub mod mutation;
pub mod operator_lifecycle;
pub mod ownership_policy;
pub mod package;
pub mod policy;
pub mod proposal_lifecycle;
pub mod proposal_materialization;
pub mod proposal_materialization_operator;
pub mod proposal_processing;
pub mod provenance;
pub mod provisioner;
pub mod registry;
pub mod request;
pub mod runtime;
pub mod semantic_review;
pub mod service_adapter;
pub mod service_contract;
pub mod snapshot;
pub mod source;
pub mod telemetry;
pub mod tool;

#[cfg(test)]
mod curation_tests;

#[cfg(test)]
mod governance_lifecycle_tests;

#[cfg(test)]
mod governance_audit_replay_tests;

#[cfg(test)]
mod merge_tests;

#[cfg(test)]
mod merge_eligibility_tests;

#[cfg(test)]
mod semantic_review_tests;

#[cfg(test)]
mod ownership_policy_tests;

#[cfg(test)]
mod evaluation_tests;

// Executable skills (YAML).
pub use adapter::{LocalSkillRuntimeProxy, SkillRuntimeProxy, SkillToolAdapter};
pub use alias::*;
pub use definition::{SkillDefinition, SkillEntryPoint};
pub use facade::{
    load_executable_skill_definitions, ExecutableSkillToolSet, SkillCatalogSourceView,
    SkillRuntimeFacade,
};
pub use governance::*;
pub use governance_store::*;
pub use handle::{SkillRuntimeHandle, SkillRuntimeState};
pub use lifecycle::*;
pub use lifecycle_state_machine::*;
pub use merge::*;
pub use merge_eligibility::*;
pub use mutation::*;
pub use operator_lifecycle::*;
pub use ownership_policy::*;
pub use package::{agent_skill_package_descriptor, skill_entry_package_descriptor};
pub use policy::{PolicyDecision, SkillExposureContext, SkillExposurePolicy, SkillPolicyChain};
pub use proposal_lifecycle::*;
pub use proposal_materialization::*;
pub use proposal_materialization_operator::*;
pub use proposal_processing::*;
pub use registry::SkillRegistry;
pub use request::{SkillSnapshotRequest, SkillSnapshotRequestBuilder};
pub use snapshot::SkillRegistrySnapshot;
pub use source::{SkillSource, SkillSourceSet};
pub use telemetry::*;
pub use tool::SkillTool;

// Agent Skills (SKILL.md / agentskills.io).
pub use agent_skill::{
    ActivatedSkill, AgentSkill, SkillEntry, SkillExposure, SkillInstallSpec, SkillInvocationPolicy,
    SkillMcpServerConfig, SkillMetadata, SkillSourceScope,
};
pub use catalog::{CatalogEntry, SkillCatalog};
pub use curation::*;
pub use discovery::{DiscoveredSkill, SkillScope};
pub use encrypted_package::{
    encrypted_package_metadata, DecryptedPackage, EncryptedPackageAuthorizer,
    EncryptedPackageDecryptor, EncryptedPackageLoader, EncryptedPackageMetadata,
};
pub use evaluation::*;
pub use evolution::*;
pub use provenance::*;
pub use provisioner::{ClientConfig, SkillProvisioner};
pub use runtime::{
    path_belongs_to_snapshot_skill, FilteredSkill, SkillPolicy, SkillRuntime, SkillRuntimeLimits,
    SkillRuntimeOptions, SkillSnapshot, SkillSnapshotEntry,
};
pub use semantic_review::*;
pub use service_adapter::skill_service_descriptor;
pub use service_contract::*;
