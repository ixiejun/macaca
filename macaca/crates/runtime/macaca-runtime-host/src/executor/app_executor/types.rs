//! Application executor configuration and worker health value objects.
//!
//! These types are application-agnostic: they describe generic execution environment
//! settings and worker liveness semantics without binding to any specific application.

/// Worker state for tracking worker health.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    /// Worker is running normally
    Running,
    /// Command channel is closed, worker is waiting for recovery
    Disconnected,
    /// Worker has shut down gracefully
    Shutdown,
}

/// Worker health status for external health checks.
#[derive(Debug)]
pub enum WorkerHealth {
    /// Worker is healthy with recent heartbeat
    Healthy { last_heartbeat: std::time::Duration },
    /// Worker is unhealthy (no recent heartbeat)
    Unhealthy { reason: String },
    /// Worker's command channel is disconnected
    Disconnected,
    /// Worker has shut down
    Shutdown,
}

/// Configuration for an ApplicationExecutor.
#[derive(Debug, Clone)]
pub struct ApplicationExecutorConfig {
    /// Maximum number of parallel task executions.
    pub max_parallel: usize,
    /// Maximum queue size for pending tasks.
    pub max_queue_size: usize,
    /// Enable event bus for system-wide events.
    pub enable_events: bool,
}

impl Default for ApplicationExecutorConfig {
    fn default() -> Self {
        Self {
            max_parallel: 4,
            max_queue_size: 100,
            enable_events: true,
        }
    }
}

/// Configuration for the worker supervisor that handles automatic restart on failure.
#[derive(Debug, Clone)]
pub struct WorkerSupervisorConfig {
    /// Maximum number of restarts before giving up.
    pub max_restarts: u32,
    /// If the worker runs successfully for this many seconds, reset the restart counter.
    pub cooldown_reset_secs: u64,
    /// Milliseconds to wait before each restart attempt.
    pub restart_delay_ms: u64,
}

impl Default for WorkerSupervisorConfig {
    fn default() -> Self {
        Self {
            max_restarts: 5,
            cooldown_reset_secs: 300,
            restart_delay_ms: 1000,
        }
    }
}
