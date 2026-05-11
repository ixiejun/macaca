//! Repository abstractions for Plugin Control Plane.
//!
//! The repository uses the Repository pattern to isolate durable control state
//! from runtime orchestration.  The first implementation is deterministic and
//! in-memory for tests and local hosts; later persistence-backed repositories
//! can keep the same trait while storing records on disk or in a service.

use async_trait::async_trait;
use macaca_proto::{
    PluginControlRecord, PluginError, PluginId, PluginRepositoryDescriptor, PluginRepositoryKind,
};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Snapshot returned for diagnostics and shell read models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRepositorySnapshot {
    pub repositories: Vec<PluginRepositoryDescriptor>,
    pub records: Vec<PluginControlRecord>,
}

/// Mutation summary emitted by repository operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRepositoryMutation {
    pub plugin_id: PluginId,
    pub operation: String,
}

/// Replaceable plugin repository boundary.
#[async_trait]
pub trait PluginRepository: Send + Sync {
    async fn list(&self, include_disabled: bool) -> Result<Vec<PluginControlRecord>, PluginError>;
    async fn get(&self, plugin_id: &PluginId) -> Result<Option<PluginControlRecord>, PluginError>;
    async fn put(
        &self,
        record: PluginControlRecord,
        operation: &str,
    ) -> Result<PluginRepositoryMutation, PluginError>;
    async fn remove(&self, plugin_id: &PluginId) -> Result<PluginRepositoryMutation, PluginError>;
    async fn snapshot(&self) -> Result<PluginRepositorySnapshot, PluginError>;
}

/// Deterministic in-memory repository used by runtime-host tests and embedded
/// hosts that have not configured durable plugin metadata storage yet.
pub struct InMemoryPluginRepository {
    descriptor: PluginRepositoryDescriptor,
    records: RwLock<std::collections::BTreeMap<PluginId, PluginControlRecord>>,
}

impl Default for InMemoryPluginRepository {
    fn default() -> Self {
        Self::new("default", PluginRepositoryKind::UserLocal)
    }
}

impl InMemoryPluginRepository {
    /// Create one repository descriptor with a stable id and kind.
    pub fn new(repository_id: impl Into<String>, kind: PluginRepositoryKind) -> Self {
        Self {
            descriptor: PluginRepositoryDescriptor {
                repository_id: repository_id.into(),
                kind,
                enabled: true,
                writable: true,
                root_hint: None,
                metadata: Default::default(),
            },
            records: RwLock::new(Default::default()),
        }
    }
}

#[async_trait]
impl PluginRepository for InMemoryPluginRepository {
    async fn list(&self, include_disabled: bool) -> Result<Vec<PluginControlRecord>, PluginError> {
        let mut records = self
            .records
            .read()
            .await
            .values()
            .filter(|record| include_disabled || record.enabled)
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| left.plugin_id.cmp(&right.plugin_id));
        info!(
            repository_id = %self.descriptor.repository_id,
            records = records.len(),
            "plugin repository list completed"
        );
        Ok(records)
    }

    async fn get(&self, plugin_id: &PluginId) -> Result<Option<PluginControlRecord>, PluginError> {
        Ok(self.records.read().await.get(plugin_id).cloned())
    }

    async fn put(
        &self,
        record: PluginControlRecord,
        operation: &str,
    ) -> Result<PluginRepositoryMutation, PluginError> {
        let plugin_id = record.plugin_id.clone();
        self.records.write().await.insert(plugin_id.clone(), record);
        info!(
            repository_id = %self.descriptor.repository_id,
            plugin_id = %plugin_id,
            operation,
            "plugin repository record stored"
        );
        Ok(PluginRepositoryMutation {
            plugin_id,
            operation: operation.into(),
        })
    }

    async fn remove(&self, plugin_id: &PluginId) -> Result<PluginRepositoryMutation, PluginError> {
        let removed = self.records.write().await.remove(plugin_id);
        if removed.is_none() {
            warn!(plugin_id = %plugin_id, "plugin repository remove missed");
            return Err(PluginError::Unavailable(format!(
                "plugin not found: {plugin_id}"
            )));
        }
        info!(plugin_id = %plugin_id, "plugin repository record removed");
        Ok(PluginRepositoryMutation {
            plugin_id: plugin_id.clone(),
            operation: "remove".into(),
        })
    }

    async fn snapshot(&self) -> Result<PluginRepositorySnapshot, PluginError> {
        Ok(PluginRepositorySnapshot {
            repositories: vec![self.descriptor.clone()],
            records: self.list(true).await?,
        })
    }
}
