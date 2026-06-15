//! Heartbeat run lifecycle transitions for the local in-process provider.
//!
//! This module owns the **State** pattern for run mementos: requested → running →
//! terminal (succeeded/failed/skipped). Each transition records audit evidence,
//! clears pending scope coalescing keys on terminal states, and emits
//! provider-neutral tracing for replay and operations dashboards.
//!
//! Heartbeat never executes agents or application workflows here; these methods
//! only mutate memento state that external runtime observers report back through
//! typed completion commands.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use macaca_proto::{
    AutonomyAuditCorrelation, AutonomyServiceErrorKind, AutonomyStructuredError,
    HeartbeatCommandResult, HeartbeatCompleteRunCommand, HeartbeatGateDecision, HeartbeatRunId,
    HeartbeatRunState, HeartbeatWakeDisposition, MacacaResult, TraceContext, HEARTBEAT_SERVICE_ID,
};
use tracing::info;

use super::completion_sanitizer::{normalize_label, sanitized_completion_metadata};
use super::memento::StoredHeartbeatRun;
use super::{InProcessHeartbeatProvider, LOCAL_PROVIDER_ID};

impl InProcessHeartbeatProvider {
    /// Builds a successful command result envelope for wake/lifecycle responses.
    pub(super) fn result(
        &self,
        trace: TraceContext,
        run_id: Option<HeartbeatRunId>,
        state: Option<HeartbeatRunState>,
        disposition: HeartbeatWakeDisposition,
        gates: Vec<HeartbeatGateDecision>,
        accepted: bool,
        audit_id: Option<String>,
    ) -> HeartbeatCommandResult {
        HeartbeatCommandResult {
            run_id,
            state,
            disposition,
            gates,
            accepted,
            error: None,
            trace,
            audit_id,
            metadata: BTreeMap::new(),
        }
    }

