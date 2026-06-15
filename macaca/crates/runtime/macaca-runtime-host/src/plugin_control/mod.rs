//! Runtime-host Plugin Control Plane v1.
//!
//! The control plane is the management Facade for plugins.  It coordinates
//! repository state, admission checks, descriptor-safe runtime registration,
//! health snapshots, diagnostics, and audit events without executing plugin
//! code.  Execution hosts remain behind Plugin Runtime and later Plugin Fabric
//! phases, which keeps this module focused on deterministic control state.

mod admission;
mod metadata;
mod repository;
mod service;

pub use admission::{
    AdmissionCheck, AdmissionContext, AdmissionDecision, ManifestShapeAdmissionCheck,
    PluginAdmissionChain, RuntimeVersionAdmissionCheck, SourcePolicyAdmissionCheck,
};
pub use repository::{
    InMemoryPluginRepository, PluginRepository, PluginRepositoryMutation, PluginRepositorySnapshot,
};
pub use service::{PluginControlService, PluginControlServiceBuilder};

#[cfg(test)]
#[path = "plugin_control_tests.rs"]
mod plugin_control_tests;
