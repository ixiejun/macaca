//! Contract tests for the Scheduler SDK client Facade.
//!
//! These tests verify that the service-backed Adapter forwards typed commands
//! through `SystemServiceClient` and that the Null Object implementation
//! fails closed when `service.scheduler` is absent.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use macaca_proto::{
    AutonomyScope, KernelServiceId, SchedulerCommandResult, SchedulerJobDefinition,
    SchedulerJobId, SchedulerRegisterJobCommand, SchedulerRunState, SchedulerScheduleSpec,
    SchedulerTargetCommand, ServiceCommandName, ServiceTargetCommand, TraceContext,
    SCHEDULER_REGISTER_JOB_COMMAND, SCHEDULER_SERVICE_ID,
};

use super::{
    ServiceBackedSchedulerClient, SystemSchedulerClient, UnavailableSystemSchedulerClient,
};
use crate::service_client::{
    ServiceCallCommand, ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    SystemServiceClient,
};

struct EchoSchedulerServiceClient;

#[async_trait]
impl SystemServiceClient for EchoSchedulerServiceClient {
    async fn inspect_services(
        &self,
        command: &ServiceInspectionCommand,
    ) -> macaca_proto::MacacaResult<ServiceInspectionResult> {
        Ok(ServiceInspectionResult {
            scope: command.scope.clone(),
            services: vec![SCHEDULER_SERVICE_ID.into()],
        })
    }

    async fn call_service(
        &self,
        command: &ServiceCallCommand,
    ) -> macaca_proto::MacacaResult<ServiceCallResult> {
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
    SchedulerRegisterJobCommand::new(TraceContext::new("trace-sdk-scheduler"), definition).unwrap()
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
