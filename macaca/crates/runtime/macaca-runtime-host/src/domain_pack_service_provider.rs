//! Generic domain-pack service registration for package-owned providers.
//!
//! Domain packs are extension packages, not base runtime-host business logic.
//! This module owns reusable registration mechanics: composition roots supply
//! descriptor-backed [`SystemService`] instances via [`DomainPackProviderRegistration`],
//! and the runtime starts them with trace evidence.
//!
//! # Layering
//!
//! - [`DomainPackProviderRegistration`] lives in `macaca-kernel` so package crates
//!   avoid depending on this runtime-host crate (breaks `pack → runtime-host → app` cycles).
//! - Trace/result helpers live in `macaca-proto::domain_pack_contract` for the same reason.
//! - This module keeps bootstrap wiring and package-facing helper re-exports centralized.

use std::sync::Arc;

use macaca_proto::{
    DomainPackProviderSnapshot, KernelServiceId, MacacaError, MacacaResult, TraceContext,
};
use tracing::info;

use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

/// Re-export the kernel-owned registration product type.
pub use macaca_kernel::DomainPackProviderRegistration;

/// Re-export generic provider replacement adapters while preserving this module as the public
/// package-facing bootstrap surface.
pub use crate::domain_pack_provider_replacement::{
    mock_domain_pack_provider_registration, unavailable_domain_pack_provider_registration,
    DomainPackMockSystemServiceProvider, DomainPackUnavailableSystemServiceProvider,
};

/// Re-export proto-owned trace/result helpers for package adapters.
pub use macaca_proto::{
    domain_pack_command_trace as command_trace,
    domain_pack_service_adapter_error as service_adapter_error,
    domain_pack_service_result as service_result,
};

/// Started service ids returned by the domain-pack bootstrap boundary.
#[derive(Clone, Debug, Default)]
pub struct DomainPackRuntimeBundle {
    /// Kernel service ids that were registered and started during bootstrap.
    pub started_services: Vec<KernelServiceId>,
    /// Sanitized provider snapshots safe for diagnostics, trace, and shell rendering.
    pub provider_snapshots: Vec<DomainPackProviderSnapshot>,
}

/// Register and start package-owned domain-pack service providers.
///
/// Implements only generic Adapter/Bridge mechanics. Does not inspect package names,
/// service ids, asset classes, providers, or domain payloads. An empty registration
/// list is valid optional-module state and returns an empty bundle (Null Object).
pub async fn bootstrap_domain_pack_services(
    runtime: Arc<ServiceRuntime>,
    registrations: impl IntoIterator<Item = DomainPackProviderRegistration>,
    trace_prefix: impl Into<String>,
) -> MacacaResult<DomainPackRuntimeBundle> {
    let trace_prefix = trace_prefix.into();
    let mut bundle = DomainPackRuntimeBundle::default();

    for registration in registrations {
        let descriptor = registration.descriptor().clone();
        let service_id = descriptor.id.clone();
        let trace = TraceContext::new(format!("{trace_prefix}-{}", registration.trace_suffix()));
        info!(
            service_id = %service_id,
            pack_id = registration.pack_id().unwrap_or("unknown"),
            trace_id = %trace.trace_id,
            "pack_provider_registered"
        );
        runtime
            .register_provider(
                &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                    descriptor,
                    registration.service(),
                )),
                ServiceProviderFactoryContext::new(),
            )
            .await
            .map_err(runtime_error)?;
        runtime
            .start(&service_id, trace.clone())
            .await
            .map_err(runtime_error)?;
        info!(
            service_id = %service_id,
            pack_id = registration.pack_id().unwrap_or("unknown"),
            trace_id = %trace.trace_id,
            "pack_provider_started"
        );
        bundle
            .provider_snapshots
            .push(DomainPackProviderSnapshot::registered(
                registration.pack_id().unwrap_or("unknown"),
                service_id.to_string(),
                trace.trace_id.clone(),
            ));
        bundle.started_services.push(service_id);
    }

    if bundle.started_services.is_empty() {
        info!(
            trace_prefix = %trace_prefix,
            "domain-pack bootstrap completed with no package providers registered"
        );
    }

    Ok(bundle)
}

fn runtime_error(error: crate::ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::DomainPackRuntimeBundle;

    #[test]
    fn empty_domain_pack_bootstrap_returns_empty_bundle() {
        let bundle = DomainPackRuntimeBundle::default();
        assert!(bundle.started_services.is_empty());
        assert!(bundle.provider_snapshots.is_empty());
    }
}
