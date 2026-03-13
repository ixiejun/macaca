//! `aos-driver-claude-code` — Claude Code CLI driver for Agent OS.
//!
//! A user-installable plugin driver that controls Claude Code via its CLI,
//! enabling agents to perform autonomous programming tasks.

pub mod config;
pub mod driver;
pub mod output;
pub mod tools;

pub use config::{ClaudeCodeConfig, PermissionMode};
pub use driver::ClaudeCodeDriver;
