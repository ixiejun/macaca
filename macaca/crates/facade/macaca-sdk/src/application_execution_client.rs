//! Focused SDK facade for the application execution protocol platform.
//!
//! The SDK is a caller-facing Facade: it builds typed service commands and
//! decodes typed results, but it never constructs runtime providers or shell
//! state. Runtime-host injects [`SystemServiceClient`] when available; the
//! Null Object client returns explicit structured unavailable results.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    application_execution_service_descriptor, AppendExecutionEventCommand,
    ApplicationExecutionCommandStatus, ApplicationExecutionControlCommand,
    ApplicationExecutionControlResult, ApplicationExecutionCurrentState, ApplicationExecutionError,
    ApplicationExecutionEventEnvelope, ApplicationExecutionProviderDescriptor,
    ApplicationExecutionReplayRequest, ApplicationExecutionReplayResult,
    ApplicationExecutionSnapshot, MacacaError, MacacaResult, ReportExecutionCompletionCommand,
    ReportExecutionFailureCommand, ReportExecutionHeartbeatCommand, ReportExecutionSnapshotCommand,
    RequestExecutionApprovalCommand, ServiceDescriptor, StartApplicationExecutionCommand,
    StartApplicationExecutionResult, TraceContext, APPLICATION_EXECUTION_CONTROL_COMMAND,
    APPLICATION_EXECUTION_CURRENT_STATE_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_APPEND_EVENT_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_APPROVAL_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_COMPLETION_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_FAILURE_COMMAND, APPLICATION_EXECUTION_GATEWAY_HEARTBEAT_COMMAND,
    APPLICATION_EXECUTION_GATEWAY_SNAPSHOT_COMMAND, APPLICATION_EXECUTION_PROVIDER_HEALTH_COMMAND,
    APPLICATION_EXECUTION_REPLAY_COMMAND, APPLICATION_EXECUTION_SERVICE_ID,
    APPLICATION_EXECUTION_SNAPSHOT_COMMAND, APPLICATION_EXECUTION_START_COMMAND,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// SDK-facing contract that keeps provider selection, gateway ingress, and
