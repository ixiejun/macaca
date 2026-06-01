//! External application backend provider adapter.
//!
//! This module is the runtime-host Adapter for manifest-declared external
//! application backends. It validates an Application Framework execution
//! profile, maps typed protocol commands into a backend-neutral transport
//! envelope, and stores the scoped callback lease metadata needed for later
//! gateway validation. The adapter owns no application workflow semantics: it
//! only speaks the generic application execution protocol.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlResult, ApplicationExecutionError, ApplicationExecutionEventType,
    ApplicationExecutionProviderDescriptor, ApplicationExecutionProviderHealth,
    ApplicationExecutionProviderKind, ApplicationExecutionProviderLease, ApplicationExecutionScope,
    ApplicationExecutionSnapshot, ExternalApplicationBackendExecutionProfile, ServiceError,
    StartApplicationExecutionCommand, StartApplicationExecutionResult,
};
use tokio::time::{timeout, Duration as TokioDuration};
use tracing::{info, warn};
use uuid::Uuid;

use crate::application_execution_external_backend_transport::{
    ExternalApplicationBackendControlRequest, ExternalApplicationBackendStartRequest,
    ExternalApplicationBackendTransport, HttpExternalApplicationBackendTransport,
};
use crate::application_execution_provider_registry::ApplicationExecutionProvider;

const TRANSPORT_KIND: &str = "external_app_backend";

/// Provider adapter built from a manifest-declared external backend profile.
pub struct ExternalApplicationBackendProvider {
    profile: ExternalApplicationBackendExecutionProfile,
    descriptor: ApplicationExecutionProviderDescriptor,
    transport: Arc<dyn ExternalApplicationBackendTransport>,
    active_leases: RwLock<BTreeMap<String, ApplicationExecutionProviderLease>>,
}

impl fmt::Debug for ExternalApplicationBackendProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExternalApplicationBackendProvider")
            .field("provider_id", &self.descriptor.provider_id)
            .field("provider_kind", &self.descriptor.provider_kind)
            .field("protocol_version", &self.descriptor.protocol_version)
            .finish_non_exhaustive()
    }
}

impl ExternalApplicationBackendProvider {
    /// Validate a manifest profile and build the provider descriptor.
    ///
    /// The constructor is the runtime-host admission seam for Task 7.2.  It is
    /// deliberately side-effect free: no socket, HTTP client, callback lease, or
    /// app-specific runtime is created.  Registration can therefore reject
    /// malformed declarations before any provider side effect becomes possible.
    pub fn from_profile(
        profile: ExternalApplicationBackendExecutionProfile,
    ) -> Result<Self, ServiceError> {
        Self::with_transport(
            profile,
            Arc::new(HttpExternalApplicationBackendTransport::new()),
        )
    }

    /// Build a provider with an injected transport implementation.
    ///
    /// This Builder-style constructor keeps the Strategy provider testable and
    /// replaceable. Production code can use the default HTTP transport while
    /// tests, plugins, or future local IPC adapters can supply another
    /// implementation without changing selection, lease, or audit behavior.
    pub fn with_transport(
        profile: ExternalApplicationBackendExecutionProfile,
        transport: Arc<dyn ExternalApplicationBackendTransport>,
    ) -> Result<Self, ServiceError> {
        profile
            .validate()
            .map_err(|error| ServiceError::InvalidArgument(error.to_string()))?;
        let descriptor = ApplicationExecutionProviderDescriptor {
            provider_id: profile.provider_id.clone(),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            protocol_version: profile.protocol_version.clone(),
            supported_commands: profile.supported_controls.clone(),
            supported_events: vec![
                ApplicationExecutionEventType::ProviderHeartbeat,
                ApplicationExecutionEventType::ProviderSnapshot,
                ApplicationExecutionEventType::ApprovalRequested,
                ApplicationExecutionEventType::ExecutionCompleted,
                ApplicationExecutionEventType::ExecutionFailed,
            ],
            checkpoint_support: false,
            heartbeat_policy: profile.heartbeat_policy.clone(),
            control_delivery: profile
                .control_endpoint
                .as_ref()
                .map(|_| "external_backend_control_endpoint")
                .unwrap_or("external_backend_callback")
                .into(),
            capability_declarations: profile.capability_declarations.clone(),
            resource_profile: profile.resource_profile.clone(),
            transport_kind: TRANSPORT_KIND.into(),
            health_state: ApplicationExecutionProviderHealth::Healthy,
        };
        info!(
            provider_id = %descriptor.provider_id,
            protocol_version = %descriptor.protocol_version,
            supported_controls = descriptor.supported_commands.len(),
            heartbeat_required = descriptor.heartbeat_policy.required,
            "external application backend provider profile admitted"
        );
        Ok(Self {
            profile,
            descriptor,
            transport,
            active_leases: RwLock::new(BTreeMap::new()),
        })
    }

