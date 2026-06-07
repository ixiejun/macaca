//! Generic domain-pack service registration for package-owned providers.
//!
//! Domain packs are extension packages, not base runtime-host business logic.
//! This module therefore owns only the reusable registration mechanics: a host
//! can provide descriptor-backed [`SystemService`] instances, and the runtime
//! starts them with trace evidence. Concrete finance, crypto, office, or other
//! domain behavior must be supplied by a plugin/package composition root.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_llm::LlmProvider;
use macaca_proto::{
    CleanupPolicy, KernelServiceId, MacacaError, MacacaResult, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceResult, TraceContext,
};
use serde_json::Value;
use tracing::{info, warn};

use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

/// Common result builder used by descriptor-owned domain-pack adapters.
///
/// The runtime host adds only provider-neutral metadata. Package providers may
/// add their own bounded metadata inside the output payload, but OS-level
/// observability must never receive raw provider payloads, secrets, manifests,
/// package bytes, prompts, or unbounded diagnostic output.
pub fn service_result(
    output: Value,
    trace: TraceContext,
    provider_class: &'static str,
) -> ServiceCallResult {
    let mut metadata = BTreeMap::new();
    metadata.insert("provider_class".into(), provider_class.into());
    ServiceCallResult {
        output,
        trace,
        status: "ok".into(),
        metadata,
        cleanup_hint: Some(CleanupPolicy::None),
    }
}

/// Extract a trace context or fail before provider logic runs.
///
/// Keeping this helper close to the registration boundary makes the invariant
/// easy to reuse in package-provider tests: every service call must be
/// trace-addressable before any side effect or external adapter runs.
pub fn command_trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
    command
        .trace
        .clone()
        .ok_or(ServiceError::MissingTraceContext)
}

/// Descriptor-backed service registration supplied by a plugin/package.
///
/// This type is the small Abstract Factory product that the base runtime-host
/// accepts from a composition root. It keeps the OS generic: the descriptor
/// carries the package-owned service identity and command surface, while the
/// runtime-host only registers and starts the already-constructed provider.
#[derive(Clone)]
pub struct DomainPackProviderRegistration {
    descriptor: ServiceDescriptor,
    service: Arc<dyn SystemService>,
    trace_suffix: String,
}

impl DomainPackProviderRegistration {
    /// Create a new package-owned provider registration.
    ///
    /// The trace suffix is intentionally separate from service id. It gives
    /// package composition roots a stable, audit-friendly boot trace segment
    /// without requiring the runtime host to parse domain-specific ids.
    pub fn new(
        descriptor: ServiceDescriptor,
        service: Arc<dyn SystemService>,
        trace_suffix: impl Into<String>,
    ) -> Self {
        Self {
            descriptor,
            service,
            trace_suffix: trace_suffix.into(),
        }
    }
}

/// Started service ids returned by the domain-pack bootstrap boundary.
#[derive(Clone, Debug, Default)]
pub struct DomainPackRuntimeBundle {
    pub started_services: Vec<KernelServiceId>,
}

/// Register and start package-owned domain-pack service providers.
///
/// The function implements only the generic Adapter/Bridge mechanics. It does
/// not inspect package names, service ids, asset classes, providers, or domain
/// payloads. Missing registrations are a valid optional-module state and return
/// an empty bundle instead of crashing, hanging, silently falling back, or
/// fabricating successful provider output.
pub async fn bootstrap_domain_pack_services(
    runtime: Arc<ServiceRuntime>,
    registrations: impl IntoIterator<Item = DomainPackProviderRegistration>,
    trace_prefix: impl Into<String>,
) -> MacacaResult<DomainPackRuntimeBundle> {
    let trace_prefix = trace_prefix.into();
    let mut bundle = DomainPackRuntimeBundle::default();

    for registration in registrations {
        let service_id = registration.descriptor.id.clone();
        let trace = TraceContext::new(format!("{trace_prefix}-{}", registration.trace_suffix));
        info!(
            service_id = %service_id,
            trace_id = %trace.trace_id,
            "domain-pack service registering package provider"
        );
        runtime
            .register_provider(
                &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                    registration.descriptor,
                    registration.service,
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
            trace_id = %trace.trace_id,
            "domain-pack package provider started"
        );
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

/// Deprecated compatibility entrypoint for the old built-in domain-pack path.
///
/// Base runtime-host no longer owns finance or crypto providers. The signature
/// remains temporarily so existing composition roots can compile while they are
/// migrated to pass package-owned [`DomainPackProviderRegistration`] values.
/// Returning an empty bundle is the explicit Null Object behavior for optional
/// absence: callers may query services and receive structured unavailable
/// results from the service runtime instead of receiving synthetic business
/// output from the OS.
#[deprecated(
    since = "0.1.0",
    note = "register package-owned providers with bootstrap_domain_pack_services"
)]
pub async fn bootstrap_builtin_domain_pack_services(
    _runtime: Arc<ServiceRuntime>,
    _llm: Arc<dyn LlmProvider>,
    trace_prefix: impl Into<String>,
) -> MacacaResult<DomainPackRuntimeBundle> {
    let trace_prefix = trace_prefix.into();
    warn!(
        trace_prefix = %trace_prefix,
        "built-in domain-pack providers are disabled; package provider registration is required"
    );
    Ok(DomainPackRuntimeBundle::default())
}

pub fn service_adapter_error(error: MacacaError) -> ServiceError {
    ServiceError::ServiceUnavailable(error.to_string())
}

fn runtime_error(error: crate::ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::DomainPackRuntimeBundle;

    #[test]
    fn builtin_domain_pack_bootstrap_returns_empty_bundle() {
        // The deprecated built-in path is an explicit Null Object: optional
        // domain packs must be registered by composition roots, not the OS.
        let bundle = DomainPackRuntimeBundle::default();
        assert!(bundle.started_services.is_empty());
    }
}
