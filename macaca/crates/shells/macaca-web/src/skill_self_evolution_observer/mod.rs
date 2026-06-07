//! Observer bridge from Agent Execution completions into Skill self-evolution (Facade module).
//!
//! The Web shell does not own self-evolution semantics. It observes the generic
//! `service.agent_execution` result boundary, converts a verified terminal
//! success into bounded evidence references, and forwards a typed command through
//! the SDK Skill facade. The Skill service remains the owner of proposal
//! validation, policy, storage, curation, promotion, and rollback behavior.
//!
//! # Design patterns
//!
//! - **Facade** — stable `observe_agent_execution_result_for_skill_self_evolution` export.
//! - **Observer** — reacts to `AgentExecutionResult` without mutating execution outcome.
//! - **Adapter** — `projection` maps service DTOs to kernel `TaskResult` view.
//! - **Builder** — `proposal_builder` assembles bounded `SkillExperienceProposalCommand`.
//! - **Value Object** — `semantic_signal` derives Skill Creator-compatible trigger phrases.

mod forwarder;
mod observer;
mod projection;
mod proposal_builder;
mod semantic_signal;
mod types;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

pub(crate) use observer::observe_agent_execution_result_for_skill_self_evolution;
pub(crate) use types::SkillSelfEvolutionObservation;
