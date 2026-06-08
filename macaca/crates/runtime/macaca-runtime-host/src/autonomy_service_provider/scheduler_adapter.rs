//! Runtime-host adapter for the Scheduler autonomy service.
//!
//! **Pattern:** Adapter/Bridge — `ServiceRuntime` speaks generic `ServiceCommand`
//! while the injected `SchedulerService` speaks typed command DTOs.  The adapter
//! only decodes and delegates; scheduling semantics remain provider-owned.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    SchedulerDeleteJobCommand, SchedulerGetJobCommand, SchedulerJobCommand,
    SchedulerLifecycleJobCommand, SchedulerListJobsCommand, SchedulerQueryCommand,
    SchedulerRegisterJobCommand, SchedulerUpdateJobCommand, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, SCHEDULER_DELETE_JOB_COMMAND,
    SCHEDULER_GET_JOB_COMMAND, SCHEDULER_GET_RUN_COMMAND, SCHEDULER_HEALTH_COMMAND,
    SCHEDULER_LIST_JOBS_COMMAND, SCHEDULER_LIST_RUNS_COMMAND, SCHEDULER_PAUSE_JOB_COMMAND,
    SCHEDULER_REGISTER_JOB_COMMAND, SCHEDULER_RESUME_JOB_COMMAND, SCHEDULER_SERVICE_ID,
    SCHEDULER_SNAPSHOT_COMMAND, SCHEDULER_TRIGGER_JOB_COMMAND, SCHEDULER_UPDATE_JOB_COMMAND,
};
use macaca_scheduler::{SchedulerService, UnavailableSchedulerProvider};
use tracing::info;

use super::support::{command_trace, decode, service_adapter_error, service_result};

/// Runtime-host adapter for a Scheduler service provider.
///
/// The adapter implements the Adapter/Bridge pattern.  `ServiceRuntime` speaks
/// generic `ServiceCommand`, while the Scheduler provider speaks typed command
/// DTOs.  The adapter only decodes commands and delegates; actual scheduling
/// semantics remain owned by the injected provider.
pub struct HostSchedulerServiceAdapter {
    provider: Arc<dyn SchedulerService>,
}

impl HostSchedulerServiceAdapter {
    /// Wrap a concrete or remote Scheduler provider.
    pub fn new(provider: Arc<dyn SchedulerService>) -> Self {
        Self { provider }
    }

    /// Build the fail-closed Null Object provider used by default.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableSchedulerProvider::default()))
    }
}

#[async_trait]
impl SystemService for HostSchedulerServiceAdapter {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        let descriptor = self.provider.descriptor();
        info!(
            service_id = %descriptor.id,
            "scheduler system service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command_trace(&command)?;
        info!(
            service_id = SCHEDULER_SERVICE_ID,
            command = %command.name,
            trace_id = %trace.trace_id,
            "scheduler system service command accepted"
        );
        match command.name.as_str() {
            SCHEDULER_REGISTER_JOB_COMMAND => {
                let typed: SchedulerRegisterJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .register_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_UPDATE_JOB_COMMAND => {
                let typed: SchedulerUpdateJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .update_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_PAUSE_JOB_COMMAND => {
                let typed: SchedulerLifecycleJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .pause_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_RESUME_JOB_COMMAND => {
                let typed: SchedulerLifecycleJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .resume_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_DELETE_JOB_COMMAND => {
                let typed: SchedulerDeleteJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .delete_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_TRIGGER_JOB_COMMAND => {
                let typed: SchedulerJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .trigger_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_GET_JOB_COMMAND => {
                let typed: SchedulerGetJobCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .get_job(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_LIST_JOBS_COMMAND => {
                let typed: SchedulerListJobsCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .list_jobs(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_GET_RUN_COMMAND => {
                let typed: SchedulerQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .get_run(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_LIST_RUNS_COMMAND => {
                let typed: SchedulerQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .list_runs(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            SCHEDULER_HEALTH_COMMAND => service_result(
                self.provider
                    .health(trace.clone())
                    .await
                    .map_err(service_adapter_error)?,
                trace,
            ),
            SCHEDULER_SNAPSHOT_COMMAND => {
                let typed: SchedulerQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .snapshot(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Scheduler service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = SCHEDULER_SERVICE_ID,
            "scheduler system service provider stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = SCHEDULER_SERVICE_ID,
            "scheduler system service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.provider.descriptor().health)
    }
}
