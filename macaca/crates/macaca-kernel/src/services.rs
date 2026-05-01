//! Service adapters — bridge kernel-owned infrastructure to the `AgentServices` traits.
//!
//! Each adapter wraps a concrete implementation from another crate and
//! implements the corresponding trait from `aos-agent`.

use std::sync::Arc;

use async_trait::async_trait;

use macaca_agent::{IpcService, MemoryService, PersistService};
use macaca_ipc::DynMessageSender;
use macaca_proto::{IpcMessage, MacacaResult, MemoryEntry, MemoryId};
use macaca_memory::store::MemoryStore;
use macaca_persist::store::PersistStore;

// ── MemoryServiceAdapter ─────────────────────────────────────────────────────

/// Wraps any `MemoryStore` as an `AgentServices::MemoryService`.
pub struct MemoryServiceAdapter<S: MemoryStore> {
    store: S,
}

impl<S: MemoryStore> MemoryServiceAdapter<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S: MemoryStore> MemoryService for MemoryServiceAdapter<S> {
    async fn store(&self, entry: MemoryEntry) -> MacacaResult<MemoryId> {
        self.store.store(entry).await
    }

    async fn retrieve(&self, query: &str, limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
        self.store.retrieve(query, limit).await
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
    use macaca_proto::{AgentId, MemoryId, MemoryLayer};
    use std::time::Duration;

    #[tokio::test]
    async fn memory_service_adapter() {
        let session = macaca_memory::SessionMemory::new(Duration::from_secs(60));
        let adapter = MemoryServiceAdapter::new(session);

        let entry = MemoryEntry {
            id: MemoryId::new(),
            layer: MemoryLayer::Session,
            content: "test memory".into(),
            metadata: serde_json::Value::Null,
            agent_id: None,
            created_at: Utc::now(),
            expires_at: None,
        };

        let id = adapter.store(entry).await.unwrap();
        let results = adapter.retrieve("test", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
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
