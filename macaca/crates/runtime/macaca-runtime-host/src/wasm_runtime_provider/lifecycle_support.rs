//! Lifecycle support helpers for the default in-process WASM provider.
//!
//! This module keeps lifecycle policy, checkpoint memento construction,
//! ABI decision checks, and audit logging out of `default_provider.rs`.  The
//! split is deliberate: the provider remains a small composition root, while
//! this module owns the State, Specification, Memento, and Observer mechanics
//! for long-running WASM sessions.

use std::collections::BTreeMap;

use chrono::Utc;
use macaca_proto::{
    sanitize_wasm_lifecycle_metadata, ApplicationAbiError, WasmCheckpointMemento,
    WasmLifecycleAuditEvent, WasmLifecycleCommand, WasmLifecycleOperation,
    WasmLifecycleOperationStatus, WasmLifecycleReasonCode, WasmLifecycleState,
    WasmLifecycleStateMachine, WasmLifecycleTransitionResult, WasmRestoreReport,
    WasmRestoreRequest, WasmRollbackReport, WasmRollbackRequest, WasmUpgradeReport,
    WasmUpgradeRequest,
};
use tracing::{info, warn};

use super::default_provider::DefaultInProcessWasmExecutionSession;

/// Execute one lifecycle transition through the default provider.
///
/// The method validates trace and state before touching the guest instance.  It
/// then maps supported ABI lifecycle operations to guest exports.  Pause,
/// resume, and drain return structured `unsupported` results because the
/// current in-process adapter cannot honestly suspend guest memory, resume a
/// suspended instance, or drain in-flight host calls.
pub(super) fn transition_lifecycle(
    session: &DefaultInProcessWasmExecutionSession,
    command: WasmLifecycleCommand,
) -> Result<WasmLifecycleTransitionResult, ApplicationAbiError> {
    let trace = command
        .require_trace()
        .map_err(|_| ApplicationAbiError::MissingTraceContext)?;
    info!(
        session_id = %session.session_id,
        trace_id = %trace.trace_id,
        application_id = %session.request.application_id,
        ability_id = %session.request.ability_id,
        operation = %command.operation.as_code(),
        "WASM lifecycle transition requested"
    );

    let mut lifecycle = session
        .lifecycle
        .lock()
        .expect("wasm lifecycle mutex poisoned");
    let from = lifecycle.state();
    let Some(_) = macaca_proto::next_state_for_operation(&from, &command.operation) else {
        let result = rejected_result(
            session,
            command,
            from,
            WasmLifecycleReasonCode::InvalidTransition,
            Some(trace),
        );
        record_transition(session, &result);
        return Ok(result);
    };

    if matches!(
        command.operation,
        WasmLifecycleOperation::Pause
            | WasmLifecycleOperation::Resume
            | WasmLifecycleOperation::Drain
    ) {
        let result = rejected_result(
            session,
            command,
            from,
            WasmLifecycleReasonCode::Unsupported,
            Some(trace),
        );
        record_transition(session, &result);
        return Ok(result);
    }

    if let Some(export_name) = command.operation.abi_export() {
        if !session.module.has_export(export_name) {
            let result = rejected_result(
                session,
                command,
                from,
                WasmLifecycleReasonCode::RuntimeFailed,
                Some(trace),
            );
            record_transition(session, &result);
            return Ok(result);
        }
        if let Err(error) = session.instance.invoke_export(export_name) {
            let mut metadata = command.metadata.clone();
            metadata.insert("runtime_error".into(), format!("{error:?}"));
            let result = rejected_result_with_metadata(
                session,
                command.operation.clone(),
                from,
                WasmLifecycleReasonCode::RuntimeFailed,
                Some(trace),
                metadata,
            );
            record_transition(session, &result);
            return Ok(result);
        }
    }

    let mut result = lifecycle.transition(&command);
    attach_session_metadata(session, &mut result.metadata);
    drop(lifecycle);
    record_transition(session, &result);
    Ok(result)
}

