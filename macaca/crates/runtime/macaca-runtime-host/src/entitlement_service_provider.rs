//! Runtime-host provider for the Entitlement Service.
//!
//! The provider uses Adapter/Bridge: the service bus speaks provider-neutral
//! `macaca-proto` commands while this adapter delegates policy decisions and
//! persistence to the crate-local entitlement guard and `EntitlementStore`.
//! Runtime-host owns service lifecycle, trace-required dispatch, and logging;
//! it does not own Store vendor logic or application-specific commerce rules.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use macaca_kernel::SystemService;
use macaca_persist::{EntitlementStore, EventLog};
use macaca_proto::{
    CleanupPolicy, CommerceMetadata, EntitlementAuditPage, EntitlementAuditQueryCommand,
    EntitlementAuthorizeCallCommand, EntitlementAuthorizeInstallCommand,
    EntitlementAuthorizeResult, EntitlementAuthorizeStartCommand, EntitlementDecisionView,
    EntitlementMeteringRecordCommand, EntitlementQueryCommand, EntitlementQueryResult,
    EntitlementRecord, EntitlementRevokeCommand, EntitlementServiceScope,
    EntitlementServiceSnapshot, EntitlementSnapshotCommand, EntitlementState,
    EntitlementUpsertCommand, KernelServiceId, PackageManifest, PackageRuntime, PackageRuntimeKind,
    PackageType, ServiceCallResult, ServiceCapability, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, ServiceScope, ServiceType, TraceContext,
    TraceSchemaRef, ENTITLEMENT_AUDIT_QUERY_COMMAND, ENTITLEMENT_AUTHORIZE_CALL_COMMAND,
    ENTITLEMENT_AUTHORIZE_INSTALL_COMMAND, ENTITLEMENT_AUTHORIZE_START_COMMAND,
    ENTITLEMENT_METERING_RECORD_COMMAND, ENTITLEMENT_QUERY_COMMAND, ENTITLEMENT_REVOKE_COMMAND,
    ENTITLEMENT_SERVICE_ID, ENTITLEMENT_SNAPSHOT_COMMAND, ENTITLEMENT_UPSERT_COMMAND,
};
use tokio::sync::RwLock;
use tracing::info;

use crate::entitlement::{CapabilityCallContext, RuntimeEntitlementGuard};
use crate::store_entitlement_admission::{
    EntitlementOperationSpec, MeteringScopeSpec, PackageScopeSpec, StoreTraceSpec,
};

/// Entitlement Service provider backed by repository and runtime guard facade.
pub struct EntitlementSystemServiceProvider {
    descriptor: ServiceDescriptor,
    store: Option<Arc<dyn EntitlementStore>>,
    guard: Option<Arc<RuntimeEntitlementGuard>>,
    metering_count: RwLock<usize>,
}

impl EntitlementSystemServiceProvider {
    /// Create a provider backed by a real entitlement repository.
    ///
    /// The provider constructs the guard internally so host composition roots do
    /// not receive a second public entitlement facade. This keeps authorization
    /// behavior behind the service boundary while preserving repository
    /// injection for tests and local runtimes.
    pub fn new(store: Arc<dyn EntitlementStore>, event_log: Option<Arc<EventLog>>) -> Self {
        let guard = match event_log {
            Some(event_log) => Arc::new(RuntimeEntitlementGuard::with_event_log(
                Arc::clone(&store),
                event_log,
            )),
            None => Arc::new(RuntimeEntitlementGuard::new(Arc::clone(&store))),
        };
        Self {
            descriptor: entitlement_service_descriptor(),
            store: Some(store),
            guard: Some(guard),
            metering_count: RwLock::new(0),
        }
    }

    /// Create a provider from an already-built guard for crate-local wiring.
    pub(crate) fn with_guard(
        store: Arc<dyn EntitlementStore>,
        guard: Arc<RuntimeEntitlementGuard>,
    ) -> Self {
        Self {
            descriptor: entitlement_service_descriptor(),
            store: Some(store),
            guard: Some(guard),
            metering_count: RwLock::new(0),
        }
    }

    /// Create a Null Object provider that returns structured unavailable.
    pub fn unavailable() -> Self {
        Self {
            descriptor: entitlement_service_descriptor(),
            store: None,
            guard: None,
            metering_count: RwLock::new(0),
        }
    }

