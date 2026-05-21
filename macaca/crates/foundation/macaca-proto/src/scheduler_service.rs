//! Provider-neutral Scheduler service contracts.
//!
//! The Scheduler service owns durable job definitions, due-run materialization,
//! leases, retry policy, missed-run handling, and run-history snapshots.  This
//! module intentionally defines only typed DTOs and state machines.  Concrete
//! cron parsers, timer loops, storage engines, and dispatch workers must live in
//! replaceable service providers registered by runtime-host composition.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    autonomy_service::{non_empty, validate_trace},
    AgentExecutionIntent, ApplicationId, AutonomyPayloadRef, AutonomyScope,
    AutonomyStructuredError, CapabilityId, KernelServiceId, MacacaResult, ServiceCommand,
    ServiceCommandName, TaskId, TraceContext,
};

/// Stable service id used by runtime-host registration and SDK clients.
pub const SCHEDULER_SERVICE_ID: &str = "service.scheduler";

/// Command names accepted by Scheduler service providers.
pub const SCHEDULER_REGISTER_JOB_COMMAND: &str = "scheduler.job.register";
pub const SCHEDULER_UPDATE_JOB_COMMAND: &str = "scheduler.job.update";
pub const SCHEDULER_PAUSE_JOB_COMMAND: &str = "scheduler.job.pause";
pub const SCHEDULER_RESUME_JOB_COMMAND: &str = "scheduler.job.resume";
pub const SCHEDULER_DELETE_JOB_COMMAND: &str = "scheduler.job.delete";
pub const SCHEDULER_TRIGGER_JOB_COMMAND: &str = "scheduler.job.trigger";
pub const SCHEDULER_GET_JOB_COMMAND: &str = "scheduler.job.get";
pub const SCHEDULER_LIST_JOBS_COMMAND: &str = "scheduler.job.list";
pub const SCHEDULER_GET_RUN_COMMAND: &str = "scheduler.run.get";
pub const SCHEDULER_LIST_RUNS_COMMAND: &str = "scheduler.run.list";
pub const SCHEDULER_HEALTH_COMMAND: &str = "scheduler.health";
pub const SCHEDULER_SNAPSHOT_COMMAND: &str = "scheduler.snapshot";

/// Stable identity for a scheduler job definition.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchedulerJobId(String);

impl SchedulerJobId {
    /// Create a job id from provider-assigned or imported durable state.
    pub fn new(value: impl Into<String>) -> MacacaResult<Self> {
        Ok(Self(non_empty(value.into(), "scheduler job id is required")?))
    }

    /// Return the raw identifier string for persistence adapters.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable identity for one materialized scheduler run.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchedulerRunId(String);

impl SchedulerRunId {
    /// Create a run id from provider-assigned durable state.
    pub fn new(value: impl Into<String>) -> MacacaResult<Self> {
        Ok(Self(non_empty(value.into(), "scheduler run id is required")?))
    }

    /// Return the raw identifier string for persistence adapters.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Provider-neutral schedule expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchedulerScheduleSpec {
    /// Run once at a concrete UTC timestamp.
    At { run_at: DateTime<Utc> },
    /// Run repeatedly after a bounded interval in milliseconds.
    Every {
        interval_ms: u64,
        anchor: Option<DateTime<Utc>>,
    },
    /// Run according to a cron expression interpreted by the provider.
    CronExpression {
        expression: String,
        timezone: String,
        start_after: Option<DateTime<Utc>>,
        end_before: Option<DateTime<Utc>>,
    },
}

impl SchedulerScheduleSpec {
    /// Validate the schedule without binding the OS contract to a cron parser.
    pub fn validate(&self) -> MacacaResult<()> {
        match self {
            Self::At { .. } => Ok(()),
            Self::Every { interval_ms, .. } if *interval_ms > 0 => Ok(()),
            Self::Every { .. } => Err(crate::MacacaError::Config(
                "scheduler interval must be positive".into(),
            )),
            Self::CronExpression {
                expression,
                timezone,
                ..
            } => {
                non_empty(expression.clone(), "scheduler cron expression is required")?;
                non_empty(timezone.clone(), "scheduler cron timezone is required")?;
                Ok(())
            }
        }
    }
}

/// Strategy used when ticks were missed while the provider was unavailable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchedulerMissedRunPolicy {
    Skip,
    FireOnce,
    CatchUp { max_runs: u32 },
}

