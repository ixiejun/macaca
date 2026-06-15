//! `aos-cli` — command-line interface for Agent OS.
//!
//! Provides the library backing the `aos` binary, including
//! command implementations for managing the kernel and agents.

pub mod command_handlers;
pub mod commands;
pub mod logging;
pub mod skill_operations;
pub mod tool_operations;
pub mod workbench_operations;

pub use commands::{
    execute_list_agents, execute_list_plugins, execute_run_kernel, execute_show_status,
};
