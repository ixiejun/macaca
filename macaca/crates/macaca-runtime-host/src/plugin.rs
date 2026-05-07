//! Runtime-host Plugin Runtime v0 facade.
//!
//! This module coordinates plugin manifest validation, safe host selection,
//! kernel registry calls, lifecycle events, and structured logging.  It does
//! not execute arbitrary third-party code.  Phase 07 only supports descriptor
//! and built-in-adapter hosts; future WASM/native/process runtimes can be added
//! behind the same factory/proxy boundary.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::PluginRegistry;
use macaca_proto::{
    PluginError, PluginLifecycleEvent, PluginLifecycleState, PluginManifest, PluginRuntimeKind,
    TraceContext,
};
use tracing::{info, warn};

#[path = "plugin_builtin.rs"]
mod plugin_builtin;

pub use plugin_builtin::{BuiltInPluginDescriptor, BuiltInPluginDescriptorAdapter};

/// Result alias for plugin runtime operations.
pub type PluginRuntimeResult<T> = Result<T, PluginError>;

/// Specification-style manifest validator.
///
/// Validation stays separate from lifecycle orchestration so new rules can be
/// added without changing host factory or registry code.  The validator rejects
/// arbitrary executable runtime kinds in Phase 07.
#[derive(Debug, Clone)]
pub struct PluginManifestValidator {
    require_signature: bool,
}

impl Default for PluginManifestValidator {
    fn default() -> Self {
        Self {
            require_signature: true,
        }
    }
}

impl PluginManifestValidator {
    /// Create a validator with Phase 07 defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allow tests and local descriptor adapters to opt out of signature
    /// metadata while keeping production defaults strict.
    pub fn allow_unsigned(mut self) -> Self {
        self.require_signature = false;
        self
    }

    /// Validate manifest shape, runtime kind, permissions, resources, and
    /// signature metadata before any registry mutation occurs.
    pub fn validate(&self, manifest: &PluginManifest) -> PluginRuntimeResult<()> {
        info!(
            plugin_id = %manifest.plugin_id,
            runtime_kind = %manifest.runtime.kind,
            "plugin manifest validation started"
        );
        if manifest.plugin_id.as_str().is_empty() {
            return Err(PluginError::MissingPluginId);
        }
        if manifest.version.as_str().is_empty() {
            return Err(PluginError::MissingPluginVersion);
        }
        if manifest.developer_id.as_str().is_empty() {
            return Err(PluginError::MissingDeveloperId);
        }
        if !manifest.runtime.kind.is_supported_v0() {
            return Err(PluginError::UnsupportedRuntime(
                manifest.runtime.kind.to_string(),
            ));
        }
        if self.require_signature && manifest.signature.is_none() {
            return Err(PluginError::MissingSignature);
        }
        self.validate_permissions(manifest)?;
        self.validate_resources(manifest)?;
        info!(plugin_id = %manifest.plugin_id, "plugin manifest validation passed");
        Ok(())
    }

    fn validate_permissions(&self, manifest: &PluginManifest) -> PluginRuntimeResult<()> {
        let declared: std::collections::BTreeSet<_> =
            manifest.permissions.iter().map(|p| p.id.as_str()).collect();
        for service in &manifest.provides_services {
            for permission in service
                .required_permissions
                .iter()
                .chain(service.service.required_permissions.iter())
            {
                if !declared.contains(permission.as_str()) {
                    warn!(
                        plugin_id = %manifest.plugin_id,
                        permission = %permission,
                        "plugin manifest missing required permission"
                    );
                    return Err(PluginError::MissingPermission(permission.clone()));
                }
            }
        }
        for capability in &manifest.provides_capabilities {
            for permission in &capability.required_permissions {
                if !declared.contains(permission.as_str()) {
                    return Err(PluginError::MissingPermission(permission.clone()));
                }
            }
        }
        Ok(())
    }

    fn validate_resources(&self, manifest: &PluginManifest) -> PluginRuntimeResult<()> {
        let privileged = !manifest.provides_services.is_empty()
            || !manifest.provides_capabilities.is_empty()
            || manifest
                .runtime
                .metadata
                .get("requires_resource")
                .is_some_and(|value| value == "true");
        if privileged && manifest.resources.is_empty() {
            warn!(plugin_id = %manifest.plugin_id, "plugin manifest missing resource declaration");
            return Err(PluginError::MissingResource("resource declaration".into()));
        }
        Ok(())
    }
}

