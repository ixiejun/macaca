//! SDK Memory client facade for Route C S5.
//!
//! The client keeps upper layers scoped and service-oriented.  It forwards
//! typed memory commands through the generic service client instead of exposing
//! vector stores, memory runtimes, or backend factories to Web/CLI/framework.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_memory::{
    MemoryForgetCommand, MemoryGetCommand, MemoryGetResult, MemoryPrefetchCommand,
    MemoryRecallCommand, MemoryRecallResult, MemoryRememberCommand, MemoryRememberResult,
    MemoryServiceSnapshot, MemoryServiceSnapshotCommand, MemoryStatusCommand, MemoryStatusReport,
    MEMORY_FORGET_COMMAND, MEMORY_GET_COMMAND, MEMORY_PREFETCH_COMMAND, MEMORY_RECALL_COMMAND,
    MEMORY_REMEMBER_COMMAND, MEMORY_SERVICE_ID, MEMORY_SNAPSHOT_COMMAND, MEMORY_STATUS_COMMAND,
};
use macaca_proto::{MacacaError, MacacaResult};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Memory client consumed by serviceized callers.
#[async_trait]
pub trait SystemMemoryClient: Send + Sync {
    async fn remember(&self, command: MemoryRememberCommand) -> MacacaResult<MemoryRememberResult>;
    async fn recall(&self, command: MemoryRecallCommand) -> MacacaResult<MemoryRecallResult>;
    async fn prefetch(&self, command: MemoryPrefetchCommand) -> MacacaResult<MemoryRecallResult>;
    async fn get(&self, command: MemoryGetCommand) -> MacacaResult<MemoryGetResult>;
    async fn forget(&self, command: MemoryForgetCommand) -> MacacaResult<()>;
    async fn status(&self, command: MemoryStatusCommand) -> MacacaResult<MemoryStatusReport>;
    async fn snapshot(
        &self,
        command: MemoryServiceSnapshotCommand,
    ) -> MacacaResult<MemoryServiceSnapshot>;
}

/// Null-object Memory client used before a runtime is wired.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemMemoryClient;

#[async_trait]
impl SystemMemoryClient for UnavailableSystemMemoryClient {
    async fn remember(&self, command: MemoryRememberCommand) -> MacacaResult<MemoryRememberResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for remember");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }

    async fn recall(&self, command: MemoryRecallCommand) -> MacacaResult<MemoryRecallResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for recall");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }

    async fn prefetch(&self, command: MemoryPrefetchCommand) -> MacacaResult<MemoryRecallResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for prefetch");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }

    async fn get(&self, command: MemoryGetCommand) -> MacacaResult<MemoryGetResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for get");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }

    async fn forget(&self, command: MemoryForgetCommand) -> MacacaResult<()> {
        warn!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for forget");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }

    async fn status(&self, command: MemoryStatusCommand) -> MacacaResult<MemoryStatusReport> {
        info!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for status");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }

    async fn snapshot(
        &self,
        command: MemoryServiceSnapshotCommand,
    ) -> MacacaResult<MemoryServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk memory client unavailable for snapshot");
        Err(MacacaError::Config("Memory service is unavailable".into()))
    }
}

/// Runtime-backed Memory client implemented over the generic SDK service client.
#[derive(Clone)]
pub struct ServiceBackedMemoryClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedMemoryClient {
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemMemoryClient for ServiceBackedMemoryClient {
    async fn remember(&self, command: MemoryRememberCommand) -> MacacaResult<MemoryRememberResult> {
        call(&*self.service, MEMORY_REMEMBER_COMMAND, command).await
    }

    async fn recall(&self, command: MemoryRecallCommand) -> MacacaResult<MemoryRecallResult> {
        call(&*self.service, MEMORY_RECALL_COMMAND, command).await
    }

    async fn prefetch(&self, command: MemoryPrefetchCommand) -> MacacaResult<MemoryRecallResult> {
        call(&*self.service, MEMORY_PREFETCH_COMMAND, command).await
    }

    async fn get(&self, command: MemoryGetCommand) -> MacacaResult<MemoryGetResult> {
        call(&*self.service, MEMORY_GET_COMMAND, command).await
    }

    async fn forget(&self, command: MemoryForgetCommand) -> MacacaResult<()> {
        let _: serde_json::Value = call(&*self.service, MEMORY_FORGET_COMMAND, command).await?;
        Ok(())
    }

    async fn status(&self, command: MemoryStatusCommand) -> MacacaResult<MemoryStatusReport> {
        call(&*self.service, MEMORY_STATUS_COMMAND, command).await
    }

    async fn snapshot(
        &self,
        command: MemoryServiceSnapshotCommand,
    ) -> MacacaResult<MemoryServiceSnapshot> {
        call(&*self.service, MEMORY_SNAPSHOT_COMMAND, command).await
    }
}

async fn call<T, R>(
    service: &dyn SystemServiceClient,
    command_name: &str,
    command: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let result = service
        .call_service(&ServiceCallCommand::new(
            MEMORY_SERVICE_ID,
            command_name,
            serde_json::to_value(command)?,
        )?)
        .await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}
