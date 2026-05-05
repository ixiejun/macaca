//! Memory Fabric core abstractions.

pub mod adapter;
pub mod capability;
pub mod facade;
pub mod lifecycle;
pub mod provider;
pub mod router;
pub mod scope;
pub mod status;

#[cfg(test)]
mod tests;

pub use adapter::{BuiltinAgentPrivateMemory, BuiltinSessionSharedMemory, MemoryFabricFacade};
pub use capability::{
    MemoryArtifactCapability, MemoryFlushCapability, MemoryGovernanceCapability,
    MemoryLifecycleCapability, MemoryPromptCapability, MemorySearchCapability,
    MemoryStoreCapability,
};
pub use facade::{
    MemoryDeleteRequest, MemoryFacade, MemoryGetRequest, MemoryPrefetchRequest,
    MemorySearchRequest, MemoryWriteRequest,
};
pub use lifecycle::{MemoryLifecycleEvent, MemoryLifecycleEventKind};
pub use provider::{MemoryProvider, MemoryProviderDescriptor};
pub use router::{DefaultMemoryRouter, MemoryRoute, MemoryRouter};
pub use scope::{MemoryIdentity, MemoryScope, MemoryVisibility};
pub use status::{MemoryCapabilitySet, MemoryStatusReport};
