//! Agent service injection primitives.

use async_trait::async_trait;
use macaca_memory::{RecallQuery, RecallResult, RememberText};
use macaca_proto::{IpcMessage, MacacaResult, MemoryId};

static NOOP_MEMORY_SERVICE: NoopMemoryService = NoopMemoryService;
static NOOP_IPC_SERVICE: NoopIpcService = NoopIpcService;
static NOOP_PERSIST_SERVICE: NoopPersistService = NoopPersistService;

// ── Service injection traits ──────────────────────────────────────────────────

/// Stores and retrieves memories for an agent.
#[async_trait]
pub trait MemoryService: Send + Sync {
    /// Remember text through the canonical memory facade.
    async fn remember_text(&self, input: RememberText) -> MacacaResult<MemoryId>;

    /// Recall memories through the canonical memory facade.
    async fn recall(&self, query: RecallQuery) -> MacacaResult<RecallResult>;
}

/// Sends IPC messages to other agents or topics.
#[async_trait]
pub trait IpcService: Send + Sync {
    /// Send a message to another agent or topic.
    async fn send(&self, msg: IpcMessage) -> MacacaResult<()>;
}

/// Persists and restores agent state as key-value checkpoints.
#[async_trait]
pub trait PersistService: Send + Sync {
    /// Save data under a key.
    async fn save(&self, key: &str, data: &[u8]) -> MacacaResult<()>;
    /// Load data by key.
    async fn load(&self, key: &str) -> MacacaResult<Option<Vec<u8>>>;
}

/// Default no-op memory service used when no memory backend is attached.
pub struct NoopMemoryService;

#[async_trait]
impl MemoryService for NoopMemoryService {
    async fn remember_text(&self, _input: RememberText) -> MacacaResult<MemoryId> {
        Ok(MemoryId::new())
    }

    async fn recall(&self, _query: RecallQuery) -> MacacaResult<RecallResult> {
        Ok(RecallResult::new(Vec::new()))
    }
}

/// Default no-op IPC service used when no IPC backend is attached.
pub struct NoopIpcService;

#[async_trait]
impl IpcService for NoopIpcService {
    async fn send(&self, _msg: IpcMessage) -> MacacaResult<()> {
        Ok(())
    }
}

/// Default no-op persist service used when no persistence backend is attached.
pub struct NoopPersistService;

#[async_trait]
impl PersistService for NoopPersistService {
    async fn save(&self, _key: &str, _data: &[u8]) -> MacacaResult<()> {
        Ok(())
    }

    async fn load(&self, _key: &str) -> MacacaResult<Option<Vec<u8>>> {
        Ok(None)
    }
}

// ── AgentServices ─────────────────────────────────────────────────────────────

/// Optional kernel-provided services injected into an agent at run time.
pub struct AgentServices {
    pub memory: Option<Box<dyn MemoryService>>,
    pub ipc: Option<Box<dyn IpcService>>,
    pub persist: Option<Box<dyn PersistService>>,
}

impl AgentServices {
    /// Create the canonical builder for service bundles.
    pub fn builder() -> AgentServicesBuilder {
        AgentServicesBuilder::new()
    }

    /// Return the configured memory service or a no-op fallback.
    pub fn memory_service(&self) -> &dyn MemoryService {
        self.memory.as_deref().unwrap_or(&NOOP_MEMORY_SERVICE)
    }

    /// Return the configured IPC service or a no-op fallback.
    pub fn ipc_service(&self) -> &dyn IpcService {
        self.ipc.as_deref().unwrap_or(&NOOP_IPC_SERVICE)
    }

    /// Return the configured persistence service or a no-op fallback.
    pub fn persist_service(&self) -> &dyn PersistService {
        self.persist.as_deref().unwrap_or(&NOOP_PERSIST_SERVICE)
    }
}

impl Default for AgentServices {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// Builder for [`AgentServices`].
pub struct AgentServicesBuilder {
    memory: Option<Box<dyn MemoryService>>,
    ipc: Option<Box<dyn IpcService>>,
    persist: Option<Box<dyn PersistService>>,
}

impl AgentServicesBuilder {
    pub fn new() -> Self {
        Self {
            memory: None,
            ipc: None,
            persist: None,
        }
    }

    pub fn memory(mut self, memory: Box<dyn MemoryService>) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn ipc(mut self, ipc: Box<dyn IpcService>) -> Self {
        self.ipc = Some(ipc);
        self
    }

