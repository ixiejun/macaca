//! Route C host bootstrap helpers for presentation-shell thinness.
//!
//! This module is intentionally hosted in `macaca-runtime-host` because service
//! provider lifecycle belongs to the runtime host, not to Web routes or CLI
//! commands.  The code uses a small Builder/Abstract Factory shape: callers
//! provide typed handles for already-existing repositories and policies, then
//! the bootstrap registers and starts provider-neutral `SystemService`
//! instances through `ServiceRuntime`.  Web/CLI can consume the resulting
//! runtime through SDK clients without owning provider lifecycle semantics.

use std::sync::Arc;

use macaca_persist::{EntitlementStore, PaymentStore};
use macaca_proto::{KernelServiceId, MacacaError, MacacaResult, PaymentTerms, TraceContext};
use tracing::info;

use crate::{
    entitlement_service_descriptor, evm_service_descriptor, payment_service_descriptor,
    store_service_descriptor, web3_service_descriptor, EntitlementRuntimeFacade,
    EntitlementSystemServiceProvider, EvmSystemServiceProvider, PaymentSystemServiceProvider,
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory, StoreSystemServiceProvider, Web3SystemServiceProvider,
};

/// Typed input handles needed for the S9-S11 service families.
///
/// The struct deliberately contains capability-level handles instead of a Web
/// `AppState` reference.  That keeps the bootstrap reusable for Web, CLI,
/// gateway, or future background hosts and prevents presentation state from
/// leaking into runtime-host ownership.
#[allow(deprecated)]
#[derive(Clone)]
pub struct RouteCOptionalServicesBootstrapInputs {
    pub entitlement_store: Arc<dyn EntitlementStore>,
    pub entitlement_facade: Arc<EntitlementRuntimeFacade>,
    pub payment_store: Arc<dyn PaymentStore>,
    pub payment_terms: PaymentTerms,
}

#[allow(deprecated)]
impl RouteCOptionalServicesBootstrapInputs {
    /// Build S9-S11 bootstrap inputs from explicit repositories/facades.
    ///
    /// Payment terms are passed in instead of constructed here so future hosts
    /// can select policy or terms through configuration, entitlement, store, or
    /// package metadata without hardcoding provider-specific behavior here.
    pub fn new(
        entitlement_store: Arc<dyn EntitlementStore>,
        entitlement_facade: Arc<EntitlementRuntimeFacade>,
        payment_store: Arc<dyn PaymentStore>,
        payment_terms: PaymentTerms,
    ) -> Self {
        Self {
            entitlement_store,
            entitlement_facade,
            payment_store,
            payment_terms,
        }
    }
}

/// Sanitized diagnostic row emitted by the bootstrap boundary.
///
/// Diagnostics intentionally expose service ids, operation names, status, and
/// trace ids only.  They must never include secrets, prompt bodies, provider
/// credentials, raw package bytes, raw payment credentials, wallet secrets, raw
/// signed transactions, ABI/bytecode, or unbounded tool payloads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteCBootstrapDiagnostic {
    pub service_id: KernelServiceId,
    pub operation: &'static str,
    pub status: &'static str,
    pub trace_id: String,
}

impl RouteCBootstrapDiagnostic {
    fn completed(
        service_id: KernelServiceId,
        operation: &'static str,
        trace: &TraceContext,
    ) -> Self {
        Self {
            service_id,
            operation,
            status: "completed",
            trace_id: trace.trace_id.clone(),
        }
    }
}

/// Result of bootstrapping S9-S11 services into an existing runtime.
///
/// The bundle is small because Web already owns the `Arc<ServiceRuntime>` and
/// can build SDK clients through its existing service-client adapter.  Returning
/// started ids and diagnostics is enough for tests, logs, and future host
/// status surfaces without creating a service locator.
#[derive(Clone, Debug, Default)]
pub struct RouteCHostRuntimeBundle {
    pub started_services: Vec<KernelServiceId>,
    pub diagnostics: Vec<RouteCBootstrapDiagnostic>,
}

/// Builder that registers and starts S9-S11 Route C service families.
pub struct RouteCOptionalServicesBootstrap {
    runtime: Arc<ServiceRuntime>,
    inputs: RouteCOptionalServicesBootstrapInputs,
    trace_prefix: String,
}

#[allow(deprecated)]
impl RouteCOptionalServicesBootstrap {
    /// Create a bootstrap bound to a host-owned service runtime.
    pub fn new(
        runtime: Arc<ServiceRuntime>,
        inputs: RouteCOptionalServicesBootstrapInputs,
        trace_prefix: impl Into<String>,
    ) -> Self {
        Self {
            runtime,
            inputs,
            trace_prefix: trace_prefix.into(),
        }
    }

