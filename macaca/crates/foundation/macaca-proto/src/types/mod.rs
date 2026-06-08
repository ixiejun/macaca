//! Domain model types — facade module re-exporting focused submodules.
//!
//! Split from monolithic `types.rs` (P5 iteration 82) to satisfy the file-size gate
//! while preserving the public `macaca_proto::types::*` surface unchanged.

mod agent;
mod execution;
mod identity;
mod llm;
mod memory;
mod messaging;
mod planning;
mod task;
mod todo;

#[cfg(test)]
mod tests;

pub use agent::*;
pub use execution::*;
pub use identity::*;
pub use llm::*;
pub use memory::*;
pub use messaging::*;
pub use planning::*;
pub use task::*;
pub use todo::*;
