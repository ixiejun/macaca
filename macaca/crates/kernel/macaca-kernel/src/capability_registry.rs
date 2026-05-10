//! Capability registry primitives for the microkernel facade.
//!
//! The registry stores provider-neutral capability descriptors.  It does not
//! execute capabilities and it does not choose a concrete provider.  Those
//! responsibilities belong to system services and future service-bus phases.

use std::collections::BTreeMap;
use std::sync::RwLock;

use macaca_proto::{
    CapabilityDescriptor, CapabilityId, KernelPrimitiveError, KernelPrimitiveResult,
};

/// Registry contract for discovering capability descriptors.
///
/// Callers depend on this trait rather than a concrete map so later phases can
/// replace the in-memory implementation with a service-backed registry without
/// changing the kernel facade API.
pub trait CapabilityRegistry: Send + Sync {
    /// Register a descriptor and reject duplicate capability ids.
    fn register_capability(&self, descriptor: CapabilityDescriptor) -> KernelPrimitiveResult<()>;

    /// Look up a descriptor by id.
    fn get_capability(
        &self,
        id: &CapabilityId,
    ) -> KernelPrimitiveResult<Option<CapabilityDescriptor>>;

    /// Return all registered descriptors in deterministic id order.
    fn list_capabilities(&self) -> KernelPrimitiveResult<Vec<CapabilityDescriptor>>;
}

/// In-memory capability registry used by the additive Phase 01 facade.
///
/// This implementation protects the minimum invariant required by the
/// microkernel boundary: one capability id maps to at most one descriptor in a
/// registry instance.  It is intentionally small but not a placeholder; later
/// registries must preserve the same duplicate and lookup semantics.
#[derive(Default)]
pub struct InMemoryCapabilityRegistry {
    descriptors: RwLock<BTreeMap<CapabilityId, CapabilityDescriptor>>,
}

impl InMemoryCapabilityRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }
}

impl CapabilityRegistry for InMemoryCapabilityRegistry {
    fn register_capability(&self, descriptor: CapabilityDescriptor) -> KernelPrimitiveResult<()> {
        if descriptor.id.is_empty() {
            return Err(KernelPrimitiveError::InvalidIdentifier(
                "capability id must not be empty".into(),
            ));
        }

        let mut descriptors = self.descriptors.write().map_err(|_| {
            KernelPrimitiveError::Unavailable("capability registry lock poisoned".into())
        })?;
        if descriptors.contains_key(&descriptor.id) {
            return Err(KernelPrimitiveError::InvalidIdentifier(format!(
                "capability {} is already registered",
                descriptor.id
            )));
        }
        descriptors.insert(descriptor.id.clone(), descriptor);
        Ok(())
    }

    fn get_capability(
        &self,
        id: &CapabilityId,
    ) -> KernelPrimitiveResult<Option<CapabilityDescriptor>> {
        let descriptors = self.descriptors.read().map_err(|_| {
            KernelPrimitiveError::Unavailable("capability registry lock poisoned".into())
        })?;
        Ok(descriptors.get(id).cloned())
    }

    fn list_capabilities(&self) -> KernelPrimitiveResult<Vec<CapabilityDescriptor>> {
        let descriptors = self.descriptors.read().map_err(|_| {
            KernelPrimitiveError::Unavailable("capability registry lock poisoned".into())
        })?;
        Ok(descriptors.values().cloned().collect())
    }
}
