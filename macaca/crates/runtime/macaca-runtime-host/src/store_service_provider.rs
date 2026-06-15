//! Runtime-host provider for the Store Service.
//!
//! Store Service v1 is metadata-first.  It exposes package inspection, resolve,
//! install, status, and snapshot commands without downloading raw package
//! payloads or owning payment settlement.  Paid install authorization is
//! delegated to Entitlement Service semantics through a crate-local guard.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_persist::{EntitlementStore, EventLog};
use macaca_proto::{
    CleanupPolicy, KernelServiceId, PackageManifest, PackageRuntime, PackageRuntimeKind,
    PackageType, ServiceCallResult, ServiceCapability, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, ServiceScope, ServiceType, StoreInstallResult,
    StorePackageInspectCommand, StorePackageInspectResult, StorePackageInstallCommand,
    StorePackageResolveCommand, StorePackageScope, StorePackageStatusCommand,
    StorePackageStatusResult, StorePackageView, StoreServiceSnapshot, StoreSnapshotCommand,
    TraceContext, TraceSchemaRef, STORE_PACKAGE_INSPECT_COMMAND, STORE_PACKAGE_INSTALL_COMMAND,
    STORE_PACKAGE_RESOLVE_COMMAND, STORE_PACKAGE_STATUS_COMMAND, STORE_SERVICE_ID,
    STORE_SNAPSHOT_COMMAND,
};
use tokio::sync::RwLock;
use tracing::info;

use crate::entitlement::RuntimeEntitlementGuard;
use crate::store_entitlement_admission::{FreeOpenFastPathSpec, PackageScopeSpec, StoreTraceSpec};

/// Store Service provider backed by an in-memory metadata index.
pub struct StoreSystemServiceProvider {
    descriptor: ServiceDescriptor,
    entitlement: Option<Arc<RuntimeEntitlementGuard>>,
    packages: RwLock<HashMap<String, StorePackageView>>,
}

impl StoreSystemServiceProvider {
    /// Create a Store provider that can delegate paid authorization.
    ///
    /// The provider accepts repository handles rather than a public entitlement
    /// facade. It builds the guard internally so Store remains a service
    /// adapter and callers cannot depend on runtime-host authorization internals.
    pub fn new(
        entitlement_store: Arc<dyn EntitlementStore>,
        event_log: Option<Arc<EventLog>>,
    ) -> Self {
        let entitlement = match event_log {
            Some(event_log) => Arc::new(RuntimeEntitlementGuard::with_event_log(
                entitlement_store,
                event_log,
            )),
            None => Arc::new(RuntimeEntitlementGuard::new(entitlement_store)),
        };
        Self {
            descriptor: store_service_descriptor(),
            entitlement: Some(entitlement),
            packages: RwLock::new(HashMap::new()),
        }
    }

    /// Create a Store provider from an already-built entitlement guard.
    ///
    /// This is crate-local so route bootstrap can share one guard between
    /// Entitlement and Store services without exposing the guard as public API.
    pub(crate) fn with_entitlement_guard(entitlement: Arc<RuntimeEntitlementGuard>) -> Self {
        Self {
            descriptor: store_service_descriptor(),
            entitlement: Some(entitlement),
            packages: RwLock::new(HashMap::new()),
        }
    }

