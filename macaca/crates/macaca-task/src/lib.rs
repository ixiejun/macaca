//! aos-task: task decomposition, scheduling, and lifecycle.

pub mod decompose;
pub mod queue;
pub mod tracker;

pub use decompose::{SimpleDecomposer, TaskDecomposer};
pub use queue::TaskQueue;
pub use tracker::TaskTracker;
