use std::sync::Arc;

use macaca_proto::error::{MacacaError, MacacaResult};
use macaca_proto::types::{AgentId, AgentManifest};

use crate::store::PersistStore;

const SNAPSHOT_PREFIX: &str = "snapshot/";

/// Manages serialized [`AgentManifest`] snapshots in a [`PersistStore`].
pub struct CheckpointManager {
    store: Arc<dyn PersistStore>,
}

impl CheckpointManager {
    pub fn new(store: Arc<dyn PersistStore>) -> Self {
        Self { store }
    }

    fn key(agent_id: &AgentId) -> String {
        format!("{}{}", SNAPSHOT_PREFIX, agent_id.0)
    }

    /// Serialize and persist an agent state snapshot.
    pub async fn save_snapshot(
        &self,
        agent_id: &AgentId,
        state: &AgentManifest,
    ) -> MacacaResult<()> {
        let bytes = serde_json::to_vec(state)?;
        self.store.set(&Self::key(agent_id), &bytes).await
    }

    /// Load and deserialize an agent state snapshot, returning `None` if absent.
    pub async fn load_snapshot(&self, agent_id: &AgentId) -> MacacaResult<Option<AgentManifest>> {
        match self.store.get(&Self::key(agent_id)).await? {
            None => Ok(None),
            Some(bytes) => {
                let manifest = serde_json::from_slice(&bytes)?;
                Ok(Some(manifest))
            }
        }
    }

    /// Return the [`AgentId`]s of all stored snapshots.
    pub async fn list_snapshots(&self) -> MacacaResult<Vec<AgentId>> {
        let keys = self.store.list_keys(SNAPSHOT_PREFIX).await?;
        let mut ids = Vec::with_capacity(keys.len());
        for key in keys {
            let uuid_str = key
                .strip_prefix(SNAPSHOT_PREFIX)
                .ok_or_else(|| MacacaError::Persist(format!("unexpected key format: {key}")))?;
            let uuid = uuid::Uuid::parse_str(uuid_str)
                .map_err(|e| MacacaError::Persist(format!("invalid uuid in key {key}: {e}")))?;
            ids.push(AgentId(uuid));
        }
        Ok(ids)
    }

    /// Delete a stored snapshot.
    pub async fn delete_snapshot(&self, agent_id: &AgentId) -> MacacaResult<()> {
        self.store.delete(&Self::key(agent_id)).await
    }
}
