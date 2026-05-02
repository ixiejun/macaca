//! Agent trait and service-injection types.

use async_trait::async_trait;
use macaca_llm::LlmProvider;
use macaca_proto::{
    AgentId, AgentOutput, AgentState, Capability, IpcMessage, MacacaResult, MemoryEntry, MemoryId,
};
use macaca_tools::ToolCatalog;

static NOOP_MEMORY_SERVICE: NoopMemoryService = NoopMemoryService;
static NOOP_IPC_SERVICE: NoopIpcService = NoopIpcService;
static NOOP_PERSIST_SERVICE: NoopPersistService = NoopPersistService;

// ── Service injection traits ──────────────────────────────────────────────────

/// Stores and retrieves memories for an agent.
#[async_trait]
pub trait MemoryService: Send + Sync {
    /// Store a memory entry. Returns its ID.
    async fn store(&self, entry: MemoryEntry) -> MacacaResult<MemoryId>;
    /// Retrieve memories matching a query.
    async fn retrieve(&self, query: &str, limit: usize) -> MacacaResult<Vec<MemoryEntry>>;
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
    async fn store(&self, _entry: MemoryEntry) -> MacacaResult<MemoryId> {
        Ok(MemoryId::new())
    }

    async fn retrieve(&self, _query: &str, _limit: usize) -> MacacaResult<Vec<MemoryEntry>> {
        Ok(Vec::new())
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
    /// Create an empty services bundle (no services attached).
    pub fn empty() -> Self {
        Self {
            memory: None,
            ipc: None,
            persist: None,
        }
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
        Self::empty()
    }
}

// ── Agent trait ───────────────────────────────────────────────────────────────

/// The core trait every Agent OS agent must implement.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Unique identifier for this agent instance.
    fn id(&self) -> AgentId;

    /// The capabilities this agent declares.
    fn capabilities(&self) -> &[Capability];

    /// Current lifecycle state of the agent.
    fn state(&self) -> AgentState;

    /// Execute the agent's main logic.
    async fn run(
        &self,
        llm: &dyn LlmProvider,
        tools: &dyn ToolCatalog,
        services: &AgentServices,
    ) -> MacacaResult<AgentOutput>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{AgentId, MemoryLayer, MessageId};

    #[tokio::test]
    async fn empty_services_use_noop_fallbacks() {
        let services = AgentServices::empty();

        let memory_id = services
            .memory_service()
            .store(MemoryEntry {
                id: MemoryId::new(),
                layer: MemoryLayer::Session,
                content: "noop".into(),
                metadata: serde_json::json!({}),
                agent_id: None,
                created_at: chrono::Utc::now(),
                expires_at: None,
            })
            .await
            .unwrap();
        let memories = services.memory_service().retrieve("noop", 5).await.unwrap();
        let persist = services.persist_service().load("missing").await.unwrap();
        services
            .ipc_service()
            .send(IpcMessage {
                id: MessageId::new(),
                from: AgentId::new(),
                to: Some(AgentId::new()),
                topic: "noop".into(),
                payload: serde_json::json!({ "kind": "noop" }),
                timestamp: chrono::Utc::now(),
            })
            .await
            .unwrap();

        assert_ne!(memory_id, MemoryId::default());
        assert!(memories.is_empty());
        assert!(persist.is_none());
    }
}
