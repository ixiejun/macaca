//! Provider-neutral dispatch strategies used by the autonomy supervisor.
//!
//! Scheduler owns durable run state, but it must not execute target commands
//! directly.  This module keeps dispatch as runtime-host strategy code that
//! routes through existing service boundaries and records only safe status
//! classes.  Unsupported target categories are explicit skipped outcomes rather
//! than panics or fake success.
//!
//! **Pattern:** Facade module tree — `strategies` owns the Command Router entry,
//! handler modules own per-target syscall surfaces, and `outcome` encodes safe
//! run-control value objects for Scheduler transitions.
//!
//! Module layout:
//! - `outcome.rs` — Value Object for dispatch result classes
//! - `strategies.rs` — Strategy struct + `dispatch` Command Router
//! - `agent_execution_dispatch.rs` — scheduled agent execution Strategy + Memento resolver
//! - `service_dispatch.rs` — generic service and heartbeat Adapter handlers
//! - `tests.rs` — Contract tests at service boundaries

mod agent_execution_dispatch;
mod outcome;
mod service_dispatch;
mod strategies;

#[cfg(test)]
mod tests;

pub use outcome::AutonomyDispatchOutcome;
pub use strategies::AutonomyDispatchStrategies;
