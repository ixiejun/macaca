//! CLI adapters for Skill governance and curation operations (Facade module).
//!
//! The CLI is a **presentation shell**, not a provider composition root.  These
//! commands build provider-neutral SDK DTOs, optionally reach a live runtime
//! through the public Web HTTP facade, and print bounded JSON summaries.  They
//! do not read skill packages, classify curation actions, or implement lifecycle
//! policy locally.
//!
//! # Design patterns
//!
//! - **Facade** — `mod.rs` re-exports public types and `execute_*` entrypoints so
//!   `command_handlers.rs` keeps stable import paths.
//! - **Adapter** — [`live_client`] translates CLI operator refs into bounded HTTP
//!   payloads against the Web Skill operations API.
//! - **Null Object** — [`UnavailableSystemSkillClient`] path when no live app id is
//!   supplied, preserving diagnostic unavailable semantics without faking success.
//! - **Strategy** — [`types::SkillCliLifecycleAction`] maps CLI verbs to SDK and
//!   HTTP route segments without duplicating policy in the shell.
//!
//! # Traceability
//!
//! Every command path emits `tracing` at forwarding boundaries with
//! `trace_id` / `app_id` / `command` dimensions for audit correlation.

mod curation;
mod lifecycle;
mod live_client;
mod output;
mod proposals;
mod snapshot;
mod support;
mod types;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

pub use curation::{execute_skill_curation_run, execute_skill_rollback};
pub use lifecycle::execute_skill_lifecycle;
pub use proposals::execute_skill_proposal_decision;
pub use snapshot::execute_skill_operations_snapshot;
pub use types::{
    SkillCliEvidenceRefs, SkillCliLifecycleAction, SkillCliRuntimeTarget,
};
