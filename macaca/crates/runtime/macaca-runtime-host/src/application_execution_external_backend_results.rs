//! Structured result builders for the external backend provider.
//!
//! These helpers keep error-result construction separate from provider routing.
//! They are plain Builder-style functions over provider-neutral protocol DTOs,
//! so they cannot introduce application-specific behavior or concrete backend
//! dependencies.

use macaca_proto::{
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlResult, ApplicationExecutionError, ApplicationExecutionProviderKind,
    ServiceError, StartApplicationExecutionCommand, StartApplicationExecutionResult,
};
use tracing::warn;

/// Build a structured invalid-schema start result.
pub(crate) fn start_invalid_schema(
    provider_id: &str,
    command: StartApplicationExecutionCommand,
    reason: impl Into<String>,
) -> StartApplicationExecutionResult {
    start_error(
        provider_id,
        command,
        ApplicationExecutionCommandStatus::InvalidSchema,
        reason,
        false,
    )
}

/// Build a structured provider-failed or timeout start result.
pub(crate) fn start_provider_failed(
    provider_id: &str,
    command: StartApplicationExecutionCommand,
    reason: impl Into<String>,
    retryable: bool,
) -> StartApplicationExecutionResult {
    start_error(
        provider_id,
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

/// Build a structured control delivery failure result.
pub(crate) fn control_error(
    provider_id: &str,
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
        provider_id = %provider_id,
        status = ?status,
        reason = %reason,
        "external application backend control failed"
    );
    ApplicationExecutionControlResult {
        status,
        scope: scope.clone(),
        provider_id: Some(provider_id.into()),
        provider_kind: ApplicationExecutionProviderKind::ExternalAppBackend,
        event_cursor: None,
        error: Some(ApplicationExecutionError {
            code: status,
            layer: "service.application_execution.external_backend".into(),
            operation: "control".into(),
            application_id: Some(scope.application_id),
            session_id: Some(scope.session_id),
            run_id: Some(scope.run_id),
            provider_id: Some(provider_id.into()),
            provider_kind: Some(ApplicationExecutionProviderKind::ExternalAppBackend),
            trace_id: Some(command.trace.trace_id),
            reason,
            retryable,
        }),
    }
}

fn start_error(
    provider_id: &str,
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
        provider_id = %provider_id,
        status = ?status,
        reason = %reason,
        "external application backend start failed"
    );
    StartApplicationExecutionResult {
        status,
        session_id: session_id.clone(),
        run_id: run_id.clone(),
        provider_id: Some(provider_id.into()),
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
            provider_id: Some(provider_id.into()),
            provider_kind: Some(ApplicationExecutionProviderKind::ExternalAppBackend),
            trace_id: Some(command.trace.trace_id),
            reason,
            retryable,
        }),
    }
}

/// Convert a service error into a sanitized provider-failed reason string.
pub(crate) fn service_error_reason(error: ServiceError) -> String {
    error.to_string()
}
