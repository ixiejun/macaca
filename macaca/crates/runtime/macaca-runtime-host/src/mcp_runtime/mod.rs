//! Agent OS level MCP registry and runtime glue (Facade module tree).
//!
//! This module owns Agent OS policy, status and toolkit registration on top
//! of the protocol primitives in [`macaca_framework::mcp`]. It was promoted
//! from `macaca-web` so any Agent OS host (web, CLI, gateway) can mount MCP
//! runtimes without depending on HTTP scaffolding.
//!
//! Product-name handling is **not** encoded in control flow here — see
//! [`crate::skill_mcp_mapping_registry`] for the declarative registry that maps
//! skill install specs to MCP definitions and concurrency isolation policies.
//!
//! ## Design patterns
//!
//! - **Facade** — [`McpRuntimeFacade`] is the stable host-facing entry point.
//! - **Strategy** — [`client::McpClientFactory`] injects protocol client creation.
//! - **Adapter** — YAML/skill definitions → [`McpServerDefinition`].
//! - **Registry** — invocation session registry + descriptor index.
//! - **Template Method** — service-backed invoke: validate → lease → call → release.
//! - **Observer** — toolkit `on_close` callbacks notify lease release.
//!
//! ## Module layout
//!
//! | File | Responsibility |
//! |------|----------------|
//! | `types.rs` | Value objects + manager struct |
//! | `policy.rs` | Concurrency isolation policy |
//! | `facade.rs` | Host-facing facade API |
//! | `manager.rs` | Catalog, probe, resources |
//! | `manager_invoke.rs` | Invoke, register, cleanup |
//! | `probe.rs` | Connectivity probe |
//! | `descriptors.rs` | Descriptor construction |
//! | `config_entry.rs` | YAML entry adapter |
//! | `registration.rs` | Toolkit registration |
//! | `client.rs` | Protocol client factory |
//! | `helpers.rs` | Shared pure helpers |
//! | `skill_definitions.rs` | Skill snapshot resolver |
//! | `tests/` | Unit tests (fixtures, catalog, facade lifecycle, invocation) |

mod client;
mod config_entry;
mod descriptors;
mod facade;
mod helpers;
mod manager;
mod manager_invoke;
mod policy;
mod probe;
mod registration;
mod skill_definitions;
mod types;

#[cfg(test)]
mod tests;

pub use facade::McpRuntimeFacade;
pub use policy::apply_concurrency_isolation;
pub use probe::probe_definition_statuses;
pub use skill_definitions::definitions_from_skill_snapshot_with_registry;
pub use types::{
    ConcurrencyIsolationPolicy, McpDefinitionSource, McpLifecycleScope, McpRegistryConfig,
    McpRuntimeContext, McpRuntimeKey, McpRuntimeStatus, McpRuntimeStatusState,
    McpServerConfigEntry, McpServerDefinition, McpToolPolicy,
};
