//! Fork persistence — restore after restart and terminal cleanup.
//!
//! Uses `KernelPersistencePort` so kernel owns replay semantics without importing
//! a concrete store implementation.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tracing::{error, info, warn};

use super::manager::ForkManager;
use super::types::ForkContext;

impl ForkManager {
    /// Restore non-terminal forks from the persistence store after a restart.
    pub async fn restore_forks(&mut self) {
        let store = match &self.store {
            Some(s) => Arc::clone(s),
            None => return,
        };

        let prefix = format!("fork/{}/", self.app_id.0);
        let keys = match store.list_keys(&prefix).await {
            Ok(k) => k,
            Err(e) => {
                error!(error = %e, "[FORK] Failed to list fork keys during restore");
                return;
            }
        };

        let mut forks = self.forks.write().await;
        let mut restored = 0usize;

        for key in &keys {
            let bytes = match store.get(key).await {
                Ok(Some(b)) => b,
                Ok(None) => continue,
                Err(e) => {
                    warn!(key = %key, error = %e, "[FORK] Failed to read fork during restore");
                    continue;
                }
            };

            match serde_json::from_slice::<ForkContext>(&bytes) {
                Ok(fork) => {
                    // Skip terminal forks — they don't need restoring
                    if fork.is_terminal() {
                        // Clean up stale terminal entries from store
                        if let Err(e) = store.delete(key).await {
                            warn!(key = %key, error = %e, "[FORK] Failed to clean up terminal fork from store");
                        }
                        continue;
                    }
                    forks.insert(fork.id, fork);
                    restored += 1;
                }
                Err(e) => {
                    warn!(key = %key, error = %e, "[FORK] Failed to deserialize fork during restore");
                }
            }
        }

        if restored > 0 {
            info!(restored = restored, app_id = %self.app_id.0, "[FORK] Restored forks from store");
        }
    }

    /// Clean up old merged forks.
    pub async fn cleanup(&self, older_than: Duration) -> usize {
        use chrono::TimeDelta;
        let now = Utc::now();
        let older_than_delta = TimeDelta::from_std(older_than).unwrap_or(TimeDelta::zero());
        let mut to_remove = vec![];

        {
            let forks = self.forks.read().await;
            for (id, fork) in forks.iter() {
                if fork.is_terminal() {
                    if let Some(completed_at) = fork.completed_at {
                        if (now - completed_at) > older_than_delta {
                            to_remove.push(*id);
                        }
                    }
                }
            }
        }

        let mut forks = self.forks.write().await;
        for id in &to_remove {
            forks.remove(id);
        }

        to_remove.len()
    }
}
