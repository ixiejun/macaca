//! Toolkit construction and tool policy for framework-based agents (Facade module).
//!
//! This module follows the Facade + Adapter pattern used by [`crate::routes`] and
//! [`crate::loop_manager`]. `builder` orchestrates service-backed tool registration;
//! `policy` resolves capability-driven allowlists; `mcp_bridge` adapts MCP service
//! snapshots into auditable runtime events; `agent_tools` registers todo/task tools;
//! `workspace_tools` scopes file/shell primitives to the application workspace root.
//!
//! `FrameworkRunner` consumes only [`build_toolkit`] from this facade, keeping agent
//! construction decoupled from contributor implementation details.

mod agent_tools;
mod builder;
mod mcp_bridge;
mod policy;
mod workspace_tools;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

pub(crate) use builder::build_toolkit;
