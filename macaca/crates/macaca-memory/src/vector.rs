use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::debug;

use macaca_proto::{MacacaError, MacacaResult};

use crate::store::{VectorSearchResult, VectorStore};

// ── Milvus REST types ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct MilvusInsertRequest {
    #[serde(rename = "collectionName")]
    collection_name: String,
    data: Vec<Value>,
}

#[derive(Debug, Serialize)]
struct MilvusSearchRequest {
    #[serde(rename = "collectionName")]
    collection_name: String,
    data: Vec<Vec<f32>>,
    limit: usize,
    #[serde(rename = "outputFields")]
    output_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MilvusSearchResponse {
    data: Vec<MilvusHit>,
}

#[derive(Debug, Deserialize)]
struct MilvusHit {
    id: Value,
    distance: f32,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize)]
struct MilvusDeleteRequest {
    #[serde(rename = "collectionName")]
    collection_name: String,
    filter: String,
}

#[derive(Debug, Serialize)]
struct CreateCollectionRequest {
    #[serde(rename = "collectionName")]
    collection_name: String,
    dimension: usize,
}

/// Milvus vector store backed by the Milvus REST API.
pub struct MilvusStore {
    client: reqwest::Client,
    base_url: String,
    collection: String,
    dimension: usize,
}

impl MilvusStore {
    pub fn new(base_url: impl Into<String>, collection: impl Into<String>, dimension: usize) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            collection: collection.into(),
            dimension,
        }
    }

    /// Ensure the collection exists, creating it if necessary.
    pub async fn ensure_collection(&self) -> MacacaResult<()> {
        let url = format!("{}/v2/vectordb/collections/create", self.base_url);
        let body = CreateCollectionRequest {
            collection_name: self.collection.clone(),
            dimension: self.dimension,
        };
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| MacacaError::Memory(format!("milvus create collection: {e}")))?;

        // 200 = created, some versions return 409 if already exists — both are acceptable.
        if resp.status().is_success() || resp.status().as_u16() == 409 {
            debug!(collection = %self.collection, "milvus: collection ready");
            return Ok(());
        }
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Err(MacacaError::Memory(format!(
            "milvus create collection {status}: {text}"
        )))
    }
}

#[async_trait]
impl VectorStore for MilvusStore {
    async fn upsert(&self, id: &str, vector: Vec<f32>, payload: Value) -> MacacaResult<()> {
        let url = format!("{}/v2/vectordb/entities/insert", self.base_url);
        let mut data_obj = match payload {
            Value::Object(m) => m,
            _ => serde_json::Map::new(),
        };
        data_obj.insert("id".into(), Value::String(id.to_string()));
        data_obj.insert("vector".into(), serde_json::to_value(&vector)?);

        let body = MilvusInsertRequest {
            collection_name: self.collection.clone(),
            data: vec![Value::Object(data_obj)],
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| MacacaError::Memory(format!("milvus upsert: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MacacaError::Memory(format!("milvus upsert {status}: {text}")));
        }
        Ok(())
    }

    async fn search(&self, vector: Vec<f32>, limit: usize) -> MacacaResult<Vec<VectorSearchResult>> {
        let url = format!("{}/v2/vectordb/entities/search", self.base_url);
        let body = MilvusSearchRequest {
            collection_name: self.collection.clone(),
            data: vec![vector],
            limit,
            output_fields: vec!["*".into()],
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| MacacaError::Memory(format!("milvus search: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MacacaError::Memory(format!("milvus search {status}: {text}")));
        }

        let parsed: MilvusSearchResponse = resp
            .json()
            .await
            .map_err(|e| MacacaError::Memory(format!("milvus search parse: {e}")))?;

        Ok(parsed
            .data
            .into_iter()
            .map(|hit| {
                let id = match &hit.id {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                VectorSearchResult {
                    id,
                    score: hit.distance,
                    payload: Value::Object(hit.extra.into_iter().collect()),
                }
            })
            .collect())
    }

    async fn delete(&self, id: &str) -> MacacaResult<()> {
        let url = format!("{}/v2/vectordb/entities/delete", self.base_url);
        let body = MilvusDeleteRequest {
            collection_name: self.collection.clone(),
            filter: format!("id == \"{}\"", id),
        };

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| MacacaError::Memory(format!("milvus delete: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(MacacaError::Memory(format!("milvus delete {status}: {text}")));
        }
        Ok(())
    }
}

// ── InMemoryVectorStore ────────────────────────────────────────────────────────

struct VectorEntry {
    vector: Vec<f32>,
    payload: Value,
}

/// Brute-force in-memory vector store using cosine similarity.
/// Suitable for testing and small workloads.
pub struct InMemoryVectorStore {
    entries: Arc<RwLock<HashMap<String, VectorEntry>>>,
}

impl InMemoryVectorStore {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn upsert(&self, id: &str, vector: Vec<f32>, payload: Value) -> MacacaResult<()> {
        let mut guard = self.entries.write().await;
        guard.insert(id.to_string(), VectorEntry { vector, payload });
        Ok(())
    }

    async fn search(&self, vector: Vec<f32>, limit: usize) -> MacacaResult<Vec<VectorSearchResult>> {
        let guard = self.entries.read().await;
        let mut scored: Vec<(String, f32, Value)> = guard
            .iter()
            .map(|(id, entry)| {
                let score = cosine_similarity(&vector, &entry.vector);
                (id.clone(), score, entry.payload.clone())
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored
            .into_iter()
            .map(|(id, score, payload)| VectorSearchResult { id, score, payload })
            .collect())
    }

    async fn delete(&self, id: &str) -> MacacaResult<()> {
        let mut guard = self.entries.write().await;
        guard.remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_upsert_and_search() {
        let store = InMemoryVectorStore::new();
        store
            .upsert("a", vec![1.0, 0.0, 0.0], serde_json::json!({"label": "a"}))
            .await
            .unwrap();
        store
            .upsert("b", vec![0.0, 1.0, 0.0], serde_json::json!({"label": "b"}))
            .await
            .unwrap();

        let results = store.search(vec![1.0, 0.0, 0.0], 2).await.unwrap();
        assert_eq!(results[0].id, "a");
        assert!(results[0].score > results[1].score);
    }

    #[tokio::test]
    async fn in_memory_delete() {
        let store = InMemoryVectorStore::new();
        store
            .upsert("x", vec![1.0], serde_json::json!({}))
            .await
            .unwrap();
        store.delete("x").await.unwrap();
        let results = store.search(vec![1.0], 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn in_memory_limit_respected() {
        let store = InMemoryVectorStore::new();
        for i in 0..5u32 {
            store
                .upsert(&format!("v{i}"), vec![i as f32, 0.0], serde_json::json!({}))
                .await
                .unwrap();
        }
        let results = store.search(vec![1.0, 0.0], 2).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    #[ignore]
    async fn milvus_live() {
        let store = MilvusStore::new("http://localhost:19530", "test_collection", 4);
        store.ensure_collection().await.unwrap();
        store
            .upsert("m1", vec![1.0, 0.0, 0.0, 0.0], serde_json::json!({"text": "hello"}))
            .await
            .unwrap();
        let results = store.search(vec![1.0, 0.0, 0.0, 0.0], 1).await.unwrap();
        assert!(!results.is_empty());
        store.delete("m1").await.unwrap();
    }
}
