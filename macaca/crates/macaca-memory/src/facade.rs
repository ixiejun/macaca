use macaca_proto::{AgentId, MemoryEntry, MemoryId, MemoryLayer};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct RememberText {
    pub content: String,
    pub layer: MemoryLayer,
    pub metadata: Value,
    pub agent_id: Option<AgentId>,
}

impl RememberText {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            layer: MemoryLayer::Session,
            metadata: Value::Null,
            agent_id: None,
        }
    }

    pub fn layer(mut self, layer: MemoryLayer) -> Self {
        self.layer = layer;
        self
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn agent_id(mut self, agent_id: AgentId) -> Self {
        self.agent_id = Some(agent_id);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecallQuery {
    pub query: String,
    pub limit: usize,
}

impl RecallQuery {
    pub fn new(query: impl Into<String>, limit: usize) -> Self {
        Self {
            query: query.into(),
            limit,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecallResult {
    pub entries: Vec<MemoryEntry>,
}

impl RecallResult {
    pub fn new(entries: Vec<MemoryEntry>) -> Self {
        Self { entries }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForgetMemory {
    pub id: MemoryId,
}