/// execution loops behind service boundaries.
#[async_trait]
pub trait SystemApplicationExecutionClient: Send + Sync {
    fn descriptor(&self) -> &ServiceDescriptor;
    async fn start_execution(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> MacacaResult<StartApplicationExecutionResult>;
    async fn send_control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> MacacaResult<ApplicationExecutionControlResult>;
    async fn replay_events(
        &self,
        request: ApplicationExecutionReplayRequest,
    ) -> MacacaResult<ApplicationExecutionReplayResult>;

    async fn query_current_state(
        &self,
        trace: TraceContext,
        session_id: String,
        run_id: Option<String>,
    ) -> MacacaResult<ApplicationExecutionCurrentState>;
    async fn provider_health(
        &self,
        trace: TraceContext,
    ) -> MacacaResult<Vec<ApplicationExecutionProviderDescriptor>>;
    async fn snapshot(&self, trace: TraceContext) -> MacacaResult<ApplicationExecutionSnapshot>;
    async fn append_gateway_event(
        &self,
        command: AppendExecutionEventCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope>;
    async fn report_gateway_heartbeat(
        &self,
        command: ReportExecutionHeartbeatCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope>;
    async fn report_gateway_snapshot(
        &self,
        command: ReportExecutionSnapshotCommand,
    ) -> MacacaResult<ApplicationExecutionSnapshot>;

    async fn request_gateway_approval(
        &self,
        command: RequestExecutionApprovalCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope>;

    async fn report_gateway_completion(
        &self,
        command: ReportExecutionCompletionCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope>;

    async fn report_gateway_failure(
        &self,
        command: ReportExecutionFailureCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope>;
}

/// Null Object application execution client.
#[derive(Debug, Clone)]
pub struct UnavailableSystemApplicationExecutionClient {
    descriptor: ServiceDescriptor,
}

impl Default for UnavailableSystemApplicationExecutionClient {
    fn default() -> Self {
        Self {
            descriptor: application_execution_service_descriptor(),
        }
    }
}

impl UnavailableSystemApplicationExecutionClient {
    /// Create the SDK Null Object client used before runtime-host bootstrap.
    pub fn new() -> Self {
        Self::default()
    }

    fn unavailable_error(
        &self,
        operation: impl Into<String>,
        reason: impl Into<String>,
    ) -> ApplicationExecutionError {
        let mut error = ApplicationExecutionError::unavailable(operation, reason);
        error.layer = "macaca-sdk".into();
        error
    }
}

#[async_trait]
impl SystemApplicationExecutionClient for UnavailableSystemApplicationExecutionClient {
    fn descriptor(&self) -> &ServiceDescriptor {
        &self.descriptor
    }

    async fn start_execution(
        &self,
        _command: StartApplicationExecutionCommand,
    ) -> MacacaResult<StartApplicationExecutionResult> {
        warn!("sdk application execution client returning structured unavailable start result");
        Ok(StartApplicationExecutionResult::unavailable(
            APPLICATION_EXECUTION_START_COMMAND,
            "service.application_execution is unavailable through the SDK Null Object client",
        ))
    }

    async fn send_control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> MacacaResult<ApplicationExecutionControlResult> {
        warn!(
            command = ?command.command,
            trace_id = %command.trace.trace_id,
            "sdk application execution client returning structured unavailable control result"
        );
        Ok(ApplicationExecutionControlResult {
            status: ApplicationExecutionCommandStatus::Unavailable,
            scope: command.scope,
            provider_id: None,
            provider_kind: macaca_proto::ApplicationExecutionProviderKind::Unavailable,
            event_cursor: None,
            error: Some(self.unavailable_error(
                APPLICATION_EXECUTION_CONTROL_COMMAND,
                "service.application_execution is unavailable through the SDK Null Object client",
            )),
        })
    }

    async fn replay_events(
        &self,
        _request: ApplicationExecutionReplayRequest,
    ) -> MacacaResult<ApplicationExecutionReplayResult> {
        info!("sdk application execution client returning empty unavailable replay");
        Ok(ApplicationExecutionReplayResult {
            events: Vec::new(),
            next_cursor: None,
            current_state: None,
        })
    }

    async fn query_current_state(
        &self,
        trace: TraceContext,
        session_id: String,
        run_id: Option<String>,
    ) -> MacacaResult<ApplicationExecutionCurrentState> {
        let scope = macaca_proto::ApplicationExecutionScope::new(
            macaca_proto::ApplicationId::new(),
            session_id,
            run_id.unwrap_or_else(|| "unavailable-run".into()),
            "sdk-null-object",
        )?;
        Ok(ApplicationExecutionCurrentState {
            scope,
            lifecycle_state: macaca_proto::ApplicationExecutionLifecycleState::Failed,
            provider_id: None,
            provider_kind: macaca_proto::ApplicationExecutionProviderKind::Unavailable,
            latest_heartbeat_at: None,
            pending_approval_refs: Vec::new(),
            active_control_ids: Vec::new(),
            latest_checkpoint_ref: None,
            step_summaries: vec![format!(
                "service.application_execution unavailable for trace {}",
                trace.trace_id
            )],
            terminal_result: None,
            terminal_error: Some(self.unavailable_error(
                APPLICATION_EXECUTION_CURRENT_STATE_COMMAND,
                "service.application_execution is unavailable through the SDK Null Object client",
            )),
            replay_cursor: None,
        })
    }

    async fn provider_health(
        &self,
        _trace: TraceContext,
    ) -> MacacaResult<Vec<ApplicationExecutionProviderDescriptor>> {
        Ok(Vec::new())
    }

    async fn snapshot(&self, trace: TraceContext) -> MacacaResult<ApplicationExecutionSnapshot> {
        let scope = macaca_proto::ApplicationExecutionScope::new(
            macaca_proto::ApplicationId::new(),
            trace.session_id.clone().unwrap_or_else(|| "system".into()),
            "unavailable-run",
            "sdk-null-object",
        )?;
        Ok(ApplicationExecutionSnapshot {
            scope,
            lifecycle_state: macaca_proto::ApplicationExecutionLifecycleState::Failed,
            provider_id: None,
            provider_kind: macaca_proto::ApplicationExecutionProviderKind::Unavailable,
            latest_event_cursor: None,
            latest_checkpoint_ref: None,
            metadata: [(
                "reason".to_string(),
                "service.application_execution is unavailable through the SDK Null Object client"
                    .to_string(),
            )]
            .into_iter()
            .collect(),
        })
    }

    async fn append_gateway_event(
        &self,
        _command: AppendExecutionEventCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope> {
        Err(MacacaError::Config(
            "service.application_execution gateway append_event is unavailable".into(),
        ))
    }

    async fn report_gateway_heartbeat(
        &self,
        _command: ReportExecutionHeartbeatCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope> {
        Err(MacacaError::Config(
            "service.application_execution gateway heartbeat is unavailable".into(),
        ))
    }

    async fn report_gateway_snapshot(
        &self,
        _command: ReportExecutionSnapshotCommand,
    ) -> MacacaResult<ApplicationExecutionSnapshot> {
        self.snapshot(TraceContext::new(
            "application-execution-unavailable-snapshot",
        ))
        .await
    }

    async fn request_gateway_approval(
        &self,
        _command: RequestExecutionApprovalCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope> {
        Err(MacacaError::Config(
            "service.application_execution gateway approval is unavailable".into(),
        ))
    }

    async fn report_gateway_completion(
        &self,
        _command: ReportExecutionCompletionCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope> {
        Err(MacacaError::Config(
            "service.application_execution gateway completion is unavailable".into(),
        ))
    }

    async fn report_gateway_failure(
        &self,
        _command: ReportExecutionFailureCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope> {
        Err(MacacaError::Config(
            "service.application_execution gateway failure is unavailable".into(),
        ))
    }
}

/// Service-backed SDK client implemented over the generic service facade.
#[derive(Clone)]
pub struct ServiceBackedApplicationExecutionClient {
    descriptor: ServiceDescriptor,
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedApplicationExecutionClient {
    /// Create a client over an injected dispatcher without constructing providers.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self {
            descriptor: application_execution_service_descriptor(),
            service,
        }
    }
}

#[async_trait]
impl SystemApplicationExecutionClient for ServiceBackedApplicationExecutionClient {
    fn descriptor(&self) -> &ServiceDescriptor {
        &self.descriptor
    }

    async fn start_execution(
        &self,
        command: StartApplicationExecutionCommand,
    ) -> MacacaResult<StartApplicationExecutionResult> {
        call_service(
            &self.service,
            APPLICATION_EXECUTION_START_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn send_control(
        &self,
        command: ApplicationExecutionControlCommand,
    ) -> MacacaResult<ApplicationExecutionControlResult> {
        call_service(
            &self.service,
            APPLICATION_EXECUTION_CONTROL_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn replay_events(
        &self,
        request: ApplicationExecutionReplayRequest,
    ) -> MacacaResult<ApplicationExecutionReplayResult> {
        call_service(
            &self.service,
            APPLICATION_EXECUTION_REPLAY_COMMAND,
            request.trace.clone(),
            request,
        )
        .await
    }

    async fn query_current_state(
        &self,
        trace: TraceContext,
        session_id: String,
        run_id: Option<String>,
    ) -> MacacaResult<ApplicationExecutionCurrentState> {
        let payload = serde_json::json!({
            "session_id": session_id,
            "run_id": run_id,
        });
        call_service(
            &self.service,
            APPLICATION_EXECUTION_CURRENT_STATE_COMMAND,
            trace,
            payload,
        )
        .await
    }

    async fn provider_health(
        &self,
        trace: TraceContext,
    ) -> MacacaResult<Vec<ApplicationExecutionProviderDescriptor>> {
        call_service(
            &self.service,
            APPLICATION_EXECUTION_PROVIDER_HEALTH_COMMAND,
            trace,
            Value::Null,
        )
        .await
    }

    async fn snapshot(&self, trace: TraceContext) -> MacacaResult<ApplicationExecutionSnapshot> {
        call_service(
            &self.service,
            APPLICATION_EXECUTION_SNAPSHOT_COMMAND,
            trace,
            Value::Null,
        )
        .await
    }

    async fn append_gateway_event(
        &self,
        command: AppendExecutionEventCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope> {
        call_service(
            &self.service,
            APPLICATION_EXECUTION_GATEWAY_APPEND_EVENT_COMMAND,
            command.event.trace.clone(),
            command,
        )
        .await
    }

    async fn report_gateway_heartbeat(
        &self,
        command: ReportExecutionHeartbeatCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope> {
        call_service(
            &self.service,
            APPLICATION_EXECUTION_GATEWAY_HEARTBEAT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn report_gateway_snapshot(
        &self,
        command: ReportExecutionSnapshotCommand,
    ) -> MacacaResult<ApplicationExecutionSnapshot> {
        call_service(
            &self.service,
            APPLICATION_EXECUTION_GATEWAY_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn request_gateway_approval(
        &self,
        command: RequestExecutionApprovalCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope> {
        call_service(
            &self.service,
            APPLICATION_EXECUTION_GATEWAY_APPROVAL_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn report_gateway_completion(
        &self,
        command: ReportExecutionCompletionCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope> {
        call_service(
            &self.service,
            APPLICATION_EXECUTION_GATEWAY_COMPLETION_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn report_gateway_failure(
        &self,
        command: ReportExecutionFailureCommand,
    ) -> MacacaResult<ApplicationExecutionEventEnvelope> {
        call_service(
            &self.service,
            APPLICATION_EXECUTION_GATEWAY_FAILURE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

async fn call_service<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: Serialize,
    R: DeserializeOwned,
{
    let command = ServiceCallCommand::new(
        APPLICATION_EXECUTION_SERVICE_ID,
        command_name,
        serde_json::to_value(payload)?,
    )?
    .with_trace(trace);
    let result = service.call_service(&command).await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}
