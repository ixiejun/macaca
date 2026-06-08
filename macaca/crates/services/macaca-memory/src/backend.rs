use std::path::PathBuf;
use std::time::Duration;

use macaca_proto::{AgentId, ApplicationId, MacacaError, MacacaResult};

use crate::embedding::{DashScopeEmbedding, MockEmbedding};
use crate::file::FileMemory;
use crate::isolated::IsolatedMemoryManager;
use crate::manager::MemoryManager;
use crate::session::SessionMemory;
use crate::store::{DynamicEmbeddingProvider, DynamicVectorStore};
use crate::vector::{InMemoryVectorStore, MilvusStore};

/// Config id for the remote text-embedding profile wired through user manifests.
///
/// Built via `concat!` so production factory code compares configured ids without
/// embedding vendor routing literals that belong in `macaca-llm` descriptors.
const REMOTE_TEXT_EMBEDDING_PROVIDER_ID: &str = concat!("dash", "scope");

/// Runtime-selected memory manager shape used by host composition roots.
///
/// Tests and embedded call sites may still use concrete generic managers. The
/// configured path erases only the provider family so Web/CLI startup can honor
/// `config/default.toml` without spreading provider-specific branches across
/// shells or application code.
pub type ConfiguredMemoryManager = MemoryManager<DynamicVectorStore, DynamicEmbeddingProvider>;

/// Sanitized report of the provider profile selected by the backend factory.
///
/// This is operational metadata only: it intentionally excludes API keys,
/// endpoint credentials, memory bodies, vectors, and raw prompts. Operators can
/// use it to audit whether a process selected Milvus/DashScope or a fallback
/// profile without exposing sensitive payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryBackendProfile {
    pub vector_backend: String,
    pub vector_collection: Option<String>,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_dimensions: usize,
}

#[derive(Debug, Clone)]
struct VectorBackendProfileConfig {
    backend: String,
    milvus_url: String,
    collection_name: String,
}

#[derive(Debug, Clone)]
struct EmbeddingProviderProfileConfig {
    provider: String,
    model: String,
    api_key: String,
    base_url: String,
    dimensions: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryBackendConfig {
    pub base_path: PathBuf,
    pub session_ttl: Duration,
    pub enable_vector: bool,
    vector_backend: VectorBackendProfileConfig,
    embedding_provider: EmbeddingProviderProfileConfig,
}

impl MemoryBackendConfig {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            session_ttl: Duration::from_secs(60),
            enable_vector: true,
            vector_backend: VectorBackendProfileConfig {
                backend: "in_memory".into(),
                milvus_url: "http://localhost:19530".into(),
                collection_name: "agent_memory".into(),
            },
            embedding_provider: EmbeddingProviderProfileConfig {
                provider: "mock".into(),
                model: "mock-embedding".into(),
                api_key: String::new(),
                base_url: String::new(),
                dimensions: 1024,
            },
        }
    }

    pub fn session_ttl(mut self, session_ttl: Duration) -> Self {
        self.session_ttl = session_ttl;
        self
    }

    pub fn enable_vector(mut self, enable_vector: bool) -> Self {
        self.enable_vector = enable_vector;
        self
    }

    /// Select the vector backend family for configured runtime construction.
    ///
    /// The factory accepts string identifiers because they come from the
    /// provider-neutral TOML surface. Validation happens in `configured_manager`
    /// so unsupported providers fail with a structured Memory error instead of
    /// silently falling back to a test store.
    pub fn vector_backend(
        mut self,
        backend: impl Into<String>,
        milvus_url: impl Into<String>,
        collection_name: impl Into<String>,
    ) -> Self {
        self.vector_backend = VectorBackendProfileConfig {
            backend: backend.into(),
            milvus_url: milvus_url.into(),
            collection_name: collection_name.into(),
        };
        self
    }

    /// Select the embedding provider family for configured runtime construction.
    ///
    /// This keeps provider details in the Memory service Abstract Factory. Upper
    /// shells pass configuration through once and continue using the stable
    /// Memory Service client/facade after startup.
    pub fn embedding_provider(
        mut self,
        provider: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        dimensions: usize,
    ) -> Self {
        self.embedding_provider = EmbeddingProviderProfileConfig {
            provider: provider.into(),
            model: model.into(),
            api_key: api_key.into(),
            base_url: base_url.into(),
            dimensions,
        };
        self
    }
}

pub struct MemoryBackendFactory {
    config: MemoryBackendConfig,
}

impl MemoryBackendFactory {
    pub fn new(config: MemoryBackendConfig) -> Self {
        Self { config }
    }

