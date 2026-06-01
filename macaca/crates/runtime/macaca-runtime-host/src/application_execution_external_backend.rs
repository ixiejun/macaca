//! External application backend provider adapter.
//!
//! This module is the runtime-host Adapter for manifest-declared external
//! application backends.  It validates an Application Framework execution
//! profile and exposes a provider descriptor that the Strategy registry can
//! reason about.  Concrete network start/control transports are intentionally
//! not implemented here yet; those later adapters must enter through this same
//! provider boundary with trace, timeout, retry, lease, and audit evidence.

use async_trait::async_trait;
use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlResult, ApplicationExecutionError, ApplicationExecutionEventType,
    ApplicationExecutionProviderDescriptor, ApplicationExecutionProviderHealth,
    ApplicationExecutionProviderKind, ApplicationExecutionSnapshot,
    ExternalApplicationBackendExecutionProfile, ServiceError, StartApplicationExecutionCommand,
    StartApplicationExecutionResult,
};
use tracing::{info, warn};

use crate::application_execution_provider_registry::ApplicationExecutionProvider;

const TRANSPORT_KIND: &str = "external_app_backend";

/// Provider adapter built from a manifest-declared external backend profile.
#[derive(Debug)]
pub struct ExternalApplicationBackendProvider {
    profile: ExternalApplicationBackendExecutionProfile,
    descriptor: ApplicationExecutionProviderDescriptor,
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
        warn!(
            application_id = %command.application_id,
            trace_id = %command.trace.trace_id,
            provider_id = %self.descriptor.provider_id,
            "external application backend start transport is not wired yet"
        );
        Ok(StartApplicationExecutionResult {
            status: ApplicationExecutionCommandStatus::Unsupported,
            session_id: session_id.clone(),
            run_id: run_id.clone(),
            provider_id: Some(self.descriptor.provider_id.clone()),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            event_cursor: None,
            control_ref: None,
            workspace_ref: command.workspace_ref,
            error: Some(ApplicationExecutionError {
                code: ApplicationExecutionCommandStatus::Unsupported,
                layer: "service.application_execution.external_backend".into(),
                operation: "start".into(),
                application_id: Some(command.application_id),
                session_id,
                run_id,
                provider_id: Some(self.descriptor.provider_id.clone()),
                provider_kind: Some(ApplicationExecutionProviderKind::ExternalAppBackend),
                trace_id: Some(command.trace.trace_id),
                reason: "external backend start transport is not configured".into(),
                retryable: false,
            }),
        })
    }

    async fn control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> Result<ApplicationExecutionControlResult, ServiceError> {
        let scope = command.scope.clone();
        warn!(
            application_id = %command.scope.application_id,
            session_id = %command.scope.session_id,
            run_id = %command.scope.run_id,
            trace_id = %command.trace.trace_id,
            provider_id = %self.descriptor.provider_id,
            "external application backend control transport is not wired yet"
        );
        Ok(ApplicationExecutionControlResult {
            status: ApplicationExecutionCommandStatus::Unsupported,
            scope: scope.clone(),
            provider_id: Some(self.descriptor.provider_id.clone()),
            provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
            event_cursor: None,
            error: Some(ApplicationExecutionError {
                code: ApplicationExecutionCommandStatus::Unsupported,
                layer: "service.application_execution.external_backend".into(),
                operation: "control".into(),
                application_id: Some(scope.application_id),
                session_id: Some(scope.session_id),
                run_id: Some(scope.run_id),
                provider_id: Some(self.descriptor.provider_id.clone()),
                provider_kind: Some(ApplicationExecutionProviderKind::ExternalAppBackend),
                trace_id: Some(command.trace.trace_id),
                reason: "external backend control transport is not configured".into(),
                retryable: false,
            }),
        })
    }

    async fn snapshot(&self) -> Result<Option<ApplicationExecutionSnapshot>, ServiceError> {
        Ok(None)
    }
}
