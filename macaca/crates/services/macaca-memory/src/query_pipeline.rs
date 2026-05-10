use async_trait::async_trait;
use macaca_proto::{MacacaResult, MemoryEntry};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::{MemoryFacade, MemorySearchRequest};
use crate::store::EmbeddingProvider;

/// Query strategy mode selected by runtime configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryQueryMode {
    Keyword,
    Vector,
    Hybrid,
    Filtered,
}

/// Result of a query pipeline run.
#[derive(Debug, Clone)]
pub struct MemoryQueryPipelineResult {
    pub entries: Vec<MemoryEntry>,
    pub diagnostics: Vec<String>,
    pub used_keyword: bool,
    pub used_vector: bool,
}

/// Strategy boundary for memory query execution.
#[async_trait]
pub trait MemoryQueryPipeline: Send + Sync {
    async fn search(&self, request: MemorySearchRequest)
        -> MacacaResult<MemoryQueryPipelineResult>;
}

/// Facade-backed query pipeline with optional embedding probe and metadata filters.
pub struct DefaultMemoryQueryPipeline<F, E> {
    facade: F,
    embedding: Option<E>,
    mode: MemoryQueryMode,
    metadata_equals: Vec<(String, Value)>,
}

impl<F, E> DefaultMemoryQueryPipeline<F, E> {
    pub fn new(facade: F) -> Self {
        Self {
            facade,
            embedding: None,
            mode: MemoryQueryMode::Keyword,
            metadata_equals: Vec::new(),
        }
    }

    pub fn with_embedding(mut self, embedding: E) -> Self {
        self.embedding = Some(embedding);
        self
    }

    pub fn with_mode(mut self, mode: MemoryQueryMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_metadata_eq(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata_equals.push((key.into(), value));
        self
    }
}

#[async_trait]
impl<F, E> MemoryQueryPipeline for DefaultMemoryQueryPipeline<F, E>
where
    F: MemoryFacade,
    E: EmbeddingProvider,
{
    async fn search(
        &self,
        request: MemorySearchRequest,
    ) -> MacacaResult<MemoryQueryPipelineResult> {
        let mut diagnostics = Vec::new();
        let mut used_vector = false;

        if matches!(self.mode, MemoryQueryMode::Vector | MemoryQueryMode::Hybrid) {
            match &self.embedding {
                Some(embedding) => match embedding.embed(vec![request.query.clone()]).await {
                    Ok(_) => {
                        used_vector = true;
                        diagnostics.push("vector_probe_ok".into());
                    }
                    Err(error) if self.mode == MemoryQueryMode::Hybrid => {
                        diagnostics.push(format!("vector_degraded:{error}"));
                    }
                    Err(error) => return Err(error),
                },
                None if self.mode == MemoryQueryMode::Hybrid => {
                    diagnostics.push("vector_degraded:no_embedding_provider".into());
                }
                None => {
                    return Err(macaca_proto::MacacaError::Memory(
                        "vector query requires embedding provider".into(),
                    ));
                }
            }
        }

        let mut entries = self.facade.search(request).await?;
        if !self.metadata_equals.is_empty() || self.mode == MemoryQueryMode::Filtered {
            diagnostics.push("metadata_filter_applied".into());
            entries.retain(|entry| {
                self.metadata_equals
                    .iter()
                    .all(|(key, value)| entry.metadata.get(key) == Some(value))
            });
        }

        Ok(MemoryQueryPipelineResult {
            entries,
            diagnostics,
            used_keyword: true,
            used_vector,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::MemoryCapabilitySet;
    use crate::core::{
        MemoryDeleteRequest, MemoryGetRequest, MemoryStatusReport, MemoryWriteRequest,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use macaca_proto::{ApplicationId, MacacaError, MemoryId, MemoryLayer};

    struct FakeFacade {
        entries: Vec<MemoryEntry>,
    }

    #[async_trait]
    impl MemoryFacade for FakeFacade {
        async fn remember(&self, _request: MemoryWriteRequest) -> MacacaResult<MemoryId> {
            Ok(MemoryId::new())
        }

        async fn search(&self, _request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>> {
            Ok(self.entries.clone())
        }

        async fn get(&self, _request: MemoryGetRequest) -> MacacaResult<Option<MemoryEntry>> {
            Ok(None)
        }

        async fn delete(&self, _request: MemoryDeleteRequest) -> MacacaResult<()> {
            Ok(())
        }

        fn status(&self) -> MemoryStatusReport {
            MemoryStatusReport::healthy("fake", MemoryCapabilitySet::basic_store_search())
        }
    }

    struct FailingEmbedding;

    #[async_trait]
    impl EmbeddingProvider for FailingEmbedding {
        async fn embed(&self, _texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
            Err(MacacaError::Memory("embedding unavailable".into()))
        }

        fn dimensions(&self) -> usize {
            3
        }
    }

    fn entry(content: &str, metadata: Value) -> MemoryEntry {
        MemoryEntry {
            id: MemoryId::new(),
            layer: MemoryLayer::Session,
            content: content.into(),
            metadata,
            agent_id: None,
            created_at: Utc::now(),
            expires_at: None,
        }
    }

    #[tokio::test]
    async fn hybrid_query_falls_back_to_keyword_when_embedding_fails() {
        let pipeline = DefaultMemoryQueryPipeline::new(FakeFacade {
            entries: vec![entry("keyword hit", serde_json::json!({"kind":"doc"}))],
        })
        .with_embedding(FailingEmbedding)
        .with_mode(MemoryQueryMode::Hybrid);

        let result = pipeline
            .search(MemorySearchRequest::new(
                crate::MemoryScope::session_shared(ApplicationId::new(), "s1"),
                "keyword",
                10,
            ))
            .await
            .unwrap();

        assert_eq!(result.entries.len(), 1);
        assert!(result.used_keyword);
        assert!(!result.used_vector);
        assert!(result
            .diagnostics
            .iter()
            .any(|item| item.contains("vector_degraded")));
    }

    #[tokio::test]
    async fn filtered_query_keeps_only_matching_metadata() {
        let pipeline: DefaultMemoryQueryPipeline<_, crate::MockEmbedding> =
            DefaultMemoryQueryPipeline::new(FakeFacade {
                entries: vec![
                    entry("doc", serde_json::json!({"kind":"doc"})),
                    entry("note", serde_json::json!({"kind":"note"})),
                ],
            })
            .with_mode(MemoryQueryMode::Filtered)
            .with_metadata_eq("kind", serde_json::json!("note"));

        let result = pipeline
            .search(MemorySearchRequest::new(
                crate::MemoryScope::session_shared(ApplicationId::new(), "s1"),
                "anything",
                10,
            ))
            .await
            .unwrap();

        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].content, "note");
        assert!(result
            .diagnostics
            .contains(&"metadata_filter_applied".to_string()));
    }
}
