//! Web-layer memory recall **Strategy** adapters (Facade module).
//!
//! Connects the Memory Service to the context-engine report contract via
//! request-only injection paths. Callers pass a focused SDK memory client and
//! scope so concrete memory providers remain swappable without changing chat
//! orchestration.
//!
//! # Design patterns
//!
//! - **Facade** — re-exports `apply_active_recall` and `apply_preflight_memory`.
//! - **Strategy** — two injectors (active recall vs preflight `memory_search`).
//! - **Adapter** — [`adapter`] maps memory rows to context reports and message slots.
//!
//! ## Composer migration
//!
//! Prefer [`macaca_context::MemoryActiveRecallContextProvider`] when
//! `active_vector_memory` is enabled. Pass `composer_recall_active = true` from
//! [`crate::context_reporting_model::ContextReportingChatModel::composer_handles_active_vector_recall`]
//! so active recall here is skipped (avoids duplicate retrieval).

mod active_recall;
mod adapter;
mod preflight_recall;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

pub(crate) use active_recall::apply_active_recall;
pub(crate) use preflight_recall::apply_preflight_memory;
