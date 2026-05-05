use serde::{Deserialize, Serialize};

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
    pub fn basic_store_search() -> Self {
        Self {
            store: true,
            search: true,
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryStatusReport {
    pub provider_id: String,
    pub healthy: bool,
    pub capabilities: MemoryCapabilitySet,
    pub message: Option<String>,
}

impl MemoryStatusReport {
    pub fn healthy(provider_id: impl Into<String>, capabilities: MemoryCapabilitySet) -> Self {
        Self {
            provider_id: provider_id.into(),
            healthy: true,
            capabilities,
            message: None,
        }
    }
}
