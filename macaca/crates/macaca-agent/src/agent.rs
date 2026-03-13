//! Agent trait and service-injection types.

use async_trait::async_trait;
use macaca_llm::LlmProvider;
use macaca_proto::{AgentId, AgentOutput, AgentState, MacacaResult, Capability, MemoryEntry, MemoryId, IpcMessage};
use macaca_tools::ToolSet;

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
        tools: &dyn ToolSet,
        services: &AgentServices,
    ) -> MacacaResult<AgentOutput>;
}
