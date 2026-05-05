use async_trait::async_trait;
use macaca_proto::{MacacaResult, MemoryEntry, MemoryId};
use serde_json::Value;

use super::facade::{
    MemoryDeleteRequest, MemoryPrefetchRequest, MemorySearchRequest, MemoryWriteRequest,
};
use super::lifecycle::MemoryLifecycleEvent;
use super::scope::MemoryScope;

#[async_trait]
pub trait MemoryStoreCapability: Send + Sync {
    async fn store_memory(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId>;
}

#[async_trait]
pub trait MemorySearchCapability: Send + Sync {
    async fn search_memory(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>>;
}

#[async_trait]
pub trait MemoryPromptCapability: Send + Sync {
    async fn render_prompt_context(&self, request: MemoryPrefetchRequest) -> MacacaResult<String>;
}

#[async_trait]
pub trait MemoryLifecycleCapability: Send + Sync {
    async fn lifecycle_events(&self, scope: MemoryScope)
        -> MacacaResult<Vec<MemoryLifecycleEvent>>;
}

#[async_trait]
pub trait MemoryFlushCapability: Send + Sync {
    async fn flush(&self, scope: MemoryScope) -> MacacaResult<()>;
}

#[async_trait]
pub trait MemoryArtifactCapability: Send + Sync {
    async fn artifacts(&self, scope: MemoryScope) -> MacacaResult<Vec<Value>>;
}

#[async_trait]
pub trait MemoryGovernanceCapability: Send + Sync {
    async fn validate_delete(&self, request: &MemoryDeleteRequest) -> MacacaResult<()>;
}
