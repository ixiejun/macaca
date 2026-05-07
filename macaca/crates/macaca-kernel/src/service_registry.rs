//! System service registry primitives for the microkernel facade.
//!
//! The registry records service identity and scope only.  It intentionally
//! avoids storing provider clients, driver handles, gateway transports, or
//! application workflows so the kernel remains a stable invariant layer.

use std::collections::BTreeMap;
use std::sync::RwLock;

use macaca_proto::{KernelPrimitiveError, KernelPrimitiveResult, KernelServiceId, ServiceScope};

/// Registry contract for provider-neutral system services.
pub trait SystemServiceRegistry: Send + Sync {
    /// Register a service id and its scope.
    fn register_service(
        &self,
        id: KernelServiceId,
        scope: ServiceScope,
    ) -> KernelPrimitiveResult<()>;

    /// Look up the scope associated with a service id.
    fn get_service_scope(
        &self,
        id: &KernelServiceId,
    ) -> KernelPrimitiveResult<Option<ServiceScope>>;

    /// Return every registered service in deterministic id order.
    fn list_services(&self) -> KernelPrimitiveResult<Vec<(KernelServiceId, ServiceScope)>>;
}

/// In-memory service registry for Phase 01 additive contracts.
///
/// The implementation enforces unique service ids and keeps scope descriptors
/// deterministic for tests and diagnostics.  Later distributed registries can
/// preserve this contract while changing storage or synchronization.
#[derive(Default)]
pub struct InMemorySystemServiceRegistry {
    services: RwLock<BTreeMap<KernelServiceId, ServiceScope>>,
}

impl InMemorySystemServiceRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }
}

impl SystemServiceRegistry for InMemorySystemServiceRegistry {
    fn register_service(
        &self,
        id: KernelServiceId,
        scope: ServiceScope,
    ) -> KernelPrimitiveResult<()> {
        if id.is_empty() {
            return Err(KernelPrimitiveError::InvalidIdentifier(
                "service id must not be empty".into(),
            ));
        }

        let mut services = self.services.write().map_err(|_| {
            KernelPrimitiveError::Unavailable("service registry lock poisoned".into())
        })?;
        if services.contains_key(&id) {
            return Err(KernelPrimitiveError::InvalidIdentifier(format!(
                "service {} is already registered",
                id
            )));
        }
        services.insert(id, scope);
        Ok(())
    }

    fn get_service_scope(
        &self,
        id: &KernelServiceId,
    ) -> KernelPrimitiveResult<Option<ServiceScope>> {
        let services = self.services.read().map_err(|_| {
            KernelPrimitiveError::Unavailable("service registry lock poisoned".into())
        })?;
        Ok(services.get(id).cloned())
    }

    fn list_services(&self) -> KernelPrimitiveResult<Vec<(KernelServiceId, ServiceScope)>> {
        let services = self.services.read().map_err(|_| {
            KernelPrimitiveError::Unavailable("service registry lock poisoned".into())
        })?;
        Ok(services
            .iter()
            .map(|(id, scope)| (id.clone(), scope.clone()))
            .collect())
    }
}
