use serde::{Deserialize, Serialize};

/// Capability bitmap for a memory provider or adapter.
///
/// The fabric uses this lightweight structure for diagnostics and for future
/// routing/policy decisions. It avoids downcasting concrete providers just to
/// ask whether they support search, lifecycle hooks, artifacts, or governance.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCapabilitySet {
    pub store: bool,
    pub search: bool,
    pub prompt: bool,
    pub lifecycle: bool,
    pub flush: bool,
    pub artifact: bool,
    pub governance: bool,
}

impl MemoryCapabilitySet {
    /// Helper for the common builtin case: the provider can write and recall memory.
    pub fn basic_store_search() -> Self {
        Self {
            store: true,
            search: true,
            ..Self::default()
        }
    }
}

/// Health/status snapshot returned by a `MemoryFacade`.
///
/// This report is deliberately small enough to be surfaced in runtime
/// diagnostics without exposing backend internals or memory contents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryStatusReport {
    pub provider_id: String,
    pub healthy: bool,
    pub capabilities: MemoryCapabilitySet,
    pub message: Option<String>,
}

impl MemoryStatusReport {
    /// Convenience constructor for a healthy provider status.
    pub fn healthy(provider_id: impl Into<String>, capabilities: MemoryCapabilitySet) -> Self {
        Self {
            provider_id: provider_id.into(),
            healthy: true,
            capabilities,
            message: None,
        }
    }
}
