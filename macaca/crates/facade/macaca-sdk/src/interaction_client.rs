//! Focused SDK facade for `service.interaction`.
//!
//! Shells and application framework code use this client to read/list
//! Thread/Turn/Item state without reaching into runtime-host providers or
//! presentation memory.  The service-backed implementation is a thin Facade
//! over `SystemServiceClient`; provider construction remains in Runtime Host.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::workbench::interaction::*;
use macaca_proto::{MacacaError, MacacaResult, TraceContext, WorkbenchCommandResult};
use serde::{de::DeserializeOwned, Serialize};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

#[async_trait]
pub trait SystemInteractionClient: Send + Sync {
    async fn start_thread(
        &self,
        command: InteractionCommand<ThreadStartRequest>,
    ) -> MacacaResult<InteractionCommandResult>;
    async fn read_thread(
        &self,
        command: InteractionCommand<ThreadRefRequest>,
    ) -> MacacaResult<InteractionCommandResult>;
    async fn list_threads(
        &self,
        command: InteractionCommand<ThreadListRequest>,
    ) -> MacacaResult<InteractionCommandResult>;
    async fn start_turn(
        &self,
        command: InteractionCommand<TurnStartRequest>,
    ) -> MacacaResult<InteractionCommandResult>;
    async fn append_item(
        &self,
        command: InteractionCommand<ItemAppendRequest>,
    ) -> MacacaResult<InteractionCommandResult>;
    async fn list_items(
        &self,
        command: InteractionCommand<ItemListRequest>,
    ) -> MacacaResult<InteractionCommandResult>;
    async fn snapshot(
        &self,
        command: InteractionCommand<InteractionSnapshotRequest>,
    ) -> MacacaResult<InteractionCommandResult>;
}

#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemInteractionClient;

#[async_trait]
impl SystemInteractionClient for UnavailableSystemInteractionClient {
    async fn start_thread(
        &self,
        command: InteractionCommand<ThreadStartRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        unavailable(command.trace, "Interaction service is unavailable")
    }

    async fn read_thread(
        &self,
        command: InteractionCommand<ThreadRefRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        unavailable(command.trace, "Interaction service is unavailable")
    }

    async fn list_threads(
        &self,
        command: InteractionCommand<ThreadListRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        unavailable(command.trace, "Interaction service is unavailable")
    }

    async fn start_turn(
        &self,
        command: InteractionCommand<TurnStartRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        unavailable(command.trace, "Interaction service is unavailable")
    }

    async fn append_item(
        &self,
        command: InteractionCommand<ItemAppendRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        unavailable(command.trace, "Interaction service is unavailable")
    }

    async fn list_items(
        &self,
        command: InteractionCommand<ItemListRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        unavailable(command.trace, "Interaction service is unavailable")
    }

    async fn snapshot(
        &self,
        command: InteractionCommand<InteractionSnapshotRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        unavailable(command.trace, "Interaction service is unavailable")
    }
}

#[derive(Clone)]
pub struct ServiceBackedInteractionClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedInteractionClient {
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemInteractionClient for ServiceBackedInteractionClient {
    async fn start_thread(
        &self,
        command: InteractionCommand<ThreadStartRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        call(
            &self.service,
            THREAD_START_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn read_thread(
        &self,
        command: InteractionCommand<ThreadRefRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        call(
            &self.service,
            THREAD_READ_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn list_threads(
        &self,
        command: InteractionCommand<ThreadListRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        call(
            &self.service,
            THREAD_LIST_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn start_turn(
        &self,
        command: InteractionCommand<TurnStartRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        call(
            &self.service,
            TURN_START_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn append_item(
        &self,
        command: InteractionCommand<ItemAppendRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        call(
            &self.service,
            ITEM_APPEND_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn list_items(
        &self,
        command: InteractionCommand<ItemListRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        call(
            &self.service,
            ITEM_LIST_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn snapshot(
        &self,
        command: InteractionCommand<InteractionSnapshotRequest>,
    ) -> MacacaResult<InteractionCommandResult> {
        call(
            &self.service,
            SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

fn unavailable(
    trace: TraceContext,
    reason: impl Into<String>,
) -> MacacaResult<InteractionCommandResult> {
    let reason = reason.into();
    warn!(trace_id = %trace.trace_id, reason = %reason, "sdk interaction client unavailable");
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
        "sdk interaction client dispatching service command"
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
    async fn unavailable_interaction_client_returns_structured_state() {
        let scope =
            WorkbenchCommandScope::new(ApplicationId::from_name("generic-app"), "session").unwrap();
        let command = InteractionCommand::new(
            scope,
            TraceContext::new("trace-sdk-interaction"),
            "thread-start",
            ThreadStartRequest { title: None },
        )
        .unwrap();
        let result = UnavailableSystemInteractionClient
            .start_thread(command)
            .await
            .unwrap();

        assert!(matches!(
            result.status,
            WorkbenchCommandStatus::Unavailable { .. }
        ));
    }
}
