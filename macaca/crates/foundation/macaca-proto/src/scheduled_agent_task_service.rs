//! Provider-neutral Scheduled Agent Task service contracts.
//!
//! A scheduled agent task is a durable user intent, not a Scheduler-owned
//! prompt.  This module defines the typed command/result vocabulary used by
//! Web, SDK clients, entry-agent tools, Scheduler dispatch, and runtime-host.
//! The owning service may accept raw prompt material at the create boundary,
//! but every Scheduler-facing and observability-facing structure carries only a
//! bounded [`AutonomyPayloadRef`], digest, redacted summary, trace id, audit id,
//! and safe metadata.  Keeping that split explicit preserves the microkernel
//! boundary: Scheduler owns time and leases, Agent Execution owns LLM/tool
//! execution, and this service owns prompt payload mementos plus audit links.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    autonomy_service::{non_empty, validate_trace},
    AgentExecutionIntent, AgentExecutionPolicyContext, ApplicationId, AutonomyPayloadRef,
    AutonomyScope, AutonomyStructuredError, MacacaResult, SchedulerJobId, SchedulerRunId,
    SchedulerScheduleSpec, ServiceCommand, ServiceCommandName, TaskId, TraceContext,
};

/// Stable service id used by runtime-host registration and focused SDK clients.
pub const SCHEDULED_AGENT_TASK_SERVICE_ID: &str = "service.scheduled_agent_task";

/// Command that admits a new durable scheduled-agent-task intent.
pub const SCHEDULED_AGENT_TASK_CREATE_COMMAND: &str = "scheduled_agent_task.create";

/// Command that returns one redacted scheduled-agent-task summary.
pub const SCHEDULED_AGENT_TASK_GET_COMMAND: &str = "scheduled_agent_task.get";

/// Command that returns redacted scheduled-agent-task summaries within a scope.
pub const SCHEDULED_AGENT_TASK_LIST_COMMAND: &str = "scheduled_agent_task.list";

/// Command that cancels a scheduled-agent-task intent and its Scheduler job.
pub const SCHEDULED_AGENT_TASK_CANCEL_COMMAND: &str = "scheduled_agent_task.cancel";

/// Command that resolves an owned payload reference for runtime-host dispatch.
pub const SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND: &str =
    "scheduled_agent_task.payload.resolve";

/// Command that records Scheduler/Agent Execution final result evidence.
pub const SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND: &str = "scheduled_agent_task.result.record";

/// Command that returns provider health and bounded diagnostics.
pub const SCHEDULED_AGENT_TASK_HEALTH_COMMAND: &str = "scheduled_agent_task.health";

/// Stable identity for a scheduled-agent-task intent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ScheduledAgentTaskId(String);

impl ScheduledAgentTaskId {
    /// Create a task id from provider-assigned, imported, or test state.
    pub fn new(value: impl Into<String>) -> MacacaResult<Self> {
        Ok(Self(non_empty(
            value.into(),
            "scheduled agent task id is required",
        )?))
    }

    /// Return the raw identifier string for service providers and SDK adapters.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Provider-neutral schedule accepted by Scheduled Agent Task create commands.
///
/// This enum mirrors Scheduler schedule primitives without importing any
/// provider-specific cron or timer implementation.  The owning service converts
/// the value into [`SchedulerScheduleSpec`] immediately before registering a
/// Scheduler job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduledAgentTaskSchedule {
    /// Run once at a concrete UTC timestamp.
    At { run_at: DateTime<Utc> },
    /// Run repeatedly after a bounded interval in milliseconds.
    Every { interval_ms: u64 },
    /// Run according to a cron expression interpreted by Scheduler providers.
    CronExpression {
        expression: String,
        timezone: String,
    },
}

impl ScheduledAgentTaskSchedule {
    /// Validate schedule syntax at the DTO boundary without materializing due
    /// runs.  Concrete due-time calculation remains owned by Scheduler.
    pub fn validate(&self) -> MacacaResult<()> {
        match self {
            Self::At { .. } => Ok(()),
            Self::Every { interval_ms } if *interval_ms > 0 => Ok(()),
            Self::Every { .. } => Err(crate::MacacaError::Config(
                "scheduled agent task interval must be positive".into(),
            )),
            Self::CronExpression {
                expression,
                timezone,
            } => {
                non_empty(expression.clone(), "cron expression is required")?;
                non_empty(timezone.clone(), "cron timezone is required")?;
                Ok(())
            }
        }
    }

