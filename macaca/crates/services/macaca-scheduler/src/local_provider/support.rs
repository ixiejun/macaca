//! Shared constants and command-result builders for the in-process Scheduler provider.
//!
//! This module centralizes provider-neutral identifiers and the accepted/rejected
//! `SchedulerCommandResult` envelope builders.  Keeping them here avoids duplicating
//! correlation and audit metadata rules across service, run-control, and
//! materialization modules.

use std::collections::BTreeMap;

use macaca_proto::{
    AutonomyAuditCorrelation, AutonomyServiceErrorKind, AutonomyStructuredError,
    SchedulerCommandResult, SchedulerJobId, SchedulerJobLifecycleState, SchedulerRunId,
    SchedulerRunState, TraceContext,
};

/// Local provider identifier embedded in sanitized snapshots and audit logs.
pub(super) const LOCAL_PROVIDER_ID: &str = "local.in_memory";

/// Maximum number of due runs materialized in one refresh pass.
///
/// The bound prevents a restarted provider from creating an unbounded burst of
/// run mementos if a very old interval job is resumed.  Job-level
/// `CatchUp.max_runs` may lower this ceiling further.
pub(super) const MATERIALIZATION_LIMIT: usize = 64;

/// Maximum number of sanitized audit identifiers retained in snapshots.
///
/// Local memory must remain bounded because scheduler snapshots can be read by
/// shells and diagnostics.  The audit id itself is safe metadata; raw command
/// payloads and target details remain outside this memento.
pub(super) const AUDIT_RETENTION_LIMIT: usize = 128;

impl super::InProcessSchedulerProvider {
    /// Build an accepted command result with optional job/run lifecycle fields.
    ///
    /// Callers may attach `audit_id` after the memento write completes.
    pub(super) fn result(
        &self,
        trace: TraceContext,
        job_id: Option<SchedulerJobId>,
        run_id: Option<SchedulerRunId>,
        lifecycle: Option<SchedulerJobLifecycleState>,
        run_state: Option<SchedulerRunState>,
    ) -> SchedulerCommandResult {
        SchedulerCommandResult {
            job_id,
            run_id,
            lifecycle,
            run_state,
            accepted: true,
            error: None,
            trace,
            audit_id: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Build a rejected command result with a structured autonomy error envelope.
    ///
    /// When trace correlation cannot be derived, the error field is omitted so
    /// shells still receive a deterministic rejection without leaking internals.
    pub(super) fn error_result(
        &self,
        trace: TraceContext,
        kind: AutonomyServiceErrorKind,
        reason_code: &'static str,
        safe_message: impl Into<String>,
    ) -> SchedulerCommandResult {
        let correlation = AutonomyAuditCorrelation::from_trace(trace.clone()).ok();
        SchedulerCommandResult {
            job_id: None,
            run_id: None,
            lifecycle: None,
            run_state: None,
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
}
