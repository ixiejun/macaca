//! SDK status client boundary for shell-facing system status snapshots.
//!
//! Status reads are intentionally modeled as a client Strategy so CLI, Web,
//! tests, and future remote shells can use the same command-free snapshot API
//! without depending on Web process state.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use macaca_kernel::Kernel;
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

/// Backward-compatible alias for the pre-S3 status data-source name.
pub trait SystemStatusDataSource: SystemStatusClient {}

impl<T> SystemStatusDataSource for T where T: SystemStatusClient {}

/// Adapter that reads status from a prepared immutable snapshot.
pub struct StaticSystemStatusDataSource {
    snapshot: SystemStatusSnapshot,
}

impl StaticSystemStatusDataSource {
    /// Create a status source from an already prepared snapshot.
    pub fn new(snapshot: SystemStatusSnapshot) -> Self {
        Self { snapshot }
    }
}

#[async_trait]
impl SystemStatusClient for StaticSystemStatusDataSource {
    async fn status_snapshot(&self) -> MacacaResult<SystemStatusSnapshot> {
        info!("sdk status client returning static status snapshot");
        let snapshot = self.snapshot.clone();
        if snapshot.max_agents == 0 {
            warn!("sdk status client snapshot reported zero max_agents");
        }
        Ok(snapshot)
    }
}

/// Build a CLI-oriented status snapshot from lower-layer runtime objects.
///
/// This helper is still a compatibility adapter: it reads existing local
/// runtime values but does not construct providers or define presentation
/// output. CLI formatting remains outside the SDK.
pub async fn kernel_status_snapshot(
    kernel: &Kernel,
    loaded_apps: usize,
    max_agents: usize,
    llm_provider: impl Into<String>,
    gateway_enabled: bool,
) -> SystemStatusSnapshot {
    SystemStatusSnapshot {
        version: env!("CARGO_PKG_VERSION").into(),
        agent_count: kernel.agent_count().await,
        loaded_apps,
        max_agents,
        llm_provider: llm_provider.into(),
        app_runtime: "macaca-app/AppRuntime".into(),
        gateway_enabled,
    }
}
