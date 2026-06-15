use async_trait::async_trait;
use macaca_proto::{MacacaResult, MemoryEntry, MemoryId, MemoryLayer};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::scope::MemoryScope;
use super::status::MemoryStatusReport;

/// Request for writing a memory into the fabric.
///
/// The request keeps the *what* and the *where* together:
/// - `content` is the payload the caller wants to persist.
/// - `scope` tells the fabric which isolation domain owns the memory.
/// - `layer` preserves the stable memory layering model.
/// - `metadata` carries source/provenance fields that adapters may enrich.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryWriteRequest {
    pub scope: MemoryScope,
    pub content: String,
    pub layer: MemoryLayer,
    pub metadata: Value,
}

impl MemoryWriteRequest {
    /// Create a write request with sensible defaults.
    ///
    /// New fabric callers can start from this constructor and then opt into a
    /// different layer or custom metadata through the builder-style helpers.
    pub fn new(scope: MemoryScope, content: impl Into<String>) -> Self {
        Self {
            scope,
            content: content.into(),
            layer: MemoryLayer::Session,
            metadata: Value::Null,
        }
    }

    /// Override the layer used when the underlying store persists this entry.
    pub fn layer(mut self, layer: MemoryLayer) -> Self {
        self.layer = layer;
        self
    }

    /// Attach caller-provided metadata before the request enters the router/adapters.
    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Request for semantic or lexical recall within a given memory scope.
///
/// The scope is part of the contract so recall never runs "globally" by
/// accident. Routing can still widen the recall surface in a controlled way,
/// for example from agent-private to a composite private+shared lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySearchRequest {
    pub scope: MemoryScope,
    pub query: String,
    pub limit: usize,
}

impl MemorySearchRequest {
    /// Create a bounded search request.
    pub fn new(scope: MemoryScope, query: impl Into<String>, limit: usize) -> Self {
        Self {
            scope,
            query: query.into(),
            limit,
        }
    }
}

/// Request for point lookup by memory id under an explicit scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGetRequest {
    pub scope: MemoryScope,
    pub id: MemoryId,
}

/// Request for deleting a single memory entry under an explicit scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDeleteRequest {
    pub scope: MemoryScope,
    pub id: MemoryId,
}

/// Request for "prompt-oriented" recall.
///
/// `prefetch` exists because some providers may want to build a compact prompt
/// snippet differently from a normal search result list. The default facade
/// implementation simply reuses `search`, which keeps the core contract small
/// while still leaving room for smarter providers later.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPrefetchRequest {
    pub scope: MemoryScope,
    pub query: String,
    pub limit: usize,
}

/// Minimal provider-neutral memory contract used by the new fabric core.
///
/// The trait is intentionally CRUD-shaped:
/// - routers decide *which* backend should handle a scope
/// - adapters translate requests into concrete managers/stores
/// - callers depend on one stable facade instead of knowing concrete managers
///
/// This keeps the memory fabric extensible without forcing all consumers to
/// couple to `MemoryManager`, `IsolatedMemoryManager`, or future providers.
#[async_trait]
pub trait MemoryFacade: Send + Sync {
    /// Persist a memory entry and return its canonical id.
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId>;
    /// Recall matching entries for the given scoped query.
    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>>;
    /// Fetch one memory entry by id.
    async fn get(&self, request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>>;
    /// Delete one memory entry by id.
    async fn delete(&self, request: MemoryDeleteRequest) -> MacacaResult<()>;

    /// Build request-scoped recall context.
    ///
    /// Most providers can safely treat prompt prefetch as "search with the same
    /// constraints". Providers that need custom scoring, truncation, or prompt
    /// formatting can override this method without changing the public contract.
    async fn prefetch(&self, request: MemoryPrefetchRequest) -> MacacaResult<Vec<MemoryEntry>> {
        self.search(MemorySearchRequest::new(
            request.scope,
            request.query,
            request.limit,
        ))
        .await
    }

    /// Report liveness and capability metadata for diagnostics and routing.
    fn status(&self) -> MemoryStatusReport;
}
