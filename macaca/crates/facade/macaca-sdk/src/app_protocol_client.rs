//! Focused SDK facade for `service.app_protocol`.
//!
//! The app protocol client keeps shells and application adapters behind the
//! SDK Facade pattern.  It forwards typed protocol commands to the owning
//! system service through `SystemServiceClient`; provider construction,
//! socket listeners, websocket tasks, and queue implementations remain in
//! Runtime Host composition roots.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::workbench::app_protocol::*;
use macaca_proto::{MacacaError, MacacaResult, TraceContext, WorkbenchCommandResult};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

#[async_trait]
pub trait SystemAppProtocolClient: Send + Sync {
    async fn initialize(
        &self,
        command: AppProtocolCommand<AppProtocolInitializeRequest>,
    ) -> MacacaResult<AppProtocolCommandResult>;

    async fn create_subscription(
        &self,
        command: AppProtocolCommand<AppProtocolSubscriptionCreateRequest>,
    ) -> MacacaResult<AppProtocolCommandResult>;

    async fn close_subscription(
        &self,
        command: AppProtocolCommand<AppProtocolSubscriptionCloseRequest>,
    ) -> MacacaResult<AppProtocolCommandResult>;

    async fn emit_notification(
        &self,
        command: AppProtocolCommand<AppProtocolNotificationEmitRequest>,
    ) -> MacacaResult<AppProtocolCommandResult>;

    async fn receive_json_rpc(
        &self,
        command: AppProtocolCommand<AppProtocolJsonRpcReceiveRequest>,
    ) -> MacacaResult<AppProtocolCommandResult>;

    async fn health(
        &self,
        command: AppProtocolCommand<AppProtocolHealthReadRequest>,
    ) -> MacacaResult<AppProtocolCommandResult>;
}

#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemAppProtocolClient;

#[async_trait]
impl SystemAppProtocolClient for UnavailableSystemAppProtocolClient {
    async fn initialize(
        &self,
        command: AppProtocolCommand<AppProtocolInitializeRequest>,
    ) -> MacacaResult<AppProtocolCommandResult> {
        unavailable(command.trace, "App protocol service is unavailable")
    }

    async fn create_subscription(
        &self,
        command: AppProtocolCommand<AppProtocolSubscriptionCreateRequest>,
    ) -> MacacaResult<AppProtocolCommandResult> {
        unavailable(command.trace, "App protocol service is unavailable")
    }

    async fn close_subscription(
        &self,
        command: AppProtocolCommand<AppProtocolSubscriptionCloseRequest>,
    ) -> MacacaResult<AppProtocolCommandResult> {
        unavailable(command.trace, "App protocol service is unavailable")
    }

    async fn emit_notification(
        &self,
        command: AppProtocolCommand<AppProtocolNotificationEmitRequest>,
    ) -> MacacaResult<AppProtocolCommandResult> {
        unavailable(command.trace, "App protocol service is unavailable")
    }

    async fn receive_json_rpc(
        &self,
        command: AppProtocolCommand<AppProtocolJsonRpcReceiveRequest>,
    ) -> MacacaResult<AppProtocolCommandResult> {
        unavailable(command.trace, "App protocol service is unavailable")
    }

    async fn health(
        &self,
        command: AppProtocolCommand<AppProtocolHealthReadRequest>,
    ) -> MacacaResult<AppProtocolCommandResult> {
        unavailable(command.trace, "App protocol service is unavailable")
    }
}

#[derive(Clone)]
pub struct ServiceBackedAppProtocolClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedAppProtocolClient {
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemAppProtocolClient for ServiceBackedAppProtocolClient {
    async fn initialize(
        &self,
        command: AppProtocolCommand<AppProtocolInitializeRequest>,
    ) -> MacacaResult<AppProtocolCommandResult> {
        call(
            &self.service,
            INITIALIZE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn create_subscription(
        &self,
        command: AppProtocolCommand<AppProtocolSubscriptionCreateRequest>,
    ) -> MacacaResult<AppProtocolCommandResult> {
        call(
            &self.service,
            SUBSCRIPTION_CREATE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn close_subscription(
        &self,
        command: AppProtocolCommand<AppProtocolSubscriptionCloseRequest>,
    ) -> MacacaResult<AppProtocolCommandResult> {
        call(
            &self.service,
            SUBSCRIPTION_CLOSE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn emit_notification(
        &self,
        command: AppProtocolCommand<AppProtocolNotificationEmitRequest>,
    ) -> MacacaResult<AppProtocolCommandResult> {
        call(
            &self.service,
            NOTIFICATION_EMIT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn receive_json_rpc(
        &self,
        command: AppProtocolCommand<AppProtocolJsonRpcReceiveRequest>,
    ) -> MacacaResult<AppProtocolCommandResult> {
        call(
            &self.service,
            JSON_RPC_RECEIVE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn health(
        &self,
        command: AppProtocolCommand<AppProtocolHealthReadRequest>,
    ) -> MacacaResult<AppProtocolCommandResult> {
        call(
            &self.service,
            HEALTH_READ_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

fn unavailable(
    trace: TraceContext,
    reason: impl Into<String>,
) -> MacacaResult<AppProtocolCommandResult> {
    let reason = reason.into();
    warn!(trace_id = %trace.trace_id, reason = %reason, "sdk app protocol client unavailable");
    Ok(WorkbenchCommandResult::unavailable(trace, reason))
}

async fn call<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: Serialize,
    R: DeserializeOwned,
{
    info!(
        service_id = SERVICE_ID,
        command = command_name,
        trace_id = %trace.trace_id,
        "sdk app protocol client dispatching service command"
    );
    let command =
        ServiceCallCommand::new(SERVICE_ID, command_name, serde_json::to_value(payload)?)?
            .with_trace(trace);
    let result = service.call_service(&command).await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{ApplicationId, WorkbenchCommandScope, WorkbenchCommandStatus};

    #[tokio::test]
    async fn unavailable_app_protocol_client_returns_structured_state() {
        let scope =
            WorkbenchCommandScope::new(ApplicationId::from_name("generic-app"), "session").unwrap();
        let command = AppProtocolCommand::new(
            scope,
            TraceContext::new("trace-sdk-app-protocol"),
            "initialize",
            AppProtocolInitializeRequest {
                connection_id: None,
                protocol_version: "app-protocol.v1".into(),
                client: AppProtocolClientMetadata {
                    client_name: "test-client".into(),
                    client_version: None,
                    transport: AppProtocolTransportKind::WebSocket,
                    wants_notifications: true,
                },
                requested_capabilities: Vec::new(),
            },
        )
        .unwrap();

        let result = UnavailableSystemAppProtocolClient
            .initialize(command)
            .await
            .unwrap();

        assert!(matches!(
            result.status,
            WorkbenchCommandStatus::Unavailable { .. }
        ));
    }
}
