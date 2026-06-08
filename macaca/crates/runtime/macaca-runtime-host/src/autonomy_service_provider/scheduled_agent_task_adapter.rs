//! Runtime-host adapter for the Scheduled Agent Task autonomy service.
//!
//! **Pattern:** Adapter/Bridge — decodes generic `ServiceCommand` envelopes into
//! typed scheduled-task DTOs.  Prompt storage, schedule evaluation, and agent
//! execution remain owned by downstream services, not this adapter.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    CancelScheduledAgentTaskCommand, CreateScheduledAgentTaskCommand,
    RecordScheduledAgentTaskResultCommand, ResolveScheduledAgentTaskPayloadCommand,
    ScheduledAgentTaskQueryCommand, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, SCHEDULED_AGENT_TASK_CANCEL_COMMAND,
    SCHEDULED_AGENT_TASK_CREATE_COMMAND, SCHEDULED_AGENT_TASK_GET_COMMAND,
    SCHEDULED_AGENT_TASK_HEALTH_COMMAND, SCHEDULED_AGENT_TASK_LIST_COMMAND,
    SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND, SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND,
    SCHEDULED_AGENT_TASK_SERVICE_ID,
};
use macaca_scheduled_agent_task::{
    ScheduledAgentTaskService, UnavailableScheduledAgentTaskProvider,
};
use tracing::info;

use super::support::{command_trace, decode, service_adapter_error, service_result};

/// Runtime-host adapter for a Scheduled Agent Task service provider.
///
/// The adapter is the service-runtime Bridge for scheduled task intent.  It
/// decodes generic `ServiceCommand` envelopes into typed DTOs and delegates to
/// the injected provider.  It does not store prompts, compute schedules, or
/// execute agents; those responsibilities stay in the scheduled-task service,
/// Scheduler service, and Agent Execution service respectively.
pub struct ScheduledAgentTaskSystemServiceProvider {
    provider: Arc<dyn ScheduledAgentTaskService>,
}

impl ScheduledAgentTaskSystemServiceProvider {
    /// Wrap a concrete, remote, plugin, mock, or unavailable provider.
    pub fn new(provider: Arc<dyn ScheduledAgentTaskService>) -> Self {
        Self { provider }
    }

    /// Build the fail-closed Null Object provider used by default.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableScheduledAgentTaskProvider::default()))
    }
}

#[async_trait]
impl SystemService for ScheduledAgentTaskSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        let descriptor = self.provider.descriptor();
        info!(
            service_id = %descriptor.id,
            "scheduled agent task system service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command_trace(&command)?;
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            command = %command.name,
            trace_id = %trace.trace_id,
            "scheduled agent task system service command accepted"
        );
        match command.name.as_str() {
            SCHEDULED_AGENT_TASK_CREATE_COMMAND => {
                let typed: CreateScheduledAgentTaskCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .create_task(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULED_AGENT_TASK_GET_COMMAND => {
                let typed: ScheduledAgentTaskQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .get_task(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULED_AGENT_TASK_LIST_COMMAND => {
                let typed: ScheduledAgentTaskQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .list_tasks(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULED_AGENT_TASK_CANCEL_COMMAND => {
                let typed: CancelScheduledAgentTaskCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .cancel_task(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULED_AGENT_TASK_RESOLVE_PAYLOAD_COMMAND => {
                let typed: ResolveScheduledAgentTaskPayloadCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .resolve_payload(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULED_AGENT_TASK_RECORD_RESULT_COMMAND => {
                let typed: RecordScheduledAgentTaskResultCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .record_result(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULED_AGENT_TASK_HEALTH_COMMAND => service_result(
                self.provider
                    .health(trace.clone())
                    .await
                    .map_err(service_adapter_error)?,
                trace,
            ),
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Scheduled Agent Task service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            "scheduled agent task system service provider stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = SCHEDULED_AGENT_TASK_SERVICE_ID,
            "scheduled agent task system service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.provider.descriptor().health)
    }
}
