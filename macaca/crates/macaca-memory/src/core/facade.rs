use async_trait::async_trait;
use macaca_proto::{MacacaResult, MemoryEntry, MemoryId, MemoryLayer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::scope::MemoryScope;
use super::status::MemoryStatusReport;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryWriteRequest {
    pub scope: MemoryScope,
    pub content: String,
    pub layer: MemoryLayer,
    pub metadata: Value,
}

impl MemoryWriteRequest {
    pub fn new(scope: MemoryScope, content: impl Into<String>) -> Self {
        Self {
            scope,
            content: content.into(),
            layer: MemoryLayer::Session,
            metadata: Value::Null,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchRequest {
    pub scope: MemoryScope,
    pub query: String,
    pub limit: usize,
}

impl MemorySearchRequest {
    pub fn new(scope: MemoryScope, query: impl Into<String>, limit: usize) -> Self {
        Self {
            scope,
            query: query.into(),
            limit,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGetRequest {
    pub scope: MemoryScope,
    pub id: MemoryId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDeleteRequest {
    pub scope: MemoryScope,
    pub id: MemoryId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPrefetchRequest {
    pub scope: MemoryScope,
    pub query: String,
    pub limit: usize,
}

#[async_trait]
pub trait MemoryFacade: Send + Sync {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId>;
    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>>;
    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>>;
    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()>;

    async fn prefetch(&self, request: MemoryPrefetchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        self.search(MemorySearchRequest::new(
            request.scope,
            request.query,
            request.limit,
        ))
        .await
    }

    fn status(&self) -> MemoryStatusReport;
}
