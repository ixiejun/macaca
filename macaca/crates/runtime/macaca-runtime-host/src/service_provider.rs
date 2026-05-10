//! Provider-neutral factory contracts for ServiceRuntime v1.
//!
//! The runtime host must not become a hardcoded constructor for LLM, memory,
//! driver, gateway, skill, MCP, payment, Web3, EVM, or application-specific
//! providers.  This module uses the Abstract Factory pattern: a factory receives
//! neutral runtime context and returns an `Arc<dyn SystemService>`.  Concrete
//! provider selection remains outside the runtime core and can later come from
//! package manifests, plugin manifests, configuration, or store metadata.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{KernelServiceId, ServiceDescriptor};

use crate::service_runtime_error::ServiceRuntimeError;

/// Context passed to provider factories during registration.
///
/// The context intentionally contains only provider-neutral data.  It is safe
/// for built-in services, plugin services, and future remote services because
/// it does not encode provider categories or application workflow names.
#[derive(Debug, Clone, Default)]
pub struct ServiceProviderFactoryContext {
    pub metadata: BTreeMap<String, String>,
}

impl ServiceProviderFactoryContext {
    /// Create an empty context for deterministic tests and simple registrations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attach one metadata key/value pair without changing factory semantics.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Product returned by a service provider factory.
///
/// The descriptor is copied next to the service instance so the runtime can
/// validate identity, register bus handlers, emit diagnostics, and expose
/// snapshots without asking callers to downcast provider-specific types.
#[derive(Clone)]
pub struct ServiceProviderInstance {
    pub descriptor: ServiceDescriptor,
    pub service: Arc<dyn SystemService>,
    pub metadata: BTreeMap<String, String>,
}

impl ServiceProviderInstance {
    /// Build a provider-neutral service instance from a descriptor and service.
    pub fn new(descriptor: ServiceDescriptor, service: Arc<dyn SystemService>) -> Self {
        Self {
            descriptor,
            service,
            metadata: BTreeMap::new(),
        }
    }

    /// Return the stable service id advertised by the descriptor.
    pub fn service_id(&self) -> &KernelServiceId {
        &self.descriptor.id
    }
}

/// Abstract factory for system services hosted by `ServiceRuntime`.
///
/// Implementations may wrap built-in services, plugin-created services, remote
/// service adapters, or test doubles.  The trait returns only the
/// provider-neutral `SystemService` boundary so the runtime stays generic.
#[async_trait]
pub trait ServiceProviderFactory: Send + Sync {
    /// Create one service instance for registration.
    async fn create(
        &self,
        context: ServiceProviderFactoryContext,
    ) -> Result<ServiceProviderInstance, ServiceRuntimeError>;
}

/// Deterministic factory used by tests and simple built-in registrations.
///
/// This factory is deliberately tiny: it returns a prebuilt service instance and
/// performs no provider-category branching.  It is useful for ServiceRuntime
/// tests because it proves the runtime only needs the `SystemService` contract.
#[derive(Clone)]
pub struct StaticServiceProviderFactory {
    instance: ServiceProviderInstance,
}

impl StaticServiceProviderFactory {
    /// Create a factory that always returns the same provider-neutral instance.
    pub fn new(instance: ServiceProviderInstance) -> Self {
        Self { instance }
    }
}

#[async_trait]
impl ServiceProviderFactory for StaticServiceProviderFactory {
    async fn create(
        &self,
        _context: ServiceProviderFactoryContext,
    ) -> Result<ServiceProviderInstance, ServiceRuntimeError> {
        Ok(self.instance.clone())
    }
}
