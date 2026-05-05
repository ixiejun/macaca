use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use macaca_proto::{MacacaResult, MemoryId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::scope::{MemoryScope, MemoryVisibility};
use crate::store::{VectorSearchResult, VectorStore};
use crate::vector_topology::{
    DefaultVectorTopologyResolver, VectorMemoryTopology, VectorTopologyResolver,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorCollectionSchema {
    pub dimension: usize,
    pub metric: String,
    pub metadata_fields: Vec<String>,
}

impl VectorCollectionSchema {
    pub fn cosine(dimension: usize) -> Self {
        Self {
            dimension,
            metric: "cosine".into(),
            metadata_fields: vec![
                "memory_id".into(),
                "scope".into(),
                "visibility".into(),
                "provenance".into(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMemoryRecord {
    pub memory_id: MemoryId,
    pub scope: MemoryScope,
    pub visibility: MemoryVisibility,
    pub content: String,
    pub vector: Vec<f32>,
    pub metadata: Value,
    pub provenance: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub confidence: Option<f32>,
    pub freshness: Option<f32>,
    pub conflict: Option<Value>,
}

impl VectorMemoryRecord {
    pub fn new(
        memory_id: MemoryId,
        scope: MemoryScope,
        content: impl Into<String>,
        vector: Vec<f32>,
    ) -> Self {
        let visibility = scope.visibility;
        Self {
            memory_id,
            scope,
            visibility,
            content: content.into(),
            vector,
            metadata: Value::Null,
            provenance: Value::Null,
            created_at: Utc::now(),
            updated_at: None,
            confidence: None,
            freshness: None,
            conflict: None,
        }
    }

    pub fn metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn provenance(mut self, provenance: Value) -> Self {
        self.provenance = provenance;
        self
    }

    pub fn payload(&self, topology: &VectorMemoryTopology) -> Value {
        serde_json::json!({
            "memory_id": self.memory_id.0.to_string(),
            "scope": self.scope,
            "visibility": self.visibility,
            "content": self.content,
            "metadata": self.metadata,
            "provenance": self.provenance,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "confidence": self.confidence,
            "freshness": self.freshness,
            "conflict": self.conflict,
            "vector_topology": topology,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMemoryHit {
    pub memory_id: MemoryId,
    pub score: f32,
    pub scope: MemoryScope,
    pub visibility: MemoryVisibility,
    pub snippet: String,
    pub provenance: Value,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorBackendStatus {
    pub backend_id: String,
    pub topology: Option<VectorMemoryTopology>,
    pub dimension: usize,
    pub available: bool,
    pub last_error: Option<String>,
}

#[async_trait]
pub trait VectorCollectionStoreFactory: Send + Sync {
    fn backend_id(&self) -> &'static str;
    fn dimension(&self) -> usize;
    fn store_for(&self, topology: &VectorMemoryTopology) -> Arc<dyn VectorStore>;
    async fn prepare_topology(&self, _topology: &VectorMemoryTopology) -> MacacaResult<()> {
        Ok(())
    }
}

#[async_trait]
pub trait VectorMemoryBackend: Send + Sync {
    async fn ensure_topology(
        &self,
        scope: &MemoryScope,
        schema: &VectorCollectionSchema,
    ) -> MacacaResult<VectorMemoryTopology>;
    async fn upsert_record(&self, record: VectorMemoryRecord) -> MacacaResult<()>;
    async fn search(
        &self,
        scope: &MemoryScope,
        vector: Vec<f32>,
        limit: usize,
    ) -> MacacaResult<Vec<VectorMemoryHit>>;
    async fn delete(&self, scope: &MemoryScope, memory_id: &MemoryId) -> MacacaResult<()>;
    async fn status(&self, scope: Option<&MemoryScope>) -> VectorBackendStatus;
}

pub struct TopologyVectorMemoryBackend<F, R = DefaultVectorTopologyResolver> {
    factory: F,
    resolver: R,
}

impl<F> TopologyVectorMemoryBackend<F, DefaultVectorTopologyResolver> {
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            resolver: DefaultVectorTopologyResolver,
        }
    }
}

impl<F, R> TopologyVectorMemoryBackend<F, R> {
    pub fn with_resolver(factory: F, resolver: R) -> Self {
        Self { factory, resolver }
    }
}

#[async_trait]
impl<F, R> VectorMemoryBackend for TopologyVectorMemoryBackend<F, R>
where
    F: VectorCollectionStoreFactory,
    R: VectorTopologyResolver,
{
    async fn ensure_topology(
        &self,
        scope: &MemoryScope,
        _schema: &VectorCollectionSchema,
    ) -> MacacaResult<VectorMemoryTopology> {
        let topology = self.resolver.resolve(scope)?;
        self.factory.prepare_topology(&topology).await?;
        Ok(topology)
    }

    async fn upsert_record(&self, record: VectorMemoryRecord) -> MacacaResult<()> {
        let topology = self.resolver.resolve(&record.scope)?;
        let store = self.factory.store_for(&topology);
        store
            .upsert(
                &record.memory_id.0.to_string(),
                record.vector.clone(),
                record.payload(&topology),
            )
            .await
    }

    async fn search(
        &self,
        scope: &MemoryScope,
        vector: Vec<f32>,
        limit: usize,
    ) -> MacacaResult<Vec<VectorMemoryHit>> {
        let topology = self.resolver.resolve(scope)?;
        let store = self.factory.store_for(&topology);
        let hits = store.search(vector, limit).await?;
        Ok(hits
            .into_iter()
            .filter_map(VectorMemoryHit::from_store_hit)
            .collect())
    }

    async fn delete(&self, scope: &MemoryScope, memory_id: &MemoryId) -> MacacaResult<()> {
        let topology = self.resolver.resolve(scope)?;
        let store = self.factory.store_for(&topology);
        store.delete(&memory_id.0.to_string()).await
    }

    async fn status(&self, scope: Option<&MemoryScope>) -> VectorBackendStatus {
        let topology = scope.and_then(|scope| self.resolver.resolve(scope).ok());
        VectorBackendStatus {
            backend_id: self.factory.backend_id().into(),
            topology,
            dimension: self.factory.dimension(),
            available: true,
            last_error: None,
        }
    }
}

impl VectorMemoryHit {
    pub fn from_store_hit(hit: VectorSearchResult) -> Option<Self> {
        let memory_id = hit
            .payload
            .get("memory_id")
            .and_then(Value::as_str)
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .map(MemoryId)?;
        let scope: MemoryScope = serde_json::from_value(hit.payload.get("scope")?.clone()).ok()?;
        Some(Self {
            memory_id,
            score: hit.score,
            visibility: scope.visibility,
            scope,
            snippet: hit
                .payload
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            provenance: hit
                .payload
                .get("provenance")
                .cloned()
                .unwrap_or(Value::Null),
            metadata: hit.payload.get("metadata").cloned().unwrap_or(Value::Null),
        })
    }
}

pub struct MilvusCollectionStoreFactory {
    pub base_url: String,
    pub dimension: usize,
}

impl MilvusCollectionStoreFactory {
    pub fn new(base_url: impl Into<String>, dimension: usize) -> Self {
        Self {
            base_url: base_url.into(),
            dimension,
        }
    }
}

#[async_trait]
impl VectorCollectionStoreFactory for MilvusCollectionStoreFactory {
    fn backend_id(&self) -> &'static str {
        "milvus"
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn store_for(&self, topology: &VectorMemoryTopology) -> Arc<dyn VectorStore> {
        Arc::new(crate::vector::MilvusStore::new(
            self.base_url.clone(),
            topology.collection.clone(),
            self.dimension,
        ))
    }

    async fn prepare_topology(&self, topology: &VectorMemoryTopology) -> MacacaResult<()> {
        let store = crate::vector::MilvusStore::new(
            self.base_url.clone(),
            topology.collection.clone(),
            self.dimension,
        );
        store.ensure_collection().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::InMemoryVectorStore;
    use macaca_proto::{AgentId, ApplicationId};
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct InMemoryBackendFactory {
        stores: Arc<Mutex<HashMap<String, Arc<InMemoryVectorStore>>>>,
    }

    impl VectorCollectionStoreFactory for InMemoryBackendFactory {
        fn backend_id(&self) -> &'static str {
            "in_memory"
        }

        fn dimension(&self) -> usize {
            3
        }

        fn store_for(&self, topology: &VectorMemoryTopology) -> Arc<dyn VectorStore> {
            let key = format!("{}::{}", topology.database, topology.collection);
            let mut guard = self.stores.lock().unwrap();
            let store = guard
                .entry(key)
                .or_insert_with(|| Arc::new(InMemoryVectorStore::new()))
                .clone();
            store as Arc<dyn VectorStore>
        }
    }

    async fn assert_private_collections_are_isolated<F>(backend: &TopologyVectorMemoryBackend<F>)
    where
        F: VectorCollectionStoreFactory,
    {
        let app_id = ApplicationId::new();
        let agent_a = MemoryScope::agent_private(app_id, AgentId::new());
        let agent_b = MemoryScope::agent_private(app_id, AgentId::new());
        let record_a = VectorMemoryRecord::new(
            MemoryId::new(),
            agent_a.clone(),
            "agent a private vector",
            vec![1.0, 0.0, 0.0],
        );
        let record_b = VectorMemoryRecord::new(
            MemoryId::new(),
            agent_b.clone(),
            "agent b private vector",
            vec![1.0, 0.0, 0.0],
        );

        backend.upsert_record(record_a.clone()).await.unwrap();
        backend.upsert_record(record_b.clone()).await.unwrap();

        let hits_a = backend
            .search(&agent_a, vec![1.0, 0.0, 0.0], 10)
            .await
            .unwrap();
        let hits_b = backend
            .search(&agent_b, vec![1.0, 0.0, 0.0], 10)
            .await
            .unwrap();

        assert_eq!(hits_a.len(), 1);
        assert_eq!(hits_a[0].memory_id, record_a.memory_id);
        assert_eq!(hits_b.len(), 1);
        assert_eq!(hits_b[0].memory_id, record_b.memory_id);
    }

    async fn assert_shared_collection_is_explicit<F>(backend: &TopologyVectorMemoryBackend<F>)
    where
        F: VectorCollectionStoreFactory,
    {
        let app_id = ApplicationId::new();
        let agent = AgentId::new();
        let private_scope = MemoryScope::agent_private(app_id, agent);
        let shared_scope = MemoryScope::session_shared(app_id, "session-1");

        backend
            .upsert_record(VectorMemoryRecord::new(
                MemoryId::new(),
                private_scope.clone(),
                "private only",
                vec![1.0, 0.0, 0.0],
            ))
            .await
            .unwrap();

        let shared_hits = backend
            .search(&shared_scope, vec![1.0, 0.0, 0.0], 10)
            .await
            .unwrap();
        assert!(shared_hits.is_empty());
    }

    #[tokio::test]
    async fn backend_resolves_topology_and_round_trips_hits() {
        let backend = TopologyVectorMemoryBackend::new(InMemoryBackendFactory::default());
        let scope = MemoryScope::agent_private(ApplicationId::new(), AgentId::new());
        let record = VectorMemoryRecord::new(
            MemoryId::new(),
            scope.clone(),
            "vector backend record",
            vec![1.0, 0.0, 0.0],
        )
        .provenance(serde_json::json!({"source": "test"}));

        let topology = backend
            .ensure_topology(&scope, &VectorCollectionSchema::cosine(3))
            .await
            .unwrap();
        assert!(topology.database.starts_with("app_"));
        assert!(topology.collection.starts_with("agent_"));

        backend.upsert_record(record.clone()).await.unwrap();
        let hits = backend
            .search(&scope, vec![1.0, 0.0, 0.0], 10)
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].memory_id, record.memory_id);
        assert_eq!(hits[0].provenance["source"], "test");
    }

    #[tokio::test]
    async fn backend_status_reports_topology() {
        let backend = TopologyVectorMemoryBackend::new(InMemoryBackendFactory::default());
        let scope = MemoryScope::session_shared(ApplicationId::new(), "session-1");
        let status = backend.status(Some(&scope)).await;

        assert_eq!(status.backend_id, "in_memory");
        assert_eq!(status.dimension, 3);
        assert!(status.topology.is_some());
    }

    #[tokio::test]
    async fn conformance_private_collections_are_isolated() {
        let backend = TopologyVectorMemoryBackend::new(InMemoryBackendFactory::default());
        assert_private_collections_are_isolated(&backend).await;
    }

    #[tokio::test]
    async fn conformance_shared_collection_is_explicit() {
        let backend = TopologyVectorMemoryBackend::new(InMemoryBackendFactory::default());
        assert_shared_collection_is_explicit(&backend).await;
    }
}
