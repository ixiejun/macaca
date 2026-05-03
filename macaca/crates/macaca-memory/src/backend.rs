use std::path::PathBuf;
use std::time::Duration;

use macaca_proto::{AgentId, ApplicationId};

use crate::embedding::MockEmbedding;
use crate::file::FileMemory;
use crate::isolated::IsolatedMemoryManager;
use crate::manager::MemoryManager;
use crate::session::SessionMemory;
use crate::vector::InMemoryVectorStore;

#[derive(Debug, Clone)]
pub struct MemoryBackendConfig {
    pub base_path: PathBuf,
    pub session_ttl: Duration,
    pub enable_vector: bool,
}

impl MemoryBackendConfig {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            session_ttl: Duration::from_secs(60),
            enable_vector: true,
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
}
