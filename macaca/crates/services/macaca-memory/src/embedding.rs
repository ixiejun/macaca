use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::debug;

use macaca_proto::{MacacaError, MacacaResult};

use crate::store::EmbeddingProvider;

const DEFAULT_DASHSCOPE_EMBED_URL: &str =
    "https://dashscope.aliyuncs.com/api/v1/services/embeddings/text-embedding/text-embedding";
const DIMENSIONS: usize = 1024;

#[derive(Debug, Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: EmbedInput<'a>,
    parameters: EmbedParameters,
}

#[derive(Debug, Serialize)]
struct EmbedInput<'a> {
    texts: &'a [String],
}

#[derive(Debug, Serialize)]
struct EmbedParameters {
    text_type: &'static str,
}

#[derive(Debug, Deserialize)]
struct EmbedResponse {
    output: EmbedOutput,
}

#[derive(Debug, Deserialize)]
struct EmbedOutput {
    embeddings: Vec<EmbedItem>,
}

#[derive(Debug, Deserialize)]
struct EmbedItem {
    embedding: Vec<f32>,
}

/// DashScope (阿里百炼) embedding provider using text-embedding-v4.
/// Requires `DASHSCOPE_API_KEY` environment variable.
pub struct DashScopeEmbedding {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
}

impl DashScopeEmbedding {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url: DEFAULT_DASHSCOPE_EMBED_URL.to_owned(),
            model: "text-embedding-v4".to_owned(),
        }
    }

    /// Create with a custom base URL.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        let url = base_url.into();
        if !url.is_empty() {
            self.base_url = url;
        }
        self
    }

    /// Override the embedding model selected by runtime configuration.
    ///
    /// DashScope currently defaults to `text-embedding-v4`, but keeping the
    /// model configurable avoids baking a provider-specific model name into the
    /// Memory service composition path. Empty config values preserve the safe
    /// default so older configuration files continue to work.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        let model = model.into();
        if !model.trim().is_empty() {
            self.model = model;
        }
        self
    }

    /// Create from the `DASHSCOPE_API_KEY` environment variable.
    pub fn from_env() -> MacacaResult<Self> {
        let key = std::env::var("DASHSCOPE_API_KEY")
            .map_err(|_| MacacaError::Memory("DASHSCOPE_API_KEY not set".into()))?;
        Ok(Self::new(key))
    }
}

#[async_trait]
impl EmbeddingProvider for DashScopeEmbedding {
    async fn embed(&self, texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
        debug!(count = texts.len(), "dashscope: embedding texts");

        let body = EmbedRequest {
            model: &self.model,
            input: EmbedInput { texts: &texts },
            parameters: EmbedParameters {
                text_type: "document",
            },
        };

        let response = self
            .client
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| MacacaError::Memory(format!("dashscope request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(MacacaError::Memory(format!(
                "dashscope error {status}: {text}"
            )));
        }

        let parsed: EmbedResponse = response
            .json()
            .await
            .map_err(|e| MacacaError::Memory(format!("dashscope parse error: {e}")))?;

        Ok(parsed
            .output
            .embeddings
            .into_iter()
            .map(|e| e.embedding)
            .collect())
    }

    fn dimensions(&self) -> usize {
        DIMENSIONS
    }
}

/// Mock embedding provider for testing — returns deterministic fake vectors.
pub struct MockEmbedding {
    dims: usize,
}

impl MockEmbedding {
    pub fn new(dims: usize) -> Self {
        Self { dims }
    }
}

impl Default for MockEmbedding {
    fn default() -> Self {
        Self::new(DIMENSIONS)
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbedding {
    async fn embed(&self, texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
        Ok(texts
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let mut v = vec![0.0f32; self.dims];
                // Deterministic value based on position.
                v[i % self.dims] = 1.0;
                v
            })
            .collect())
    }

    fn dimensions(&self) -> usize {
        self.dims
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_embedding_dimensions() {
        let mock = MockEmbedding::new(128);
        let result = mock
            .embed(vec!["hello".into(), "world".into()])
            .await
            .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].len(), 128);
        assert_eq!(result[1].len(), 128);
    }

    #[tokio::test]
    async fn mock_embedding_deterministic() {
        let mock = MockEmbedding::default();
        let a = mock.embed(vec!["test".into()]).await.unwrap();
        let b = mock.embed(vec!["test".into()]).await.unwrap();
        assert_eq!(a, b);
    }

    #[tokio::test]
    #[ignore]
    async fn dashscope_embed_live() {
        let provider = DashScopeEmbedding::from_env().unwrap();
        let result = provider.embed(vec!["hello world".into()]).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), DIMENSIONS);
    }
}
