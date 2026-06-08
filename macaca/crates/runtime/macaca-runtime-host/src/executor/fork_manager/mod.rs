//! Fork Manager — facade module (P5 allowlist convergence).
//!
//! Manages lifecycle of forked (child) agents in the Fork-Join delegation model.
//! Runtime-host owns orchestration; durability flows through `KernelPersistencePort`
//! without binding to a concrete persistence provider.
//!
//! ## Design patterns
//!
//! - **Facade** — this module re-exports the public fork API unchanged.
//! - **State** — [`ForkContext`] encodes fork lifecycle memento.
//! - **Observer** — [`HookEvent`] broadcast notifies coordinators and SSE adapters.
//! - **Specification** — [`ForkManager::validate_result`] checks acceptance criteria.
//! - **Memento** — optional `KernelPersistencePort` restore after restart.
//!
//! ## Module layout
//!
//! | File | Responsibility |
//! |------|----------------|
//! | `types.rs` | Value objects + hook event taxonomy |
//! | `manager.rs` | `ForkManager` struct + constructors |
//! | `lifecycle.rs` | Create/suspend/resume/complete state transitions |
//! | `validation_merge.rs` | Acceptance validation + parent merge |
//! | `persistence.rs` | Store restore/cleanup helpers |
//! | `hooks_queries.rs` | Hook subscription + fork queries |
//! | `tests.rs` | Unit tests |

mod hooks_queries;
mod lifecycle;
mod manager;
mod persistence;
mod types;
mod validation_merge;

#[cfg(test)]
mod tests;

pub use types::{
    DelegateResult, ForkContext, HookCallback, HookEvent, MergeResult,
};
pub use manager::ForkManager;
