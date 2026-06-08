//! Constructor surface for `YamlApplicationManifestAdapter`.
//!
//! Factory methods follow the **Builder** pattern: callers construct an adapter
//! from a parsed legacy manifest and optionally attach resolved agents before
//! invoking projection.

use macaca_sdk::AgentConfig;

use crate::model::AppManifest;

use super::types::YamlApplicationManifestAdapter;

impl YamlApplicationManifestAdapter {
    /// Create an adapter from the parsed legacy YAML manifest.
    ///
    /// Resolved agents are empty until the caller invokes
    /// [`with_resolved_agents`](Self::with_resolved_agents).
    pub fn new(manifest: AppManifest) -> Self {
        Self {
            manifest,
            resolved_agents: Vec::new(),
        }
    }

    /// Attach resolved file-based agents when the caller has already performed
    /// legacy file resolution.
    ///
    /// The adapter uses resolved facts for capabilities and permissions but
    /// never stores raw agent configuration bodies in Manifest v1 metadata.
    pub fn with_resolved_agents(mut self, resolved_agents: Vec<AgentConfig>) -> Self {
        self.resolved_agents = resolved_agents;
        self
    }
}
