//! Hook subscription and fork query surface.

use macaca_proto::{ApplicationId, ForkId};

use super::manager::ForkManager;
use super::types::{ForkContext, HookEvent};

impl ForkManager {
    /// Get a fork by ID.
    pub async fn get_fork(&self, fork_id: ForkId) -> Option<ForkContext> {
        self.forks.read().await.get(&fork_id).cloned()
    }

    /// List all forks for an application.
    pub async fn list_forks(&self, application_id: &ApplicationId) -> Vec<ForkContext> {
        self.forks
            .read()
            .await
            .values()
            .filter(|f| f.application_id == *application_id)
            .cloned()
            .collect()
    }

    /// Subscribe to hook events (broadcast channel).
    ///
    /// Multiple subscribers can receive all hook events for:
    /// - Monitoring fork lifecycle
    /// - Notifying coordinators when delegated tasks complete
    /// - Validation feedback
    pub fn subscribe_to_hooks(&self) -> tokio::sync::broadcast::Receiver<HookEvent> {
        self.hook_broadcast.subscribe()
    }

    /// Emit a hook event to both internal channel and broadcast subscribers.
    pub(crate) fn emit_hook_event(&self, event: HookEvent) {
        // Send to broadcast channel (don't await, just try)
        let _ = self.hook_broadcast.send(event.clone());
    }
}