    /// Return the callback identity reference used later for scoped leases.
    pub fn callback_identity_ref(&self) -> &str {
        &self.profile.callback_identity_ref
    }

    /// Return the event schema version expected at callback ingress.
    pub fn event_schema_version(&self) -> &str {
        &self.profile.event_schema_version
    }

    fn request_timeout(&self) -> TokioDuration {
        TokioDuration::from_millis(self.profile.request_timeout_ms)
    }

    fn build_lease(&self, scope: ApplicationExecutionScope) -> ApplicationExecutionProviderLease {
        let now = Utc::now();
        let heartbeat_timeout = millis(self.profile.heartbeat_policy.timeout_ms);
        let request_timeout = millis(self.profile.request_timeout_ms);
        ApplicationExecutionProviderLease {
            lease_id: format!("lease-{}", Uuid::new_v4()),
            provider_id: self.descriptor.provider_id.clone(),
            scope,
            expires_at: now + request_timeout,
            heartbeat_deadline: now + heartbeat_timeout,
            callback_identity_ref: self.profile.callback_identity_ref.clone(),
            allowed_event_types: self.descriptor.supported_events.clone(),
            allowed_controls: self.profile.supported_controls.clone(),
        }
    }

    fn store_lease(&self, lease: ApplicationExecutionProviderLease) -> Result<(), ServiceError> {
        self.active_leases
            .write()
            .map_err(|_| {
                ServiceError::AdapterFailure("external backend lease lock poisoned".into())
            })?
            .insert(lease.lease_id.clone(), lease);
        Ok(())
    }

    fn lease_for_scope(
        &self,
        scope: &ApplicationExecutionScope,
    ) -> Result<Option<ApplicationExecutionProviderLease>, ServiceError> {
        Ok(self
            .active_leases
            .read()
            .map_err(|_| {
                ServiceError::AdapterFailure("external backend lease lock poisoned".into())
            })?
            .values()
            .find(|lease| {
                lease.scope.application_id == scope.application_id
                    && lease.scope.session_id == scope.session_id
                    && lease.scope.run_id == scope.run_id
            })
            .cloned())
    }
}

#[async_trait]
impl ApplicationExecutionProvider for ExternalApplicationBackendProvider {
    fn describe(&self) -> ApplicationExecutionProviderDescriptor {
        self.descriptor.clone()
    }

