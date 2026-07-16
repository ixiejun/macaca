//! Provider health helpers for the host-owned ServiceRuntime facade.
//!
//! Keeping these helpers outside `service_runtime.rs` preserves the Facade file
//! size while still letting the runtime record provider-reported health after
//! successful lifecycle and call operations.  The logic is intentionally generic:
//! it reads only the `SystemService` health contract and never branches on pack,
//! provider, model, application, workflow, or business-domain identifiers.

use std::sync::Arc;

use macaca_proto::{KernelServiceId, ServiceHealth};

use crate::{service_runtime::ServiceRuntime, service_runtime_error::ServiceRuntimeError};

impl ServiceRuntime {
    pub(super) fn service_for(
        &self,
        service_id: &KernelServiceId,
    ) -> Result<Arc<dyn macaca_kernel::SystemService>, ServiceRuntimeError> {
        let services = self.services.read().map_err(super::support::lock_error)?;
        let record = services
            .get(service_id)
            .ok_or_else(|| ServiceRuntimeError::UnknownService(service_id.to_string()))?;
        Ok(record.service.clone())
    }

    /// Read provider-reported health after a successful lifecycle or call operation.
    ///
    /// The runtime lifecycle can be running while the provider capability is still
    /// unavailable, degraded, or preview-only.  Capturing provider health here keeps
    /// snapshots honest without making `ServiceRuntime` understand pack-specific
    /// provider classes, model names, application workflows, or business payloads.
    pub(super) async fn observed_health_after_success(
        &self,
        service_id: &KernelServiceId,
        service: &Arc<dyn macaca_kernel::SystemService>,
        fallback: ServiceHealth,
    ) -> ServiceHealth {
        match service.health().await {
            Ok(health) => health,
            Err(error) => {
                tracing::warn!(
                    service_id = %service_id,
                    error = %error,
                    "service runtime retained fallback health after provider health probe failed"
                );
                fallback
            }
        }
    }
}
