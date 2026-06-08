//! Runtime view of a loaded application aggregate.
//!
//! `LoadedApp` pairs the parsed manifest with kernel-registered agent ids and
//! the current lifecycle status. It is a read model used by registry and runtime
//! surfaces after admission succeeds.

use macaca_proto::AgentId;

use super::core::AppStatus;
use super::manifest::AppManifest;

/// A loaded application with its associated agent ids and status.
#[derive(Debug, Clone)]
pub struct LoadedApp {
    /// The parsed manifest.
    pub manifest: AppManifest,
    /// Ids of agents registered in the kernel for this app.
    pub agent_ids: Vec<AgentId>,
    /// Current status.
    pub status: AppStatus,
}
