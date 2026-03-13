//! `aos-runtime` — Agentic execution runtime for Agent OS.
//!
//! Provides the core agentic loop that enables agents to iteratively
//! call LLMs, execute tools, and accumulate results until a task is complete.

pub mod agentic_loop;
pub mod permission;

pub use agentic_loop::{AgenticLoop, RuntimeConfig, LoopResult};
pub use permission::{PermissionChecker, DefaultPermissionChecker};
