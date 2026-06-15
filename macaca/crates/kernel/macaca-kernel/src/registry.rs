//! Agent registry — manifest-only identity store for registered agents.

use macaca_proto::{AgentId, AgentManifest, MacacaError, MacacaResult};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Thread-safe manifest registry for all agents known to the kernel.
///
/// Runtime agent objects are resolved by upper-layer execution ports; the kernel
/// stores only declarative identity and metadata (K8s-style manifest registry).
pub struct AgentRegistry {
    manifests: RwLock<HashMap<AgentId, AgentManifest>>,
    max_agents: usize,
}

impl AgentRegistry {
    /// Create a new registry with the given agent cap.
    pub fn new(max_agents: usize) -> Self {
        Self {
            manifests: RwLock::new(HashMap::new()),
            max_agents,
        }
    }

    /// Register an agent manifest. Returns the agent id on success.
    pub async fn register(&self, manifest: AgentManifest) -> MacacaResult<AgentId> {
        let mut map = self.manifests.write().await;

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

        map.insert(id, manifest);
        tracing::info!(agent_id = %id.0, "agent manifest registered");
        Ok(id)
    }

    /// Unregister an agent by id.
    pub async fn unregister(&self, id: &AgentId) -> MacacaResult<()> {
        let mut map = self.manifests.write().await;
        if map.remove(id).is_none() {
            return Err(MacacaError::NotFound(format!("Agent {} not found", id.0)));
        }
        tracing::info!(agent_id = %id.0, "agent manifest unregistered");
        Ok(())
    }

    /// Return the manifest for the given agent id, if present.
    pub async fn get(&self, id: &AgentId) -> MacacaResult<Option<AgentManifest>> {
        let map = self.manifests.read().await;
        Ok(map.get(id).map(|m| m.clone()))
    }

    /// Return manifests for all registered agents.
    pub async fn list(&self) -> Vec<AgentManifest> {
        let map = self.manifests.read().await;
        map.values().cloned().collect()
    }

    /// Number of currently registered agents.
    pub async fn count(&self) -> usize {
        self.manifests.read().await.len()
    }

    /// Returns whether a manifest exists for the given id.
    pub async fn contains(&self, id: &AgentId) -> bool {
        self.manifests.read().await.contains_key(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use macaca_proto::{AgentState, Permission, PermissionLevel};

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

    #[tokio::test]
    async fn register_and_count() {
        let reg = AgentRegistry::new(10);
        let id = AgentId::new();
        let manifest = make_manifest(id, AgentState::Running);
        let returned_id = reg.register(manifest).await.unwrap();
        assert_eq!(returned_id, id);
        assert_eq!(reg.count().await, 1);
    }

    #[tokio::test]
    async fn get_returns_manifest() {
        let reg = AgentRegistry::new(10);
        let id = AgentId::new();
        let manifest = make_manifest(id, AgentState::Running);
        reg.register(manifest).await.unwrap();
        let got = reg.get(&id).await.unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().id, id);
    }

    #[tokio::test]
    async fn unregister_removes_agent() {
        let reg = AgentRegistry::new(10);
        let id = AgentId::new();
        let manifest = make_manifest(id, AgentState::Running);
        reg.register(manifest).await.unwrap();
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
        let id1 = AgentId::new();
        let id2 = AgentId::new();
        reg.register(make_manifest(id1, AgentState::Running))
            .await
            .unwrap();
        let err = reg
            .register(make_manifest(id2, AgentState::Running))
            .await
            .unwrap_err();
        assert!(matches!(err, MacacaError::Agent(_)));
    }

    #[tokio::test]
    async fn duplicate_registration_is_error() {
        let reg = AgentRegistry::new(10);
        let id = AgentId::new();
        let manifest = make_manifest(id, AgentState::Running);
        reg.register(manifest.clone()).await.unwrap();
        let err = reg.register(manifest).await.unwrap_err();
        assert!(matches!(err, MacacaError::Agent(_)));
    }

    #[tokio::test]
    async fn list_returns_all_manifests() {
        let reg = AgentRegistry::new(10);
        for _ in 0..3 {
            let id = AgentId::new();
            reg.register(make_manifest(id, AgentState::Running))
                .await
                .unwrap();
        }
        assert_eq!(reg.list().await.len(), 3);
    }
}
