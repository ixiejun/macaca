//! `aos-cli` — command-line interface for Agent OS.
//!
//! Provides the library backing the `aos` binary, including
//! command implementations for managing the kernel and agents.

#![deny(deprecated)]

pub mod command_handlers;
pub mod commands;
pub mod logging;

#[allow(deprecated)]
pub use commands::{create_kernel, list_agents, run_kernel, show_status};
pub use commands::{
    create_kernel_with_stub_provider, execute_list_agents, execute_list_plugins,
    execute_run_kernel, execute_show_status,
};