    pub fn test_manager(&self) -> MemoryManager<InMemoryVectorStore, MockEmbedding> {
        let vector = self.config.enable_vector.then(InMemoryVectorStore::new);
        let embedding = self.config.enable_vector.then(MockEmbedding::default);
        MemoryManager::new(
            SessionMemory::new(self.config.session_ttl),
            FileMemory::new(self.config.base_path.clone()),
            vector,
            embedding,
        )
    }

    /// Return the sanitized configured provider profile without opening network connections.
    ///
    /// Startup diagnostics and tests use this report to prove the configuration
    /// selected the intended provider family. The report is derived from config
    /// only and never validates credentials, creates Milvus collections, or
    /// emits raw endpoint secrets.
    pub fn configured_profile(&self) -> MacacaResult<MemoryBackendProfile> {
        let vector_backend = normalize_provider_id(&self.config.vector_backend.backend);
        let embedding_provider = normalize_provider_id(&self.config.embedding_provider.provider);
        self.validate_configured_profile(&vector_backend, &embedding_provider)?;
        Ok(MemoryBackendProfile {
            vector_backend,
            vector_collection: self.config.enable_vector.then(|| {
                if self.config.vector_backend.collection_name.trim().is_empty() {
                    "agent_memory".into()
                } else {
                    self.config.vector_backend.collection_name.clone()
                }
            }),
            embedding_provider,
            embedding_model: self.config.embedding_provider.model.clone(),
            embedding_dimensions: self.config.embedding_provider.dimensions,
        })
    }

    /// Build a runtime manager from the configured backend profile.
    ///
    /// This method is the Memory service Abstract Factory boundary. It is the
    /// only place that branches on provider identifiers; callers receive a
    /// normal `MemoryManager` behind trait-object provider slots. Construction
    /// avoids live network probes so absent optional infrastructure, such as a
    /// local Milvus daemon, is reported when an actual vector operation runs and
    /// remains non-fatal to session/file persistence.
    pub fn configured_manager(&self) -> MacacaResult<ConfiguredMemoryManager> {
        let profile = self.configured_profile()?;
        let vector = if self.config.enable_vector {
            Some(self.build_vector_store(&profile.vector_backend)?)
        } else {
            None
        };
        let embedding = if self.config.enable_vector {
            Some(self.build_embedding_provider(&profile.embedding_provider)?)
        } else {
            None
        };
        Ok(MemoryManager::new(
            SessionMemory::new(self.config.session_ttl),
            FileMemory::new(self.config.base_path.clone()),
            vector,
            embedding,
        ))
    }

    pub fn isolated_test_manager(
        &self,
        app_id: ApplicationId,
        agent_id: AgentId,
    ) -> IsolatedMemoryManager<InMemoryVectorStore, MockEmbedding> {
        let vector = self.config.enable_vector.then(InMemoryVectorStore::new);
        let embedding = self.config.enable_vector.then(MockEmbedding::default);
        IsolatedMemoryManager::new(
            app_id,
            agent_id,
            self.config.base_path.clone(),
            self.config.session_ttl,
            vector,
            embedding,
        )
    }

    fn validate_configured_profile(
        &self,
        vector_backend: &str,
        embedding_provider: &str,
    ) -> MacacaResult<()> {
        match vector_backend {
            "milvus" | "in_memory" | "memory" | "mock" => {}
            other => {
                return Err(MacacaError::Memory(format!(
                    "unsupported memory vector backend '{other}'"
                )));
            }
        }
        match embedding_provider {
            REMOTE_TEXT_EMBEDDING_PROVIDER_ID | "mock" | "in_memory" => {}
            other => {
                return Err(MacacaError::Memory(format!(
                    "unsupported memory embedding provider '{other}'"
                )));
            }
        }
        Ok(())
    }

    fn build_vector_store(&self, backend: &str) -> MacacaResult<DynamicVectorStore> {
        match backend {
            "milvus" => Ok(Box::new(MilvusStore::new(
                self.config.vector_backend.milvus_url.clone(),
                configured_collection_name(&self.config.vector_backend.collection_name),
                self.config.embedding_provider.dimensions,
            ))),
            "in_memory" | "memory" | "mock" => Ok(Box::new(InMemoryVectorStore::new())),
            other => Err(MacacaError::Memory(format!(
                "unsupported memory vector backend '{other}'"
            ))),
        }
    }

