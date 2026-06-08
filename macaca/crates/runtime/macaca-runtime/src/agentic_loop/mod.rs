//! Agentic loop: LLM ↔ tool iteration until stop or max iterations.
//!
//! Split from monolithic `agentic_loop.rs` (P5 iteration 89) using a Facade module tree.
//!
//! # Design patterns
//! - **Facade**: `mod.rs` re-exports preserve `macaca_runtime::agentic_loop::*`.
//! - **Template Method**: [`iteration`] defines one LLM+tool round-trip.
//! - **Observer**: event-sink variants emit [`AgentExecutionEvent`] for audit/trace.
//! - **Decorator**: [`pausable`] wraps the core loop with suspend/resume gates.
//! - **Strategy**: context engine selection is config-driven, not application-specific.

mod execute;
mod helpers;
mod iteration;
mod pausable;
mod types;

#[cfg(test)]
mod tests;

pub use types::{AgenticLoop, LoopResult, RuntimeConfig};
