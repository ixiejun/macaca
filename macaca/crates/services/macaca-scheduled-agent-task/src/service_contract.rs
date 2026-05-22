//! Service contract and Null Object provider for `service.scheduled_agent_task`.
//!
//! This trait is intentionally command-oriented.  Runtime decorators can attach
//! trace, policy, resource, entitlement, and metering behavior before providers
//! mutate payload mementos or register Scheduler jobs.  Provider absence is a
//! first-class unavailable result rather than a panic or fake success.

use async_trait::async_trait;
use macaca_proto::{
    AutonomyStructuredError, CancelScheduledAgentTaskCommand, CapabilityId, CleanupPolicy,
    CreateScheduledAgentTaskCommand, KernelServiceId, MacacaResult,
    RecordScheduledAgentTaskResultCommand, ResolveScheduledAgentTaskPayloadCommand,
    ScheduledAgentTaskCommandResult, ScheduledAgentTaskQueryCommand,
    ScheduledAgentTaskResolvedPayload, ScheduledAgentTaskServiceSnapshot,
    ScheduledAgentTaskSummary, ServiceCapability, ServiceDescriptor, ServiceHealth,
    ServiceLifecycleState, ServiceScope, ServiceType, TraceContext, TraceSchemaRef,
    SCHEDULED_AGENT_TASK_CANCEL_COMMAND, SCHEDULED_AGENT_TASK_CREATE_COMMAND,
    SCHEDULED_AGENT_TASK_GET_COMMAND, SCHEDULED_AGENT_TASK_HEALTH_COMMAND,
    SCHEDULED_AGENT_TASK_LIST_COMMAND, SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND,
    SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND, SCHEDULED_AGENT_TASK_SERVICE_ID,
};
use tracing::{info, warn};

/// Provider-neutral Scheduled Agent Task service interface.
#[async_trait]
pub trait ScheduledAgentTaskService: Send + Sync {
    /// Return the service descriptor used by registry, facade, and diagnostics.
    fn descriptor(&self) -> ServiceDescriptor;

    /// Return a lightweight health/snapshot view for diagnostics surfaces.
    async fn health(&self, trace: TraceContext) -> MacacaResult<ScheduledAgentTaskServiceSnapshot>;

