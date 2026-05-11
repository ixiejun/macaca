//! Plugin Capability Registry v1 runtime-host module.
//!
//! The module exposes descriptor discovery, conflict Strategy, ownership
//! registry, and service facade as separate components.  Keeping these roles
//! split prevents the registry from becoming a behavior hub and keeps future
//! execution hosts pluggable.

pub mod admission;
pub mod conflict;
pub mod discovery;
pub mod registry;
pub mod service;

pub use admission::{
    CapabilityCallAdmissionChain, CapabilityCallAdmissionCheck, CapabilityCallAdmissionContext,
    CapabilityCallAdmissionDecision, CapabilityHintAdmissionCheck,
    CapabilityOwnershipAdmissionCheck,
};
pub use conflict::{
    detect_slot_conflict, FailClosedSlotConflictPolicy, PluginCapabilityConflictPolicy,
};
pub use discovery::{built_in_service_capability, discover_manifest_capabilities};
pub use registry::{PluginCapabilityRegistry, PluginCapabilityRegistryBuilder};
pub use service::{PluginCapabilityService, PluginCapabilityServiceBuilder};

#[cfg(test)]
#[path = "plugin_capability_tests.rs"]
mod plugin_capability_tests;
