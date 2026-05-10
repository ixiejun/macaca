//! Provider runtime for the memory fabric.
//!
//! This module is intentionally split into small files so the runtime can stay
//! pluggable without turning into a monolith. The registry selects providers by
//! configuration profile, the factory builds concrete adapters, and the runtime
//! layer later on can add resilience, redaction, and MCP/remote bridges.

pub mod config;
pub mod factory;
pub mod mcp;
pub mod registry;
pub mod remote;
pub mod resilience;
pub mod runtime;
pub mod slot;
#[cfg(test)]
mod tests;

pub use config::{
    MemoryProviderConfig, MemoryProviderEndpointConfig, MemoryProviderMcpConfig,
    MemoryProviderMcpServerConfig, MemoryProviderProfileConfig, MemoryProviderProfilesConfig,
    MemoryProviderResilienceConfig, MemoryProviderRuntimeConfig, MemoryProviderToolConfig,
    MemoryProviderTransportKind,
};
pub use factory::{
    BuiltinMemoryProviderFactory, MemoryProviderFactory, MemoryProviderFactoryResult,
};
pub use mcp::{mcp_provider_diagnostics, McpMemoryProvider, MemoryMcpClient};
pub use registry::{MemoryProviderRegistry, MemoryProviderRegistryError};
pub use remote::{MemoryProviderRemoteEnvelope, MemoryProviderRemoteResult, RemoteMemoryProvider};
pub use runtime::{MemoryProviderRuntime, MemoryProviderRuntimeStatus};
pub use slot::{
    MemoryProviderComponentSlot, MemoryProviderSlotBinding, MemoryProviderSlotKind,
    MemoryProviderSlotOverride, MemoryProviderSlotScope,
};