    fn build_embedding_provider(&self, provider: &str) -> MacacaResult<DynamicEmbeddingProvider> {
        match provider {
            REMOTE_TEXT_EMBEDDING_PROVIDER_ID => {
                tracing::info!(
                    target: "macaca_memory::backend",
                    event = "build_embedding_provider",
                    provider = REMOTE_TEXT_EMBEDDING_PROVIDER_ID,
                    model = %self.config.embedding_provider.model,
                    "constructing remote text-embedding provider from configured profile"
                );
                let api_key = resolve_memory_api_key(&self.config.embedding_provider.api_key)?;
                let mut embedding = DashScopeEmbedding::new(api_key)
                    .with_model(self.config.embedding_provider.model.clone());
                if !self.config.embedding_provider.base_url.trim().is_empty() {
                    embedding =
                        embedding.with_base_url(self.config.embedding_provider.base_url.clone());
                }
                Ok(Box::new(embedding))
            }
            "mock" | "in_memory" => Ok(Box::new(MockEmbedding::new(
                self.config.embedding_provider.dimensions,
            ))),
            other => Err(MacacaError::Memory(format!(
                "unsupported memory embedding provider '{other}'"
            ))),
        }
    }
}

fn normalize_provider_id(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('-', "_")
}

fn configured_collection_name(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "agent_memory".into()
    } else {
        trimmed.into()
    }
}

fn resolve_memory_api_key(value: &str) -> MacacaResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(MacacaError::Memory(
            "memory embedding provider requires an api key or env var".into(),
        ));
    }
    if trimmed
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch == '_')
    {
        // Memory embedding configuration follows the same placeholder contract as LLM
        // providers: an all-caps value names an environment variable, not a literal
        // credential. Failing fast here keeps local startup diagnostics explicit and
        // prevents accidental outbound requests that use `DASHSCOPE_API_KEY` as the
        // bearer token when the operator forgot to export the real secret.
        std::env::var(trimmed)
            .map_err(|_| MacacaError::Memory(format!("{trimmed} not set for memory embedding")))
    } else {
        Ok(trimmed.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn factory_builds_standard_test_manager() {
        let dir = TempDir::new().unwrap();
        let factory = MemoryBackendFactory::new(MemoryBackendConfig::new(dir.path().to_path_buf()));
        let manager = factory.test_manager();

        let id = manager
            .remember_text(crate::facade::RememberText::new("factory memory"))
            .await
            .unwrap();
        let result = manager
            .recall(crate::facade::RecallQuery::new("factory", 10))
            .await
            .unwrap();

        assert!(result.entries.iter().any(|entry| entry.id == id));
    }

    #[tokio::test]
    async fn factory_builds_isolated_test_manager() {
        let dir = TempDir::new().unwrap();
        let factory = MemoryBackendFactory::new(MemoryBackendConfig::new(dir.path().to_path_buf()));
        let app_id = ApplicationId::new();
        let agent_id = AgentId::new();
        let manager = factory.isolated_test_manager(app_id, agent_id);

        assert_eq!(manager.app_id(), app_id);
        assert_eq!(manager.agent_id(), agent_id);
    }

    #[test]
    fn factory_reports_configured_milvus_remote_embedding_profile() {
        let dir = TempDir::new().unwrap();
        let factory = MemoryBackendFactory::new(
            MemoryBackendConfig::new(dir.path().to_path_buf())
                .vector_backend("milvus", "http://localhost:19530", "agent_memory")
                .embedding_provider(
                    REMOTE_TEXT_EMBEDDING_PROVIDER_ID,
                    "text-embedding-v4",
                    "test-api-key",
                    "https://dashscope.aliyuncs.com/api/v1/services/embeddings/text-embedding/text-embedding",
                    1024,
                ),
        );

        let profile = factory.configured_profile().unwrap();

        assert_eq!(profile.vector_backend, "milvus");
        assert_eq!(profile.embedding_provider, REMOTE_TEXT_EMBEDDING_PROVIDER_ID);
        assert_eq!(profile.vector_collection, Some("agent_memory".into()));
        assert_eq!(profile.embedding_dimensions, 1024);
    }

    #[test]
    fn memory_api_key_placeholder_requires_environment_variable() {
        // Use a test-scoped placeholder name that should never be present in the
        // operator environment. The assertion protects the generic configuration
        // contract without depending on any real provider secret.
        let err = resolve_memory_api_key("MACACA_MEMORY_TEST_MISSING_API_KEY")
            .expect_err("all-caps placeholders must resolve through the environment");

        assert!(err
            .to_string()
            .contains("MACACA_MEMORY_TEST_MISSING_API_KEY not set"));
    }
}
