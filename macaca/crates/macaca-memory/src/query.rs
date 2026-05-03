use async_trait::async_trait;
use serde_json::Value;

use macaca_proto::MacacaResult;

use crate::store::{VectorSearchResult, VectorStore};

#[derive(Debug, Clone)]
pub struct VectorQuery {
    pub vector: Vec<f32>,
    pub limit: usize,
    pub metadata_equals: Vec<(String, Value)>,
}

impl VectorQuery {
    pub fn new(vector: Vec<f32>, limit: usize) -> Self {
        Self {
            vector,
            limit,
            metadata_equals: Vec::new(),
        }
    }

    pub fn with_metadata_eq(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata_equals.push((key.into(), value));
        self
    }
}

#[async_trait]
pub trait VectorQueryStrategy<S: VectorStore>: Send + Sync {
    async fn search(&self, store: &S, query: VectorQuery) -> MacacaResult<Vec<VectorSearchResult>>;
}

pub struct SimilarityVectorQueryStrategy;

#[async_trait]
impl<S: VectorStore> VectorQueryStrategy<S> for SimilarityVectorQueryStrategy {
    async fn search(&self, store: &S, query: VectorQuery) -> MacacaResult<Vec<VectorSearchResult>> {
        let mut results = store.search(query.vector, query.limit).await?;
        if !query.metadata_equals.is_empty() {
            results.retain(|hit| {
                query
                    .metadata_equals
                    .iter()
                    .all(|(key, value)| hit.payload.get(key) == Some(value))
            });
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector::InMemoryVectorStore;

    #[tokio::test]
    async fn default_strategy_matches_similarity_ordering() {
        let store = InMemoryVectorStore::new();
        store
            .upsert("a", vec![1.0, 0.0], serde_json::json!({"kind": "doc"}))
            .await
            .unwrap();
        store
            .upsert("b", vec![0.0, 1.0], serde_json::json!({"kind": "doc"}))
            .await
            .unwrap();

        let strategy = SimilarityVectorQueryStrategy;
        let results = strategy
            .search(&store, VectorQuery::new(vec![1.0, 0.0], 2))
            .await
            .unwrap();

        assert_eq!(results[0].id, "a");
    }

    #[tokio::test]
    async fn strategy_filters_by_metadata() {
        let store = InMemoryVectorStore::new();
        store
            .upsert("a", vec![1.0, 0.0], serde_json::json!({"kind": "doc"}))
            .await
            .unwrap();
        store
            .upsert("b", vec![1.0, 0.0], serde_json::json!({"kind": "note"}))
            .await
            .unwrap();

        let strategy = SimilarityVectorQueryStrategy;
        let results = strategy
            .search(
                &store,
                VectorQuery::new(vec![1.0, 0.0], 10)
                    .with_metadata_eq("kind", serde_json::json!("note")),
            )
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "b");
    }
}