    /// Convert the intent-level schedule into the Scheduler-owned schedule
    /// shape.  This is a pure DTO conversion; it does not compute due times.
    pub fn into_scheduler_spec(self) -> SchedulerScheduleSpec {
        match self {
            Self::At { run_at } => SchedulerScheduleSpec::At { run_at },
            Self::Every { interval_ms } => SchedulerScheduleSpec::Every {
                interval_ms,
                anchor: None,
            },
            Self::CronExpression {
                expression,
                timezone,
            } => SchedulerScheduleSpec::CronExpression {
                expression,
                timezone,
                start_after: None,
                end_before: None,
            },
        }
    }
}

/// Policy facts carried with scheduled task intent before side effects happen.
///
/// The current shape intentionally reuses generic metadata rather than encoding
/// product-domain rules.  Providers and decorators can extend policy decisions
/// through service metadata, capability declarations, budget gates, and future
/// entitlement checks without changing Scheduler semantics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledAgentTaskPolicy {
    pub execution_policy: AgentExecutionPolicyContext,
    pub required_capabilities: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Command accepted by `service.scheduled_agent_task` for new intent.
///
/// `user_prompt` is deliberately present only on this create command.  The
/// service provider must persist it as an owned payload memento and then pass a
/// redacted [`AutonomyPayloadRef`] to Scheduler.  No Scheduler job, Scheduler
/// run, safe summary, log, or frontend response should contain this field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateScheduledAgentTaskCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub schedule: ScheduledAgentTaskSchedule,
    pub target_agent: String,
    pub user_prompt: String,
    pub delegated_context: serde_json::Value,
    pub policy: ScheduledAgentTaskPolicy,
    pub metadata: BTreeMap<String, String>,
}

impl CreateScheduledAgentTaskCommand {
    /// Build a validated create command from the minimal required intent facts.
    pub fn new(
        trace: TraceContext,
        scope: AutonomyScope,
        schedule: ScheduledAgentTaskSchedule,
        target_agent: impl Into<String>,
        user_prompt: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_trace(
            &trace,
            "scheduled agent task create command requires trace_id",
        )?;
        schedule.validate()?;
        Ok(Self {
            trace,
            scope,
            schedule,
            target_agent: non_empty(
                target_agent.into(),
                "scheduled agent task target_agent is required",
            )?,
            user_prompt: non_empty(
                user_prompt.into(),
                "scheduled agent task user_prompt is required",
            )?,
            delegated_context: serde_json::json!({}),
            policy: ScheduledAgentTaskPolicy::default(),
            metadata: BTreeMap::new(),
        })
    }

    /// Attach bounded delegated context without changing trusted system prompt
    /// ownership. Runtime Agent Context still builds the system prompt later.
    pub fn with_delegated_context(mut self, context: serde_json::Value) -> Self {
        self.delegated_context = context;
        self
    }

    /// Attach safe caller metadata through a Builder-style method.  Providers
    /// must continue filtering secret-like keys before logging or snapshots.
    pub fn with_metadata(mut self, metadata: BTreeMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Convert this typed command into the generic service runtime shape.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(SCHEDULED_AGENT_TASK_CREATE_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}

/// Query command for get/list operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledAgentTaskQueryCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub task_id: Option<ScheduledAgentTaskId>,
    pub limit: Option<usize>,
}

impl ScheduledAgentTaskQueryCommand {
    /// Build a traced query command.  Scope remains provider-neutral so future
    /// tenant/session filters can reuse the same service surface.
    pub fn new(
        trace: TraceContext,
        scope: AutonomyScope,
        task_id: Option<ScheduledAgentTaskId>,
    ) -> MacacaResult<Self> {
        validate_trace(
            &trace,
            "scheduled agent task query command requires trace_id",
        )?;
        Ok(Self {
            trace,
            scope,
            task_id,
            limit: None,
        })
    }
}

/// Command for cancelling one scheduled-agent-task intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CancelScheduledAgentTaskCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub task_id: ScheduledAgentTaskId,
    pub reason_code: String,
    pub metadata: BTreeMap<String, String>,
}

