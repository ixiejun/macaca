//! Memory Fabric core abstractions.
//!
//! This module defines the provider-neutral API introduced by the
//! `add-memory-fabric-core` change:
//! - `scope` models ownership and visibility
//! - `facade` defines the stable request/response contract
//! - `router` maps scope to logical memory lanes
//! - `adapter` bridges the contract to existing builtin managers
//!
//! The goal is to let the rest of Macaca depend on one generic memory boundary
//! while concrete storage implementations remain replaceable behind adapters.

pub mod active_recall;
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

pub use active_recall::{
    ActiveRecallBudget, ActiveRecallCandidate, ActiveRecallCapability, ActiveRecallDecision,
    ActiveRecallRequest, ActiveRecallResult, DefaultActiveRecallStrategy,
};
pub use adapter::{BuiltinAgentPrivateMemory, BuiltinSessionSharedMemory, MemoryFabricFacade};
pub use capability::{
    MemoryArtifactCapability, MemoryFlushCapability, MemoryGovernanceCapability,
    MemoryKnowledgeCapability, MemoryLifecycleCapability, MemoryPromptCapability,
    MemorySearchCapability, MemoryStoreCapability,
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
