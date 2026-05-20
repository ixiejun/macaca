//! Service adapters — bridge kernel-owned infrastructure to the `AgentServices` traits.
//!
//! Each adapter wraps a concrete implementation from another crate and
//! implements the corresponding trait from `aos-agent`.  The persistence
//! adapter depends on a kernel-owned port, so the kernel exposes the agent
//! service contract without naming a database provider.

use std::sync::Arc;

use async_trait::async_trait;

use macaca_agent::{IpcService, PersistService};
use macaca_ipc::DynMessageSender;
use macaca_proto::{IpcMessage, MacacaResult};

use crate::persistence::KernelPersistencePort;

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

/// Wraps a `KernelPersistencePort` as an `AgentServices::PersistService`.
///
/// Keys are scoped to the agent by prefixing with `agent/{agent_id}/`.
pub struct PersistServiceAdapter {
    store: Arc<dyn KernelPersistencePort>,
    key_prefix: String,
}

impl PersistServiceAdapter {
    /// Create with a key prefix for agent isolation.
    ///
    /// The adapter is intentionally a small Bridge: application agents still
    /// speak the legacy `PersistService` trait, while kernel composition only
    /// needs the provider-neutral persistence port.
    pub fn new(store: Arc<dyn KernelPersistencePort>, agent_id: &macaca_proto::AgentId) -> Self {
        tracing::info!(
            agent_id = %agent_id,
            backend = store.backend_name(),
            "agent persistence service adapter initialized"
        );
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
    use macaca_proto::{AgentId, MacacaResult};
    use tokio::sync::RwLock;

    #[derive(Default)]
    struct TestPersistencePort {
        values: RwLock<std::collections::BTreeMap<String, Vec<u8>>>,
    }

    #[async_trait]
    impl KernelPersistencePort for TestPersistencePort {
        async fn get(&self, key: &str) -> MacacaResult<Option<Vec<u8>>> {
            Ok(self.values.read().await.get(key).cloned())
        }

        async fn set(&self, key: &str, value: &[u8]) -> MacacaResult<()> {
            self.values
                .write()
                .await
                .insert(key.to_string(), value.to_vec());
            Ok(())
        }

        async fn delete(&self, key: &str) -> MacacaResult<()> {
            self.values.write().await.remove(key);
            Ok(())
        }

        async fn list_keys(&self, prefix: &str) -> MacacaResult<Vec<String>> {
            Ok(self
                .values
                .read()
                .await
                .keys()
                .filter(|key| key.starts_with(prefix))
                .cloned()
                .collect())
        }

        fn backend_name(&self) -> &'static str {
            "test-agent-persistence-memory"
        }
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
        let store: Arc<dyn KernelPersistencePort> = Arc::new(TestPersistencePort::default());

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
        let store: Arc<dyn KernelPersistencePort> = Arc::new(TestPersistencePort::default());

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
