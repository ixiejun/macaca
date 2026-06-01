//! Snapshot helpers for `service.application_execution`.
//!
//! The service provider stays a thin command adapter. This module owns the
//! Memento-to-EventLog bridge for provider diagnostics: when a provider snapshot
//! declares that a structured failure event is required, the helper appends one
//! generic `ExecutionFailed` event through the same durable EventLog adapter
//! used by gateway ingress and control paths.

use macaca_proto::{
    ApplicationExecutionEventType, ApplicationExecutionPayload, ApplicationExecutionSnapshot,
    ServiceError, ServiceResult, TraceContext, APPLICATION_EXECUTION_SNAPSHOT_COMMAND,
};
use tracing::info;

use crate::application_execution_event_builder::build_event;
use crate::application_execution_event_store::ApplicationExecutionEventStore;
use crate::application_execution_provider_registry::ApplicationExecutionProviderRegistry;
use crate::application_execution_service_logs::log_snapshot;

/// Collect provider snapshots and append required failure events.
///
/// Providers do not receive a direct EventLog dependency because they are
/// replaceable Strategy adapters. Instead, they expose bounded snapshot metadata
/// such as `failure_event_required=true`; this service helper interprets that
/// provider-neutral signal at the service boundary where durable append is
/// already allowed and trace context is available.
pub(crate) async fn collect_provider_snapshots(
    store: &ApplicationExecutionEventStore,
    registry: &ApplicationExecutionProviderRegistry,
    trace: TraceContext,
) -> ServiceResult<Vec<ApplicationExecutionSnapshot>> {
    let snapshots = registry.snapshots().await?;
    log_snapshot(
        APPLICATION_EXECUTION_SNAPSHOT_COMMAND,
        &trace.trace_id,
        "collected",
        None,
    );
    for snapshot in &snapshots {
        if requires_failure_event(snapshot) {
            append_snapshot_failure(store, snapshot, trace.clone()).await?;
        }
    }
    Ok(snapshots)
}

fn requires_failure_event(snapshot: &ApplicationExecutionSnapshot) -> bool {
    snapshot
        .metadata
        .get("failure_event_required")
        .map(|value| value == "true")
        .unwrap_or(false)
}

async fn append_snapshot_failure(
    store: &ApplicationExecutionEventStore,
    snapshot: &ApplicationExecutionSnapshot,
    trace: TraceContext,
) -> Result<(), ServiceError> {
    let provider_id = snapshot
        .provider_id
        .clone()
        .unwrap_or_else(|| "provider.unavailable".into());
    let event = build_event(
        &snapshot.scope,
        ApplicationExecutionEventType::ExecutionFailed,
        &provider_id,
        snapshot.provider_kind,
        trace.clone(),
        ApplicationExecutionPayload {
            summary: "application execution provider heartbeat deadline expired".into(),
            data: Some(serde_json::json!({
                "provider_id": provider_id,
                "heartbeat_status": snapshot.metadata.get("heartbeat_status"),
                "stale_lease_count": snapshot.metadata.get("stale_lease_count"),
            })),
            payload_ref: None,
            truncated: false,
        },
        format!(
            "provider-stale:{}:{}:{}",
            provider_id, snapshot.scope.session_id, snapshot.scope.run_id
        ),
    );
    let persisted = store.append_idempotent(event).await?;
    info!(
        application_id = %snapshot.scope.application_id,
        session_id = %snapshot.scope.session_id,
        run_id = %snapshot.scope.run_id,
        provider_id = %provider_id,
        trace_id = %trace.trace_id,
        event_cursor = ?persisted.seq,
        "application execution provider stale failure event appended"
    );
    Ok(())
}