/// Runtime guard wraps validation so future policy engines can be inserted.
#[derive(Debug, Clone)]
pub struct PluginRuntimeGuard {
    validator: PluginManifestValidator,
}

impl PluginRuntimeGuard {
    /// Create a guard from a validator strategy.
    pub fn new(validator: PluginManifestValidator) -> Self {
        Self { validator }
    }

    /// Validate plugin activation prerequisites.
    pub fn authorize(&self, manifest: &PluginManifest) -> PluginRuntimeResult<()> {
        self.validator.validate(manifest)
    }
}

/// Host abstraction selected by `PluginHostFactory`.
///
/// Phase 07 hosts are descriptor-only; methods exist so future proxy hosts can
/// start/stop external runtimes without changing the facade API.
#[async_trait]
pub trait PluginHost: Send + Sync {
    /// Return the runtime kind this host strategy supports.
    fn runtime_kind(&self) -> PluginRuntimeKind;

    /// Start host resources. Descriptor hosts only log and return success.
    async fn start(&self, manifest: &PluginManifest) -> PluginRuntimeResult<()>;

    /// Stop host resources. Descriptor hosts only log and return success.
    async fn stop(&self, manifest: &PluginManifest) -> PluginRuntimeResult<()>;
}

/// Descriptor host used for manifest-only and built-in adapter plugins.
#[derive(Debug, Default)]
pub struct DescriptorPluginHost;

#[async_trait]
impl PluginHost for DescriptorPluginHost {
    fn runtime_kind(&self) -> PluginRuntimeKind {
        PluginRuntimeKind::DescriptorOnly
    }

    async fn start(&self, manifest: &PluginManifest) -> PluginRuntimeResult<()> {
        info!(
            plugin_id = %manifest.plugin_id,
            "descriptor plugin host start accepted without executing code"
        );
        Ok(())
    }

    async fn stop(&self, manifest: &PluginManifest) -> PluginRuntimeResult<()> {
        info!(
            plugin_id = %manifest.plugin_id,
            "descriptor plugin host stop accepted without executing code"
        );
        Ok(())
    }
}

/// Abstract Factory for plugin host strategies.
#[derive(Debug, Default, Clone)]
pub struct PluginHostFactory;

impl PluginHostFactory {
    /// Select the host strategy for a runtime declaration.
    pub fn create(&self, kind: &PluginRuntimeKind) -> PluginRuntimeResult<Arc<dyn PluginHost>> {
        match kind {
            PluginRuntimeKind::DescriptorOnly | PluginRuntimeKind::BuiltInAdapter => {
                info!(runtime_kind = %kind, "plugin descriptor host selected");
                Ok(Arc::new(DescriptorPluginHost))
            }
            other => {
                warn!(runtime_kind = %other, "plugin host runtime unavailable in v0");
                Err(PluginError::UnsupportedRuntime(other.to_string()))
            }
        }
    }
}

/// Coordinates lifecycle transitions and lifecycle event construction.
pub struct PluginLifecycleController {
    registry: Arc<PluginRegistry>,
}

impl PluginLifecycleController {
    /// Create a lifecycle controller over the kernel registry facade.
    pub fn new(registry: Arc<PluginRegistry>) -> Self {
        Self { registry }
    }

    /// Advance lifecycle state through the kernel registry and return an
    /// auditable event object for persistence by callers.
    pub async fn transition(
        &self,
        manifest: &PluginManifest,
        previous: Option<PluginLifecycleState>,
        next: PluginLifecycleState,
        operation: &str,
        trace: Option<TraceContext>,
    ) -> PluginRuntimeResult<PluginLifecycleEvent> {
        if let Some(previous) = previous.clone() {
            if previous != next {
                self.registry
                    .transition(&manifest.plugin_id, next.clone())
                    .await?;
            }
        }
        let event = PluginLifecycleEvent::new(manifest, previous, next, operation, "ok", trace);
        info!(
            plugin_id = %manifest.plugin_id,
            operation,
            state = ?event.next_state,
            "plugin lifecycle event built"
        );
        Ok(event)
    }
}

/// High-level facade used by hosts and future CLI/Web adapters.
pub struct PluginRuntimeFacade {
    guard: PluginRuntimeGuard,
    host_factory: PluginHostFactory,
    registry: Arc<PluginRegistry>,
}

