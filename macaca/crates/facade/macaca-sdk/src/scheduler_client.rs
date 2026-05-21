//! SDK Scheduler Service client for autonomous recurring work.
//!
//! This client is intentionally a thin Facade/Adapter over the generic
//! `SystemServiceClient`.  It gives applications and upper shells typed
//! Scheduler commands while preserving Macaca's microkernel boundary: the SDK
//! never constructs timers, queues, stores, cron engines, or concrete
//! providers.  All real scheduling semantics remain inside replaceable
//! `service.scheduler` providers registered by runtime-host composition.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    MacacaError, MacacaResult, SchedulerCommandResult, SchedulerJobCommand,
    SchedulerJobDefinition, SchedulerQueryCommand, SchedulerRegisterJobCommand,
    SchedulerRunSummary, SchedulerServiceSnapshot, TraceContext, SCHEDULER_DELETE_JOB_COMMAND,
    SCHEDULER_GET_JOB_COMMAND, SCHEDULER_GET_RUN_COMMAND, SCHEDULER_HEALTH_COMMAND,
    SCHEDULER_LIST_JOBS_COMMAND, SCHEDULER_LIST_RUNS_COMMAND, SCHEDULER_PAUSE_JOB_COMMAND,
    SCHEDULER_REGISTER_JOB_COMMAND, SCHEDULER_RESUME_JOB_COMMAND, SCHEDULER_SERVICE_ID,
    SCHEDULER_SNAPSHOT_COMMAND, SCHEDULER_TRIGGER_JOB_COMMAND, SCHEDULER_UPDATE_JOB_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Scheduler client consumed by SDK users and thin shells.
///
/// The trait exposes Scheduler's typed service contract without leaking the
/// provider implementation.  Implementations may be runtime-backed, remote, or
/// fail-closed Null Objects; callers should treat structured errors and
/// unavailable snapshots as normal service states rather than panicking.
#[async_trait]
pub trait SystemSchedulerClient: Send + Sync {
    async fn register_job(
        &self,
        command: SchedulerRegisterJobCommand,
    ) -> MacacaResult<SchedulerCommandResult>;
    async fn update_job(&self, command: SchedulerJobCommand)
        -> MacacaResult<SchedulerCommandResult>;
    async fn pause_job(&self, command: SchedulerJobCommand) -> MacacaResult<SchedulerCommandResult>;
    async fn resume_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult>;
    async fn delete_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult>;
    async fn trigger_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult>;
    async fn get_job(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Option<SchedulerJobDefinition>>;
    async fn list_jobs(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Vec<SchedulerJobDefinition>>;
    async fn get_run(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Option<SchedulerRunSummary>>;
    async fn list_runs(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Vec<SchedulerRunSummary>>;
    async fn health(&self, trace: TraceContext) -> MacacaResult<SchedulerServiceSnapshot>;
    async fn snapshot(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerServiceSnapshot>;
}

/// Null-object Scheduler client used when `service.scheduler` is absent.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemSchedulerClient;

#[async_trait]
impl SystemSchedulerClient for UnavailableSystemSchedulerClient {
    async fn register_job(
        &self,
        command: SchedulerRegisterJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            "sdk scheduler client unavailable for register_job"
        );
        Err(unavailable_error())
    }

    async fn update_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk scheduler client unavailable for update_job");
        Err(unavailable_error())
    }

    async fn pause_job(&self, command: SchedulerJobCommand) -> MacacaResult<SchedulerCommandResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk scheduler client unavailable for pause_job");
        Err(unavailable_error())
    }

    async fn resume_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk scheduler client unavailable for resume_job");
        Err(unavailable_error())
    }

    async fn delete_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk scheduler client unavailable for delete_job");
        Err(unavailable_error())
    }

