use serde::{Deserialize, Serialize};

/// Identifies the logical lane that a provider binding serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryProviderSlotKind {
    AgentPrivate,
    SessionShared,
    Embedding,
    VectorBackend,
    ActiveRecall,
    KnowledgeCompiler,
}

/// Scope qualifier for provider overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryProviderSlotScope {
    Default,
    Agent,
    Session,
}

impl MemoryProviderSlotScope {
    /// Convert a logical override scope into a stable string label.
    ///
    /// The runtime uses this label in diagnostics so operators can tell whether
    /// a binding came from the profile default or from a narrower override.
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Agent => "agent",
            Self::Session => "session",
        }
    }
}

/// A resolved provider binding for one slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderSlotBinding {
    pub provider_id: String,
    pub slot: MemoryProviderSlotKind,
    pub scope: MemoryProviderSlotScope,
}

impl MemoryProviderSlotBinding {
    pub fn new(
        provider_id: impl Into<String>,
        slot: MemoryProviderSlotKind,
        scope: MemoryProviderSlotScope,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            slot,
            scope,
        }
    }
}

/// Per-agent/session override for a single provider slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderSlotOverride {
    pub slot: MemoryProviderSlotKind,
    pub provider_id: String,
}

/// Component slots exposed by a profile.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryProviderComponentSlot {
    pub agent_private_provider: Option<String>,
    pub session_shared_provider: Option<String>,
    pub embedding_provider: Option<String>,
    pub vector_backend: Option<String>,
    pub active_recall: Option<String>,
    pub knowledge_compiler: Option<String>,
}
