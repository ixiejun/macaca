//! Provider-neutral Scheduler job management DTOs.
//!
//! This module keeps the operator-facing job management contract separate from
//! the durable job definition contract. Shells should render sanitized
//! summaries and submit typed commands, while Scheduler providers retain
//! ownership of lifecycle semantics, due-run materialization, and persistence.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    autonomy_service::{non_empty, validate_trace},
    AutonomyScope, MacacaError, MacacaResult, SchedulerJobDefinition, SchedulerJobId,
    SchedulerJobLifecycleState, SchedulerScheduleSpec, SchedulerTargetCommand, TraceContext,
};

/// Maximum number of jobs a shell may request in one list operation.
pub const SCHEDULER_JOB_LIST_LIMIT_MAX: usize = 200;

/// Sanitized Scheduler job view returned to SDK, Web, CLI, and frontend shells.
///
/// This is the Memento pattern applied at the service boundary. It carries
/// enough state for operators to understand and manage jobs, but avoids raw
/// provider payloads, prompts, manifests, package bytes, and sensitive
/// execution data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchedulerJobSummary {
    pub job_id: SchedulerJobId,
    pub scope: AutonomyScope,
    pub schedule: SchedulerScheduleSpec,
    pub target: SchedulerTargetCommand,
    pub lifecycle: SchedulerJobLifecycleState,
    pub metadata: BTreeMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_scheduled_at: Option<DateTime<Utc>>,
    pub trace_id: Option<String>,
    pub audit_id: Option<String>,
}

/// Query command for listing Scheduler jobs within a provider-neutral scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerListJobsCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub limit: usize,
}

impl SchedulerListJobsCommand {
    /// Build a traced bounded list command.
    pub fn new(
        trace: TraceContext,
        scope: AutonomyScope,
        limit: Option<usize>,
    ) -> MacacaResult<Self> {
        validate_trace(&trace, "scheduler list jobs command requires trace_id")?;
        let limit = limit.unwrap_or(100).clamp(1, SCHEDULER_JOB_LIST_LIMIT_MAX);
        Ok(Self {
            trace,
            scope,
            limit,
        })
    }
}

/// Query command for reading one Scheduler job summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerGetJobCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub job_id: SchedulerJobId,
}

impl SchedulerGetJobCommand {
    /// Build a traced job lookup command.
    pub fn new(
        trace: TraceContext,
        scope: AutonomyScope,
        job_id: SchedulerJobId,
    ) -> MacacaResult<Self> {
        validate_trace(&trace, "scheduler get job command requires trace_id")?;
        Ok(Self {
            trace,
            scope,
            job_id,
        })
    }
}

/// Command for replacing provider-neutral mutable fields on an existing job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchedulerUpdateJobCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub job_id: SchedulerJobId,
    pub definition: SchedulerJobDefinition,
    pub reason_code: String,
}

impl SchedulerUpdateJobCommand {
    /// Build a traced update command with a non-empty safe reason code.
    pub fn new(
        trace: TraceContext,
        scope: AutonomyScope,
        job_id: SchedulerJobId,
        mut definition: SchedulerJobDefinition,
        reason_code: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_trace(&trace, "scheduler update job command requires trace_id")?;
        definition.schedule.validate()?;
        definition.job_id = Some(job_id.clone());
        Ok(Self {
            trace,
            scope,
            job_id,
            definition,
            reason_code: non_empty(
                reason_code.into(),
                "scheduler update reason_code is required",
            )?,
        })
    }
}

/// Safe lifecycle operation supported by generic schedule management shells.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchedulerJobLifecycleCommand {
    Pause,
    Resume,
}

impl SchedulerJobLifecycleCommand {
    /// Parse lifecycle operations from HTTP/string inputs without allowing Web
    /// shell code to invent new Scheduler lifecycle semantics.
    pub fn parse(value: &str) -> MacacaResult<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pause" | "paused" => Ok(Self::Pause),
            "resume" | "resumed" => Ok(Self::Resume),
            other => Err(MacacaError::Config(format!(
                "unsupported scheduler lifecycle operation '{other}'"
            ))),
        }
    }
}

/// Command for lifecycle transitions that do not change schedule definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerLifecycleJobCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub job_id: SchedulerJobId,
    pub lifecycle: SchedulerJobLifecycleCommand,
    pub reason_code: String,
    pub metadata: BTreeMap<String, String>,
}

impl SchedulerLifecycleJobCommand {
    /// Build a traced lifecycle command with provider-neutral operation data.
    pub fn new(
        trace: TraceContext,
        scope: AutonomyScope,
        job_id: SchedulerJobId,
        lifecycle: SchedulerJobLifecycleCommand,
        reason_code: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_trace(&trace, "scheduler lifecycle job command requires trace_id")?;
        Ok(Self {
            trace,
            scope,
            job_id,
            lifecycle,
            reason_code: non_empty(
                reason_code.into(),
                "scheduler lifecycle reason_code is required",
            )?,
            metadata: BTreeMap::new(),
        })
    }
}

/// Command for deleting one Scheduler job from an application scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerDeleteJobCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub job_id: SchedulerJobId,
    pub reason_code: String,
}

impl SchedulerDeleteJobCommand {
    /// Build a traced delete command with a safe reason code.
    pub fn new(
        trace: TraceContext,
        scope: AutonomyScope,
        job_id: SchedulerJobId,
        reason_code: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_trace(&trace, "scheduler delete job command requires trace_id")?;
        Ok(Self {
            trace,
            scope,
            job_id,
            reason_code: non_empty(
                reason_code.into(),
                "scheduler delete reason_code is required",
            )?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace() -> TraceContext {
        TraceContext::new("trace-scheduler-job-management-test")
    }

    #[test]
    fn list_jobs_command_clamps_excessive_limit() {
        let command = SchedulerListJobsCommand::new(
            trace(),
            AutonomyScope::global(),
            Some(SCHEDULER_JOB_LIST_LIMIT_MAX + 100),
        )
        .unwrap();
        assert_eq!(command.limit, SCHEDULER_JOB_LIST_LIMIT_MAX);
    }

    #[test]
    fn lifecycle_parser_rejects_unknown_operation() {
        let error = SchedulerJobLifecycleCommand::parse("restart").unwrap_err();
        assert!(error
            .to_string()
            .contains("unsupported scheduler lifecycle operation"));
    }

    #[test]
    fn delete_command_rejects_blank_reason() {
        let job_id = SchedulerJobId::new("job.validation").unwrap();
        let error = SchedulerDeleteJobCommand::new(trace(), AutonomyScope::global(), job_id, " ")
            .unwrap_err();
        assert!(error.to_string().contains("reason_code"));
    }
}
