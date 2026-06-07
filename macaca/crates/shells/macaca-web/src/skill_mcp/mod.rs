//! Skill-backed MCP runtime integration (Facade module).
//!
//! Standard AgentSkills provide instructions. Some skills also describe MCP
//! servers that provide the executable tools. This module bridges eligible,
//! visible skill snapshots into framework toolkit tools.
//!
//! # Design patterns
//!
//! - **Facade** — stable exports for routes and `framework_toolkit`.
//! - **Cache-Aside** — per-session skill snapshot persistence (`snapshot`).
//! - **Strategy** — explicit server config vs bundled mapping registry (`server_resolution`).
//! - **Adapter** — probe/register paths delegate to runtime-host MCP factory (`probe`).

mod events;
mod governance_telemetry;
mod probe;
mod server_resolution;
mod snapshot;
mod types;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

pub use types::{SkillMcpStatus, SkillMcpStatusState};

pub(crate) use probe::{probe_skill_mcp_servers, register_skill_backed_mcp_tools};
pub(crate) use snapshot::load_or_build_skill_snapshot;
