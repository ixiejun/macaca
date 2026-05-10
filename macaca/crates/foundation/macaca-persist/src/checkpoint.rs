use std::sync::Arc;

use macaca_proto::error::{MacacaError, MacacaResult};
use macaca_proto::types::{AgentId, AgentManifest};

use crate::store::PersistStore;

const SNAPSHOT_PREFIX: &str = "snapshot/";

/// Snapshot memento for restore-critical session persistence state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub session_id: String,
    pub last_event_cursor: u64,
    pub checkpoint_key: Option<String>,
    pub coordinator_resume_key: Option<String>,
}

/// Builder-style additive entry for constructing checkpoint saves.
#[derive(Debug, Default, Clone)]
pub struct CheckpointBuilder {
    agent_id: Option<AgentId>,
    state: Option<AgentManifest>,
}

#[derive(Debug, Clone)]
pub struct CheckpointRecord {
    pub agent_id: AgentId,
    pub state: AgentManifest,
}

impl CheckpointBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn agent_id(mut self, agent_id: AgentId) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    pub fn state(mut self, state: AgentManifest) -> Self {
        self.state = Some(state);
        self
    }

    pub fn build(self) -> MacacaResult<CheckpointRecord> {
        let agent_id = self
            .agent_id
            .ok_or_else(|| MacacaError::Persist("checkpoint builder missing agent_id".into()))?;
        let state = self
            .state
            .ok_or_else(|| MacacaError::Persist("checkpoint builder missing state".into()))?;
        Ok(CheckpointRecord { agent_id, state })
    }
}

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
        self.save_record(
            CheckpointBuilder::new()
                .agent_id(*agent_id)
                .state(state.clone())
                .build()?,
        )
        .await
    }

    /// Persist a checkpoint record built through the additive builder entry.
    pub async fn save_record(&self, record: CheckpointRecord) -> MacacaResult<()> {
        let bytes = serde_json::to_vec(&record.state)?;
        self.store.set(&Self::key(&record.agent_id), &bytes).await
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
