//! Plan notebook — agent self-planning with sequential subtask execution (**Facade**).
//!
//! `PlanNotebook` lets an agent decompose a complex goal into ordered subtasks,
//! track progress through them, and receive hint messages guiding its next action.
//!
//! Invariant: at most one subtask can be `InProgress` at any time.
//!
//! # Module tree (P4 iteration 110)
//! - `types` — `SubTask`, `SubTaskState`, `PlanState` value objects
//! - `error` — `PlanError` taxonomy
//! - `aggregate` — `Plan` subtask state machine
//! - `notebook` — `PlanNotebook` manager + `StateModule` persistence
//! - `tests` — contract tests for transitions and hints

mod aggregate;
mod error;
mod notebook;
mod types;

#[cfg(test)]
mod tests;

pub use aggregate::Plan;
pub use error::PlanError;
pub use notebook::PlanNotebook;
pub use types::{PlanState, SubTask, SubTaskState};
