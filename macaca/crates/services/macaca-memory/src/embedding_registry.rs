use std::collections::HashMap;
use std::sync::Arc;

use macaca_proto::{MacacaError, MacacaResult};

use crate::store::EmbeddingProvider;

/// Factory for one embedding provider implementation.
pub trait EmbeddingProviderFactory: Send + Sync {
    fn provider_id(&self) -> &str;
    fn dimensions(&self) -> usize;
    fn build(&self) -> MacacaResult<Arc<dyn EmbeddingProvider>>;
}

/// Registry that resolves embedding providers by id.
#[derive(Default)]
pub struct EmbeddingProviderRegistry {
    default_provider_id: Option<String>,
    factories: HashMap<String, Arc<dyn EmbeddingProviderFactory>>,
}

impl EmbeddingProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_default(mut self, provider_id: impl Into<String>) -> Self {
        self.default_provider_id = Some(provider_id.into());
        self
    }

    pub fn register(&mut self, factory: Arc<dyn EmbeddingProviderFactory>) {
        self.factories
            .insert(factory.provider_id().to_string(), factory);
    }

    pub fn resolve(&self, provider_id: Option<&str>) -> MacacaResult<Arc<dyn EmbeddingProvider>> {
        let id = provider_id
            .map(str::to_string)
            .or_else(|| self.default_provider_id.clone())
            .ok_or_else(|| MacacaError::Memory("no embedding provider configured".into()))?;
        let factory = self
            .factories
            .get(&id)
            .ok_or_else(|| MacacaError::Memory(format!("unknown embedding provider id: {id}")))?;
        factory.build()
    }

    pub fn dimensions(&self, provider_id: Option<&str>) -> MacacaResult<usize> {
        let id = provider_id
            .map(str::to_string)
            .or_else(|| self.default_provider_id.clone())
            .ok_or_else(|| MacacaError::Memory("no embedding provider configured".into()))?;
        self.factories
            .get(&id)
            .map(|factory| factory.dimensions())
            .ok_or_else(|| MacacaError::Memory(format!("unknown embedding provider id: {id}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockFactory;

    impl EmbeddingProviderFactory for MockFactory {
        fn provider_id(&self) -> &str {
            "mock"
        }

        fn dimensions(&self) -> usize {
            3
        }

        fn build(&self) -> MacacaResult<Arc<dyn EmbeddingProvider>> {
            Ok(Arc::new(crate::MockEmbedding::new(3)))
        }
    }

    #[tokio::test]
    async fn registry_resolves_default_embedding_provider() {
        let mut registry = EmbeddingProviderRegistry::new().with_default("mock");
        registry.register(Arc::new(MockFactory));

        let provider = registry.resolve(None).unwrap();
        let vectors = provider.embed(vec!["hello".into()]).await.unwrap();

        assert_eq!(registry.dimensions(None).unwrap(), 3);
        assert_eq!(vectors.len(), 1);
        assert_eq!(vectors[0].len(), 3);
    }

    #[tokio::test]
    async fn registry_reports_unknown_provider() {
        let registry = EmbeddingProviderRegistry::new().with_default("missing");
        let error = match registry.resolve(None) {
            Ok(_) => panic!("expected unknown provider error"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("unknown embedding provider"));
    }
}
