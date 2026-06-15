//! Host-owned Context service client facade.
//!
//! This module is temporarily hosted in `macaca-host-composition` because the
//! Context service command DTOs still include runtime-host/context-service
//! strategy types. The Adapter still dispatches through the generic SDK
//! `SystemServiceClient`, so every call remains trace-bearing and auditable.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{MacacaError, MacacaResult};
use tracing::{info, warn};

use crate::runtime_host::{
    ContextActiveRecallCommand, ContextActiveRecallServiceResult, ContextAssembleCommand,
    ContextAssembleServiceResult, ContextEngineInventoryCommand, ContextProviderInventoryCommand,
    ContextServiceSnapshot, ContextServiceSnapshotCommand, CONTEXT_ACTIVE_RECALL_COMMAND,
    CONTEXT_ASSEMBLE_COMMAND, CONTEXT_ENGINE_INVENTORY_COMMAND, CONTEXT_PROVIDER_INVENTORY_COMMAND,
    CONTEXT_SERVICE_ID, CONTEXT_SNAPSHOT_COMMAND,
};

/// Focused Context client consumed by host composition and Web shell adapters.
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
        warn!(trace_id = %command.trace.trace_id, "host context client unavailable for assemble");
        Err(MacacaError::Config("Context service is unavailable".into()))
    }

    async fn active_recall(
        &self,
        command: ContextActiveRecallCommand,
    ) -> MacacaResult<ContextActiveRecallServiceResult> {
        warn!(trace_id = %command.trace.trace_id, "host context client unavailable for active recall");
        Err(MacacaError::Config("Context service is unavailable".into()))
    }

    async fn provider_inventory(
        &self,
        command: ContextProviderInventoryCommand,
    ) -> MacacaResult<serde_json::Value> {
        info!(trace_id = %command.trace.trace_id, "host context client unavailable for provider inventory");
        Ok(serde_json::json!({"providers": [], "status": "unavailable"}))
    }

    async fn engine_inventory(
        &self,
        command: ContextEngineInventoryCommand,
    ) -> MacacaResult<serde_json::Value> {
        info!(trace_id = %command.trace.trace_id, "host context client unavailable for engine inventory");
        Ok(serde_json::json!({"engines": [], "status": "unavailable"}))
    }

    async fn snapshot(
        &self,
        command: ContextServiceSnapshotCommand,
    ) -> MacacaResult<ContextServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "host context client unavailable for snapshot");
        Err(MacacaError::Config("Context service is unavailable".into()))
    }
}

/// Runtime-backed Context client implemented over the generic SDK service client.
#[derive(Clone)]
pub struct ServiceBackedContextClient {
    service: Arc<dyn macaca_sdk::SystemServiceClient>,
}

impl ServiceBackedContextClient {
    pub fn new(service: Arc<dyn macaca_sdk::SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemContextClient for ServiceBackedContextClient {
    async fn assemble(
        &self,
        command: ContextAssembleCommand,
    ) -> MacacaResult<ContextAssembleServiceResult> {
        let trace = command.trace.clone();
        call(&*self.service, CONTEXT_ASSEMBLE_COMMAND, trace, command).await
    }

    async fn active_recall(
        &self,
        command: ContextActiveRecallCommand,
    ) -> MacacaResult<ContextActiveRecallServiceResult> {
        let trace = command.trace.clone();
        call(
            &*self.service,
            CONTEXT_ACTIVE_RECALL_COMMAND,
            trace,
            command,
        )
        .await
    }

    async fn provider_inventory(
        &self,
        command: ContextProviderInventoryCommand,
    ) -> MacacaResult<serde_json::Value> {
        let trace = command.trace.clone();
        call(
            &*self.service,
            CONTEXT_PROVIDER_INVENTORY_COMMAND,
            trace,
            command,
        )
        .await
    }

    async fn engine_inventory(
        &self,
        command: ContextEngineInventoryCommand,
    ) -> MacacaResult<serde_json::Value> {
        let trace = command.trace.clone();
        call(
            &*self.service,
            CONTEXT_ENGINE_INVENTORY_COMMAND,
            trace,
            command,
        )
        .await
    }

    async fn snapshot(
        &self,
        command: ContextServiceSnapshotCommand,
    ) -> MacacaResult<ContextServiceSnapshot> {
        let trace = command.trace.clone();
        call(&*self.service, CONTEXT_SNAPSHOT_COMMAND, trace, command).await
    }
}

async fn call<T, R>(
    service: &dyn macaca_sdk::SystemServiceClient,
    command_name: &str,
    trace: macaca_proto::TraceContext,
    command: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    // The generic service envelope carries the same trace as the typed command
    // so service-runtime trace-required middleware can admit the request without
    // peeking into provider-owned payloads.
    let result = service
        .call_service(
            &macaca_sdk::ServiceCallCommand::new(
                CONTEXT_SERVICE_ID,
                command_name,
                serde_json::to_value(command)?,
            )?
            .with_trace(trace),
        )
        .await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}
