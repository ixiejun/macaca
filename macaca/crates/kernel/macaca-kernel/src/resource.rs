//! Resource mediation primitives for the microkernel facade.
//!
//! The resource manager records declared ownership of broad resource scopes.
//! It does not open files, start browsers, spawn drivers, or allocate network
//! sockets.  Concrete resource handling remains in system services; the kernel
//! only protects registration invariants and makes conflicts explicit.

use std::collections::BTreeSet;
use std::sync::RwLock;

use macaca_proto::{KernelPrimitiveError, KernelPrimitiveResult, ResourceScope};

/// Contract for registering and querying kernel-visible resource scopes.
pub trait ResourceManager: Send + Sync {
    /// Register a resource scope and reject duplicates.
    fn register_resource(&self, scope: ResourceScope) -> KernelPrimitiveResult<()>;

    /// Check whether a scope has already been registered.
    fn has_resource(&self, scope: &ResourceScope) -> KernelPrimitiveResult<bool>;

    /// List registered scopes in deterministic order.
    fn list_resources(&self) -> KernelPrimitiveResult<Vec<ResourceScope>>;
}

/// In-memory resource manager used by the Phase 01 facade.
///
/// The manager protects the duplicate-registration invariant that later
/// browser, workspace, driver, storage, and network services must preserve.
#[derive(Default)]
pub struct InMemoryResourceManager {
    resources: RwLock<BTreeSet<ResourceScope>>,
}

impl InMemoryResourceManager {
    /// Create an empty resource manager.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ResourceManager for InMemoryResourceManager {
    fn register_resource(&self, scope: ResourceScope) -> KernelPrimitiveResult<()> {
        let mut resources = self.resources.write().map_err(|_| {
            KernelPrimitiveError::Unavailable("resource manager lock poisoned".into())
        })?;
        if !resources.insert(scope.clone()) {
            return Err(KernelPrimitiveError::ResourceAlreadyRegistered(scope));
        }
        Ok(())
    }

    fn has_resource(&self, scope: &ResourceScope) -> KernelPrimitiveResult<bool> {
        let resources = self.resources.read().map_err(|_| {
            KernelPrimitiveError::Unavailable("resource manager lock poisoned".into())
        })?;
        Ok(resources.contains(scope))
    }

    fn list_resources(&self) -> KernelPrimitiveResult<Vec<ResourceScope>> {
        let resources = self.resources.read().map_err(|_| {
            KernelPrimitiveError::Unavailable("resource manager lock poisoned".into())
        })?;
        Ok(resources.iter().cloned().collect())
    }
}