    /// Admit durable scheduled-agent-task intent and register the Scheduler job.
    async fn create_task(
        &self,
        command: CreateScheduledAgentTaskCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult>;

    /// Read one redacted scheduled-agent-task summary.
    async fn get_task(
        &self,
        command: ScheduledAgentTaskQueryCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskSummary>>;

    /// List redacted scheduled-agent-task summaries within the requested scope.
    async fn list_tasks(
        &self,
        command: ScheduledAgentTaskQueryCommand,
    ) -> MacacaResult<Vec<ScheduledAgentTaskSummary>>;

    /// Cancel one scheduled-agent-task intent and ask Scheduler to delete its job.
    async fn cancel_task(
        &self,
        command: CancelScheduledAgentTaskCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult>;

    /// Resolve prompt material for trusted runtime-host dispatch.
    async fn resolve_payload(
        &self,
        command: ResolveScheduledAgentTaskPayloadCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskResolvedPayload>>;

    /// Record final Scheduler/Agent Execution result evidence in the summary.
    async fn record_result(
        &self,
        command: RecordScheduledAgentTaskResultCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult>;
}

/// Fail-closed provider used when scheduled-agent-task capability is absent.
#[derive(Debug, Clone)]
pub struct UnavailableScheduledAgentTaskProvider {
    provider_id: String,
    reason: String,
}

impl UnavailableScheduledAgentTaskProvider {
    /// Create an unavailable provider with a safe reason.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            provider_id: "unavailable".into(),
            reason: reason.into(),
        }
    }

    fn unavailable_result(
        &self,
        command_name: &'static str,
        trace: TraceContext,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        warn!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            command = command_name,
            trace_id = trace.trace_id.as_str(),
            "scheduled agent task command rejected because no provider is available"
        );
        let error = AutonomyStructuredError::unavailable(trace.clone(), self.reason.clone())?;
        Ok(ScheduledAgentTaskCommandResult::rejected(trace, error))
    }
}

impl Default for UnavailableScheduledAgentTaskProvider {
    fn default() -> Self {
        Self::new("scheduled agent task provider is not installed")
    }
}

#[async_trait]
impl ScheduledAgentTaskService for UnavailableScheduledAgentTaskProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        let mut descriptor = ServiceDescriptor::new(
            KernelServiceId::new(SCHEDULED_AGENT_TASK_SERVICE_ID),
            ServiceType::new("autonomy.scheduled_agent_task"),
            TraceSchemaRef::new("macaca.trace.scheduled_agent_task.v1"),
        );
        descriptor.lifecycle_state = ServiceLifecycleState::Registered;
        descriptor.health = ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        };
        descriptor.supported_scopes = vec![ServiceScope::Global];
        descriptor.cleanup_policy = CleanupPolicy::None;
        descriptor.capabilities = vec![
            ServiceCapability::new(
                CapabilityId::new(SCHEDULED_AGENT_TASK_CREATE_COMMAND),
                "Create generic scheduled agent task intent",
            ),
            ServiceCapability::new(
                CapabilityId::new(SCHEDULED_AGENT_TASK_LIST_COMMAND),
                "List redacted scheduled agent task summaries",
            ),
            ServiceCapability::new(
                CapabilityId::new(SCHEDULED_AGENT_TASK_HEALTH_COMMAND),
                "Read scheduled agent task service health",
            ),
            ServiceCapability::new(
                CapabilityId::new(SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND),
                "Resolve owned scheduled agent task payloads for dispatch",
            ),
            ServiceCapability::new(
                CapabilityId::new(SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND),
                "Record sanitized scheduled agent task result evidence",
            ),
        ];
        descriptor
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<ScheduledAgentTaskServiceSnapshot> {
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            trace_id = trace.trace_id.as_str(),
            "scheduled agent task health requested from unavailable provider"
        );
        Ok(ScheduledAgentTaskServiceSnapshot::unavailable(
            self.reason.clone(),
        ))
    }

    async fn create_task(
        &self,
        command: CreateScheduledAgentTaskCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        self.unavailable_result(SCHEDULED_AGENT_TASK_CREATE_COMMAND, command.trace)
    }

    async fn get_task(
        &self,
        command: ScheduledAgentTaskQueryCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskSummary>> {
        warn!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            command = SCHEDULED_AGENT_TASK_GET_COMMAND,
            trace_id = command.trace.trace_id.as_str(),
            "scheduled agent task get requested but provider is unavailable"
        );
        Ok(None)
    }

    async fn list_tasks(
        &self,
        command: ScheduledAgentTaskQueryCommand,
    ) -> MacacaResult<Vec<ScheduledAgentTaskSummary>> {
        warn!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            command = SCHEDULED_AGENT_TASK_LIST_COMMAND,
            trace_id = command.trace.trace_id.as_str(),
            "scheduled agent task list requested but provider is unavailable"
        );
        Ok(Vec::new())
    }

    async fn cancel_task(
        &self,
        command: CancelScheduledAgentTaskCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        self.unavailable_result(SCHEDULED_AGENT_TASK_CANCEL_COMMAND, command.trace)
    }

    async fn resolve_payload(
        &self,
        command: ResolveScheduledAgentTaskPayloadCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskResolvedPayload>> {
        warn!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            command = SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND,
            trace_id = command.trace.trace_id.as_str(),
            "scheduled agent task payload resolve requested but provider is unavailable"
        );
        Ok(None)
    }

    async fn record_result(
        &self,
        command: RecordScheduledAgentTaskResultCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        self.unavailable_result(SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND, command.trace)
    }
}