    /// Register and start Store/Entitlement, Payment, Web3, and EVM services.
    ///
    /// The method preserves existing S9-S11 behavior: Store/Entitlement use the
    /// provided local repositories, Payment uses the local simulated adapter,
    /// and Web3/EVM start as structured unavailable optional modules.
    pub async fn bootstrap(self) -> MacacaResult<RouteCHostRuntimeBundle> {
        info!(
            trace_prefix = %self.trace_prefix,
            "route c optional services bootstrap started"
        );
        let mut bundle = RouteCHostRuntimeBundle::default();

        self.register_and_start(
            entitlement_service_descriptor(),
            Arc::new(EntitlementSystemServiceProvider::new(
                Arc::clone(&self.inputs.entitlement_store),
                Arc::clone(&self.inputs.entitlement_facade),
            )),
            "entitlement-service",
            &mut bundle,
        )
        .await?;
        self.register_and_start(
            store_service_descriptor(),
            Arc::new(StoreSystemServiceProvider::new(Arc::clone(
                &self.inputs.entitlement_facade,
            ))),
            "store-service",
            &mut bundle,
        )
        .await?;
        self.register_and_start(
            payment_service_descriptor(),
            Arc::new(PaymentSystemServiceProvider::local_simulated(
                Arc::clone(&self.inputs.payment_store),
                self.inputs.payment_terms.clone(),
            )),
            "payment-service",
            &mut bundle,
        )
        .await?;
        self.register_and_start(
            web3_service_descriptor(),
            Arc::new(Web3SystemServiceProvider::unavailable()),
            "web3-service",
            &mut bundle,
        )
        .await?;
        self.register_and_start(
            evm_service_descriptor(),
            Arc::new(EvmSystemServiceProvider::unavailable()),
            "evm-service",
            &mut bundle,
        )
        .await?;

        info!(
            services = bundle.started_services.len(),
            "route c optional services bootstrap completed"
        );
        Ok(bundle)
    }

    async fn register_and_start(
        &self,
        descriptor: macaca_proto::ServiceDescriptor,
        service: Arc<dyn macaca_kernel::SystemService>,
        trace_suffix: &'static str,
        bundle: &mut RouteCHostRuntimeBundle,
    ) -> MacacaResult<()> {
        let service_id = descriptor.id.clone();
        let trace = TraceContext::new(format!("{}-{trace_suffix}", self.trace_prefix));
        info!(
            service_id = %service_id,
            trace_id = %trace.trace_id,
            "route c optional services registering provider"
        );
        self.runtime
            .register_provider(
                &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                    descriptor, service,
                )),
                ServiceProviderFactoryContext::new(),
            )
            .await
            .map_err(runtime_error)?;
        info!(
            service_id = %service_id,
            trace_id = %trace.trace_id,
            "route c optional services starting provider"
        );
        self.runtime
            .start(&service_id, trace.clone())
            .await
            .map_err(runtime_error)?;
        bundle.started_services.push(service_id.clone());
        bundle
            .diagnostics
            .push(RouteCBootstrapDiagnostic::completed(
                service_id,
                "register_start",
                &trace,
            ));
        Ok(())
    }
}

fn runtime_error(error: crate::ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}

/// Convenience wrapper used by presentation hosts.
pub async fn bootstrap_route_c_optional_services(
    runtime: Arc<ServiceRuntime>,
    inputs: RouteCOptionalServicesBootstrapInputs,
    trace_prefix: impl Into<String>,
) -> MacacaResult<RouteCHostRuntimeBundle> {
    RouteCOptionalServicesBootstrap::new(runtime, inputs, trace_prefix)
        .bootstrap()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_persist::{InMemoryEntitlementStore, InMemoryPaymentStore};
    use macaca_proto::{
        ENTITLEMENT_SERVICE_ID, EVM_SERVICE_ID, PAYMENT_SERVICE_ID, STORE_SERVICE_ID,
        WEB3_SERVICE_ID,
    };

    #[tokio::test]
    #[allow(deprecated)]
    async fn bootstrap_registers_s9_s11_services() {
        let (runtime, inputs) = test_runtime_and_inputs();

        let bundle =
            bootstrap_route_c_optional_services(Arc::clone(&runtime), inputs, "test-bootstrap")
                .await
                .unwrap();

        assert_eq!(bundle.started_services.len(), 5);
        assert!(bundle
            .started_services
            .contains(&KernelServiceId::new(ENTITLEMENT_SERVICE_ID)));
        assert!(bundle
            .started_services
            .contains(&KernelServiceId::new(STORE_SERVICE_ID)));
        assert!(bundle
            .started_services
            .contains(&KernelServiceId::new(PAYMENT_SERVICE_ID)));
        assert!(bundle
            .started_services
            .contains(&KernelServiceId::new(WEB3_SERVICE_ID)));
        assert!(bundle
            .started_services
            .contains(&KernelServiceId::new(EVM_SERVICE_ID)));
        assert_eq!(bundle.diagnostics.len(), 5);
        assert!(bundle
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.status == "completed"));
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn bootstrap_rejects_duplicate_services() {
        let (runtime, inputs) = test_runtime_and_inputs();
        bootstrap_route_c_optional_services(Arc::clone(&runtime), inputs.clone(), "test-bootstrap")
            .await
            .unwrap();

        let error = bootstrap_route_c_optional_services(runtime, inputs, "test-bootstrap")
            .await
            .unwrap_err();
        let error_text = format!("{error:?} {error}");
        assert!(
            error_text.contains("DuplicateService")
                || error_text.contains("duplicate")
                || error_text.contains("already registered"),
            "expected duplicate-service rejection, got: {error_text}"
        );
    }

    #[allow(deprecated)]
    fn test_runtime_and_inputs() -> (Arc<ServiceRuntime>, RouteCOptionalServicesBootstrapInputs) {
        let runtime = Arc::new(ServiceRuntime::new(crate::ServiceRuntimeConfig::default()));
        let entitlement_store: Arc<dyn EntitlementStore> =
            Arc::new(InMemoryEntitlementStore::new());
        let payment_store: Arc<dyn PaymentStore> = Arc::new(InMemoryPaymentStore::new());
        let entitlement_facade = Arc::new(EntitlementRuntimeFacade::new(Arc::clone(
            &entitlement_store,
        )));
        (
            runtime,
            RouteCOptionalServicesBootstrapInputs::new(
                entitlement_store,
                entitlement_facade,
                payment_store,
                macaca_proto::local_simulated_terms("1", "UNIT"),
            ),
        )
    }
}
