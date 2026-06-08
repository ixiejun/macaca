//! Kernel scheduling and agent lifecycle limits.

use serde::{Deserialize, Serialize};

/// Bounded kernel runtime limits (max agents, heartbeat cadence, timeouts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    pub max_agents: usize,
    pub heartbeat_interval_ms: u64,
    pub agent_timeout_ms: u64,
}
