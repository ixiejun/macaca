//! ReActAgent — Reasoning-Action loop implementation (**Facade**).
//!
//! The ReActAgent drives the core Think → Act → Observe cycle:
//! 1. **Reasoning**: call the LLM with the current memory context
//! 2. **Acting**: if the LLM requests tool calls, execute each one and store results
//! 3. Repeat until the LLM produces a plain text reply or `max_iters` is exceeded
//!
//! # Module tree (P4 iteration 111)
//! - `core` — `ReActAgent` struct and builder API
//! - `loop` — reasoning / acting / summarize loop steps
//! - `agent_trait` — `Agent` trait implementation
//! - `helpers` — response-to-memory conversion
//! - `tests` — contract tests for loop and context assembly

mod agent_trait;
mod core;
mod helpers;
mod r#loop;

#[cfg(test)]
mod tests;

pub use core::ReActAgent;
