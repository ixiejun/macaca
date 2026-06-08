//! Graceful shutdown and worker health inspection.
//!
//! Health checks combine worker state (Running/Disconnected/Shutdown) with
//! heartbeat recency to surface liveness for external monitors.

use std::sync::atomic::Ordering as AtomicOrdering;
use tracing::info;

use crate::executor::ExecutorCommand;

use super::executor::ApplicationExecutor;
use super::types::{WorkerHealth, WorkerState};

impl ApplicationExecutor {
    /// Shutdown the executor gracefully.
    pub async fn shutdown(&self) {
        // Set the atomic flag first so the supervisor won't restart the worker.
        self.shutdown_requested.store(true, AtomicOrdering::SeqCst);
        *self.shutdown.write().await = true;
        let _ = self
            .command_tx
            .read()
            .await
            .send(ExecutorCommand::Shutdown)
            .await;
        info!(application_id = %self.application_id, "Executor shutdown initiated");
    }

    /// Check if worker is healthy.
    ///
    /// Returns the current health status of the worker based on:
    /// - Current worker state (Running/Disconnected/Shutdown)
    /// - Time since last heartbeat
    pub async fn check_worker_health(&self) -> WorkerHealth {
        let state = self.worker_state.read().await;
        match *state {
            WorkerState::Running => {
                let elapsed = self.worker_heartbeat.read().await.elapsed();
                if elapsed < std::time::Duration::from_secs(30) {
                    WorkerHealth::Healthy {
                        last_heartbeat: elapsed,
                    }
                } else {
                    WorkerHealth::Unhealthy {
                        reason: format!("No heartbeat for {:?}", elapsed),
                    }
                }
            }
            WorkerState::Disconnected => WorkerHealth::Disconnected,
            WorkerState::Shutdown => WorkerHealth::Shutdown,
        }
    }

    /// Check if worker is healthy (simple boolean).
    pub async fn is_worker_healthy(&self) -> bool {
        matches!(
            self.check_worker_health().await,
            WorkerHealth::Healthy { .. }
        )
    }

    /// Return how many times the worker has been restarted by the supervisor.
    pub fn restart_count(&self) -> u32 {
        self.restart_count.load(AtomicOrdering::SeqCst)
    }
}
