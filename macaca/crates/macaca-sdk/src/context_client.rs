//! SDK Context client facade for Route C S5.
//!
//! The Context client is intentionally a thin command facade.  Context engine
//! selection, active recall orchestration, and provider inventory are owned by
//! Context Service implementations, not by Web/CLI/framework presentation code.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_context::{
    ContextActiveRecallCommand, ContextActiveRecallServiceResult, ContextAssembleCommand,
    ContextAssembleServiceResult, ContextEngineInventoryCommand, ContextProviderInventoryCommand,
    ContextServiceSnapshot, ContextServiceSnapshotCommand, CONTEXT_ACTIVE_RECALL_COMMAND,
    CONTEXT_ASSEMBLE_COMMAND, CONTEXT_ENGINE_INVENTORY_COMMAND, CONTEXT_PROVIDER_INVENTORY_COMMAND,
    CONTEXT_SERVICE_ID, CONTEXT_SNAPSHOT_COMMAND,
};
use macaca_proto::{MacacaError, MacacaResult};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Context client consumed by serviceized callers.
#[async_trait]
pub trait SystemContextClient: Send + Sync {
    async fn assemble(
        &self,
        command: ContextAssembleCommand,
    ) -> MacacaResult<ContextAssembleServiceResult>;
    async fn active_recall(
        &self,
        command: ContextActiveRecallCommand,
    ) -> MacacaResult<ContextActiveRecallServiceResult>;
    async fn provider_inventory(
        &self,
        command: ContextProviderInventoryCommand,
    ) -> MacacaResult<serde_json::Value>;
    async fn engine_inventory(
        &self,
        command: ContextEngineInventoryCommand,
    ) -> MacacaResult<serde_json::Value>;
    async fn snapshot(
        &self,
        command: ContextServiceSnapshotCommand,
    ) -> MacacaResult<ContextServiceSnapshot>;
}

/// Null-object Context client used before a runtime is wired.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemContextClient;

#[async_trait]
impl SystemContextClient for UnavailableSystemContextClient {
    async fn assemble(
        &self,
        command: ContextAssembleCommand,
    ) -> MacacaResult<ContextAssembleServiceResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk context client unavailable for assemble");
        Err(MacacaError::Config("Context service is unavailable".into()))
    }

    async fn active_recall(
        &self,
        command: ContextActiveRecallCommand,
    ) -> MacacaResult<ContextActiveRecallServiceResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk context client unavailable for active recall");
        Err(MacacaError::Config("Context service is unavailable".into()))
    }

    async fn provider_inventory(
        &self,
        command: ContextProviderInventoryCommand,
    ) -> MacacaResult<serde_json::Value> {
        info!(trace_id = %command.trace.trace_id, "sdk context client unavailable for provider inventory");
        Ok(serde_json::json!({"providers": [], "status": "unavailable"}))
    }

    async fn engine_inventory(
        &self,
        command: ContextEngineInventoryCommand,
    ) -> MacacaResult<serde_json::Value> {
        info!(trace_id = %command.trace.trace_id, "sdk context client unavailable for engine inventory");
        Ok(serde_json::json!({"engines": [], "status": "unavailable"}))
    }

    async fn snapshot(
        &self,
        command: ContextServiceSnapshotCommand,
    ) -> MacacaResult<ContextServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk context client unavailable for snapshot");
        Err(MacacaError::Config("Context service is unavailable".into()))
    }
}

/// Runtime-backed Context client implemented over the generic SDK service client.
#[derive(Clone)]
pub struct ServiceBackedContextClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedContextClient {
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemContextClient for ServiceBackedContextClient {
    async fn assemble(
        &self,
        command: ContextAssembleCommand,
    ) -> MacacaResult<ContextAssembleServiceResult> {
        call(&*self.service, CONTEXT_ASSEMBLE_COMMAND, command).await
    }

    async fn active_recall(
        &self,
        command: ContextActiveRecallCommand,
    ) -> MacacaResult<ContextActiveRecallServiceResult> {
        call(&*self.service, CONTEXT_ACTIVE_RECALL_COMMAND, command).await
    }

    async fn provider_inventory(
        &self,
        command: ContextProviderInventoryCommand,
    ) -> MacacaResult<serde_json::Value> {
        call(&*self.service, CONTEXT_PROVIDER_INVENTORY_COMMAND, command).await
    }

    async fn engine_inventory(
        &self,
        command: ContextEngineInventoryCommand,
    ) -> MacacaResult<serde_json::Value> {
        call(&*self.service, CONTEXT_ENGINE_INVENTORY_COMMAND, command).await
    }

    async fn snapshot(
        &self,
        command: ContextServiceSnapshotCommand,
    ) -> MacacaResult<ContextServiceSnapshot> {
        call(&*self.service, CONTEXT_SNAPSHOT_COMMAND, command).await
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
            CONTEXT_SERVICE_ID,
            command_name,
            serde_json::to_value(command)?,
        )?)
        .await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}
