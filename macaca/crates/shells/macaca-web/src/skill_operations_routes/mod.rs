//! Skill governance and curation operations route adapters (Facade module).
//!
//! The Web shell exposes a bounded operator snapshot, but all governance,
//! curation, alias, and proposal semantics remain owned by `service.skill`.
//! This module is therefore an **Adapter** facade: submodules build typed SDK
//! commands, attach trace/scope metadata, log sanitized counts, and serialize
//! service DTOs without reading skill files or classifying lifecycle state locally.
//!
//! # Design patterns
//!
//! - **Facade** — `mod.rs` re-exports handlers so `bootstrap.rs` keeps stable paths.
//! - **Adapter** — HTTP JSON ↔ SDK command DTOs in handler modules.
//! - **Strategy** — lifecycle action parsing and policy hint assembly in [`adapter`].
//!
//! `bootstrap.rs` registers handlers via `skill_operations_routes::` paths.

mod adapter;
mod curation_handlers;
mod evaluation_handlers;
mod lifecycle_handlers;
mod materialization_handlers;
mod proposal_handlers;
mod snapshot_handlers;
mod types;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

pub use curation_handlers::{
    post_skill_curation_apply, post_skill_curation_rollback, post_skill_curation_run,
};
pub use evaluation_handlers::post_skill_evaluation_report;
pub use lifecycle_handlers::post_skill_lifecycle_operation;
pub use materialization_handlers::post_skill_materialization_operator_run;
pub use proposal_handlers::{post_skill_proposal_promote, post_skill_proposal_reject};
pub use snapshot_handlers::get_skill_operations;
pub use types::{SkillEvaluationReportRequest, SkillOperatorCommandRequest};
