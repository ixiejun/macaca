//! ForkManager struct and constructors.
//!
//! Owns in-memory fork registry, hook channels, and optional persistence port.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use macaca_kernel::KernelPersistencePort;
use macaca_proto::{ApplicationId, ForkId};
use tracing::info;

use super::types::{ForkContext, HookEvent, MAX_PARALLEL_FORKS};

/// Fork Manager - manages lifecycle of all forks.
///
/// Fields are `pub(crate)` so sibling modules (`lifecycle`, `persistence`, etc.)
/// can implement behavior via extension `impl` blocks without exposing internals
/// outside the `fork_manager` module tree.
pub struct ForkManager {
    /// All active forks.
    pub(crate) forks: Arc<RwLock<HashMap<ForkId, ForkContext>>>,
    /// Maximum parallel forks allowed.
    pub(crate) max_parallel_forks: usize,
    /// Channel for hook event callbacks.
    pub(crate) hook_tx: mpsc::Sender<HookEvent>,
    /// Broadcast sender for hook events (multiple subscribers).
    pub(crate) hook_broadcast: tokio::sync::broadcast::Sender<HookEvent>,
    /// Optional persistence port for durability across restarts.
    pub(crate) store: Option<Arc<dyn KernelPersistencePort>>,
    /// Application ID used for key namespacing in the store.
    pub(crate) app_id: ApplicationId,
}

impl ForkManager {
    /// Create a new ForkManager.
    pub fn new() -> Self {
        let (hook_tx, _hook_rx) = mpsc::channel(100);
        let (hook_broadcast, _) = tokio::sync::broadcast::channel(256);
        Self {
            forks: Arc::new(RwLock::new(HashMap::new())),
            max_parallel_forks: MAX_PARALLEL_FORKS,
            hook_tx,
            hook_broadcast,
            store: None,
            app_id: macaca_proto::ApplicationId::new(),
        }
    }

    /// Create a new ForkManager with optional persistence.
    pub fn new_with_store(
        store: Option<Arc<dyn KernelPersistencePort>>,
        app_id: macaca_proto::ApplicationId,
    ) -> Self {
        let (hook_tx, _hook_rx) = mpsc::channel(100);
        let (hook_broadcast, _) = tokio::sync::broadcast::channel(256);
        if let Some(store) = &store {
            info!(
                app_id = %app_id,
                backend = store.backend_name(),
                "[FORK] Persistence enabled"
            );
        }
        Self {
            forks: Arc::new(RwLock::new(HashMap::new())),
            max_parallel_forks: MAX_PARALLEL_FORKS,
            hook_tx,
            hook_broadcast,
            store,
            app_id,
        }
    }
}

impl Default for ForkManager {
    fn default() -> Self {
        Self::new()
    }
}