/// Create a metadata-only checkpoint memento for the active session.
///
/// The checkpoint deliberately stores artifact references, digest prefixes,
/// lifecycle state, ABI version, and sanitized metadata only.  Raw guest memory
/// is not captured by this provider because the current engine has no
/// encrypted, bounded, policy-approved artifact path for memory snapshots.
pub(super) fn checkpoint(
    session: &DefaultInProcessWasmExecutionSession,
    command: WasmLifecycleCommand,
) -> Result<WasmCheckpointMemento, ApplicationAbiError> {
    let trace = command
        .require_trace()
        .map_err(|_| ApplicationAbiError::MissingTraceContext)?;
    let state = session
        .lifecycle
        .lock()
        .expect("wasm lifecycle mutex poisoned")
        .state();
    let mut metadata = command.metadata;
    attach_static_metadata(session, &mut metadata);
    metadata.insert(
        "reason_code".into(),
        WasmLifecycleReasonCode::Completed.as_code().into(),
    );
    let checkpoint = WasmCheckpointMemento {
        checkpoint_id: format!("{}:{}", session.session_id, Utc::now().timestamp_millis()),
        session_id: session.session_id.clone(),
        application_id: session.request.application_id.clone(),
        ability_id: session.request.ability_id.clone(),
        lifecycle_state: state.clone(),
        artifact: session.request.artifact.clone(),
        artifact_digest_prefix: artifact_digest_prefix(session),
        abi_version: session.request.profile.abi_version.clone(),
        created_at: Utc::now(),
        trace: Some(trace.clone()),
        metadata: sanitize_wasm_lifecycle_metadata(metadata),
    };
    let result = WasmLifecycleTransitionResult::completed(
        WasmLifecycleOperation::Checkpoint,
        state.clone(),
        state,
        Some(trace),
        checkpoint.metadata.clone(),
    );
    record_transition(session, &result);
    Ok(checkpoint)
}

/// Restore lifecycle metadata from an ABI-matching checkpoint memento.
pub(super) fn restore(
    session: &DefaultInProcessWasmExecutionSession,
    request: WasmRestoreRequest,
) -> Result<WasmRestoreReport, ApplicationAbiError> {
    let trace = request
        .trace
        .ok_or(ApplicationAbiError::MissingTraceContext)?;
    let abi_matches = request.checkpoint.abi_version == session.request.profile.abi_version
        && request.checkpoint.artifact_digest_prefix == artifact_digest_prefix(session);
    let status = if abi_matches {
        *session
            .lifecycle
            .lock()
            .expect("wasm lifecycle mutex poisoned") =
            WasmLifecycleStateMachine::from_state(request.checkpoint.lifecycle_state.clone());
        WasmLifecycleOperationStatus::Completed
    } else {
        WasmLifecycleOperationStatus::Rejected
    };
    let reason = if abi_matches {
        WasmLifecycleReasonCode::Completed
    } else {
        WasmLifecycleReasonCode::AbiMismatch
    };
    let mut metadata = request.metadata;
    attach_static_metadata(session, &mut metadata);
    metadata.insert("abi_matches".into(), abi_matches.to_string());
    let report = WasmRestoreReport {
        status,
        reason_code: reason.as_code().into(),
        checkpoint_id: request.checkpoint.checkpoint_id,
        lifecycle_state: request.checkpoint.lifecycle_state,
        trace: Some(trace.clone()),
        metadata: sanitize_wasm_lifecycle_metadata(metadata),
    };
    record_report_event(
        session,
        WasmLifecycleOperation::Restore,
        report.status.clone(),
        report.lifecycle_state.clone(),
        report.reason_code.clone(),
        Some(trace),
        report.metadata.clone(),
    );
    Ok(report)
}

/// Evaluate a metadata-only upgrade request against ABI match metadata.
pub(super) fn upgrade(
    session: &DefaultInProcessWasmExecutionSession,
    request: WasmUpgradeRequest,
) -> Result<WasmUpgradeReport, ApplicationAbiError> {
    let trace = request
        .trace
        .ok_or(ApplicationAbiError::MissingTraceContext)?;
    let abi_matches = request.target_abi_version == session.request.profile.abi_version;
    let status = if abi_matches {
        WasmLifecycleOperationStatus::Completed
    } else {
        WasmLifecycleOperationStatus::Rejected
    };
    let reason = if abi_matches {
        WasmLifecycleReasonCode::Completed
    } else {
        WasmLifecycleReasonCode::AbiMismatch
    };
    let mut metadata = request.metadata;
    attach_static_metadata(session, &mut metadata);
    metadata.insert("abi_matches".into(), abi_matches.to_string());
    let report = WasmUpgradeReport {
        status,
        reason_code: reason.as_code().into(),
        source_artifact: session.request.artifact.clone(),
        source_artifact_digest_prefix: artifact_digest_prefix(session),
        target_artifact: request.target_artifact,
        target_artifact_digest_prefix: request.target_artifact_digest.chars().take(12).collect(),
        abi_matches,
        trace: Some(trace.clone()),
        metadata: sanitize_wasm_lifecycle_metadata(metadata),
    };
    record_report_event(
        session,
        WasmLifecycleOperation::Upgrade,
        report.status.clone(),
        session
            .lifecycle
            .lock()
            .expect("wasm lifecycle mutex poisoned")
            .state(),
        report.reason_code.clone(),
        Some(trace),
        report.metadata.clone(),
    );
    Ok(report)
}

