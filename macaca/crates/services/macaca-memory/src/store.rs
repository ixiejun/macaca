use async_trait::async_trait;
use macaca_proto::{AgentId, MacacaResult, MemoryEntry, MemoryId, TaskId};
use serde_json::Value;

/// Context for memory retrieval queries (distinct from execution `TaskContext`).
#[derive(Debug, Clone)]
pub struct MemoryQueryContext {
    pub task_id: TaskId,
    pub description: String,
    pub agent_id: AgentId,
    pub history: Vec<String>,
}

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
    async fn list(
        &self,
        agent_id: Option<&AgentId>,
        limit: usize,
    ) -> MacacaResult<Vec<MemoryEntry>>;
}

/// Automatically retrieves memory relevant to a task context.
#[async_trait]
pub trait MemoryRetriever: Send + Sync {
    async fn auto_retrieve(&self, context: &MemoryQueryContext) -> MacacaResult<Vec<MemoryEntry>>;
}

/// Converts text into embedding vectors.
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>>;
    fn dimensions(&self) -> usize;
}

/// Type-erased embedding provider used by configuration-driven composition roots.
///
/// `MemoryManager` is intentionally generic for zero-cost test and embedded
/// configurations, but a web/runtime composition root must select providers
/// from configuration at startup. This alias and the forwarding implementation
/// keep that Abstract Factory decision outside callers while preserving the
/// same `EmbeddingProvider` contract after construction.
pub type DynamicEmbeddingProvider = Box<dyn EmbeddingProvider>;

#[async_trait]
impl EmbeddingProvider for DynamicEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
        self.as_ref().embed(texts).await
    }

    fn dimensions(&self) -> usize {
        self.as_ref().dimensions()
    }
}

/// Stores and searches vectors by similarity.
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, id: &str, vector: Vec<f32>, payload: Value) -> MacacaResult<()>;
    async fn search(&self, vector: Vec<f32>, limit: usize)
        -> MacacaResult<Vec<VectorSearchResult>>;
    async fn delete(&self, id: &str) -> MacacaResult<()>;
}

/// Type-erased vector store used when the backend family is chosen from config.
///
/// The box is still a concrete provider instance owned by the Memory service
/// composition root. Upper layers never observe whether this is Milvus,
/// in-memory, or a future plugin-backed store; they only receive stable Memory
/// Service behavior and sanitized status.
pub type DynamicVectorStore = Box<dyn VectorStore>;

#[async_trait]
impl VectorStore for DynamicVectorStore {
    async fn upsert(&self, id: &str, vector: Vec<f32>, payload: Value) -> MacacaResult<()> {
        self.as_ref().upsert(id, vector, payload).await
    }

    async fn search(
        &self,
        vector: Vec<f32>,
        limit: usize,
    ) -> MacacaResult<Vec<VectorSearchResult>> {
        self.as_ref().search(vector, limit).await
    }

    async fn delete(&self, id: &str) -> MacacaResult<()> {
        self.as_ref().delete(id).await
    }
}
