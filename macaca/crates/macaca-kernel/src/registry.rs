//! Agent registry — stores and manages registered agents and their manifests.

use macaca_agent::Agent;
use macaca_proto::{AgentId, AgentManifest, MacacaError, MacacaResult};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

// ── AgentEntry ────────────────────────────────────────────────────────────────

/// A registered agent paired with its manifest.
pub struct AgentEntry {
    pub agent: Box<dyn Agent>,
    pub manifest: AgentManifest,
}

// ── AgentRegistry ─────────────────────────────────────────────────────────────

/// Thread-safe registry of all agents known to the kernel.
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<AgentId, AgentEntry>>>,
    max_agents: usize,
}

impl AgentRegistry {
    /// Create a new registry with the given agent cap.
    pub fn new(max_agents: usize) -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            max_agents,
        }
    }

    /// Register an agent. Returns the agent's id on success.
    ///
    /// Fails with `MacacaError::Agent` if the registry is at capacity or the
    /// agent id is already registered.
    pub async fn register(
        &self,
        agent: Box<dyn Agent>,
        manifest: AgentManifest,
    ) -> MacacaResult<AgentId> {
        let mut map = self.agents.write().await;

        if map.len() >= self.max_agents {
            return Err(MacacaError::Agent(format!(
                "Registry is at capacity ({} agents)",
                self.max_agents
            )));
        }

        let id = manifest.id;
        if map.contains_key(&id) {
            return Err(MacacaError::Agent(format!(
                "Agent {} is already registered",
                id.0
            )));
        }

        map.insert(id, AgentEntry { agent, manifest });
        tracing::info!(agent_id = %id.0, "agent registered");
        Ok(id)
    }

    /// Unregister an agent by id.
    pub async fn unregister(&self, id: &AgentId) -> MacacaResult<()> {
        let mut map = self.agents.write().await;
        if map.remove(id).is_none() {
            return Err(MacacaError::NotFound(format!("Agent {} not found", id.0)));
        }
        tracing::info!(agent_id = %id.0, "agent unregistered");
        Ok(())
    }

    /// Return the manifest for the given agent id, if present.
    pub async fn get(&self, id: &AgentId) -> MacacaResult<Option<AgentManifest>> {
        let map = self.agents.read().await;
        Ok(map.get(id).map(|e| e.manifest.clone()))
    }

    /// Return manifests for all registered agents.
    pub async fn list(&self) -> Vec<AgentManifest> {
        let map = self.agents.read().await;
        map.values().map(|e| e.manifest.clone()).collect()
    }

    /// Number of currently registered agents.
    pub async fn count(&self) -> usize {
        self.agents.read().await.len()
    }

    /// Run a closure against the `AgentEntry` for the given id.
    ///
    /// Used internally by the kernel to invoke `agent.run(...)` while holding
    /// only a read lock.
    pub(crate) async fn with_agent<F, R>(&self, id: &AgentId, f: F) -> MacacaResult<R>
    where
        F: FnOnce(&AgentEntry) -> R,
    {
        let map = self.agents.read().await;
        let entry = map
            .get(id)
            .ok_or_else(|| MacacaError::NotFound(format!("Agent {} not found", id.0)))?;
        Ok(f(entry))
    }

    /// Obtain a read guard to the internal agent map.
    /// Used by the kernel for async agent execution.
    pub(crate) async fn agents_read(
        &self,
    ) -> tokio::sync::RwLockReadGuard<'_, HashMap<AgentId, AgentEntry>> {
        self.agents.read().await
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use macaca_agent::AgentServices;
    use macaca_llm::LlmProvider;
    use macaca_proto::{
        AgentOutput, AgentState, Capability, Permission, PermissionLevel, TokenUsage,
    };
    use macaca_tools::ToolSet;

    fn make_manifest(id: AgentId, state: AgentState) -> AgentManifest {
        AgentManifest {
            id,
            name: "test-agent".into(),
            capabilities: vec![],
            permission: Permission {
                level: PermissionLevel::User,
                allowed_tools: vec![],
                allowed_paths: vec![],
                network_access: false,
            },
            state,
            created_at: Utc::now(),
            model: String::new(),
        }
    }

    struct MockAgent {
        id: AgentId,
        state: AgentState,
    }

    #[async_trait]
    impl Agent for MockAgent {
        fn id(&self) -> AgentId {
            self.id
        }
        fn capabilities(&self) -> &[Capability] {
            &[]
        }
        fn state(&self) -> AgentState {
            self.state
        }
        async fn run(
            &self,
            _llm: &dyn LlmProvider,
            _tools: &dyn ToolSet,
            _services: &AgentServices,
        ) -> MacacaResult<AgentOutput> {
            Ok(AgentOutput {
                result: "ok".into(),
                artifacts: vec![],
                tokens_used: TokenUsage {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                },
            })
        }
    }

    fn mock_agent(state: AgentState) -> (AgentId, Box<dyn Agent>, AgentManifest) {
        let id = AgentId::new();
        let agent = Box::new(MockAgent { id, state });
        let manifest = make_manifest(id, state);
        (id, agent, manifest)
    }

    #[tokio::test]
    async fn register_and_count() {
        let reg = AgentRegistry::new(10);
        let (id, agent, manifest) = mock_agent(AgentState::Running);
        let returned_id = reg.register(agent, manifest).await.unwrap();
        assert_eq!(returned_id, id);
        assert_eq!(reg.count().await, 1);
    }

    #[tokio::test]
    async fn get_returns_manifest() {
        let reg = AgentRegistry::new(10);
        let (id, agent, manifest) = mock_agent(AgentState::Running);
        reg.register(agent, manifest).await.unwrap();
        let got = reg.get(&id).await.unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().id, id);
    }

    #[tokio::test]
    async fn unregister_removes_agent() {
        let reg = AgentRegistry::new(10);
        let (id, agent, manifest) = mock_agent(AgentState::Running);
        reg.register(agent, manifest).await.unwrap();
        reg.unregister(&id).await.unwrap();
        assert_eq!(reg.count().await, 0);
    }

    #[tokio::test]
    async fn unregister_missing_is_error() {
        let reg = AgentRegistry::new(10);
        let id = AgentId::new();
        let err = reg.unregister(&id).await.unwrap_err();
        assert!(matches!(err, MacacaError::NotFound(_)));
    }

    #[tokio::test]
    async fn max_agents_enforced() {
        let reg = AgentRegistry::new(1);
        let (_, agent1, manifest1) = mock_agent(AgentState::Running);
        let (_, agent2, manifest2) = mock_agent(AgentState::Running);
        reg.register(agent1, manifest1).await.unwrap();
        let err = reg.register(agent2, manifest2).await.unwrap_err();
        assert!(matches!(err, MacacaError::Agent(_)));
    }

    #[tokio::test]
    async fn duplicate_registration_is_error() {
        let reg = AgentRegistry::new(10);
        let id = AgentId::new();
        let manifest = make_manifest(id, AgentState::Running);
        let agent1 = Box::new(MockAgent {
            id,
            state: AgentState::Running,
        });
        let agent2 = Box::new(MockAgent {
            id,
            state: AgentState::Running,
        });
        reg.register(agent1, manifest.clone()).await.unwrap();
        let err = reg.register(agent2, manifest).await.unwrap_err();
        assert!(matches!(err, MacacaError::Agent(_)));
    }

    #[tokio::test]
    async fn list_returns_all_manifests() {
        let reg = AgentRegistry::new(10);
        for _ in 0..3 {
            let (_, agent, manifest) = mock_agent(AgentState::Running);
            reg.register(agent, manifest).await.unwrap();
        }
        assert_eq!(reg.list().await.len(), 3);
    }
}
