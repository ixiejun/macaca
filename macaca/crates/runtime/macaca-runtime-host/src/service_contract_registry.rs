//! Versioned service contract registry for host-side service routing.
//!
//! This module implements the Registry pattern for service contracts. It keeps
//! contract metadata data-driven so runtime code does not hardcode
//! application/domain behavior.

use std::collections::BTreeMap;
use std::sync::RwLock;

use macaca_proto::KernelServiceId;

/// Canonical service contract descriptor resolved by the router before dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceContractDescriptor {
    /// Stable service identifier (for example `service.market_data`).
    pub service_id: KernelServiceId,
    /// Version label for contract compatibility checks.
    pub version: String,
    /// Structured error codes documented by this contract version.
    pub error_codes: Vec<String>,
    /// Default timeout ceiling recommended by the contract.
    pub timeout_ceiling_ms: u64,
    /// Whether the command surface is idempotent under retries.
    pub idempotent: bool,
    /// Arbitrary extensible metadata for diagnostics.
    pub metadata: BTreeMap<String, String>,
}

impl ServiceContractDescriptor {
    /// Build a compact v1 descriptor with deterministic defaults.
    pub fn v1(service_id: KernelServiceId) -> Self {
        Self {
            service_id,
            version: "v1".into(),
            error_codes: Vec::new(),
            timeout_ceiling_ms: 30_000,
            idempotent: true,
            metadata: BTreeMap::new(),
        }
    }
}

/// Read/write registry contract for service descriptors.
pub trait ServiceContractRegistry: Send + Sync {
    /// Register one service contract version.
    fn register(&self, descriptor: ServiceContractDescriptor);
    /// Resolve one service contract by service id and optional version hint.
    fn resolve(
        &self,
        service_id: &KernelServiceId,
        version_hint: Option<&str>,
    ) -> Option<ServiceContractDescriptor>;
}

/// In-memory contract registry with deterministic version ordering.
#[derive(Default)]
pub struct InMemoryServiceContractRegistry {
    descriptors: RwLock<BTreeMap<KernelServiceId, BTreeMap<String, ServiceContractDescriptor>>>,
}

impl InMemoryServiceContractRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ServiceContractRegistry for InMemoryServiceContractRegistry {
    fn register(&self, descriptor: ServiceContractDescriptor) {
        let mut guard = self
            .descriptors
            .write()
            .expect("service contract registry lock poisoned");
        guard
            .entry(descriptor.service_id.clone())
            .or_default()
            .insert(descriptor.version.clone(), descriptor);
    }

    fn resolve(
        &self,
        service_id: &KernelServiceId,
        version_hint: Option<&str>,
    ) -> Option<ServiceContractDescriptor> {
        let guard = self
            .descriptors
            .read()
            .expect("service contract registry lock poisoned");
        let versions = guard.get(service_id)?;
        if let Some(version) = version_hint {
            return versions.get(version).cloned();
        }
        versions.values().next_back().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_latest_version_without_hint() {
        let registry = InMemoryServiceContractRegistry::new();
        let service_id = KernelServiceId::new("service.contract");
        let mut v1 = ServiceContractDescriptor::v1(service_id.clone());
        v1.version = "v1".into();
        let mut v2 = ServiceContractDescriptor::v1(service_id.clone());
        v2.version = "v2".into();
        registry.register(v1);
        registry.register(v2.clone());
        let resolved = registry.resolve(&service_id, None).unwrap();
        assert_eq!(resolved.version, "v2");
    }
}