impl CancelScheduledAgentTaskCommand {
    /// Build a traced cancellation command with a safe reason code.
    pub fn new(
        trace: TraceContext,
        scope: AutonomyScope,
        task_id: ScheduledAgentTaskId,
        reason_code: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_trace(
            &trace,
            "scheduled agent task cancel command requires trace_id",
        )?;
        Ok(Self {
            trace,
            scope,
            task_id,
            reason_code: non_empty(reason_code.into(), "cancel reason_code is required")?,
            metadata: BTreeMap::new(),
        })
    }
}

/// Command used by runtime-host dispatch to resolve owned payload material.
///
/// Runtime Host needs raw user prompt only at the exact moment it creates an
/// [`crate::AgentExecutionCommand`].  This command keeps the resolution
/// auditable and service-owned rather than letting Scheduler or Runtime Host
/// persist prompt bodies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolveScheduledAgentTaskPayloadCommand {
    pub trace: TraceContext,
    pub payload_ref: AutonomyPayloadRef,
    pub scheduler_job_id: Option<SchedulerJobId>,
    pub scheduler_run_id: Option<SchedulerRunId>,
    pub metadata: BTreeMap<String, String>,
}

impl ResolveScheduledAgentTaskPayloadCommand {
    /// Build a traced payload-resolution command from a Scheduler target ref.
    pub fn new(trace: TraceContext, payload_ref: AutonomyPayloadRef) -> MacacaResult<Self> {
        validate_trace(
            &trace,
            "scheduled agent task payload resolve command requires trace_id",
        )?;
        Ok(Self {
            trace,
            payload_ref,
            scheduler_job_id: None,
            scheduler_run_id: None,
            metadata: BTreeMap::new(),
        })
    }
}

