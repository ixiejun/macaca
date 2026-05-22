//! SDK Heartbeat Service client for autonomous wake-loop coordination.
//!
//! The heartbeat client is a focused Facade over generic service-runtime calls.
//! It gives upper layers typed wake, cancel, query, health, and snapshot
//! methods while keeping wake coalescing, gate evaluation, dispatch, and
//! provider health inside replaceable `service.heartbeat` implementations.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    HeartbeatCancelWakeCommand, HeartbeatCommandResult, HeartbeatProfileMutationResult,
    HeartbeatQueryCommand, HeartbeatRunSummary, HeartbeatServiceSnapshot,
    HeartbeatUpdateProfileCommand, HeartbeatWakeCommand, MacacaError, MacacaResult, TraceContext,
    HEARTBEAT_CANCEL_WAKE_COMMAND, HEARTBEAT_GET_RUN_COMMAND, HEARTBEAT_HEALTH_COMMAND,
    HEARTBEAT_LIST_RUNS_COMMAND, HEARTBEAT_SERVICE_ID, HEARTBEAT_SNAPSHOT_COMMAND,
    HEARTBEAT_UPDATE_PROFILE_COMMAND, HEARTBEAT_WAKE_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Heartbeat client consumed by SDK users and thin shells.
#[async_trait]
pub trait SystemHeartbeatClient: Send + Sync {
    async fn wake(&self, command: HeartbeatWakeCommand) -> MacacaResult<HeartbeatCommandResult>;
    async fn cancel_wake(
        &self,
        command: HeartbeatCancelWakeCommand,
    ) -> MacacaResult<HeartbeatCommandResult>;
    async fn get_run(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<Option<HeartbeatRunSummary>>;
    async fn list_runs(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<Vec<HeartbeatRunSummary>>;
    async fn health(&self, trace: TraceContext) -> MacacaResult<HeartbeatServiceSnapshot>;
    async fn snapshot(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<HeartbeatServiceSnapshot>;
    async fn update_profile(
        &self,
        command: HeartbeatUpdateProfileCommand,
    ) -> MacacaResult<HeartbeatProfileMutationResult>;
}

/// Null-object Heartbeat client used when `service.heartbeat` is absent.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemHeartbeatClient;

#[async_trait]
impl SystemHeartbeatClient for UnavailableSystemHeartbeatClient {
    async fn wake(&self, command: HeartbeatWakeCommand) -> MacacaResult<HeartbeatCommandResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk heartbeat client unavailable for wake");
        Err(unavailable_error())
    }

    async fn cancel_wake(
        &self,
        command: HeartbeatCancelWakeCommand,
    ) -> MacacaResult<HeartbeatCommandResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk heartbeat client unavailable for cancel_wake");
        Err(unavailable_error())
    }

    async fn get_run(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<Option<HeartbeatRunSummary>> {
        info!(trace_id = %command.trace.trace_id, "sdk heartbeat client unavailable for get_run");
        Ok(None)
    }

    async fn list_runs(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<Vec<HeartbeatRunSummary>> {
        info!(trace_id = %command.trace.trace_id, "sdk heartbeat client unavailable for list_runs");
        Ok(Vec::new())
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<HeartbeatServiceSnapshot> {
        info!(trace_id = %trace.trace_id, "sdk heartbeat client returning unavailable health");
        Ok(HeartbeatServiceSnapshot::unavailable(
            "Heartbeat service is unavailable",
        ))
    }

    async fn snapshot(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<HeartbeatServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk heartbeat client returning unavailable snapshot");
        Ok(HeartbeatServiceSnapshot::unavailable(
            "Heartbeat service is unavailable",
        ))
    }

    async fn update_profile(
        &self,
        command: HeartbeatUpdateProfileCommand,
    ) -> MacacaResult<HeartbeatProfileMutationResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            profile_id = %command.profile_id.as_str(),
            "sdk heartbeat client unavailable for update_profile"
        );
        Err(unavailable_error())
    }
}

/// Runtime-backed Heartbeat client implemented over generic service calls.
#[derive(Clone)]
pub struct ServiceBackedHeartbeatClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedHeartbeatClient {
    /// Create a service-backed client from a generic service dispatcher.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemHeartbeatClient for ServiceBackedHeartbeatClient {
    async fn wake(&self, command: HeartbeatWakeCommand) -> MacacaResult<HeartbeatCommandResult> {
        call(
            &self.service,
            HEARTBEAT_WAKE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn cancel_wake(
        &self,
        command: HeartbeatCancelWakeCommand,
    ) -> MacacaResult<HeartbeatCommandResult> {
        call(
            &self.service,
            HEARTBEAT_CANCEL_WAKE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn get_run(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<Option<HeartbeatRunSummary>> {
        call(
            &self.service,
            HEARTBEAT_GET_RUN_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn list_runs(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<Vec<HeartbeatRunSummary>> {
        call(
            &self.service,
            HEARTBEAT_LIST_RUNS_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<HeartbeatServiceSnapshot> {
        call(
            &self.service,
            HEARTBEAT_HEALTH_COMMAND,
            trace.clone(),
            serde_json::json!({}),
        )
        .await
    }

    async fn snapshot(
        &self,
        command: HeartbeatQueryCommand,
    ) -> MacacaResult<HeartbeatServiceSnapshot> {
        call(
            &self.service,
            HEARTBEAT_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn update_profile(
        &self,
        command: HeartbeatUpdateProfileCommand,
    ) -> MacacaResult<HeartbeatProfileMutationResult> {
        call(
            &self.service,
            HEARTBEAT_UPDATE_PROFILE_COMMAND,
            command.trace.clone(),
            command,
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
        HEARTBEAT_SERVICE_ID,
        command_name,
        serde_json::to_value(payload).map_err(|error| MacacaError::Config(error.to_string()))?,
    )?
    .with_trace(trace);
    let result = service.call_service(&command).await?;
    serde_json::from_value(result.output).map_err(|error| MacacaError::Config(error.to_string()))
}

fn unavailable_error() -> MacacaError {
    MacacaError::Config("Heartbeat service is unavailable".into())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use async_trait::async_trait;
    use macaca_proto::{
        AutonomyScope, HeartbeatRunId, HeartbeatRunState, HeartbeatWakeDisposition,
    };

    use super::*;
    use crate::service_client::{
        ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    };

    struct EchoHeartbeatServiceClient;

    #[async_trait]
    impl SystemServiceClient for EchoHeartbeatServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: vec![HEARTBEAT_SERVICE_ID.into()],
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            assert_eq!(command.service_id, HEARTBEAT_SERVICE_ID);
            assert_eq!(command.command_name, HEARTBEAT_WAKE_COMMAND);
            let typed: HeartbeatWakeCommand =
                serde_json::from_value(command.payload.clone()).unwrap();
            Ok(ServiceCallResult {
                service_id: command.service_id.clone(),
                output: serde_json::to_value(HeartbeatCommandResult {
                    run_id: Some(HeartbeatRunId::new("heartbeat.sdk").unwrap()),
                    state: Some(HeartbeatRunState::Requested),
                    disposition: HeartbeatWakeDisposition::Accepted,
                    gates: Vec::new(),
                    accepted: true,
                    error: None,
                    trace: typed.trace,
                    audit_id: Some("audit.heartbeat.sdk".into()),
                    metadata: BTreeMap::new(),
                })
                .unwrap(),
            })
        }
    }

    fn wake_command() -> HeartbeatWakeCommand {
        HeartbeatWakeCommand::scheduled_tick(
            TraceContext::new("trace-sdk-heartbeat"),
            AutonomyScope::global(),
            "scope.sdk",
        )
        .unwrap()
    }

    #[tokio::test]
    async fn service_backed_heartbeat_client_dispatches_wake_command() {
        let client = ServiceBackedHeartbeatClient::new(Arc::new(EchoHeartbeatServiceClient));
        let result = client.wake(wake_command()).await.unwrap();
        assert!(result.accepted);
        assert_eq!(result.run_id.unwrap().as_str(), "heartbeat.sdk");
    }

    #[tokio::test]
    async fn unavailable_heartbeat_client_fails_closed_for_wake() {
        let client = UnavailableSystemHeartbeatClient;
        let err = client.wake(wake_command()).await.unwrap_err();
        assert!(err.to_string().contains("Heartbeat service is unavailable"));
    }
}