    async fn start(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> Result<StartApplicationExecutionResult, ServiceError> {
        let session_id = command.session_id.clone();
        let run_id = command.run_id.clone();
        let Some(session_value) = session_id.clone() else {
            return Ok(
                self.start_invalid_schema(command, "external backend start requires session_id")
            );
        };
        let Some(run_value) = run_id.clone() else {
            return Ok(self.start_invalid_schema(command, "external backend start requires run_id"));
        };
        let scope = ApplicationExecutionScope {
            application_id: command.application_id,
            session_id: session_value,
            run_id: run_value,
            tenant_id: command.tenant_id.clone(),
            actor: command.actor.clone(),
        };
        let lease = self.build_lease(scope);
        let request = ExternalApplicationBackendStartRequest {
            endpoint: self.profile.start_endpoint.clone(),
            application_id: command.application_id,
            session_id: session_id.clone(),
            run_id: run_id.clone(),
            workspace_ref: command.workspace_ref.clone(),
            task_input: command.task_input.clone(),
            requested_capabilities: command.requested_capabilities.clone(),
            callback_gateway_ref: self.profile.callback_gateway_ref.clone(),
            callback_identity_ref: self.profile.callback_identity_ref.clone(),
            lease_id: lease.lease_id.clone(),
            allowed_event_types: lease.allowed_event_types.clone(),
            allowed_controls: lease.allowed_controls.clone(),
            heartbeat_policy: self.profile.heartbeat_policy.clone(),
            event_schema_version: self.profile.event_schema_version.clone(),
            trace: command.trace.clone(),
            idempotency_key: command.idempotency_key.clone(),
        };
        info!(
            application_id = %command.application_id,
            trace_id = %command.trace.trace_id,
            provider_id = %self.descriptor.provider_id,
            lease_id = %lease.lease_id,
            "external application backend start request dispatching"
        );
        let delivered = timeout(self.request_timeout(), self.transport.start(request)).await;
        match delivered {
            Ok(Ok(())) => {
                self.store_lease(lease.clone())?;
                info!(
                    application_id = %command.application_id,
                    session_id = %lease.scope.session_id,
                    run_id = %lease.scope.run_id,
                    trace_id = %command.trace.trace_id,
                    provider_id = %self.descriptor.provider_id,
                    lease_id = %lease.lease_id,
                    "external application backend start accepted"
                );
                Ok(StartApplicationExecutionResult {
                    status: ApplicationExecutionCommandStatus::Accepted,
                    session_id,
                    run_id,
                    provider_id: Some(self.descriptor.provider_id.clone()),
                    provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
                    event_cursor: None,
                    control_ref: Some(format!(
                        "application-execution.external-backend://{}/{}/{}/{}",
                        self.descriptor.provider_id,
                        lease.scope.session_id,
                        lease.scope.run_id,
                        lease.lease_id
                    )),
                    workspace_ref: command.workspace_ref,
                    error: None,
                })
            }
            Ok(Err(error)) => Ok(self.start_provider_failed(command, error.to_string(), false)),
            Err(_) => Ok(self.start_provider_failed(
                command,
                "external backend start request timed out",
                true,
            )),
        }
    }

    async fn control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionControlResult, ServiceError> {
        let scope = command.scope.clone();
        if !self
            .profile
            .supported_controls
            .iter()
            .any(|supported| supported == &command.command)
        {
            return Ok(self.control_error(
                command,
                ApplicationExecutionCommandStatus::Unsupported,
                "control kind is not declared by external backend profile",
                false,
            ));
        }
        let Some(endpoint) = self.profile.control_endpoint.clone() else {
            return Ok(self.control_error(
                command,
                ApplicationExecutionCommandStatus::Unsupported,
                "external backend control endpoint is not configured",
                false,
            ));
        };
        let lease = self.lease_for_scope(&command.scope)?;
        if let Some(lease) = &lease {
            if !lease
                .allowed_controls
                .iter()
                .any(|allowed| allowed == &command.command)
            {
                return Ok(self.control_error(
                    command,
                    ApplicationExecutionCommandStatus::Denied,
                    "control kind is not allowed by active external backend lease",
                    false,
                ));
            }
        }
        let request = ExternalApplicationBackendControlRequest {
            endpoint,
            scope: command.scope.clone(),
            command: command.command,
            control_id: command.control_id.clone(),
            reason_code: command.reason_code.clone(),
            trace: command.trace.clone(),
            payload: command.payload.clone(),
            lease_id: lease.as_ref().map(|lease| lease.lease_id.clone()),
            idempotency_key: command.idempotency_key.clone(),
        };
        info!(
            application_id = %command.scope.application_id,
            session_id = %command.scope.session_id,
            run_id = %command.scope.run_id,
            trace_id = %command.trace.trace_id,
            provider_id = %self.descriptor.provider_id,
            control_id = %command.control_id,
            "external application backend control request dispatching"
        );
        match timeout(self.request_timeout(), self.transport.control(request)).await {
            Ok(Ok(())) => Ok(ApplicationExecutionControlResult {
                status: ApplicationExecutionCommandStatus::Delivered,
                scope,
                provider_id: Some(self.descriptor.provider_id.clone()),
                provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
                event_cursor: None,
                error: None,
            }),
            Ok(Err(error)) => Ok(self.control_error(
                command,
                ApplicationExecutionCommandStatus::ProviderFailed,
                error.to_string(),
                false,
            )),
            Err(_) => Ok(self.control_error(
                command,
                ApplicationExecutionCommandStatus::Timeout,
                "external backend control request timed out",
                true,
            )),
        }
    }

    async fn snapshot(&self) -> Result<Option<ApplicationExecutionSnapshot>, ServiceError> {
        Ok(None)
    }
}

impl ExternalApplicationBackendProvider {
    fn start_invalid_schema(
        &self,
        command: StartApplicationExecutionCommand,
        reason: impl Into<String>,
    ) -> StartApplicationExecutionResult {
        self.start_error(
            command,
            ApplicationExecutionCommandStatus::InvalidSchema,
            reason,
            false,
        )
    }

