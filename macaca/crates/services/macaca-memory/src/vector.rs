use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::debug;

use crate::store::{VectorSearchResult, VectorStore};
use macaca_proto::{MacacaError, MacacaResult};

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
    #[serde(rename = "annsField")]
    anns_field: String,
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
    schema: CreateCollectionSchema,
    #[serde(rename = "indexParams")]
    index_params: Vec<CreateCollectionIndexParam>,
}

#[derive(Debug, Serialize)]
struct CreateCollectionSchema {
    #[serde(rename = "autoId")]
    auto_id: bool,
    #[serde(rename = "enableDynamicField")]
    enable_dynamic_field: bool,
    fields: Vec<CreateCollectionField>,
}

#[derive(Debug, Serialize)]
struct CreateCollectionField {
    #[serde(rename = "fieldName")]
    field_name: String,
    #[serde(rename = "dataType")]
    data_type: String,
    #[serde(rename = "isPrimary", skip_serializing_if = "std::ops::Not::not")]
    is_primary: bool,
    #[serde(rename = "elementTypeParams")]
    element_type_params: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct CreateCollectionIndexParam {
    #[serde(rename = "fieldName")]
    field_name: String,
    #[serde(rename = "indexName")]
    index_name: String,
    #[serde(rename = "metricType")]
    metric_type: String,
}

/// Milvus vector store backed by the Milvus REST API.
pub struct MilvusStore {
    client: reqwest::Client,
    base_url: String,
    collection: String,
    dimension: usize,
}

impl MilvusStore {
    pub fn new(
        base_url: impl Into<String>,
        collection: impl Into<String>,
        dimension: usize,
    ) -> Self {
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
        let body = CreateCollectionRequest::new(self.collection.clone(), self.dimension);
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
        let text = crate::providers::resilience::sanitize_provider_error_body(&resp.text().await.unwrap_or_default());
        Err(MacacaError::Memory(format!(
            "milvus create collection {status}: {text}"
        )))
    }
}

impl CreateCollectionRequest {
    /// Build the explicit Milvus schema required by Macaca memory records.
    ///
    /// Milvus' compact REST create API defaults the primary key field to
    /// `Int64`. Macaca memory ids are UUID strings, so the default schema
    /// accepts the create call but makes later string-key inserts/searches
    /// effectively unusable. The explicit schema keeps `id` as `VarChar`,
    /// enables dynamic metadata fields, and creates a cosine vector index for
    /// the `vector` field used by [`MilvusStore::upsert`].
    fn new(collection_name: impl Into<String>, dimension: usize) -> Self {
        Self {
            collection_name: collection_name.into(),
            schema: CreateCollectionSchema {
                auto_id: false,
                enable_dynamic_field: true,
                fields: vec![
                    CreateCollectionField {
                        field_name: "id".into(),
                        data_type: "VarChar".into(),
                        is_primary: true,
                        element_type_params: HashMap::from([("max_length".into(), "128".into())]),
                    },
                    CreateCollectionField {
                        field_name: "vector".into(),
                        data_type: "FloatVector".into(),
                        is_primary: false,
                        element_type_params: HashMap::from([("dim".into(), dimension.to_string())]),
                    },
                ],
            },
            index_params: vec![CreateCollectionIndexParam {
                field_name: "vector".into(),
                index_name: "vector".into(),
                metric_type: "COSINE".into(),
            }],
        }
    }
}

#[async_trait]
impl VectorStore for MilvusStore {
    async fn upsert(&self, id: &str, vector: Vec<f32>, payload: Value) -> MacacaResult<()> {
        // Milvus collections are runtime infrastructure, not application data.
        // Creating them lazily on the first write keeps local and plugin-backed
        // memory runtimes self-healing after a fresh database start while still
        // surfacing real schema/network errors as structured Memory failures.
        self.ensure_collection().await?;

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
            let text = crate::providers::resilience::sanitize_provider_error_body(&resp.text().await.unwrap_or_default());
            return Err(MacacaError::Memory(format!(
                "milvus upsert {status}: {text}"
            )));
        }
        Ok(())
    }

    async fn search(
        &self,
        vector: Vec<f32>,
        limit: usize,
    ) -> MacacaResult<Vec<VectorSearchResult>> {
        let url = format!("{}/v2/vectordb/entities/search", self.base_url);
        let body = MilvusSearchRequest {
            collection_name: self.collection.clone(),
            anns_field: "vector".into(),
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
            let text = crate::providers::resilience::sanitize_provider_error_body(&resp.text().await.unwrap_or_default());
            return Err(MacacaError::Memory(format!(
                "milvus search {status}: {text}"
            )));
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
            let text = crate::providers::resilience::sanitize_provider_error_body(&resp.text().await.unwrap_or_default());
            return Err(MacacaError::Memory(format!(
                "milvus delete {status}: {text}"
            )));
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

    async fn search(
        &self,
        vector: Vec<f32>,
        limit: usize,
    ) -> MacacaResult<Vec<VectorSearchResult>> {
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

    #[test]
    fn milvus_create_collection_request_uses_string_memory_ids() {
        let body = CreateCollectionRequest::new("agent_memory", 1024);
        let value = serde_json::to_value(body).unwrap();

        assert_eq!(value["collectionName"], "agent_memory");
        assert_eq!(value["schema"]["autoId"], false);
        assert_eq!(value["schema"]["enableDynamicField"], true);
        assert_eq!(value["schema"]["fields"][0]["fieldName"], "id");
        assert_eq!(value["schema"]["fields"][0]["dataType"], "VarChar");
        assert_eq!(value["schema"]["fields"][0]["isPrimary"], true);
        assert_eq!(
            value["schema"]["fields"][0]["elementTypeParams"]["max_length"],
            "128"
        );
        assert_eq!(value["schema"]["fields"][1]["fieldName"], "vector");
        assert_eq!(value["schema"]["fields"][1]["dataType"], "FloatVector");
        assert_eq!(
            value["schema"]["fields"][1]["elementTypeParams"]["dim"],
            "1024"
        );
        assert_eq!(value["indexParams"][0]["metricType"], "COSINE");
    }

    #[test]
    fn milvus_search_request_targets_vector_field_explicitly() {
        let body = MilvusSearchRequest {
            collection_name: "agent_memory".into(),
            anns_field: "vector".into(),
            data: vec![vec![1.0, 0.0, 0.0, 0.0]],
            limit: 1,
            output_fields: vec!["*".into()],
        };
        let value = serde_json::to_value(body).unwrap();

        assert_eq!(value["collectionName"], "agent_memory");
        assert_eq!(value["annsField"], "vector");
        assert_eq!(value["limit"], 1);
        assert_eq!(value["outputFields"][0], "*");
    }

    #[tokio::test]
    #[ignore]
    async fn milvus_live() {
        let store = MilvusStore::new("http://localhost:19530", "test_collection", 4);
        store.ensure_collection().await.unwrap();
        store
            .upsert(
                "m1",
                vec![1.0, 0.0, 0.0, 0.0],
                serde_json::json!({"text": "hello"}),
            )
            .await
            .unwrap();
        let mut results = Vec::new();
        for _ in 0..10 {
            results = store.search(vec![1.0, 0.0, 0.0, 0.0], 1).await.unwrap();
            if !results.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        assert!(!results.is_empty());
        store.delete("m1").await.unwrap();
    }
}