/// Roll back lifecycle metadata to an ABI-matching checkpoint memento.
pub(super) fn rollback(
    session: &DefaultInProcessWasmExecutionSession,
    request: WasmRollbackRequest,
) -> Result<WasmRollbackReport, ApplicationAbiError> {
    let restore_request = WasmRestoreRequest {
        checkpoint: request.checkpoint,
        trace: request.trace,
        metadata: request.metadata,
    };
    let restored = restore(session, restore_request)?;
    Ok(WasmRollbackReport {
        status: restored.status,
        reason_code: restored.reason_code,
        checkpoint_id: restored.checkpoint_id,
        restored_lifecycle_state: restored.lifecycle_state,
        trace: restored.trace,
        metadata: restored.metadata,
    })
}

fn rejected_result(
    session: &DefaultInProcessWasmExecutionSession,
    command: WasmLifecycleCommand,
    state: WasmLifecycleState,
    reason: WasmLifecycleReasonCode,
    trace: Option<macaca_proto::TraceContext>,
) -> WasmLifecycleTransitionResult {
    rejected_result_with_metadata(
        session,
        command.operation,
        state,
        reason,
        trace,
        command.metadata,
    )
}

fn rejected_result_with_metadata(
    session: &DefaultInProcessWasmExecutionSession,
    operation: WasmLifecycleOperation,
    state: WasmLifecycleState,
    reason: WasmLifecycleReasonCode,
    trace: Option<macaca_proto::TraceContext>,
    mut metadata: BTreeMap<String, String>,
) -> WasmLifecycleTransitionResult {
    attach_static_metadata(session, &mut metadata);
    WasmLifecycleTransitionResult::rejected(operation, state, reason, trace, metadata)
}

fn record_transition(
    session: &DefaultInProcessWasmExecutionSession,
    result: &WasmLifecycleTransitionResult,
) {
    let event = WasmLifecycleAuditEvent::from_transition(
        session.session_id.clone(),
        session.request.application_id.clone(),
        session.request.ability_id.clone(),
        result,
    );
    session
        .audit_events
        .lock()
        .expect("wasm lifecycle audit mutex poisoned")
        .push(event);
    match result.status {
        WasmLifecycleOperationStatus::Completed => info!(
            session_id = %session.session_id,
            operation = %result.operation.as_code(),
            from = ?result.from,
            to = ?result.to,
            reason_code = %result.reason_code,
            "WASM lifecycle transition completed"
        ),
        _ => warn!(
            session_id = %session.session_id,
            operation = %result.operation.as_code(),
            from = ?result.from,
            to = ?result.to,
            reason_code = %result.reason_code,
            "WASM lifecycle transition failed closed"
        ),
    }
}

fn record_report_event(
    session: &DefaultInProcessWasmExecutionSession,
    operation: WasmLifecycleOperation,
    status: WasmLifecycleOperationStatus,
    state: WasmLifecycleState,
    reason_code: String,
    trace: Option<macaca_proto::TraceContext>,
    metadata: BTreeMap<String, String>,
) {
    let result = WasmLifecycleTransitionResult {
        operation,
        status,
        from: state.clone(),
        to: state,
        reason_code,
        trace,
        metadata,
    };
    record_transition(session, &result);
}

fn attach_session_metadata(
    session: &DefaultInProcessWasmExecutionSession,
    metadata: &mut BTreeMap<String, String>,
) {
    attach_static_metadata(session, metadata);
    metadata.insert(
        "reason_code".into(),
        WasmLifecycleReasonCode::Completed.as_code().into(),
    );
}

fn attach_static_metadata(
    session: &DefaultInProcessWasmExecutionSession,
    metadata: &mut BTreeMap<String, String>,
) {
    metadata.insert(
        "runtime_kind".into(),
        session.request.profile.runtime_kind.to_string(),
    );
    metadata.insert("session_id".into(), session.session_id.clone());
    metadata.insert(
        "application_id".into(),
        session.request.application_id.clone(),
    );
    metadata.insert("ability_id".into(), session.request.ability_id.clone());
    metadata.insert(
        "artifact_digest_prefix".into(),
        artifact_digest_prefix(session),
    );
}

fn artifact_digest_prefix(session: &DefaultInProcessWasmExecutionSession) -> String {
    session.artifact_digest.chars().take(12).collect()
}
