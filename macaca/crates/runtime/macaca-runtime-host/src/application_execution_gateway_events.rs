//! Gateway callback handlers for application execution service commands.
//!
//! The service provider owns dispatch, while this module owns the repeated
//! Adapter work for external callback commands: decode a typed command, build a
//! sanitized protocol event, log bounded ingress metadata, and append through
//! the EventLog adapter.  The functions never branch on application names,
//! provider names, workflow names, or business domains.

use macaca_proto::{
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionSnapshot,
    ReportExecutionCompletionCommand, ReportExecutionFailureCommand,
    ReportExecutionHeartbeatCommand, ReportExecutionSnapshotCommand,
    RequestExecutionApprovalCommand, ServiceError, ServiceResult,
    APPLICATION_EXECUTION_GATEWAY_APPROVAL_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_COMPLETION_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_FAILURE_COMMAND, APPLICATION_EXECUTION_GATEWAY_HEARTBEAT_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_SNAPSHOT_COMMAND,
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

/// Decode, validate, persist, and return one bounded provider snapshot callback.
///
/// Snapshots are Memento-style diagnostics from an external backend or remote
/// agent.  The gateway stores a durable `ProviderSnapshot` event before the
/// snapshot is returned so replay and current-state projection remain the
/// authoritative recovery path.  The snapshot payload is intentionally reduced
/// to bounded metadata and cursors; raw provider memory, prompts, package bytes,
/// and implementation-specific state must stay outside this DTO.
pub(crate) async fn append_gateway_snapshot(
    store: &ApplicationExecutionEventStore,
    payload: serde_json::Value,
) -> ServiceResult<ApplicationExecutionSnapshot> {
    let typed: ReportExecutionSnapshotCommand =
        serde_json::from_value(payload).map_err(adapter_error)?;
    let snapshot = typed.snapshot;
    let scope = snapshot.scope.clone();
    let provider_id = snapshot
        .provider_id
        .clone()
        .unwrap_or_else(|| "gateway".into());
    let event_trace = typed.trace;
    store.validate_gateway_ingress(
        typed.lease_id.as_deref(),
        &typed.callback_identity_ref,
        Some(&scope),
        snapshot.provider_id.as_deref(),
        ApplicationExecutionEventType::ProviderSnapshot,
    )?;
    let event = build_event(
        &scope,
        ApplicationExecutionEventType::ProviderSnapshot,
        &provider_id,
        snapshot.provider_kind,
        event_trace.clone(),
        ApplicationExecutionPayload {
            summary: "provider snapshot reported".into(),
            data: Some(serde_json::json!({
                "lifecycle_state": format!("{:?}", snapshot.lifecycle_state),
                "latest_event_cursor": snapshot.latest_event_cursor.clone(),
                "latest_checkpoint_ref": snapshot.latest_checkpoint_ref.clone(),
                "metadata": snapshot.metadata.clone(),
            })),
            payload_ref: None,
            truncated: false,
        },
        format!("snapshot:{}", event_trace.trace_id),
    );
    log_gateway_ingress(
        &scope.application_id.to_string(),
        &scope.session_id,
        &scope.run_id,
        APPLICATION_EXECUTION_GATEWAY_SNAPSHOT_COMMAND,
        &event_trace.trace_id,
        "snapshot_reported",
    );
    store.append_idempotent(event).await?;
    Ok(snapshot)
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
