//! aos-memory: three-layer memory system for Agent OS.

pub mod backend;
pub mod cache;
pub mod embedding;
pub mod facade;
pub mod file;
pub mod isolated;
pub mod manager;
pub mod query;
pub mod session;
pub mod snapshot;
pub mod store;
pub mod vector;

pub use backend::{MemoryBackendConfig, MemoryBackendFactory};
pub use cache::{CachedEmbeddingProvider, EmbeddingCache};
pub use embedding::{DashScopeEmbedding, MockEmbedding};
pub use facade::{ForgetMemory, RecallQuery, RecallResult, RememberText};
pub use file::FileMemory;
pub use isolated::{IsolatedMemoryManager, TestIsolatedMemoryManager};
pub use manager::{MemoryManager, TestMemoryManager};
pub use query::{SimilarityVectorQueryStrategy, VectorQuery, VectorQueryStrategy};
pub use session::SessionMemory;
pub use snapshot::{MemorySnapshot, MemorySnapshotStore};
pub use store::{EmbeddingProvider, MemoryRetriever, MemoryStore, VectorSearchResult, VectorStore};
pub use vector::{InMemoryVectorStore, MilvusStore};