/// Deterministic stagger strategy for fleet-safe due-run distribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerStaggerPolicy {
    pub key: String,
    pub max_delay_ms: u64,
}

/// Lease and concurrency policy for one scheduled job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerConcurrencyPolicy {
    pub max_active_runs: u32,
    pub lease_timeout_ms: u64,
    pub idempotency_key: Option<String>,
}

impl Default for SchedulerConcurrencyPolicy {
    fn default() -> Self {
        Self {
            max_active_runs: 1,
            lease_timeout_ms: 300_000,
            idempotency_key: None,
        }
    }
}

/// Bounded retry/backoff strategy applied after target command failures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerRetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub multiplier_basis_points: u32,
}

impl Default for SchedulerRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            initial_backoff_ms: 1_000,
            max_backoff_ms: 60_000,
            multiplier_basis_points: 10_000,
        }
    }
}

/// Explicit lifecycle state for durable job definitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchedulerJobLifecycleState {
    Draft,
    Active,
    Paused,
    Disabled,
    Deleted,
}

/// Explicit lifecycle state for materialized runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchedulerRunState {
    Queued,
    Leased,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
    Expired,
}

/// Provider-neutral target command stored on a scheduler job.
///
/// The scheduler stores dispatch intent and safe payload references.  Concrete
/// services still own interpretation of command payloads, and the scheduler must
/// never branch on application, workflow, provider, driver, model, gateway,
/// chain, payment, or business-domain names.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SchedulerTargetCommand {
    Service(ServiceTargetCommand),
    AgentExecution(AgentExecutionTargetCommand),
    HeartbeatWake(HeartbeatWakeTargetCommand),
    Application(ApplicationTargetCommand),
    Plugin(SchedulerPluginTargetCommand),
}

/// Generic service command target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceTargetCommand {
    pub service_id: KernelServiceId,
    pub command_name: ServiceCommandName,
    pub payload_ref: Option<AutonomyPayloadRef>,
    pub metadata: BTreeMap<String, String>,
}

/// Agent execution target that references service-owned execution payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentExecutionTargetCommand {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub task_id: Option<TaskId>,
    pub target_agent: Option<String>,
    pub execution_intent: AgentExecutionIntent,
    pub payload_ref: AutonomyPayloadRef,
    pub metadata: BTreeMap<String, String>,
}

/// Heartbeat wake target emitted by scheduler jobs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatWakeTargetCommand {
    pub wake_scope_key: String,
    pub wake_reason_code: String,
    pub payload_ref: Option<AutonomyPayloadRef>,
    pub metadata: BTreeMap<String, String>,
}

/// Application-declared capability target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationTargetCommand {
    pub application_id: ApplicationId,
    pub capability_id: CapabilityId,
    pub command_name: ServiceCommandName,
    pub payload_ref: Option<AutonomyPayloadRef>,
    pub metadata: BTreeMap<String, String>,
}

/// Plugin-declared capability target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchedulerPluginTargetCommand {
    pub plugin_id: String,
    pub capability_id: CapabilityId,
    pub command_name: ServiceCommandName,
    pub payload_ref: Option<AutonomyPayloadRef>,
    pub metadata: BTreeMap<String, String>,
}

/// Durable scheduler job definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchedulerJobDefinition {
    pub job_id: Option<SchedulerJobId>,
    pub scope: AutonomyScope,
    pub schedule: SchedulerScheduleSpec,
    pub target: SchedulerTargetCommand,
    pub lifecycle: SchedulerJobLifecycleState,
    pub missed_run_policy: SchedulerMissedRunPolicy,
    pub stagger_policy: Option<SchedulerStaggerPolicy>,
    pub concurrency: SchedulerConcurrencyPolicy,
    pub retry: SchedulerRetryPolicy,
    pub metadata: BTreeMap<String, String>,
}

