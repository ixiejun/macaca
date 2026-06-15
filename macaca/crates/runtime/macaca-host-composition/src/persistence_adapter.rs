//! Host persistence adapters for composition roots.
//!
//! The kernel receives provider-neutral ports, while host composition owns the
//! concrete local persistence providers. This Adapter keeps the Redb store at
//! the runtime/host edge and gives shells only a ready-to-inject kernel port.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::MacacaResult;
use tracing::{debug, info};

use crate::kernel::KernelPersistencePort;
use crate::persist::{PersistStore, RedbStore};

/// Adapter from the Redb-backed persistence provider to the kernel port.
///
/// The adapter contains no application behavior. It forwards key-value Memento
/// operations and records traceable diagnostics around each operation family so
/// persistence ownership remains auditable.
pub struct RedbKernelPersistenceAdapter {
    store: Arc<RedbStore>,
}

impl RedbKernelPersistenceAdapter {
    /// Create an adapter over an already-opened Redb store.
    pub fn new(store: Arc<RedbStore>) -> Self {
        info!("redb kernel persistence adapter initialized");
        Self { store }
    }
}

#[async_trait]
impl KernelPersistencePort for RedbKernelPersistenceAdapter {
    async fn get(&self, key: &str) -> MacacaResult<Option<Vec<u8>>> {
        debug!(key, "redb kernel persistence get");
        self.store.get(key).await
    }

    async fn set(&self, key: &str, value: &[u8]) -> MacacaResult<()> {
        debug!(key, bytes = value.len(), "redb kernel persistence set");
        self.store.set(key, value).await
    }

    async fn delete(&self, key: &str) -> MacacaResult<()> {
        debug!(key, "redb kernel persistence delete");
        self.store.delete(key).await
    }

    async fn list_keys(&self, prefix: &str) -> MacacaResult<Vec<String>> {
        debug!(prefix, "redb kernel persistence list");
        self.store.list_keys(prefix).await
    }

    fn backend_name(&self) -> &'static str {
        "redb"
    }
}
