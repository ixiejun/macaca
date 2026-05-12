//! Application host runtime skeletons.
//!
//! This module is the runtime-host side of the Application ABI boundary.  It
//! exposes an Abstract Factory plus Strategy trait for application runtime
//! hosts while keeping unapproved execution engines fail-closed.  The current
//! WASM implementation is a Null Object: it reports structured
//! runtime-unavailable results and never loads or executes guest bytes.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    ApplicationAbiError, ApplicationHostCommand, ApplicationHostCommandResult,
    ApplicationHostCommandStatus, PackageDescriptor, PackageRuntimeKind, TraceContext,
};
use tracing::{info, warn};

/// Runtime-host strategy selected for an application package/runtime kind.
#[async_trait]
pub trait ApplicationHostRuntime: Send + Sync {
    /// Runtime kind handled by this host.
    fn runtime_kind(&self) -> PackageRuntimeKind;

    /// Dispatch one host command through the runtime strategy.
    ///
    /// Implementations must either preserve trace context or fail closed.
    /// They must not log raw command payloads because payloads may contain user
    /// input or future guest data.
    async fn dispatch(
        &self,
        command: ApplicationHostCommand,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError>;
}

/// Abstract Factory for application host runtime strategies.
#[derive(Debug, Default, Clone)]
pub struct WasmApplicationHostFactory;

impl WasmApplicationHostFactory {
    /// Create a host from a package descriptor.
    ///
    /// The factory reads only provider-neutral package metadata.  It never
    /// opens component bytes and never creates `AppRuntime`, `Kernel`,
    /// `ServiceRuntime`, Web state, or concrete provider clients.
    pub fn create_for_package(
        &self,
        package: &PackageDescriptor,
    ) -> Arc<dyn ApplicationHostRuntime> {
        match package.manifest.runtime.kind.as_ref() {
            Some(PackageRuntimeKind::WasmComponent) => {
                warn!(
                    package_id = %package.manifest.id,
                    "WASM application host skeleton selected as runtime-unavailable"
                );
                Arc::new(UnavailableWasmApplicationHost::from_package(package))
            }
            Some(kind) => {
                warn!(
                    package_id = %package.manifest.id,
                    runtime_kind = %kind,
                    "non-WASM application host requested from WASM factory"
                );
                Arc::new(UnavailableApplicationRuntimeHost::new(
                    kind.clone(),
                    "requested application runtime is not handled by WasmApplicationHostFactory",
                ))
            }
            None => Arc::new(UnavailableApplicationRuntimeHost::new(
                PackageRuntimeKind::Custom("missing_runtime".into()),
                "package runtime kind is missing",
            )),
        }
    }

    /// Create a host directly from a runtime kind.
    ///
    /// This helper is useful for tests and service adapters that already
    /// resolved package metadata elsewhere.
    pub fn create_for_kind(&self, kind: PackageRuntimeKind) -> Arc<dyn ApplicationHostRuntime> {
        match kind {
            PackageRuntimeKind::WasmComponent => {
                Arc::new(UnavailableWasmApplicationHost::default())
            }
            other => Arc::new(UnavailableApplicationRuntimeHost::new(
                other,
                "requested application runtime is not available in host skeleton v0",
            )),
        }
    }
}

/// Unavailable-safe WASM application host.
#[derive(Debug, Clone, Default)]
pub struct UnavailableWasmApplicationHost {
    package_id: Option<String>,
    application_id: Option<String>,
}

impl UnavailableWasmApplicationHost {
    /// Build the host from sanitized package metadata.
    pub fn from_package(package: &PackageDescriptor) -> Self {
        Self {
            package_id: Some(package.manifest.id.to_string()),
            application_id: package.manifest.metadata.get("application.id").cloned(),
        }
    }
}

#[async_trait]
impl ApplicationHostRuntime for UnavailableWasmApplicationHost {
    fn runtime_kind(&self) -> PackageRuntimeKind {
        PackageRuntimeKind::WasmComponent
    }

    async fn dispatch(
        &self,
        command: ApplicationHostCommand,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError> {
        let Some(trace) = command.trace.clone() else {
            warn!(
                import = %command.import,
                "WASM application host rejected untraceable command"
            );
            return Err(ApplicationAbiError::MissingTraceContext);
        };
        warn!(
            import = %command.import,
            trace_id = %trace.trace_id,
            package_id = self.package_id.as_deref().unwrap_or("unknown"),
            application_id = self.application_id.as_deref().unwrap_or("unknown"),
            runtime_kind = %PackageRuntimeKind::WasmComponent,
            reason_code = "wasm_component_runtime_unavailable",
            "WASM application host returned structured runtime-unavailable result"
        );
        Ok(runtime_unavailable_result(
            trace,
            PackageRuntimeKind::WasmComponent,
            self.package_id.as_deref(),
            self.application_id.as_deref(),
            "wasm_component_runtime_unavailable",
            "WASM component application runtime is not installed",
        ))
    }
}

/// Generic unavailable host for unsupported or missing runtime kinds.
#[derive(Debug, Clone)]
pub struct UnavailableApplicationRuntimeHost {
    runtime_kind: PackageRuntimeKind,
    reason: String,
}

impl UnavailableApplicationRuntimeHost {
    /// Create a generic unavailable host for an application runtime kind.
    pub fn new(runtime_kind: PackageRuntimeKind, reason: impl Into<String>) -> Self {
        Self {
            runtime_kind,
            reason: reason.into(),
        }
    }
}

#[async_trait]
impl ApplicationHostRuntime for UnavailableApplicationRuntimeHost {
    fn runtime_kind(&self) -> PackageRuntimeKind {
        self.runtime_kind.clone()
    }

    async fn dispatch(
        &self,
        command: ApplicationHostCommand,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError> {
        let Some(trace) = command.trace.clone() else {
            return Err(ApplicationAbiError::MissingTraceContext);
        };
        info!(
            import = %command.import,
            trace_id = %trace.trace_id,
            runtime_kind = %self.runtime_kind,
            reason = %self.reason,
            "application host skeleton returned unavailable result"
        );
        Ok(runtime_unavailable_result(
            trace,
            self.runtime_kind.clone(),
            None,
            None,
            "application_runtime_unavailable",
            &self.reason,
        ))
    }
}

fn runtime_unavailable_result(
    trace: TraceContext,
    runtime_kind: PackageRuntimeKind,
    package_id: Option<&str>,
    application_id: Option<&str>,
    reason_code: &str,
    reason: &str,
) -> ApplicationHostCommandResult {
    let mut result = ApplicationHostCommandResult::runtime_unavailable(reason, Some(trace));
    result
        .metadata
        .insert("runtime_kind".into(), runtime_kind.to_string());
    result
        .metadata
        .insert("reason_code".into(), reason_code.into());
    if let Some(package_id) = package_id {
        result
            .metadata
            .insert("package_id".into(), package_id.into());
    }
    if let Some(application_id) = application_id {
        result
            .metadata
            .insert("application_id".into(), application_id.into());
    }
    result
}

/// Return whether a host result is a structured runtime-unavailable outcome.
pub fn is_application_runtime_unavailable(result: &ApplicationHostCommandResult) -> bool {
    matches!(
        result.status,
        ApplicationHostCommandStatus::RuntimeUnavailable { .. }
    )
}

#[cfg(test)]
mod application_hosts_tests;