    async fn trigger_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk scheduler client unavailable for trigger_job");
        Err(unavailable_error())
    }

    async fn get_job(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Option<SchedulerJobDefinition>> {
        info!(trace_id = %command.trace.trace_id, "sdk scheduler client unavailable for get_job");
        Ok(None)
    }

    async fn list_jobs(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Vec<SchedulerJobDefinition>> {
        info!(trace_id = %command.trace.trace_id, "sdk scheduler client unavailable for list_jobs");
        Ok(Vec::new())
    }

    async fn get_run(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Option<SchedulerRunSummary>> {
        info!(trace_id = %command.trace.trace_id, "sdk scheduler client unavailable for get_run");
        Ok(None)
    }

    async fn list_runs(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Vec<SchedulerRunSummary>> {
        info!(trace_id = %command.trace.trace_id, "sdk scheduler client unavailable for list_runs");
        Ok(Vec::new())
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<SchedulerServiceSnapshot> {
        info!(trace_id = %trace.trace_id, "sdk scheduler client returning unavailable health");
        Ok(SchedulerServiceSnapshot::unavailable(
            "Scheduler service is unavailable",
        ))
    }

    async fn snapshot(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk scheduler client returning unavailable snapshot");
        Ok(SchedulerServiceSnapshot::unavailable(
            "Scheduler service is unavailable",
        ))
    }
}

/// Runtime-backed Scheduler client implemented over generic service calls.
#[derive(Clone)]
pub struct ServiceBackedSchedulerClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedSchedulerClient {
    /// Create a service-backed client from a generic service dispatcher.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemSchedulerClient for ServiceBackedSchedulerClient {
    async fn register_job(
        &self,
        command: SchedulerRegisterJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        call(&self.service, SCHEDULER_REGISTER_JOB_COMMAND, command.trace.clone(), command).await
    }

    async fn update_job(&self, command: SchedulerJobCommand) -> MacacaResult<SchedulerCommandResult> {
        call(&self.service, SCHEDULER_UPDATE_JOB_COMMAND, command.trace.clone(), command).await
    }

    async fn pause_job(&self, command: SchedulerJobCommand) -> MacacaResult<SchedulerCommandResult> {
        call(&self.service, SCHEDULER_PAUSE_JOB_COMMAND, command.trace.clone(), command).await
    }

    async fn resume_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        call(&self.service, SCHEDULER_RESUME_JOB_COMMAND, command.trace.clone(), command).await
    }

    async fn delete_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        call(&self.service, SCHEDULER_DELETE_JOB_COMMAND, command.trace.clone(), command).await
    }

    async fn trigger_job(
        &self,
        command: SchedulerJobCommand,
    ) -> MacacaResult<SchedulerCommandResult> {
        call(&self.service, SCHEDULER_TRIGGER_JOB_COMMAND, command.trace.clone(), command).await
    }

    async fn get_job(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Option<SchedulerJobDefinition>> {
        call(&self.service, SCHEDULER_GET_JOB_COMMAND, command.trace.clone(), command).await
    }

    async fn list_jobs(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Vec<SchedulerJobDefinition>> {
        call(&self.service, SCHEDULER_LIST_JOBS_COMMAND, command.trace.clone(), command).await
    }

    async fn get_run(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Option<SchedulerRunSummary>> {
        call(&self.service, SCHEDULER_GET_RUN_COMMAND, command.trace.clone(), command).await
    }

    async fn list_runs(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<Vec<SchedulerRunSummary>> {
        call(&self.service, SCHEDULER_LIST_RUNS_COMMAND, command.trace.clone(), command).await
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<SchedulerServiceSnapshot> {
        call(&self.service, SCHEDULER_HEALTH_COMMAND, trace.clone(), serde_json::json!({})).await
    }

    async fn snapshot(
        &self,
        command: SchedulerQueryCommand,
    ) -> MacacaResult<SchedulerServiceSnapshot> {
        call(&self.service, SCHEDULER_SNAPSHOT_COMMAND, command.trace.clone(), command).await
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
        SCHEDULER_SERVICE_ID,
        command_name,
        serde_json::to_value(payload).map_err(|error| MacacaError::Config(error.to_string()))?,
    )?
    .with_trace(trace);
    let result = service.call_service(&command).await?;
    serde_json::from_value(result.output).map_err(|error| MacacaError::Config(error.to_string()))
}

fn unavailable_error() -> MacacaError {
    MacacaError::Config("Scheduler service is unavailable".into())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use async_trait::async_trait;
    use chrono::Utc;
    use macaca_proto::{
        AutonomyScope, KernelServiceId, SchedulerJobId, SchedulerRunState,
        SchedulerScheduleSpec, SchedulerTargetCommand, ServiceCommandName, ServiceTargetCommand,
    };

    use super::*;
    use crate::service_client::{
        ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    };

    struct EchoSchedulerServiceClient;

    #[async_trait]
    impl SystemServiceClient for EchoSchedulerServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![SCHEDULER_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            assert_eq!(command.service_id, SCHEDULER_SERVICE_ID);
            assert_eq!(command.command_name, SCHEDULER_REGISTER_JOB_COMMAND);
            let typed: SchedulerRegisterJobCommand =
                serde_json::from_value(command.payload.clone()).unwrap();
            Ok(ServiceCallResult {
                service_id: command.service_id.clone(),
                output: serde_json::to_value(SchedulerCommandResult {
                    job_id: Some(SchedulerJobId::new("job.sdk").unwrap()),
                    run_id: None,
                    lifecycle: Some(typed.definition.lifecycle),
                    run_state: Some(SchedulerRunState::Queued),
                    accepted: true,
                    error: None,
                    trace: typed.trace,
                    audit_id: Some("audit.scheduler.sdk".into()),
                    metadata: BTreeMap::new(),
                })
                .unwrap(),
            })
        }
    }

    fn register_command() -> SchedulerRegisterJobCommand {
        let target = SchedulerTargetCommand::Service(ServiceTargetCommand {
            service_id: KernelServiceId::new("service.generic"),
            command_name: ServiceCommandName::new("generic.dispatch"),
            payload_ref: None,
            metadata: BTreeMap::new(),
        });
        let definition = SchedulerJobDefinition::new(
            AutonomyScope::global(),
            SchedulerScheduleSpec::At { run_at: Utc::now() },
            target,
        )
        .unwrap();
        SchedulerRegisterJobCommand::new(TraceContext::new("trace-sdk-scheduler"), definition)
            .unwrap()
    }

    #[tokio::test]
    async fn service_backed_scheduler_client_dispatches_register_command() {
        let client = ServiceBackedSchedulerClient::new(Arc::new(EchoSchedulerServiceClient));
        let result = client.register_job(register_command()).await.unwrap();
        assert!(result.accepted);
        assert_eq!(result.job_id.unwrap().as_str(), "job.sdk");
    }

    #[tokio::test]
    async fn unavailable_scheduler_client_fails_closed_for_mutation() {
        let client = UnavailableSystemSchedulerClient;
        let err = client.register_job(register_command()).await.unwrap_err();
        assert!(err.to_string().contains("Scheduler service is unavailable"));
    }
}