    fn store(&self) -> ServiceResult<Arc<dyn EntitlementStore>> {
        self.store.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable("entitlement store is not configured".into())
        })
    }

    fn guard(&self) -> ServiceResult<Arc<RuntimeEntitlementGuard>> {
        self.guard.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable("entitlement guard is not configured".into())
        })
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

    fn manifest(
        scope: &EntitlementServiceScope,
        commerce: CommerceMetadata,
    ) -> ServiceResult<PackageManifest> {
        let package_id = scope.package_id.clone().ok_or_else(|| {
            ServiceError::AdapterFailure("entitlement authorization requires package_id".into())
        })?;
        let developer_id = scope.developer_id.clone().ok_or_else(|| {
            ServiceError::AdapterFailure("entitlement authorization requires developer_id".into())
        })?;
        let mut manifest = PackageManifest::new(
            package_id,
            PackageType::Custom("store_entitlement_subject".into()),
            "0",
            developer_id,
            PackageRuntime::new(PackageRuntimeKind::Custom("metadata_only".into()), "0"),
        );
        manifest.commerce = commerce;
        Ok(manifest)
    }
}

#[async_trait]
impl SystemService for EntitlementSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            store_configured = self.store.is_some(),
            guard_configured = self.guard.is_some(),
            "entitlement service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "entitlement service command accepted"
        );
        match command.name.as_str() {
            ENTITLEMENT_QUERY_COMMAND => {
                let typed: EntitlementQueryCommand = decode(command.payload)?;
                let result: EntitlementQueryResult = if let Some(id) = &typed.scope.entitlement_id {
                    self.store()?.get_record(id).await.map_err(commerce_error)?
                } else {
                    let package_id = typed.scope.package_id.as_ref().ok_or_else(|| {
                        ServiceError::AdapterFailure(
                            "entitlement.query requires entitlement_id or package_id".into(),
                        )
                    })?;
                    self.store()?
                        .resolve_for_package(package_id)
                        .await
                        .map_err(commerce_error)?
                };
                Self::result(result, typed.trace)
            }
            ENTITLEMENT_UPSERT_COMMAND => {
                let typed: EntitlementUpsertCommand = decode(command.payload)?;
                self.store()?
                    .upsert_record(typed.record.clone())
                    .await
                    .map_err(commerce_error)?;
                Self::result(typed.record, typed.trace)
            }
            ENTITLEMENT_REVOKE_COMMAND => {
                let typed: EntitlementRevokeCommand = decode(command.payload)?;
                let package_id = typed.scope.package_id.clone().ok_or_else(|| {
                    ServiceError::AdapterFailure("entitlement.revoke requires package_id".into())
                })?;
                let developer_id = typed.scope.developer_id.clone().ok_or_else(|| {
                    ServiceError::AdapterFailure("entitlement.revoke requires developer_id".into())
                })?;
                let now = Utc::now();
                let record = EntitlementRecord {
                    entitlement_id: typed.scope.entitlement_id.clone().unwrap_or_else(|| {
                        macaca_proto::EntitlementId::new(format!("revoked.{package_id}"))
                    }),
                    package_id,
                    developer_id,
                    state: EntitlementState::revoked(),
                    granted_at: now,
                    updated_at: now,
                    expires_at: None,
                    metadata: HashMap::from([("reason".into(), typed.reason)])
                        .into_iter()
                        .collect(),
                };
                self.store()?
                    .upsert_record(record.clone())
                    .await
                    .map_err(commerce_error)?;
                Self::result(record, typed.trace)
            }
            ENTITLEMENT_AUTHORIZE_INSTALL_COMMAND => {
                EntitlementOperationSpec::check("install")?;
                let typed: EntitlementAuthorizeInstallCommand = decode(command.payload)?;
                PackageScopeSpec::require_package(
                    typed.scope.package_id.as_ref(),
                    "entitlement.authorize.install",
                )?;
                let manifest = Self::manifest(&typed.scope, typed.commerce)?;
                let decision = self
                    .guard()?
                    .authorize_install(&manifest)
                    .await
                    .map_err(commerce_error)?;
                let view: EntitlementAuthorizeResult = EntitlementDecisionView::from(decision);
                Self::result(view, typed.trace)
            }
            ENTITLEMENT_AUTHORIZE_START_COMMAND => {
                EntitlementOperationSpec::check("start")?;
                let typed: EntitlementAuthorizeStartCommand = decode(command.payload)?;
                PackageScopeSpec::require_package(
                    typed.scope.package_id.as_ref(),
                    "entitlement.authorize.start",
                )?;
                let manifest = Self::manifest(&typed.scope, typed.commerce)?;
                let decision = self
                    .guard()?
                    .authorize_start(&manifest)
                    .await
                    .map_err(commerce_error)?;
                Self::result(EntitlementDecisionView::from(decision), typed.trace)
            }
            ENTITLEMENT_AUTHORIZE_CALL_COMMAND => {
                EntitlementOperationSpec::check("call")?;
                let typed: EntitlementAuthorizeCallCommand = decode(command.payload)?;
                MeteringScopeSpec::check(typed.quantity)?;
                let manifest = Self::manifest(&typed.scope, typed.commerce)?;
                let mut context = CapabilityCallContext::default();
                context.app_id = typed.scope.application_id.clone();
                context.session_id = typed.scope.session_id.clone();
                context.capability_id = typed.scope.capability_id.clone();
                context.quantity = typed.quantity;
                context.unit = typed.unit.clone();
                let decision = self
                    .guard()?
                    .authorize_capability_call(&manifest, context)
                    .await
                    .map_err(commerce_error)?;
                *self.metering_count.write().await += 1;
                Self::result(EntitlementDecisionView::from(decision), typed.trace)
            }
            ENTITLEMENT_AUDIT_QUERY_COMMAND => {
                let typed: EntitlementAuditQueryCommand = decode(command.payload)?;
                let package_id = typed.scope.package_id.as_ref().ok_or_else(|| {
                    ServiceError::AdapterFailure(
                        "entitlement.audit.query requires package_id".into(),
                    )
                })?;
                let mut decisions = self
                    .store()?
                    .decision_audit_by_package(package_id)
                    .await
                    .map_err(commerce_error)?
                    .into_iter()
                    .map(EntitlementDecisionView::from)
                    .collect::<Vec<_>>();
                if let Some(limit) = typed.limit {
                    decisions.truncate(limit);
                }
                Self::result(
                    EntitlementAuditPage {
                        package_id: Some(package_id.clone()),
                        decisions,
                        next_offset: None,
                    },
                    typed.trace,
                )
            }
            ENTITLEMENT_METERING_RECORD_COMMAND => {
                let typed: EntitlementMeteringRecordCommand = decode(command.payload)?;
                *self.metering_count.write().await += 1;
                info!(
                    trace_id = %typed.trace.trace_id,
                    package_id = %typed.event.package_id,
                    operation = %typed.event.operation,
                    "entitlement service metering record accepted"
                );
                Self::result(typed.event, typed.trace)
            }
            ENTITLEMENT_SNAPSHOT_COMMAND => {
                let typed: EntitlementSnapshotCommand = decode(command.payload)?;
                let snapshot =
                    EntitlementServiceSnapshot::healthy(0, 0, *self.metering_count.read().await);
                Self::result(snapshot, typed.trace)
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Entitlement service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "entitlement service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "entitlement service provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if self.store.is_some() && self.guard.is_some() {
            Ok(ServiceHealth::Healthy)
        } else {
            Ok(ServiceHealth::Unavailable {
                reason: "entitlement provider dependencies are not fully configured".into(),
            })
        }
    }
}

/// Build the provider-neutral descriptor for Entitlement Service.
pub fn entitlement_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(ENTITLEMENT_SERVICE_ID),
        ServiceType::new("entitlement"),
        TraceSchemaRef::new("trace.system_service.entitlement.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.entitlement.authorize"),
            "Authorizes commercial install/start/call operations.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.entitlement.audit"),
            "Reports sanitized entitlement decision audit pages.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.entitlement.metering"),
            "Records metering events for paid capability calls.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![
        ServiceScope::Global,
        ServiceScope::Application("*".into()),
        ServiceScope::Session("*".into()),
    ];
    descriptor.required_permissions = vec![
        "entitlement.query".into(),
        "entitlement.authorize".into(),
        "entitlement.audit".into(),
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
