//! Service adapters — bridge kernel-owned infrastructure to the `AgentServices` traits.
//!
//! Each adapter wraps a concrete implementation from another crate and
//! implements the corresponding trait from `aos-agent`.

use std::sync::Arc;

use async_trait::async_trait;

use macaca_agent::{IpcService, MemoryService, PersistService};
use macaca_ipc::DynMessageSender;
use macaca_memory::{
    EmbeddingProvider, IsolatedMemoryManager, MemoryManager, RecallQuery, RecallResult,
    RememberText, VectorStore,
};
use macaca_persist::store::PersistStore;
use macaca_proto::{IpcMessage, MacacaResult, MemoryId};

// ── MemoryServiceAdapter ─────────────────────────────────────────────────────

/// Wraps a facade-capable memory backend as an `AgentServices::MemoryService`.
pub struct MemoryServiceAdapter<M> {
    memory: M,
}

impl<M> MemoryServiceAdapter<M> {
    pub fn new(memory: M) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl<V, E> MemoryService for MemoryServiceAdapter<MemoryManager<V, E>>
where
    V: VectorStore,
    E: EmbeddingProvider,
{
    async fn remember_text(&self, input: RememberText) -> MacacaResult<MemoryId> {
        self.memory.remember_text(input).await
    }

    async fn recall(&self, query: RecallQuery) -> MacacaResult<RecallResult> {
        self.memory.recall(query).await
    }
}

#[async_trait]
impl<V, E> MemoryService for MemoryServiceAdapter<IsolatedMemoryManager<V, E>>
where
    V: VectorStore,
    E: EmbeddingProvider,
{
    async fn remember_text(&self, input: RememberText) -> MacacaResult<MemoryId> {
        self.memory.remember_text(input).await
    }

    async fn recall(&self, query: RecallQuery) -> MacacaResult<RecallResult> {
        self.memory.recall(query).await
    }
}

// ── IpcServiceAdapter ────────────────────────────────────────────────────────

/// Wraps any `bus::MessageSender` as an `AgentServices::IpcService`.
pub struct IpcServiceAdapter {
    sender: DynMessageSender,
}

impl IpcServiceAdapter {
    pub fn new(sender: DynMessageSender) -> Self {
        Self { sender }
    }
}

#[async_trait]
impl IpcService for IpcServiceAdapter {
    async fn send(&self, msg: IpcMessage) -> MacacaResult<()> {
        self.sender.send(msg).await
    }
}

// ── PersistServiceAdapter ────────────────────────────────────────────────────

/// Wraps a `PersistStore` as an `AgentServices::PersistService`.
///
/// Keys are scoped to the agent by prefixing with `agent/{agent_id}/`.
pub struct PersistServiceAdapter {
    store: Arc<dyn PersistStore>,
    key_prefix: String,
}

impl PersistServiceAdapter {
    /// Create with a key prefix for agent isolation.
    pub fn new(store: Arc<dyn PersistStore>, agent_id: &macaca_proto::AgentId) -> Self {
        Self {
            store,
            key_prefix: format!("agent/{}/", agent_id),
        }
    }

    fn scoped_key(&self, key: &str) -> String {
        format!("{}{}", self.key_prefix, key)
    }
}

#[async_trait]
impl PersistService for PersistServiceAdapter {
    async fn save(&self, key: &str, data: &[u8]) -> MacacaResult<()> {
        self.store.set(&self.scoped_key(key), data).await
    }

    async fn load(&self, key: &str) -> MacacaResult<Option<Vec<u8>>> {
        self.store.get(&self.scoped_key(key)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::{AgentId, ApplicationId};
    use std::time::Duration;

    #[tokio::test]
    async fn memory_service_adapter() {
        let dir = tempfile::tempdir().unwrap();
        let factory = macaca_memory::MemoryBackendFactory::new(
            macaca_memory::MemoryBackendConfig::new(dir.path().to_path_buf())
                .session_ttl(Duration::from_secs(60)),
        );
        let adapter = MemoryServiceAdapter::new(factory.test_manager());

        let id = adapter
            .remember_text(macaca_memory::RememberText::new("test memory"))
            .await
            .unwrap();
        let results = adapter
            .recall(macaca_memory::RecallQuery::new("test", 10))
            .await
            .unwrap();
        assert_eq!(results.entries.len(), 1);
        assert_eq!(results.entries[0].id, id);
    }

    #[tokio::test]
    async fn isolated_memory_service_adapter() {
        let dir = tempfile::tempdir().unwrap();
        let factory = macaca_memory::MemoryBackendFactory::new(
            macaca_memory::MemoryBackendConfig::new(dir.path().to_path_buf())
                .session_ttl(Duration::from_secs(60)),
        );
        let agent_id = AgentId::new();
        let adapter = MemoryServiceAdapter::new(
            factory.isolated_test_manager(ApplicationId::new(), agent_id),
        );

        let id = adapter
            .remember_text(macaca_memory::RememberText::new("isolated memory"))
            .await
            .unwrap();
        let results = adapter
            .recall(macaca_memory::RecallQuery::new("isolated", 10))
            .await
            .unwrap();
        assert_eq!(results.entries.len(), 1);
        assert_eq!(results.entries[0].id, id);
        assert_eq!(results.entries[0].agent_id, Some(agent_id));
    }

    #[tokio::test]
    async fn ipc_service_adapter() {
        let bus = macaca_ipc::LocalBus::new();
        let sender = macaca_ipc::IpcTransport::create_sender(&bus);
        let adapter = IpcServiceAdapter::new(sender);

        let msg = IpcMessage {
            id: macaca_proto::MessageId::new(),
            from: AgentId::new(),
            to: Some(AgentId::new()),
            topic: "test".into(),
            payload: serde_json::json!({"text": "hello"}),
            timestamp: Utc::now(),
        };

        // Should not error (message goes to broadcast channel).
        adapter.send(msg).await.unwrap();
    }

    #[tokio::test]
    async fn persist_service_adapter() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = macaca_persist::RedbStore::open(db_path).unwrap();
        let store: Arc<dyn PersistStore> = Arc::new(store);

        let agent_id = AgentId::new();
        let adapter = PersistServiceAdapter::new(store, &agent_id);

        adapter.save("state", b"checkpoint data").await.unwrap();
        let loaded = adapter.load("state").await.unwrap();
        assert_eq!(loaded, Some(b"checkpoint data".to_vec()));

        // Missing key
        let missing = adapter.load("nonexistent").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn persist_adapter_key_scoping() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = macaca_persist::RedbStore::open(db_path).unwrap();
        let store: Arc<dyn PersistStore> = Arc::new(store);

        let agent_a = AgentId::new();
        let agent_b = AgentId::new();

        let adapter_a = PersistServiceAdapter::new(Arc::clone(&store), &agent_a);
        let adapter_b = PersistServiceAdapter::new(Arc::clone(&store), &agent_b);

        adapter_a.save("key", b"data A").await.unwrap();
        adapter_b.save("key", b"data B").await.unwrap();

        // Each agent sees only its own data.
        assert_eq!(
            adapter_a.load("key").await.unwrap(),
            Some(b"data A".to_vec())
        );
        assert_eq!(
            adapter_b.load("key").await.unwrap(),
            Some(b"data B".to_vec())
        );
    }
}
