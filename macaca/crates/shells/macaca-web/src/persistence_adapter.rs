//! Persistence adapters for the Web composition root.
//!
//! The Web shell may choose local persistence providers while bootstrapping the
//! process, but kernel code must only receive provider-neutral ports.  This
//! Adapter keeps the concrete Redb store at the shell/runtime edge.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_sdk::kernel::KernelPersistencePort;
use macaca_sdk::runtime_host::persist::PersistStore;
use macaca_proto::MacacaResult;
use tracing::{debug, info};

/// Adapter from the Redb-backed persistence provider to the kernel port.
///
/// The adapter contains no application behavior.  It forwards key-value
/// memento operations and records traceable diagnostics around each operation
/// family so persistence ownership remains auditable.
pub struct RedbKernelPersistenceAdapter {
    store: Arc<macaca_sdk::runtime_host::persist::RedbStore>,
}

impl RedbKernelPersistenceAdapter {
    /// Create an adapter over an already-opened Redb store.
    pub fn new(store: Arc<macaca_sdk::runtime_host::persist::RedbStore>) -> Self {
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