    /// Create a metadata-only Store provider without paid authorization.
    pub fn metadata_only() -> Self {
        Self {
            descriptor: store_service_descriptor(),
            entitlement: None,
            packages: RwLock::new(HashMap::new()),
        }
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        let trace = command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)?;
        StoreTraceSpec::check(&trace)?;
        Ok(trace)
    }

    fn result<T: serde::Serialize>(
        value: T,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        Ok(ServiceCallResult {
            output: serde_json::to_value(value)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    fn package_key(scope: &StorePackageScope) -> String {
        scope.label()
    }

    fn view_from_install(command: &StorePackageInstallCommand, status: &str) -> StorePackageView {
        StorePackageView {
            package_id: command.scope.package_id.clone(),
            developer_id: command.scope.developer_id.clone(),
            package_ref: command.scope.package_ref.clone(),
            version: None,
            runtime_kind: command.runtime_kind.clone(),
            commerce: command.commerce.clone(),
            install_status: status.into(),
            source_status: "metadata_resolved".into(),
            diagnostics: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    fn manifest(command: &StorePackageInstallCommand) -> ServiceResult<PackageManifest> {
        let package_id = command.scope.package_id.clone().ok_or_else(|| {
            ServiceError::AdapterFailure("store.package.install requires package_id".into())
        })?;
        let developer_id = command.scope.developer_id.clone().ok_or_else(|| {
            ServiceError::AdapterFailure("store.package.install requires developer_id".into())
        })?;
        let mut manifest = PackageManifest::new(
            package_id,
            PackageType::Custom("store_package".into()),
            "0",
            developer_id,
            PackageRuntime::new(
                command
                    .runtime_kind
                    .clone()
                    .unwrap_or_else(|| PackageRuntimeKind::Custom("metadata_only".into())),
                "0",
            ),
        );
        manifest.commerce = command.commerce.clone();
        Ok(manifest)
    }
}

#[async_trait]
impl SystemService for StoreSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            entitlement_configured = self.entitlement.is_some(),
            "store service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "store service command accepted"
        );
        match command.name.as_str() {
            STORE_PACKAGE_INSPECT_COMMAND => {
                let typed: StorePackageInspectCommand = decode(command.payload)?;
                let packages = self.packages.read().await;
                let result: StorePackageInspectResult =
                    if let Some(package_id) = &typed.scope.package_id {
                        packages
                            .get(package_id.as_str())
                            .cloned()
                            .into_iter()
                            .collect()
                    } else if let Some(package_ref) = &typed.scope.package_ref {
                        packages.get(package_ref).cloned().into_iter().collect()
                    } else {
                        packages.values().cloned().collect()
                    };
                Self::result(result, typed.trace)
            }
            STORE_PACKAGE_RESOLVE_COMMAND => {
                let typed: StorePackageResolveCommand = decode(command.payload)?;
                let view = self
                    .packages
                    .read()
                    .await
                    .get(&Self::package_key(&typed.scope))
                    .cloned()
                    .unwrap_or_else(|| {
                        StorePackageView::unavailable(&typed.scope, "package source not found")
                    });
                Self::result(view, typed.trace)
            }
            STORE_PACKAGE_INSTALL_COMMAND => {
                let typed: StorePackageInstallCommand = decode(command.payload)?;
                PackageScopeSpec::require_package(
                    typed.scope.package_id.as_ref(),
                    STORE_PACKAGE_INSTALL_COMMAND,
                )?;
                let mut allowed = true;
                let mut reason = "metadata install accepted".to_string();
                if !FreeOpenFastPathSpec::allows_without_store(&typed.commerce) {
                    let entitlement = self.entitlement.clone().ok_or_else(|| {
                        ServiceError::ServiceUnavailable(
                            "paid package install requires Entitlement service".into(),
                        )
                    })?;
                    let manifest = Self::manifest(&typed)?;
                    let decision = entitlement
                        .authorize_install(&manifest)
                        .await
                        .map_err(commerce_error)?;
                    allowed = decision.allowed;
                    reason = decision.reason;
                }
                let view =
                    Self::view_from_install(&typed, if allowed { "installed" } else { "denied" });
                self.packages
                    .write()
                    .await
                    .insert(Self::package_key(&typed.scope), view.clone());
                let result = StoreInstallResult {
                    package: view,
                    allowed,
                    status: if allowed {
                        "installed".into()
                    } else {
                        "denied".into()
                    },
                    reason,
                    trace: typed.trace.clone(),
                };
                Self::result(result, typed.trace)
            }
            STORE_PACKAGE_STATUS_COMMAND => {
                let typed: StorePackageStatusCommand = decode(command.payload)?;
                let packages = self.packages.read().await;
                let result: StorePackageStatusResult =
                    if typed.scope.package_id.is_some() || typed.scope.package_ref.is_some() {
                        packages
                            .get(&Self::package_key(&typed.scope))
                            .cloned()
                            .into_iter()
                            .collect()
                    } else {
                        packages.values().cloned().collect()
                    };
                Self::result(result, typed.trace)
            }
            STORE_SNAPSHOT_COMMAND => {
                let typed: StoreSnapshotCommand = decode(command.payload)?;
                let package_count = self.packages.read().await.len();
                Self::result(StoreServiceSnapshot::healthy(package_count, 0), typed.trace)
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Store service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "store service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        self.packages.write().await.clear();
        info!(service_id = %self.descriptor.id, "store service provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

/// Build the provider-neutral descriptor for Store Service.
pub fn store_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(STORE_SERVICE_ID),
        ServiceType::new("store"),
        TraceSchemaRef::new("trace.system_service.store.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.store.package.inspect"),
            "Inspects sanitized package metadata.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.store.package.install"),
            "Coordinates metadata-level package install decisions.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.store.snapshot"),
            "Reports sanitized Store service snapshots.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![ServiceScope::Global, ServiceScope::Application("*".into())];
    descriptor.required_permissions = vec![
        "store.package.inspect".into(),
        "store.package.install".into(),
        "store.snapshot".into(),
    ];
    descriptor.cleanup_policy = CleanupPolicy::OnStop;
    descriptor
}

fn decode<T: serde::de::DeserializeOwned>(payload: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(payload).map_err(|error| ServiceError::AdapterFailure(error.to_string()))
}

fn commerce_error(error: impl std::fmt::Display) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
