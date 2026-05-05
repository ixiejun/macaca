use async_trait::async_trait;
use macaca_proto::{MacacaResult, MemoryEntry, MemoryId};
use serde_json::Value;

use super::facade::{
    MemoryDeleteRequest, MemoryPrefetchRequest, MemorySearchRequest, MemoryWriteRequest,
};
use super::lifecycle::MemoryLifecycleEvent;
use super::scope::MemoryScope;

/// Optional capability for providers that can persist new memories.
#[async_trait]
pub trait MemoryStoreCapability: Send + Sync {
    async fn store_memory(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId>;
}

/// Optional capability for providers that can recall memories by query.
#[async_trait]
pub trait MemorySearchCapability: Send + Sync {
    async fn search_memory(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>>;
}

/// Optional capability for providers that can render prompt-ready context.
///
/// This is intentionally separate from plain search because some providers may
/// want to apply extra summarization, ranking, or trust fencing when the output
/// is destined for a model prompt instead of a UI or API response.
#[async_trait]
pub trait MemoryPromptCapability: Send + Sync {
    async fn render_prompt_context(&self, request: MemoryPrefetchRequest) -> MacacaResult<String>;
}

/// Optional capability for querying lifecycle/audit events.
#[async_trait]
pub trait MemoryLifecycleCapability: Send + Sync {
    async fn lifecycle_events(&self, scope: MemoryScope)
        -> MacacaResult<Vec<MemoryLifecycleEvent>>;
}

/// Optional capability for providers that need an explicit flush/checkpoint step.
#[async_trait]
pub trait MemoryFlushCapability: Send + Sync {
    async fn flush(&self, scope: MemoryScope) -> MacacaResult<()>;
}

/// Optional capability for exposing artifacts associated with a scope.
#[async_trait]
pub trait MemoryArtifactCapability: Send + Sync {
    async fn artifacts(&self, scope: MemoryScope) -> MacacaResult<Vec<Value>>;
}

/// Optional capability for policy checks before destructive actions.
#[async_trait]
pub trait MemoryGovernanceCapability: Send + Sync {
    async fn validate_delete(&self, request: &MemoryDeleteRequest) -> MacacaResult<()>;
}
