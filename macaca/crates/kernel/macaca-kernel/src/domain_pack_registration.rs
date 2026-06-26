//! Domain-pack provider registration contract for optional package crates.
//!
//! Optional domain packs register [`SystemService`] implementations through
//! [`DomainPackProviderRegistration`] products consumed by the runtime-host bootstrap.
//! Keeping this type in the microkernel layer (not `macaca-runtime-host`) lets package
//! crates avoid depending on the composition host that transitively links `macaca-app`.
//!
//! # Design pattern
//! - **Abstract Factory product**: package bootstrap factories build registrations;
//!   runtime-host only registers and starts the already-constructed providers.

use std::sync::Arc;

use macaca_proto::ServiceDescriptor;

use crate::SystemService;

/// Descriptor-backed service registration supplied by an optional domain-pack package.
///
/// The trace suffix is intentionally separate from the service id so composition roots
/// can emit stable boot-trace segments without parsing domain-specific identifiers.
#[derive(Clone)]
pub struct DomainPackProviderRegistration {
    descriptor: ServiceDescriptor,
    service: Arc<dyn SystemService>,
    trace_suffix: String,
    pack_id: Option<String>,
}

impl DomainPackProviderRegistration {
    /// Create one package-owned provider registration for runtime-host bootstrap.
    pub fn new(
        descriptor: ServiceDescriptor,
        service: Arc<dyn SystemService>,
        trace_suffix: impl Into<String>,
    ) -> Self {
        Self {
            descriptor,
            service,
            trace_suffix: trace_suffix.into(),
            pack_id: None,
        }
    }

    /// Attach the descriptor pack id without making runtime-host parse service names.
    ///
    /// Package crates own this metadata because they understand which pack published the
    /// provider. Runtime-host only records the bounded value for trace/audit projection.
    pub fn with_pack_id(mut self, pack_id: impl Into<String>) -> Self {
        self.pack_id = Some(pack_id.into());
        self
    }

    /// Expose the registered descriptor for runtime-host registration wiring.
    pub fn descriptor(&self) -> &ServiceDescriptor {
        &self.descriptor
    }

    /// Expose the registered service instance for runtime-host registration wiring.
    pub fn service(&self) -> Arc<dyn SystemService> {
        Arc::clone(&self.service)
    }

    /// Expose the boot-trace suffix for runtime-host lifecycle evidence.
    pub fn trace_suffix(&self) -> &str {
        &self.trace_suffix
    }

    /// Return the optional descriptor pack id supplied by the package factory.
    pub fn pack_id(&self) -> Option<&str> {
        self.pack_id.as_deref()
    }
}
