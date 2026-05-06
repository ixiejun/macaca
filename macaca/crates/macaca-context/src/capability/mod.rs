//! Capability-context composition: skills + MCP + runtime tools as Tier-1 indices.
//!
//! Design patterns:
//! - **Adapter**: Neutral DTOs decouple [`macaca_context`] from concrete skill/MCP crates.
//! - **Composite**: Catalogs compose multiple servers/tools into one MCP candidate.
//! - **Chain-of-Responsibility**: Individual [`ContextProvider`] implementations plug into [`crate::composer::ContextFacade`].

mod model;
mod render;

pub mod mcp_provider;
pub mod runtime_tools_provider;
pub mod skill_provider;

pub use model::*;
pub use mcp_provider::{mcp_capability_provider_arc, McpContextProvider};
pub use render::{
    mcp_catalog_to_candidate, mcp_tool_collisions, runtime_tool_catalog_to_candidate,
    skill_catalog_to_candidate, skill_missing_dependency_notes,
};
pub use runtime_tools_provider::{runtime_tool_capability_provider_arc, RuntimeToolCapabilityProvider};
pub use skill_provider::{skill_capability_provider_arc, SkillContextProvider};

#[cfg(test)]
mod provider_tests;
