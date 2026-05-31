//! Gateway callback handlers for application execution service commands.
//!
//! The service provider owns dispatch, while this module owns the repeated
//! Adapter work for external callback commands: decode a typed command, build a
//! sanitized protocol event, log bounded ingress metadata, and append through
//! the EventLog adapter.  The functions never branch on application names,
//! provider names, workflow names, or business domains.

use macaca_proto::{
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ReportExecutionCompletionCommand,
    ReportExecutionFailureCommand, ReportExecutionHeartbeatCommand,
    RequestExecutionApprovalCommand, ServiceError, ServiceResult,
    APPLICATION_EXECUTION_GATEWAY_APPROVAL_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_COMPLETION_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_FAILURE_COMMAND, APPLICATION_EXECUTION_GATEWAY_HEARTBEAT_COMMAND,
};

use crate::application_execution_event_builder::build_event;
use crate::application_execution_event_store::ApplicationExecutionEventStore;
use crate::application_execution_service_logs::log_gateway_ingress;

/// Decode and append one provider heartbeat callback.
pub(crate) async fn append_gateway_heartbeat(
    store: &ApplicationExecutionEventStore,
    payload: serde_json::Value,
) -> ServiceResult<ApplicationExecutionEventEnvelope> {
    let typed: ReportExecutionHeartbeatCommand =
        serde_json::from_value(payload).map_err(adapter_error)?;
    let scope = typed.scope.clone();
    let provider_id = typed.provider_id.clone();
    let provider_kind = typed.provider_kind;
    let event_trace = typed.trace.clone();
    store.validate_gateway_ingress(
        typed.lease_id.as_deref(),
        &typed.callback_identity_ref,
        Some(&scope),
        Some(&provider_id),
        ApplicationExecutionEventType::ProviderHeartbeat,
    )?;
    let event = build_event(
        &scope,
        ApplicationExecutionEventType::ProviderHeartbeat,
        &provider_id,
        provider_kind,
        event_trace.clone(),
        ApplicationExecutionPayload::summary("provider heartbeat reported"),
        format!("heartbeat:{}", typed.reported_at.timestamp_millis()),
    );
    log_gateway_ingress(
        &scope.application_id.to_string(),
        &scope.session_id,
        &scope.run_id,
        APPLICATION_EXECUTION_GATEWAY_HEARTBEAT_COMMAND,
        &event_trace.trace_id,
        "heartbeat_reported",
    );
    store.append_idempotent(event).await
}

/// Decode and append one provider approval-request callback.
pub(crate) async fn append_gateway_approval_request(
    store: &ApplicationExecutionEventStore,
    payload: serde_json::Value,
) -> ServiceResult<ApplicationExecutionEventEnvelope> {
    let typed: RequestExecutionApprovalCommand =
        serde_json::from_value(payload).map_err(adapter_error)?;
    let scope = typed.scope.clone();
    let event_trace = typed.trace.clone();
    store.validate_gateway_ingress(
        typed.lease_id.as_deref(),
        &typed.callback_identity_ref,
        Some(&scope),
        None,
        ApplicationExecutionEventType::ApprovalRequested,
    )?;
    let event = build_event(
        &scope,
        ApplicationExecutionEventType::ApprovalRequested,
        "gateway",
        ApplicationExecutionProviderKind::ExternalAppBackend,
        event_trace.clone(),
        ApplicationExecutionPayload {
            summary: typed.prompt.summary,
            data: Some(serde_json::json!({"approval_ref": typed.approval_ref})),
            payload_ref: typed.prompt.payload_ref,
            truncated: typed.prompt.truncated,
        },
        typed.idempotency_key,
    );
    log_gateway_ingress(
        &scope.application_id.to_string(),
        &scope.session_id,
        &scope.run_id,
        APPLICATION_EXECUTION_GATEWAY_APPROVAL_COMMAND,
        &event_trace.trace_id,
        "approval_requested",
    );
    store.append_idempotent(event).await
}

/// Decode and append one provider completion callback.
pub(crate) async fn append_gateway_completion(
    store: &ApplicationExecutionEventStore,
    payload: serde_json::Value,
) -> ServiceResult<ApplicationExecutionEventEnvelope> {
    let typed: ReportExecutionCompletionCommand =
        serde_json::from_value(payload).map_err(adapter_error)?;
    let scope = typed.scope.clone();
    let event_trace = typed.trace.clone();
    store.validate_gateway_ingress(
        typed.lease_id.as_deref(),
        &typed.callback_identity_ref,
        Some(&scope),
        None,
        ApplicationExecutionEventType::ExecutionCompleted,
    )?;
    let event = build_event(
        &scope,
        ApplicationExecutionEventType::ExecutionCompleted,
        "gateway",
        ApplicationExecutionProviderKind::ExternalAppBackend,
        event_trace.clone(),
        typed.result,
        typed.idempotency_key,
    );
    log_gateway_ingress(
        &scope.application_id.to_string(),
        &scope.session_id,
        &scope.run_id,
        APPLICATION_EXECUTION_GATEWAY_COMPLETION_COMMAND,
        &event_trace.trace_id,
        "completion_reported",
    );
    store.append_idempotent(event).await
}

/// Decode and append one provider failure callback.
pub(crate) async fn append_gateway_failure(
    store: &ApplicationExecutionEventStore,
    payload: serde_json::Value,
) -> ServiceResult<ApplicationExecutionEventEnvelope> {
    let typed: ReportExecutionFailureCommand =
        serde_json::from_value(payload).map_err(adapter_error)?;
    let scope = typed.scope.clone();
    let event_trace = typed.trace.clone();
    store.validate_gateway_ingress(
        typed.lease_id.as_deref(),
        &typed.callback_identity_ref,
        Some(&scope),
        None,
        ApplicationExecutionEventType::ExecutionFailed,
    )?;
    let event = build_event(
        &scope,
        ApplicationExecutionEventType::ExecutionFailed,
        "gateway",
        ApplicationExecutionProviderKind::ExternalAppBackend,
        event_trace.clone(),
        ApplicationExecutionPayload {
            summary: typed.error.reason.clone(),
            data: Some(serde_json::to_value(typed.error).map_err(adapter_error)?),
            payload_ref: None,
            truncated: false,
        },
        typed.idempotency_key,
    );
    log_gateway_ingress(
        &scope.application_id.to_string(),
        &scope.session_id,
        &scope.run_id,
        APPLICATION_EXECUTION_GATEWAY_FAILURE_COMMAND,
        &event_trace.trace_id,
        "failure_reported",
    );
    store.append_idempotent(event).await
}

fn adapter_error(error: serde_json::Error) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