/// Command used by Runtime Host to update scheduled-agent-task result summary.
///
/// Scheduler remains the owner of run lifecycle state.  Runtime Host calls this
/// command only after a Scheduler run reaches a terminal dispatch outcome so
/// the Scheduled Agent Task service can keep its redacted summary and audit
/// chain correlated with the latest run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordScheduledAgentTaskResultCommand {
    pub trace: TraceContext,
    pub task_id: ScheduledAgentTaskId,
    pub scheduler_run_id: SchedulerRunId,
    pub scheduler_job_id: Option<SchedulerJobId>,
    pub result_status: String,
    pub agent_execution_trace_id: Option<String>,
    pub result_audit_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl RecordScheduledAgentTaskResultCommand {
    /// Build a traced result-recording command with a safe status label.
    pub fn new(
        trace: TraceContext,
        task_id: ScheduledAgentTaskId,
        scheduler_run_id: SchedulerRunId,
        result_status: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_trace(
            &trace,
            "scheduled agent task record result command requires trace_id",
        )?;
        Ok(Self {
            trace,
            task_id,
            scheduler_run_id,
            scheduler_job_id: None,
            result_status: non_empty(result_status.into(), "result_status is required")?,
            agent_execution_trace_id: None,
            result_audit_id: None,
            metadata: BTreeMap::new(),
        })
    }

    /// Attach Scheduler job correlation after the run-control transition.
    pub fn with_scheduler_job_id(mut self, scheduler_job_id: Option<SchedulerJobId>) -> Self {
        self.scheduler_job_id = scheduler_job_id;
        self
    }

    /// Attach bounded trace/audit metadata without exposing raw provider output.
    pub fn with_result_evidence(
        mut self,
        agent_execution_trace_id: Option<String>,
        result_audit_id: Option<String>,
    ) -> Self {
        self.agent_execution_trace_id = agent_execution_trace_id;
        self.result_audit_id = result_audit_id;
        self
    }

    /// Convert this typed command into the generic service runtime shape.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}

/// Payload material returned only to trusted runtime-host dispatch.
///
/// This DTO intentionally does not appear in list/get summaries.  It carries
/// exactly the fields needed to construct an `AgentExecutionCommand`, plus safe
/// digest metadata for audit correlation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledAgentTaskResolvedPayload {
    pub task_id: ScheduledAgentTaskId,
    pub application_id: ApplicationId,
    pub session_id: String,
    pub task_ref: Option<TaskId>,
    pub target_agent: String,
    pub execution_intent: AgentExecutionIntent,
    pub user_prompt: String,
    pub delegated_context: serde_json::Value,
    pub policy: AgentExecutionPolicyContext,
    pub payload_ref: AutonomyPayloadRef,
    pub payload_digest: Option<String>,
    pub trace: TraceContext,
    pub audit_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Sanitized summary safe for SDK, Web, frontend, CLI, and audit replay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledAgentTaskSummary {
    pub task_id: ScheduledAgentTaskId,
    pub scope: AutonomyScope,
    pub schedule: ScheduledAgentTaskSchedule,
    pub target_agent: String,
    pub execution_intent: AgentExecutionIntent,
    pub scheduler_job_id: Option<SchedulerJobId>,
    pub payload_ref: AutonomyPayloadRef,
    pub payload_digest: Option<String>,
    pub redacted_summary: String,
    pub lifecycle_state: String,
    pub last_run_id: Option<SchedulerRunId>,
    pub last_result_status: Option<String>,
    pub trace_id: Option<String>,
    pub audit_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl ScheduledAgentTaskSummary {
    /// Test helper that builds a redacted summary without ever accepting a raw
    /// prompt.  Production providers should build summaries from payload refs.
    #[cfg(test)]
    pub fn redacted_fixture_for_tests(
        task_id: &str,
        payload_digest: &str,
        redacted_summary: &str,
    ) -> Self {
        let task_id = ScheduledAgentTaskId::new(task_id).unwrap();
        let mut payload_ref = AutonomyPayloadRef::new(
            format!("scheduled-agent-task://{}", task_id.as_str()),
            redacted_summary,
        )
        .unwrap();
        payload_ref.content_digest = Some(payload_digest.into());
        Self {
            task_id,
            scope: AutonomyScope::global(),
            schedule: ScheduledAgentTaskSchedule::Every {
                interval_ms: 60_000,
            },
            target_agent: "agent".into(),
            execution_intent: AgentExecutionIntent::TaskWorker,
            scheduler_job_id: None,
            payload_ref,
            payload_digest: Some(payload_digest.into()),
            redacted_summary: redacted_summary.into(),
            lifecycle_state: "active".into(),
            last_run_id: None,
            last_result_status: None,
            trace_id: Some("trace-summary".into()),
            audit_id: Some("audit.summary.1".into()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Result returned by scheduled-agent-task mutation commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledAgentTaskCommandResult {
    pub accepted: bool,
    pub task_id: Option<ScheduledAgentTaskId>,
    pub scheduler_job_id: Option<SchedulerJobId>,
    pub scheduler_run_id: Option<SchedulerRunId>,
    pub payload_ref: Option<AutonomyPayloadRef>,
    pub payload_digest: Option<String>,
    pub trace: TraceContext,
    pub audit_id: Option<String>,
    pub summary: Option<ScheduledAgentTaskSummary>,
    pub error: Option<AutonomyStructuredError>,
    pub metadata: BTreeMap<String, String>,
}

impl ScheduledAgentTaskCommandResult {
    /// Build a rejected result with structured error evidence.
    pub fn rejected(trace: TraceContext, error: AutonomyStructuredError) -> Self {
        Self {
            accepted: false,
            task_id: None,
            scheduler_job_id: None,
            scheduler_run_id: None,
            payload_ref: None,
            payload_digest: None,
            trace,
            audit_id: None,
            summary: None,
            error: Some(error),
            metadata: BTreeMap::new(),
        }
    }
}

/// Provider-neutral service snapshot for health and diagnostics surfaces.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledAgentTaskServiceSnapshot {
    pub service_id: String,
    pub provider_id: String,
    pub healthy: bool,
    pub lifecycle_state: String,
    pub active_tasks: usize,
    pub stored_payloads: usize,
    pub recent_audit_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
    pub metadata: BTreeMap<String, String>,
}

impl ScheduledAgentTaskServiceSnapshot {
    /// Build a fail-closed snapshot for hosts without an active provider.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let mut metadata = BTreeMap::new();
        metadata.insert("reason".into(), reason.into());
        Self {
            service_id: SCHEDULED_AGENT_TASK_SERVICE_ID.into(),
            provider_id: "unavailable".into(),
            healthy: false,
            lifecycle_state: "unavailable".into(),
            active_tasks: 0,
            stored_payloads: 0,
            recent_audit_ids: Vec::new(),
            captured_at: Utc::now(),
            metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_command_requires_trace_prompt_and_target_agent() {
        let command = CreateScheduledAgentTaskCommand::new(
            TraceContext::new("trace-scheduled-agent-task"),
            AutonomyScope::application(ApplicationId::from_name("scheduled-task-test")),
            ScheduledAgentTaskSchedule::Every {
                interval_ms: 60_000,
            },
            "technical_analyst",
            "Analyze the latest market state and record an auditable summary.",
        );
        assert!(command.is_ok());

        let missing_prompt = CreateScheduledAgentTaskCommand::new(
            TraceContext::new("trace-scheduled-agent-task"),
            AutonomyScope::application(ApplicationId::from_name("scheduled-task-test")),
            ScheduledAgentTaskSchedule::Every {
                interval_ms: 60_000,
            },
            "technical_analyst",
            " ",
        );
        assert!(missing_prompt.is_err());

        let missing_agent = CreateScheduledAgentTaskCommand::new(
            TraceContext::new("trace-scheduled-agent-task"),
            AutonomyScope::application(ApplicationId::from_name("scheduled-task-test")),
            ScheduledAgentTaskSchedule::Every {
                interval_ms: 60_000,
            },
            " ",
            "Analyze the latest market state and record an auditable summary.",
        );
        assert!(missing_agent.is_err());
    }

    #[test]
    fn safe_summary_never_contains_raw_prompt() {
        let summary = ScheduledAgentTaskSummary::redacted_fixture_for_tests(
            "task-1",
            "digest.prompt.123",
            "Daily analysis task",
        );
        let encoded = serde_json::to_string(&summary).unwrap();
        assert!(!encoded.contains("Analyze the latest market state"));
        assert!(encoded.contains("digest.prompt.123"));
        assert!(encoded.contains("Daily analysis task"));
    }

    #[test]
    fn create_command_round_trips_through_service_command() {
        let command = CreateScheduledAgentTaskCommand::new(
            TraceContext::new("trace-scheduled-agent-task-roundtrip"),
            AutonomyScope::application(ApplicationId::from_name("scheduled-task-test")),
            ScheduledAgentTaskSchedule::Every {
                interval_ms: 120_000,
            },
            "worker",
            "Prepare a status digest.",
        )
        .unwrap()
        .with_delegated_context(serde_json::json!({"format": "summary"}));

        let service_command = command.clone().into_service_command().unwrap();
        assert_eq!(
            service_command.name.as_str(),
            SCHEDULED_AGENT_TASK_CREATE_COMMAND
        );
        assert!(service_command.trace.is_some());

        let decoded: CreateScheduledAgentTaskCommand =
            serde_json::from_value(service_command.payload).unwrap();
        assert_eq!(decoded.target_agent, "worker");
        assert_eq!(decoded.user_prompt, "Prepare a status digest.");
        assert_eq!(decoded.delegated_context["format"], "summary");
    }

    #[test]
    fn record_result_command_round_trips_through_service_command() {
        let command = RecordScheduledAgentTaskResultCommand::new(
            TraceContext::new("trace-record-result"),
            ScheduledAgentTaskId::new("scheduled-agent-task-1").unwrap(),
            SchedulerRunId::new("run-1").unwrap(),
            "succeeded",
        )
        .unwrap()
        .with_scheduler_job_id(Some(SchedulerJobId::new("job-1").unwrap()))
        .with_result_evidence(
            Some("trace-agent-execution".into()),
            Some("audit.scheduler.run.succeeded.1".into()),
        );

        let service_command = command.clone().into_service_command().unwrap();
        assert_eq!(
            service_command.name.as_str(),
            SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND
        );
        let decoded: RecordScheduledAgentTaskResultCommand =
            serde_json::from_value(service_command.payload).unwrap();
        assert_eq!(decoded.task_id, command.task_id);
        assert_eq!(decoded.scheduler_run_id, command.scheduler_run_id);
        assert_eq!(decoded.result_status, "succeeded");
    }
}
