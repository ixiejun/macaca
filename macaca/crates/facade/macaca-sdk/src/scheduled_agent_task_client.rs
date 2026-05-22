//! SDK Scheduled Agent Task client.
//!
//! The client is a focused Facade over the generic `SystemServiceClient`.  Web,
//! CLI, application runtimes, and entry-agent tools can submit typed scheduled
//! task intent without constructing Scheduler providers, payload stores, or
//! runtime-host providers.  Raw prompts pass only inside create commands and
//! remain owned by `service.scheduled_agent_task` after admission.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    CancelScheduledAgentTaskCommand, CreateScheduledAgentTaskCommand, MacacaError, MacacaResult,
    ResolveScheduledAgentTaskPayloadCommand, ScheduledAgentTaskCommandResult,
    ScheduledAgentTaskQueryCommand, ScheduledAgentTaskResolvedPayload,
    ScheduledAgentTaskServiceSnapshot, ScheduledAgentTaskSummary, TraceContext,
    SCHEDULED_AGENT_TASK_CANCEL_COMMAND, SCHEDULED_AGENT_TASK_CREATE_COMMAND,
    SCHEDULED_AGENT_TASK_GET_COMMAND, SCHEDULED_AGENT_TASK_HEALTH_COMMAND,
    SCHEDULED_AGENT_TASK_LIST_COMMAND, SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND,
    SCHEDULED_AGENT_TASK_SERVICE_ID,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Scheduled Agent Task client consumed by shells and tool adapters.
#[async_trait]
pub trait SystemScheduledAgentTaskClient: Send + Sync {
    async fn create_task(
        &self,
        command: CreateScheduledAgentTaskCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult>;
    async fn get_task(
        &self,
        command: ScheduledAgentTaskQueryCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskSummary>>;
    async fn list_tasks(
        &self,
        command: ScheduledAgentTaskQueryCommand,
    ) -> MacacaResult<Vec<ScheduledAgentTaskSummary>>;
    async fn cancel_task(
        &self,
        command: CancelScheduledAgentTaskCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult>;
    async fn resolve_payload(
        &self,
        command: ResolveScheduledAgentTaskPayloadCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskResolvedPayload>>;
    async fn health(&self, trace: TraceContext) -> MacacaResult<ScheduledAgentTaskServiceSnapshot>;
}

/// Null-object client used when the service is absent from a host.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemScheduledAgentTaskClient;

#[async_trait]
impl SystemScheduledAgentTaskClient for UnavailableSystemScheduledAgentTaskClient {
    async fn create_task(
        &self,
        command: CreateScheduledAgentTaskCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk scheduled agent task client unavailable for create_task");
        Err(unavailable_error())
    }

    async fn get_task(
        &self,
        command: ScheduledAgentTaskQueryCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskSummary>> {
        info!(trace_id = %command.trace.trace_id, "sdk scheduled agent task client unavailable for get_task");
        Ok(None)
    }

    async fn list_tasks(
        &self,
        command: ScheduledAgentTaskQueryCommand,
    ) -> MacacaResult<Vec<ScheduledAgentTaskSummary>> {
        info!(trace_id = %command.trace.trace_id, "sdk scheduled agent task client unavailable for list_tasks");
        Ok(Vec::new())
    }

    async fn cancel_task(
        &self,
        command: CancelScheduledAgentTaskCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk scheduled agent task client unavailable for cancel_task");
        Err(unavailable_error())
    }

    async fn resolve_payload(
        &self,
        command: ResolveScheduledAgentTaskPayloadCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskResolvedPayload>> {
        info!(trace_id = %command.trace.trace_id, "sdk scheduled agent task client unavailable for resolve_payload");
        Ok(None)
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<ScheduledAgentTaskServiceSnapshot> {
        info!(trace_id = %trace.trace_id, "sdk scheduled agent task client returning unavailable health");
        Ok(ScheduledAgentTaskServiceSnapshot::unavailable(
            "Scheduled Agent Task service is unavailable",
        ))
    }
}

/// Runtime-backed Scheduled Agent Task client implemented over service calls.
#[derive(Clone)]
pub struct ServiceBackedScheduledAgentTaskClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedScheduledAgentTaskClient {
    /// Create a service-backed client from a generic service dispatcher.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemScheduledAgentTaskClient for ServiceBackedScheduledAgentTaskClient {
    async fn create_task(
        &self,
        command: CreateScheduledAgentTaskCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        call(
            &self.service,
            SCHEDULED_AGENT_TASK_CREATE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn get_task(
        &self,
        command: ScheduledAgentTaskQueryCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskSummary>> {
        call(
            &self.service,
            SCHEDULED_AGENT_TASK_GET_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn list_tasks(
        &self,
        command: ScheduledAgentTaskQueryCommand,
    ) -> MacacaResult<Vec<ScheduledAgentTaskSummary>> {
        call(
            &self.service,
            SCHEDULED_AGENT_TASK_LIST_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn cancel_task(
        &self,
        command: CancelScheduledAgentTaskCommand,
    ) -> MacacaResult<ScheduledAgentTaskCommandResult> {
        call(
            &self.service,
            SCHEDULED_AGENT_TASK_CANCEL_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn resolve_payload(
        &self,
        command: ResolveScheduledAgentTaskPayloadCommand,
    ) -> MacacaResult<Option<ScheduledAgentTaskResolvedPayload>> {
        call(
            &self.service,
            SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<ScheduledAgentTaskServiceSnapshot> {
        call(
            &self.service,
            SCHEDULED_AGENT_TASK_HEALTH_COMMAND,
            trace.clone(),
            serde_json::json!({}),
        )
        .await
    }
}

async fn call<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let command = ServiceCallCommand::new(
        SCHEDULED_AGENT_TASK_SERVICE_ID,
        command_name,
        serde_json::to_value(payload).map_err(|error| MacacaError::Config(error.to_string()))?,
    )?
    .with_trace(trace);
    let result = service.call_service(&command).await?;
    serde_json::from_value(result.output).map_err(|error| MacacaError::Config(error.to_string()))
}

fn unavailable_error() -> MacacaError {
    MacacaError::Config("Scheduled Agent Task service is unavailable".into())
}
