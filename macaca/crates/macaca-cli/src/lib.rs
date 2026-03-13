//! `aos-cli` — command-line interface for Agent OS.
//!
//! Provides the library backing the `aos` binary, including
//! command implementations for managing the kernel and agents.

pub mod commands;

pub use commands::{create_kernel, list_agents, run_kernel, show_status};