    pub fn persist(mut self, persist: Box<dyn PersistService>) -> Self {
        self.persist = Some(persist);
        self
    }

    pub fn build(self) -> AgentServices {
        AgentServices {
            memory: self.memory,
            ipc: self.ipc,
            persist: self.persist,
        }
    }
}

impl Default for AgentServicesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{AgentId, MemoryEntry, MemoryLayer, MessageId};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    fn memory_entry(content: &str) -> MemoryEntry {
        MemoryEntry {
            id: MemoryId::new(),
            layer: MemoryLayer::Session,
            content: content.into(),
            metadata: serde_json::json!({}),
            agent_id: None,
            created_at: chrono::Utc::now(),
            expires_at: None,
        }
    }

    fn ipc_message() -> IpcMessage {
        IpcMessage {
            id: MessageId::new(),
            from: AgentId::new(),
            to: Some(AgentId::new()),
            topic: "noop".into(),
            payload: serde_json::json!({ "kind": "noop" }),
            timestamp: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn empty_services_use_noop_fallbacks() {
        let services = AgentServices::builder().build();

        let memory_id = services
            .memory_service()
            .remember_text(RememberText::new("noop"))
            .await
            .unwrap();
        let memories = services
            .memory_service()
            .recall(RecallQuery::new("noop", 5))
            .await
            .unwrap();
        let persist = services.persist_service().load("missing").await.unwrap();
        services.ipc_service().send(ipc_message()).await.unwrap();

        assert_ne!(memory_id, MemoryId::default());
        assert!(memories.entries.is_empty());
        assert!(persist.is_none());
    }

    struct RecordingMemoryService {
        called: Arc<AtomicBool>,
    }

    #[async_trait]
    impl MemoryService for RecordingMemoryService {
        async fn remember_text(&self, _input: RememberText) -> MacacaResult<MemoryId> {
            self.called.store(true, Ordering::SeqCst);
            Ok(MemoryId::new())
        }

        async fn recall(&self, _query: RecallQuery) -> MacacaResult<RecallResult> {
            self.called.store(true, Ordering::SeqCst);
            Ok(RecallResult::new(vec![memory_entry("recorded")]))
        }
    }

    #[tokio::test]
    async fn builder_with_memory_service_uses_provided_service() {
        let called = Arc::new(AtomicBool::new(false));
        let services = AgentServices::builder()
            .memory(Box::new(RecordingMemoryService {
                called: Arc::clone(&called),
            }))
            .build();

        let memories = services
            .memory_service()
            .recall(RecallQuery::new("x", 1))
            .await
            .unwrap();

        assert!(called.load(Ordering::SeqCst));
        assert_eq!(memories.entries.len(), 1);
        assert_eq!(memories.entries[0].content, "recorded");
    }

    struct RecordingIpcService {
        called: Arc<AtomicBool>,
    }

    #[async_trait]
    impl IpcService for RecordingIpcService {
        async fn send(&self, _msg: IpcMessage) -> MacacaResult<()> {
            self.called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn builder_with_ipc_service_uses_provided_service() {
        let called = Arc::new(AtomicBool::new(false));
        let services = AgentServices::builder()
            .ipc(Box::new(RecordingIpcService {
                called: Arc::clone(&called),
            }))
            .build();

        services.ipc_service().send(ipc_message()).await.unwrap();

        assert!(called.load(Ordering::SeqCst));
    }

    struct RecordingPersistService {
        called: Arc<AtomicBool>,
    }

    #[async_trait]
    impl PersistService for RecordingPersistService {
        async fn save(&self, _key: &str, _data: &[u8]) -> MacacaResult<()> {
            self.called.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn load(&self, _key: &str) -> MacacaResult<Option<Vec<u8>>> {
            self.called.store(true, Ordering::SeqCst);
            Ok(Some(b"recorded".to_vec()))
        }
    }

    #[tokio::test]
    async fn builder_with_persist_service_uses_provided_service() {
        let called = Arc::new(AtomicBool::new(false));
        let services = AgentServices::builder()
            .persist(Box::new(RecordingPersistService {
                called: Arc::clone(&called),
            }))
            .build();

        let data = services.persist_service().load("x").await.unwrap();

        assert!(called.load(Ordering::SeqCst));
        assert_eq!(data, Some(b"recorded".to_vec()));
    }
}
