//! Host-owned bootstrap helper for the Driver system service.
//!
//! The Web and CLI shells are presentation adapters, so they must not construct
//! driver registries, driver runtimes, or provider instances directly.  This
//! module applies a small Facade over the runtime-host composition root: callers
//! provide neutral configuration values, while runtime-host owns the concrete
//! Driver Registry, Driver Runtime, provider registration, and startup logging.

use std::sync::Arc;

use macaca_driver::{driver_service_descriptor, DriverLoadStatus, DriverRegistry, DriverRuntime};
use macaca_proto::{MacacaError, MacacaResult};

use crate::driver_service_provider::DriverSystemServiceProvider;
use crate::service_provider::{
    ServiceProviderFactoryContext, ServiceProviderInstance, StaticServiceProviderFactory,
};
use crate::service_runtime::ServiceRuntime;

/// Result emitted after runtime-host installs the Driver system service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverServiceBootstrapReport {
    /// Driver directory used by the host-owned driver runtime.
    pub drivers_dir: String,
    /// Whether startup attempted to load driver plugins immediately.
    pub auto_load_attempted: bool,
    /// Count of drivers loaded during startup, or zero when auto-load is off.
    pub loaded: usize,
    /// Count of drivers that failed during startup, or zero when auto-load is off.
    pub failed: usize,
}

/// Register the Driver system service using runtime-host owned concrete state.
///
/// This helper is intentionally narrow: it does not return `DriverRuntime`.
/// Shells receive only an auditable bootstrap report, while all later driver
/// lifecycle operations flow through `service.driver` commands and SDK clients.
pub async fn bootstrap_driver_service(
    service_runtime: Arc<ServiceRuntime>,
    drivers_dir: impl Into<String>,
    auto_load: bool,
    trace_label: impl Into<String>,
) -> MacacaResult<DriverServiceBootstrapReport> {
    let trace_label = trace_label.into();
    let drivers_dir = drivers_dir.into();
    tracing::info!(
        drivers_dir = %drivers_dir,
        auto_load,
        trace_label = %trace_label,
        "runtime-host driver service bootstrap requested"
    );

    let driver_registry = Arc::new(DriverRegistry::new());
    let driver_runtime = Arc::new(DriverRuntime::new(
        drivers_dir.clone(),
        Arc::clone(&driver_registry),
    ));

    let (loaded, failed) = if auto_load {
        let report = driver_runtime.load_all().await;
        for entry in &report.entries {
            match entry.status {
                DriverLoadStatus::Loaded => {
                    tracing::info!(
                        name = %entry.name,
                        tools = entry.tool_count.unwrap_or_default(),
                        trace_label = %trace_label,
                        "runtime-host loaded external driver"
                    );
                }
                DriverLoadStatus::Failed => {
                    tracing::error!(
                        name = %entry.name,
                        error = %entry.error.as_deref().unwrap_or("unknown error"),
                        trace_label = %trace_label,
                        "runtime-host failed to load external driver"
                    );
                }
            }
        }
        (report.loaded, report.failed)
    } else {
        tracing::info!(
            trace_label = %trace_label,
            "runtime-host driver auto-load disabled"
        );
        (0, 0)
    };

    service_runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                driver_service_descriptor(),
                Arc::new(DriverSystemServiceProvider::new(driver_runtime)),
            )),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(|err| MacacaError::Config(err.to_string()))?;

    tracing::info!(
        drivers_dir = %drivers_dir,
        loaded,
        failed,
        trace_label = %trace_label,
        "runtime-host driver service registered"
    );

    Ok(DriverServiceBootstrapReport {
        drivers_dir,
        auto_load_attempted: auto_load,
        loaded,
        failed,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use macaca_driver::DRIVER_SERVICE_ID;

    use super::*;
    use crate::service_runtime_error::ServiceRuntimeConfig;

    #[tokio::test]
    async fn bootstrap_registers_driver_service_without_exposing_runtime() {
        let temp = tempfile::tempdir().expect("tempdir");
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));

        let report = bootstrap_driver_service(
            Arc::clone(&runtime),
            temp.path().display().to_string(),
            false,
            "driver-bootstrap-test",
        )
        .await
        .expect("driver service should register");

        assert_eq!(report.loaded, 0);
        assert!(!report.auto_load_attempted);
        let snapshot = runtime.snapshot().expect("snapshot");
        assert!(snapshot
            .services
            .iter()
            .any(|service| service.descriptor.id.as_str() == DRIVER_SERVICE_ID));
    }
}
