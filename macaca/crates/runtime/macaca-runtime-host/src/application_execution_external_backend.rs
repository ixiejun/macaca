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
    ApplicationExecutionControlResult, ApplicationExecutionEventType,
    ApplicationExecutionProviderDescriptor, ApplicationExecutionProviderHealth,
    ApplicationExecutionProviderKind, ApplicationExecutionProviderLease, ApplicationExecutionScope,
    ApplicationExecutionSnapshot, ExternalApplicationBackendExecutionProfile, ServiceError,
    StartApplicationExecutionCommand, StartApplicationExecutionResult,
};
use tokio::time::{timeout, Duration as TokioDuration};
use tracing::info;
use uuid::Uuid;

use crate::application_execution_external_backend_diagnostics::ExternalBackendHeartbeatDiagnostics;
use crate::application_execution_external_backend_results::{
    control_error, service_error_reason, start_invalid_schema, start_provider_failed,
};
use crate::application_execution_external_backend_transport::{
    ExternalApplicationBackendControlRequest, ExternalApplicationBackendStartRequest,
    ExternalApplicationBackendTransport, HttpExternalApplicationBackendTransport,
};
use crate::application_execution_provider_registry::ApplicationExecutionProvider;

const TRANSPORT_KIND: &str = "external_app_backend";
const CONTROL_DELIVERY_MAX_ATTEMPTS: u32 = 2;

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

    fn lease_diagnostics(&self) -> Result<ExternalBackendHeartbeatDiagnostics, ServiceError> {
        let leases = self
            .active_leases
            .read()
            .map_err(|_| {
                ServiceError::AdapterFailure("external backend lease lock poisoned".into())
            })?
            .values()
            .cloned()
            .collect();
        Ok(ExternalBackendHeartbeatDiagnostics::new(
            Utc::now(),
            self.profile.heartbeat_policy.required,
            leases,
        ))
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
            return Ok(start_invalid_schema(
                &self.descriptor.provider_id,
                command,
                "external backend start requires session_id",
            ));
        };
        let Some(run_value) = run_id.clone() else {
            return Ok(start_invalid_schema(
                &self.descriptor.provider_id,
                command,
                "external backend start requires run_id",
            ));
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
            Ok(Err(error)) => Ok(start_provider_failed(
                &self.descriptor.provider_id,
                command,
                service_error_reason(error),
                false,
            )),
            Err(_) => Ok(start_provider_failed(
                &self.descriptor.provider_id,
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
            return Ok(control_error(
                &self.descriptor.provider_id,
                command,
                ApplicationExecutionCommandStatus::Unsupported,
                "control kind is not declared by external backend profile",
                false,
            ));
        }
        let Some(endpoint) = self.profile.control_endpoint.clone() else {
            return Ok(control_error(
                &self.descriptor.provider_id,
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
                return Ok(control_error(
                    &self.descriptor.provider_id,
                    command,
                    ApplicationExecutionCommandStatus::Denied,
                    "control kind is not allowed by active external backend lease",
                    false,
                ));
            }
        }
        let audit_evidence_ref = format!(
            "audit.application_execution.external_backend.control.{}",
            command.idempotency_key
        );
        info!(
            application_id = %command.scope.application_id,
            session_id = %command.scope.session_id,
            run_id = %command.scope.run_id,
            trace_id = %command.trace.trace_id,
            provider_id = %self.descriptor.provider_id,
            control_id = %command.control_id,
            "external application backend control request dispatching"
        );
        let mut last_failure = None;
        // The retry loop is deliberately small and idempotency-key based. The
        // provider never interprets the control payload or command semantics;
        // it only resends the same generic protocol envelope with a higher
        // attempt number and a stable audit evidence reference so the backend
        // can safely de-duplicate while operators can trace every attempt.
        for attempt in 1..=CONTROL_DELIVERY_MAX_ATTEMPTS {
            let request = ExternalApplicationBackendControlRequest {
                endpoint: endpoint.clone(),
                scope: command.scope.clone(),
                command: command.command,
                control_id: command.control_id.clone(),
                reason_code: command.reason_code.clone(),
                trace: command.trace.clone(),
                payload: command.payload.clone(),
                lease_id: lease.as_ref().map(|lease| lease.lease_id.clone()),
                idempotency_key: command.idempotency_key.clone(),
                delivery_attempt: attempt,
                audit_evidence_ref: audit_evidence_ref.clone(),
            };
            info!(
                application_id = %command.scope.application_id,
                session_id = %command.scope.session_id,
                run_id = %command.scope.run_id,
                trace_id = %command.trace.trace_id,
                provider_id = %self.descriptor.provider_id,
                control_id = %command.control_id,
                delivery_attempt = attempt,
                audit_evidence_ref = %audit_evidence_ref,
                "external application backend control delivery attempt"
            );
            match timeout(self.request_timeout(), self.transport.control(request)).await {
                Ok(Ok(())) => {
                    return Ok(ApplicationExecutionControlResult {
                        status: ApplicationExecutionCommandStatus::Delivered,
                        scope,
                        provider_id: Some(self.descriptor.provider_id.clone()),
                        provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
                        event_cursor: None,
                        error: None,
                    });
                }
                Ok(Err(error)) => last_failure = Some((error.to_string(), false)),
                Err(_) => {
                    last_failure = Some(("external backend control request timed out".into(), true))
                }
            }
        }
        let (reason, retryable) = last_failure.unwrap_or_else(|| {
            (
                "external backend control request failed before delivery".into(),
                false,
            )
        });
        Ok(control_error(
            &self.descriptor.provider_id,
            command,
            if retryable {
                ApplicationExecutionCommandStatus::Timeout
            } else {
                ApplicationExecutionCommandStatus::ProviderFailed
            },
            reason,
            retryable,
        ))
    }

    async fn snapshot(&self) -> Result<Option<ApplicationExecutionSnapshot>, ServiceError> {
        let diagnostics = self.lease_diagnostics()?;
        let snapshot = diagnostics.snapshot(&self.descriptor.provider_id);
        if let Some(snapshot) = &snapshot {
            info!(
                application_id = %snapshot.scope.application_id,
                session_id = %snapshot.scope.session_id,
                run_id = %snapshot.scope.run_id,
                provider_id = %self.descriptor.provider_id,
                heartbeat_status = snapshot
                    .metadata
                    .get("heartbeat_status")
                    .map(String::as_str)
                    .unwrap_or("unknown"),
                failure_event_required = snapshot
                    .metadata
                    .get("failure_event_required")
                    .map(String::as_str)
                    .unwrap_or("false"),
                "external application backend diagnostic snapshot evaluated"
            );
        }
        Ok(snapshot)
    }

    async fn health(&self) -> Result<ApplicationExecutionProviderHealth, ServiceError> {
        let health = self.lease_diagnostics()?.health();
        info!(
            provider_id = %self.descriptor.provider_id,
            provider_kind = ?self.descriptor.provider_kind,
            health = ?health,
            "external application backend dynamic health evaluated"
        );
        Ok(health)
    }
}

fn millis(value: u64) -> ChronoDuration {
    ChronoDuration::milliseconds(value.min(i64::MAX as u64) as i64)
}
