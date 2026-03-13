//! aos-memory: three-layer memory system for Agent OS.

pub mod embedding;
pub mod file;
pub mod isolated;
pub mod manager;
pub mod session;
pub mod store;
pub mod vector;

pub use embedding::{DashScopeEmbedding, MockEmbedding};
pub use file::FileMemory;
pub use isolated::{IsolatedMemoryManager, TestIsolatedMemoryManager};
pub use manager::{MemoryManager, TestMemoryManager};
pub use session::SessionMemory;
pub use store::{EmbeddingProvider, MemoryRetriever, MemoryStore, VectorSearchResult, VectorStore};
pub use vector::{InMemoryVectorStore, MilvusStore};