impl PluginRuntimeFacade {
    /// Create a facade with Phase 07 default validation and host factory rules.
    pub fn new(registry: Arc<PluginRegistry>) -> Self {
        Self {
            guard: PluginRuntimeGuard::new(PluginManifestValidator::new()),
            host_factory: PluginHostFactory,
            registry,
        }
    }

    /// Create a facade with explicit strategies for tests and future embedding.
    pub fn with_parts(
        registry: Arc<PluginRegistry>,
        guard: PluginRuntimeGuard,
        host_factory: PluginHostFactory,
    ) -> Self {
        Self {
            guard,
            host_factory,
            registry,
        }
    }

    /// Install and register a descriptor-safe plugin manifest.
    ///
    /// This method is intentionally a facade: callers do not need to know
    /// whether validation, host selection, registry mutation, or trace event
    /// construction happened internally.
    pub async fn install_and_register(
        &self,
        manifest: PluginManifest,
        trace: Option<TraceContext>,
    ) -> PluginRuntimeResult<Vec<PluginLifecycleEvent>> {
        self.guard.authorize(&manifest)?;
        let _host = self.host_factory.create(&manifest.runtime.kind)?;
        self.registry.install(manifest.clone()).await?;
        let installed = PluginLifecycleEvent::new(
            &manifest,
            None,
            PluginLifecycleState::Installed,
            "install",
            "ok",
            trace.clone(),
        );
        self.registry.register(&manifest.plugin_id).await?;
        let registered = PluginLifecycleEvent::new(
            &manifest,
            Some(PluginLifecycleState::Installed),
            PluginLifecycleState::Registered,
            "register",
            "ok",
            trace,
        );
        info!(plugin_id = %manifest.plugin_id, "plugin installed and registered");
        Ok(vec![installed, registered])
    }

    /// Start a registered plugin through its selected host.
    pub async fn start(
        &self,
        manifest: &PluginManifest,
        trace: Option<TraceContext>,
    ) -> PluginRuntimeResult<PluginLifecycleEvent> {
        self.registry
            .transition(&manifest.plugin_id, PluginLifecycleState::Starting)
            .await?;
        let host = self.host_factory.create(&manifest.runtime.kind)?;
        host.start(manifest).await?;
        self.registry
            .transition(&manifest.plugin_id, PluginLifecycleState::Running)
            .await?;
        Ok(PluginLifecycleEvent::new(
            manifest,
            Some(PluginLifecycleState::Starting),
            PluginLifecycleState::Running,
            "start",
            "ok",
            trace,
        ))
    }

    /// Stop a running plugin through its selected host.
    pub async fn stop(
        &self,
        manifest: &PluginManifest,
        trace: Option<TraceContext>,
    ) -> PluginRuntimeResult<PluginLifecycleEvent> {
        self.registry
            .transition(&manifest.plugin_id, PluginLifecycleState::Stopping)
            .await?;
        let host = self.host_factory.create(&manifest.runtime.kind)?;
        host.stop(manifest).await?;
        self.registry
            .transition(&manifest.plugin_id, PluginLifecycleState::Stopped)
            .await?;
        Ok(PluginLifecycleEvent::new(
            manifest,
            Some(PluginLifecycleState::Stopping),
            PluginLifecycleState::Stopped,
            "stop",
            "ok",
            trace,
        ))
    }

    /// Uninstall a stopped plugin and remove every descriptor it owns.
    pub async fn uninstall(
        &self,
        manifest: &PluginManifest,
        trace: Option<TraceContext>,
    ) -> PluginRuntimeResult<PluginLifecycleEvent> {
        self.registry.uninstall(&manifest.plugin_id).await?;
        Ok(PluginLifecycleEvent::new(
            manifest,
            Some(PluginLifecycleState::Stopped),
            PluginLifecycleState::Uninstalled,
            "uninstall",
            "ok",
            trace,
        ))
    }
}

/// Helper for constructing a structured failure lifecycle event.
pub fn plugin_failure_event(
    manifest: &PluginManifest,
    previous_state: Option<PluginLifecycleState>,
    operation: &str,
    error: &PluginError,
    trace: Option<TraceContext>,
) -> PluginLifecycleEvent {
    let mut event = PluginLifecycleEvent::new(
        manifest,
        previous_state,
        PluginLifecycleState::Failed,
        operation,
        "error",
        trace,
    );
    event.error_code = Some(error.to_string());
    event
}

#[cfg(test)]
#[path = "plugin_tests.rs"]
mod plugin_tests;
