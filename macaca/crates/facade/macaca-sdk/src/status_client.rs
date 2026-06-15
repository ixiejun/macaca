//! SDK status client boundary for shell-facing system status snapshots.
//!
//! Status reads are intentionally modeled as a client Strategy so CLI, Web,
//! tests, and future remote shells can use the same command-free snapshot API
//! without depending on Web process state.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use macaca_proto::MacacaResult;

/// Small system status snapshot used by CLI status-like commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemStatusSnapshot {
    pub version: String,
    pub agent_count: usize,
    pub loaded_apps: usize,
    pub max_agents: usize,
    pub llm_provider: String,
    pub app_runtime: String,
    pub gateway_enabled: bool,
}

/// Replaceable data source for CLI/system status inspection.
#[async_trait]
pub trait SystemStatusClient: Send + Sync {
    /// Return one shell-facing status snapshot.
    async fn status_snapshot(&self) -> MacacaResult<SystemStatusSnapshot>;
}

/// Adapter that reads status from a prepared immutable snapshot.
pub struct StaticSystemStatusClient {
    snapshot: SystemStatusSnapshot,
}

impl StaticSystemStatusClient {
    /// Create a status source from an already prepared snapshot.
    pub fn new(snapshot: SystemStatusSnapshot) -> Self {
        Self { snapshot }
    }
}

#[async_trait]
impl SystemStatusClient for StaticSystemStatusClient {
    async fn status_snapshot(&self) -> MacacaResult<SystemStatusSnapshot> {
        info!("sdk status client returning static status snapshot");
        let snapshot = self.snapshot.clone();
        if snapshot.max_agents == 0 {
            warn!("sdk status client snapshot reported zero max_agents");
        }
        Ok(snapshot)
    }
}
