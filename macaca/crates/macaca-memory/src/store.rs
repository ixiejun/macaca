use async_trait::async_trait;
use macaca_proto::{AgentId, MacacaResult, MemoryEntry, MemoryId, TaskContext};
use serde_json::Value;

/// Search result from a vector store.
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub id: String,
    pub score: f32,
    pub payload: Value,
}

/// Core trait for storing and retrieving memory entries.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn store(&self, entry: MemoryEntry) -> MacacaResult<MemoryId>;
    async fn retrieve(&self, query: &str, limit: usize) -> MacacaResult<Vec<MemoryEntry>>;
    async fn get(&self, id: &MemoryId) -> MacacaResult<Option<MemoryEntry>>;
    async fn delete(&self, id: &MemoryId) -> MacacaResult<()>;
    async fn list(&self, agent_id: Option<&AgentId>, limit: usize) -> MacacaResult<Vec<MemoryEntry>>;
}

/// Automatically retrieves memory relevant to a task context.
#[async_trait]
pub trait MemoryRetriever: Send + Sync {
    async fn auto_retrieve(&self, context: &TaskContext) -> MacacaResult<Vec<MemoryEntry>>;
}

/// Converts text into embedding vectors.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>>;
    fn dimensions(&self) -> usize;
}

/// Stores and searches vectors by similarity.
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, id: &str, vector: Vec<f32>, payload: Value) -> MacacaResult<()>;
    async fn search(&self, vector: Vec<f32>, limit: usize) -> MacacaResult<Vec<VectorSearchResult>>;
    async fn delete(&self, id: &str) -> MacacaResult<()>;
}
