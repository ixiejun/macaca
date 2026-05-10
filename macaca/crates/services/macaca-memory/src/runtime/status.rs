use serde::{Deserialize, Serialize};

/// Runtime-level status summary for the composed memory facade.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRuntimeStatus {
    pub runtime_id: String,
    pub provider_profile: Option<String>,
    pub store_available: bool,
    pub search_available: bool,
    pub active_recall_available: bool,
    pub knowledge_available: bool,
    pub diagnostics: Vec<String>,
}
