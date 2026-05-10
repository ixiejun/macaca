use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use macaca_proto::MacacaResult;

use crate::store::EmbeddingProvider;

#[derive(Clone, Default)]
pub struct EmbeddingCache {
    inner: Arc<RwLock<HashMap<String, Vec<f32>>>>,
}

impl EmbeddingCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get(&self, text: &str) -> Option<Vec<f32>> {
        self.inner.read().await.get(text).cloned()
    }

    pub async fn insert(&self, text: String, vector: Vec<f32>) {
        self.inner.write().await.insert(text, vector);
    }

    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

pub struct CachedEmbeddingProvider<E> {
    inner: E,
    cache: EmbeddingCache,
}

impl<E> CachedEmbeddingProvider<E> {
    pub fn new(inner: E) -> Self {
        Self {
            inner,
            cache: EmbeddingCache::new(),
        }
    }

    pub fn with_cache(inner: E, cache: EmbeddingCache) -> Self {
        Self { inner, cache }
    }

    pub fn cache(&self) -> &EmbeddingCache {
        &self.cache
    }
}

#[async_trait]
impl<E: EmbeddingProvider> EmbeddingProvider for CachedEmbeddingProvider<E> {
    async fn embed(&self, texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
        let mut output: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
        let mut missing = Vec::new();
        let mut missing_indices = Vec::new();

        for (index, text) in texts.iter().enumerate() {
            if let Some(vector) = self.cache.get(text).await {
                output[index] = Some(vector);
            } else {
                missing_indices.push(index);
                missing.push(text.clone());
            }
        }

        if !missing.is_empty() {
            let vectors = self.inner.embed(missing.clone()).await?;
            for ((index, text), vector) in missing_indices
                .into_iter()
                .zip(missing.into_iter())
                .zip(vectors.into_iter())
            {
                self.cache.insert(text, vector.clone()).await;
                output[index] = Some(vector);
            }
        }

        Ok(output.into_iter().flatten().collect())
    }

    fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingEmbedding {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl EmbeddingProvider for CountingEmbedding {
        async fn embed(&self, texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(texts
                .into_iter()
                .map(|text| vec![text.len() as f32])
                .collect())
        }

        fn dimensions(&self) -> usize {
            1
        }
    }

    #[tokio::test]
    async fn cached_provider_reuses_repeated_text() {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = CachedEmbeddingProvider::new(CountingEmbedding {
            calls: Arc::clone(&calls),
        });

        let first = provider.embed(vec!["hello".into()]).await.unwrap();
        let second = provider.embed(vec!["hello".into()]).await.unwrap();

        assert_eq!(first, second);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(provider.cache().len().await, 1);
    }

    #[tokio::test]
    async fn cached_provider_preserves_batch_order() {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = CachedEmbeddingProvider::new(CountingEmbedding { calls });

        let vectors = provider
            .embed(vec!["a".into(), "abcd".into(), "a".into()])
            .await
            .unwrap();

        assert_eq!(vectors, vec![vec![1.0], vec![4.0], vec![1.0]]);
    }
}