    /// Builds a structured unavailable/error result without mutating memento state.
    pub(super) fn error_result(
        &self,
        trace: TraceContext,
        kind: AutonomyServiceErrorKind,
        reason_code: &'static str,
        safe_message: impl Into<String>,
    ) -> HeartbeatCommandResult {
        let correlation = AutonomyAuditCorrelation::from_trace(trace.clone()).ok();
        HeartbeatCommandResult {
            run_id: None,
            state: None,
            disposition: HeartbeatWakeDisposition::Skipped,
            gates: Vec::new(),
            accepted: false,
            error: correlation.map(|correlation| AutonomyStructuredError {
                kind,
                reason_code: reason_code.into(),
                safe_message: safe_message.into(),
                correlation,
                metadata: BTreeMap::new(),
            }),
            trace,
            audit_id: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Mark an accepted wake as entering the future dispatch boundary.
    ///
    /// The method records lifecycle evidence only. It does not call task,
    /// application, plugin, notification, or external provider code.
    pub fn mark_run_running(
        &self,
        trace: TraceContext,
        run_id: HeartbeatRunId,
    ) -> MacacaResult<HeartbeatCommandResult> {
        self.transition_run(
            trace,
            run_id,
            HeartbeatRunState::Running,
            "running",
            |run, now| {
                run.summary.started_at = Some(now);
                run.summary.safe_status = "heartbeat dispatch boundary entered".into();
            },
        )
    }

    /// Mark a heartbeat run as completed successfully.
    pub fn mark_run_succeeded(
        &self,
        trace: TraceContext,
        run_id: HeartbeatRunId,
    ) -> MacacaResult<HeartbeatCommandResult> {
        self.transition_run(
            trace,
            run_id,
            HeartbeatRunState::Succeeded,
            "succeeded",
            |run, now| {
                run.summary.finished_at = Some(now);
                run.summary.safe_status = "heartbeat run completed".into();
            },
        )
    }

    /// Apply a terminal completion reported by an external runtime observer.
    ///
    /// Heartbeat does not execute agents, tasks, plugins, or application
    /// workflows. This method records only the sanitized terminal state reported
    /// by Runtime Host after such a boundary returns.
    pub fn complete_run(
        &self,
        command: HeartbeatCompleteRunCommand,
    ) -> MacacaResult<HeartbeatCommandResult> {
        let metadata = sanitized_completion_metadata(command.metadata);
        let reason_code = normalize_label(
            command.reason_code,
            "heartbeat completion reason is required",
        )?;
        let state = command.state;
        self.transition_run(
            command.trace,
            command.run_id,
            state.clone(),
            "complete",
            |run, now| {
                run.summary.finished_at = Some(now);
                run.summary.safe_status = match state {
                    HeartbeatRunState::Succeeded => "heartbeat delegated dispatch completed".into(),
                    HeartbeatRunState::Failed => "heartbeat delegated dispatch failed".into(),
                    HeartbeatRunState::Skipped => "heartbeat delegated dispatch skipped".into(),
                    _ => "heartbeat delegated dispatch completion recorded".into(),
                };
                run.summary
                    .metadata
                    .insert("dispatch.reason_code".into(), reason_code);
                run.summary.metadata.extend(metadata);
            },
        )
    }

    /// Mark a heartbeat run as failed with a sanitized reason code.
    pub fn mark_run_failed(
        &self,
        trace: TraceContext,
        run_id: HeartbeatRunId,
        reason_code: impl Into<String>,
    ) -> MacacaResult<HeartbeatCommandResult> {
        let reason_code =
            normalize_label(reason_code.into(), "heartbeat failure reason is required")?;
        self.transition_run(
            trace,
            run_id,
            HeartbeatRunState::Failed,
            "failed",
            |run, now| {
                run.summary.finished_at = Some(now);
                run.summary.safe_status = "heartbeat run failed".into();
                run.summary
                    .metadata
                    .insert("failure_reason_code".into(), reason_code.clone());
            },
        )
    }

    /// Mark a heartbeat run as intentionally skipped.
    pub fn mark_run_skipped(
        &self,
        trace: TraceContext,
        run_id: HeartbeatRunId,
        reason_code: impl Into<String>,
    ) -> MacacaResult<HeartbeatCommandResult> {
        let reason_code = normalize_label(reason_code.into(), "heartbeat skip reason is required")?;
        self.transition_run(
            trace,
            run_id,
            HeartbeatRunState::Skipped,
            "skipped",
            |run, now| {
                run.summary.finished_at = Some(now);
                run.summary.disposition = HeartbeatWakeDisposition::Skipped;
                run.summary.safe_status = "heartbeat run skipped".into();
                run.summary
                    .metadata
                    .insert("skip_reason_code".into(), reason_code.clone());
            },
        )
    }

    fn transition_run<F>(
        &self,
        trace: TraceContext,
        run_id: HeartbeatRunId,
        next_state: HeartbeatRunState,
        action: &'static str,
        mutate: F,
    ) -> MacacaResult<HeartbeatCommandResult>
    where
        F: FnOnce(&mut StoredHeartbeatRun, DateTime<Utc>),
    {
        Ok(self.store.write(|state| {
            let audit_id = state.record_audit(match next_state {
                HeartbeatRunState::Running => "run.running",
                HeartbeatRunState::Succeeded => "run.succeeded",
                HeartbeatRunState::Failed => "run.failed",
                HeartbeatRunState::Skipped => "run.skipped",
                HeartbeatRunState::Requested => "run.requested",
                HeartbeatRunState::Coalesced => "run.coalesced",
                HeartbeatRunState::Gated => "run.gated",
            });
            let transition = {
                let Some(run) = state.runs.get_mut(&run_id) else {
                    return self.error_result(
                        trace,
                        AutonomyServiceErrorKind::InvalidRequest,
                        "run_not_found",
                        "heartbeat run was not found",
                    );
                };
                let now = Utc::now();
                run.summary.state = next_state.clone();
                run.summary.audit_id = Some(audit_id.clone());
                mutate(run, now);
                (
                    run.summary.wake_scope_key.clone(),
                    run.summary.disposition.clone(),
                    run.summary.gates.clone(),
                )
            };
            if matches!(
                next_state,
                HeartbeatRunState::Succeeded
                    | HeartbeatRunState::Failed
                    | HeartbeatRunState::Skipped
            ) {
                state.pending_by_scope.remove(&transition.0);
            }
            info!(
                service_id = HEARTBEAT_SERVICE_ID,
                provider_id = LOCAL_PROVIDER_ID,
                run_id = run_id.as_str(),
                action,
                audit_id = audit_id.as_str(),
                trace_id = trace.trace_id.as_str(),
                "local heartbeat run lifecycle transition completed"
            );
            self.result(
                trace,
                Some(run_id),
                Some(next_state),
                transition.1,
                transition.2,
                false,
                Some(audit_id),
            )
        }))
    }
}