impl SchedulerJobDefinition {
    /// Build a new active job definition from generic schedule and target data.
    pub fn new(
        scope: AutonomyScope,
        schedule: SchedulerScheduleSpec,
        target: SchedulerTargetCommand,
    ) -> MacacaResult<Self> {
        schedule.validate()?;
        Ok(Self {
            job_id: None,
            scope,
            schedule,
            target,
            lifecycle: SchedulerJobLifecycleState::Active,
            missed_run_policy: SchedulerMissedRunPolicy::Skip,
            stagger_policy: None,
            concurrency: SchedulerConcurrencyPolicy::default(),
            retry: SchedulerRetryPolicy::default(),
            metadata: BTreeMap::new(),
        })
    }
}

/// Command for registering a durable scheduler job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchedulerRegisterJobCommand {
    pub trace: TraceContext,
    pub definition: SchedulerJobDefinition,
}

impl SchedulerRegisterJobCommand {
    /// Build a traced registration command and reject untraceable scheduling.
    pub fn new(trace: TraceContext, definition: SchedulerJobDefinition) -> MacacaResult<Self> {
        validate_trace(&trace, "scheduler register job command requires trace_id")?;
        Ok(Self { trace, definition })
    }

    /// Convert this typed DTO into the generic service runtime command shape.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(SCHEDULER_REGISTER_JOB_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}

/// Command for mutating lifecycle or metadata on an existing job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerJobCommand {
    pub trace: TraceContext,
    pub job_id: SchedulerJobId,
    pub reason_code: String,
    pub metadata: BTreeMap<String, String>,
}

impl SchedulerJobCommand {
    /// Build a traced job-scoped command for pause, resume, delete, or trigger.
    pub fn new(
        trace: TraceContext,
        job_id: SchedulerJobId,
        reason_code: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_trace(&trace, "scheduler job command requires trace_id")?;
        Ok(Self {
            trace,
            job_id,
            reason_code: non_empty(reason_code.into(), "scheduler reason_code is required")?,
            metadata: BTreeMap::new(),
        })
    }
}

/// Command for inspecting scheduler state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerQueryCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub job_id: Option<SchedulerJobId>,
    pub run_id: Option<SchedulerRunId>,
    pub limit: Option<usize>,
}

/// Bounded run summary returned by history and snapshot operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerRunSummary {
    pub run_id: SchedulerRunId,
    pub job_id: SchedulerJobId,
    pub state: SchedulerRunState,
    pub scheduled_for: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub attempt: u32,
    pub trace_id: String,
    pub audit_id: Option<String>,
    pub safe_status: String,
    pub metadata: BTreeMap<String, String>,
}

/// Typed result returned by Scheduler service commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchedulerCommandResult {
    pub job_id: Option<SchedulerJobId>,
    pub run_id: Option<SchedulerRunId>,
    pub lifecycle: Option<SchedulerJobLifecycleState>,
    pub run_state: Option<SchedulerRunState>,
    pub accepted: bool,
    pub error: Option<AutonomyStructuredError>,
    pub trace: TraceContext,
    pub audit_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Provider-neutral scheduler health snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerServiceSnapshot {
    pub service_id: String,
    pub provider_id: String,
    pub healthy: bool,
    pub lifecycle_state: String,
    pub active_jobs: usize,
    pub paused_jobs: usize,
    pub queued_runs: usize,
    pub active_runs: usize,
    pub recent_runs: Vec<SchedulerRunSummary>,
    pub last_audit_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl SchedulerServiceSnapshot {
    /// Build a fail-closed snapshot for hosts without scheduler providers.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            service_id: SCHEDULER_SERVICE_ID.into(),
            provider_id: "unavailable".into(),
            healthy: false,
            lifecycle_state: "unavailable".into(),
            active_jobs: 0,
            paused_jobs: 0,
            queued_runs: 0,
            active_runs: 0,
            recent_runs: Vec::new(),
            last_audit_ids: vec![reason.into()],
            captured_at: Utc::now(),
        }
    }
}
