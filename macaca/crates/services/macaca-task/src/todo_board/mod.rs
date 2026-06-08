//! TaskBoard (per-agent panel) and TaskSpace (per-application workspace).
//!
//! Split from monolithic `todo_board.rs` (P5 iteration 88) using a Facade module tree.
//! - Each agent has its own [`TaskBoard`] (isolated from other agents).
//! - Plan-side orchestration accesses [`TaskSpace`] which spans all boards in a session.
//!
//! # Design patterns
//! - **Facade**: `mod.rs` re-exports preserve `macaca_task::todo_board::*`.
//! - **Strategy**: lifecycle + dependency policies are injectable per board/space.
//! - **State**: explicit todo status transitions on claim/start/review paths.
//! - **Specification**: [`detect_cycles`] guards dependency graphs before admission.
//! - **Value Object**: [`ProgressSummary`] aggregates board-wide counters for observers.

mod cycle_detection;
mod task_board;
mod task_space;

#[cfg(test)]
mod tests;

pub use cycle_detection::detect_cycles;
pub use task_board::TaskBoard;
pub use task_space::{ProgressSummary, TaskSpace};
