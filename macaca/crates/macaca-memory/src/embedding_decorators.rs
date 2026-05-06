use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use macaca_proto::{MacacaError, MacacaResult};
use tokio::time::timeout;

use crate::store::EmbeddingProvider;

/// In-process embedding metrics for diagnostics and tests.
#[derive(Debug, Default)]
pub struct EmbeddingMetrics {
    call_count: AtomicUsize,
    failure_count: AtomicUsize,
    last_latency_ms: AtomicU64,
}

impl EmbeddingMetrics {
    pub fn record_call(&self, latency: Duration, failed: bool) {
        self.call_count.fetch_add(1, Ordering::Relaxed);
        if failed {
            self.failure_count.fetch_add(1, Ordering::Relaxed);
        }
        self.last_latency_ms
            .store(latency.as_millis() as u64, Ordering::Relaxed);
    }

    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::Relaxed)
    }

    pub fn failure_count(&self) -> usize {
        self.failure_count.load(Ordering::Relaxed)
    }

    pub fn last_latency_ms(&self) -> u64 {
        self.last_latency_ms.load(Ordering::Relaxed)
    }
}

/// Timeout decorator for embedding providers.
pub struct TimeoutEmbeddingProvider {
    inner: Arc<dyn EmbeddingProvider>,
    timeout: Duration,
}

impl TimeoutEmbeddingProvider {
    pub fn new(inner: Arc<dyn EmbeddingProvider>, timeout: Duration) -> Self {
        Self { inner, timeout }
    }
}

#[async_trait]
impl EmbeddingProvider for TimeoutEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
        timeout(self.timeout, self.inner.embed(texts))
            .await
            .map_err(|_| MacacaError::Memory("embedding timeout".into()))?
    }

    fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }
}

/// Retry decorator for transient memory embedding failures.
pub struct RetryEmbeddingProvider {
    inner: Arc<dyn EmbeddingProvider>,
    retries: u32,
}

impl RetryEmbeddingProvider {
    pub fn new(inner: Arc<dyn EmbeddingProvider>, retries: u32) -> Self {
        Self { inner, retries }
    }
}

#[async_trait]
impl EmbeddingProvider for RetryEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
        let mut attempts = 0u32;
        loop {
            match self.inner.embed(texts.clone()).await {
                Ok(vectors) => return Ok(vectors),
                Err(MacacaError::Memory(error)) if attempts < self.retries => {
                    attempts += 1;
                    tracing::debug!(attempts, error, "retrying embedding provider");
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }
}

/// Metrics decorator for embedding providers.
pub struct MetricsEmbeddingProvider {
    inner: Arc<dyn EmbeddingProvider>,
    metrics: Arc<EmbeddingMetrics>,
}

impl MetricsEmbeddingProvider {
    pub fn new(inner: Arc<dyn EmbeddingProvider>, metrics: Arc<EmbeddingMetrics>) -> Self {
        Self { inner, metrics }
    }

    pub fn metrics(&self) -> Arc<EmbeddingMetrics> {
        Arc::clone(&self.metrics)
    }
}

#[async_trait]
impl EmbeddingProvider for MetricsEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
        let started = Instant::now();
        let result = self.inner.embed(texts).await;
        self.metrics.record_call(started.elapsed(), result.is_err());
        result
    }

    fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct SlowEmbedding;

    #[async_trait]
    impl EmbeddingProvider for SlowEmbedding {
        async fn embed(&self, _texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
            tokio::time::sleep(Duration::from_millis(50)).await;
            Ok(vec![vec![1.0]])
        }

        fn dimensions(&self) -> usize {
            1
        }
    }

    struct FlakyEmbedding {
        calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl EmbeddingProvider for FlakyEmbedding {
        async fn embed(&self, _texts: Vec<String>) -> MacacaResult<Vec<Vec<f32>>> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if call == 0 {
                return Err(MacacaError::Memory("temporary embedding failure".into()));
            }
            Ok(vec![vec![42.0]])
        }

        fn dimensions(&self) -> usize {
            1
        }
    }

    #[tokio::test]
    async fn timeout_decorator_returns_error_without_panicking() {
        let provider =
            TimeoutEmbeddingProvider::new(Arc::new(SlowEmbedding), Duration::from_millis(5));
        let error = provider.embed(vec!["slow".into()]).await.unwrap_err();
        assert!(error.to_string().contains("embedding timeout"));
    }

    #[tokio::test]
    async fn retry_decorator_retries_then_succeeds() {
        let calls = Arc::new(AtomicUsize::new(0));
        let provider = RetryEmbeddingProvider::new(
            Arc::new(FlakyEmbedding {
                calls: Arc::clone(&calls),
            }),
            1,
        );
        let vectors = provider.embed(vec!["ok".into()]).await.unwrap();
        assert_eq!(vectors, vec![vec![42.0]]);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn metrics_decorator_records_call_and_failure_counts() {
        let calls = Arc::new(AtomicUsize::new(0));
        let metrics = Arc::new(EmbeddingMetrics::default());
        let provider =
            MetricsEmbeddingProvider::new(Arc::new(FlakyEmbedding { calls }), Arc::clone(&metrics));
        let _ = provider.embed(vec!["fail".into()]).await;
        assert_eq!(metrics.call_count(), 1);
        assert_eq!(metrics.failure_count(), 1);
    }
}
