//! `aos-runtime` — Agentic execution runtime for Agent OS.
//!
//! Provides the core agentic loop that enables agents to iteratively
//! call LLMs, execute tools, and accumulate results until a task is complete.

pub mod agentic_loop;
pub mod context_window;
pub mod loop_detector;
pub mod permission;

pub use agentic_loop::{AgenticLoop, LoopResult, RuntimeConfig};
pub use context_window::{ContextWindowConfig, ContextWindowManager};
pub use loop_detector::{LoopDetector, LoopDetectorAction, LoopDetectorConfig};
pub use permission::{DefaultPermissionChecker, PermissionChecker};
