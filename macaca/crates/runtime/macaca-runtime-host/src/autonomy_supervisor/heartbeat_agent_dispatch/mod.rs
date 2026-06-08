//! Manifest-declared heartbeat agent dispatch — facade module (P5 allowlist convergence).
//!
//! Runtime-host is the approved composition root for this bridge: Heartbeat
//! owns wake acceptance, Application Service owns manifest projection, and
//! Agent Execution owns the actual model/tool run. This Strategy connects
//! those services with typed commands and sanitized logs without making
//! Scheduler, Web routes, or filesystem scanning own heartbeat execution.
//!
//! ## Design patterns
//!
//! - **Strategy** — [`HeartbeatAgentDispatchStrategy`] encapsulates dispatch
//!   algorithm; timeout and runtime are injectable collaborators.
//! - **Observer** — [`HeartbeatAgentDispatchStrategy::record_completion`] records
//!   terminal observations back through the Heartbeat service (Memento owner).
//! - **Specification** — declaration matching and evidence gate act as
//!   composable predicates before agent execution.
//! - **Value Object** — [`HeartbeatAgentDispatchSummary`] is an immutable
//!   bounded dispatch outcome for logs and lane bookkeeping.
//!
//! ## Module layout
//!
//! | File | Responsibility |
//! |------|----------------|
//! | `summary.rs` | Dispatch outcome value object |
//! | `support.rs` | Wake metadata extraction + declaration matching |
//! | `strategy.rs` | Strategy impl + service-bus orchestration |
//! | `tests/` | Contract tests (fixtures + scenarios) |

mod strategy;
mod summary;
mod support;

#[cfg(test)]
mod tests;

pub(crate) use summary::HeartbeatAgentDispatchSummary;
pub(crate) use strategy::HeartbeatAgentDispatchStrategy;