    fn start_provider_failed(
        &self,
        command: StartApplicationExecutionCommand,
        reason: impl Into<String>,
        retryable: bool,
    ) -> StartApplicationExecutionResult {
        self.start_error(
            command,
            if retryable {
                ApplicationExecutionCommandStatus::Timeout
            } else {
                ApplicationExecutionCommandStatus::ProviderFailed
            },
            reason,
            retryable,
        )
    }

    fn start_error(
        &self,
        command: StartApplicationExecutionCommand,
        status: ApplicationExecutionCommandStatus,
        reason: impl Into<String>,
        retryable: bool,
    ) -> StartApplicationExecutionResult {
        let session_id = command.session_id.clone();
        let run_id = command.run_id.clone();
        let reason = reason.into();
        warn!(
            application_id = %command.application_id,
            trace_id = %command.trace.trace_id,
            provider_id = %self.descriptor.provider_id,
            status = ?status,
            reason = %reason,
            "external application backend start failed"
        );
        StartApplicationExecutionResult {
            status,
            session_id: session_id.clone(),
            run_id: run_id.clone(),
            provider_id: Some(self.descriptor.provider_id.clone()),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            event_cursor: None,
            control_ref: None,
            workspace_ref: command.workspace_ref,
            error: Some(ApplicationExecutionError {
                code: status,
                layer: "service.application_execution.external_backend".into(),
                operation: "start".into(),
                application_id: Some(command.application_id),
                session_id,
                run_id,
                provider_id: Some(self.descriptor.provider_id.clone()),
                provider_kind: Some(ApplicationExecutionProviderKind::ExternalAppBackend),
                trace_id: Some(command.trace.trace_id),
                reason,
                retryable,
            }),
        }
    }

    fn control_error(
        &self,
        command: ApplicationExecutionControlCommand,
        status: ApplicationExecutionCommandStatus,
        reason: impl Into<String>,
        retryable: bool,
    ) -> ApplicationExecutionControlResult {
        let scope = command.scope.clone();
        let reason = reason.into();
        warn!(
            application_id = %scope.application_id,
            session_id = %scope.session_id,
            run_id = %scope.run_id,
            trace_id = %command.trace.trace_id,
            provider_id = %self.descriptor.provider_id,
            status = ?status,
            reason = %reason,
            "external application backend control failed"
        );
        ApplicationExecutionControlResult {
            status,
            scope: scope.clone(),
            provider_id: Some(self.descriptor.provider_id.clone()),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            event_cursor: None,
            error: Some(ApplicationExecutionError {
                code: status,
                layer: "service.application_execution.external_backend".into(),
                operation: "control".into(),
                application_id: Some(scope.application_id),
                session_id: Some(scope.session_id),
                run_id: Some(scope.run_id),
                provider_id: Some(self.descriptor.provider_id.clone()),
                provider_kind: Some(ApplicationExecutionProviderKind::ExternalAppBackend),
                trace_id: Some(command.trace.trace_id),
                reason,
                retryable,
            }),
        }
    }
}

fn millis(value: u64) -> ChronoDuration {
    ChronoDuration::milliseconds(value.min(i64::MAX as u64) as i64)
}
