//! SDK LLM client facade for Route C S5.
//!
//! The SDK owns a caller-friendly Facade, not provider construction.  Clients
//! either translate typed LLM commands into generic service calls or return a
//! structured unavailable error when no runtime dispatcher is installed.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_llm::{
    LlmChatCommand, LlmChatResult, LlmModelSelectionCommand, LlmModelSelectionResult,
    LlmServiceSnapshot, LlmServiceSnapshotCommand, LLM_CHAT_COMMAND, LLM_MODEL_SELECTION_COMMAND,
    LLM_SERVICE_ID, LLM_SNAPSHOT_COMMAND,
};
use macaca_proto::{MacacaError, MacacaResult};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused LLM client consumed by framework, Web, CLI, and applications.
#[async_trait]
pub trait SystemLlmClient: Send + Sync {
    /// Dispatch a provider-neutral chat command.
    async fn chat(&self, command: LlmChatCommand) -> MacacaResult<LlmChatResult>;
    /// Resolve provider-neutral model routing metadata.
    async fn select_model(
        &self,
        command: LlmModelSelectionCommand,
    ) -> MacacaResult<LlmModelSelectionResult>;
    /// Read a sanitized LLM service snapshot.
    async fn snapshot(
        &self,
        command: LlmServiceSnapshotCommand,
    ) -> MacacaResult<LlmServiceSnapshot>;
}

/// Null-object LLM client used when no runtime-backed service is installed.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemLlmClient;

#[async_trait]
impl SystemLlmClient for UnavailableSystemLlmClient {
    async fn chat(&self, command: LlmChatCommand) -> MacacaResult<LlmChatResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            "sdk llm client unavailable for chat"
        );
        Err(MacacaError::Config("LLM service is unavailable".into()))
    }

    async fn select_model(
        &self,
        command: LlmModelSelectionCommand,
    ) -> MacacaResult<LlmModelSelectionResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            "sdk llm client unavailable for model selection"
        );
        Err(MacacaError::Config("LLM service is unavailable".into()))
    }

    async fn snapshot(
        &self,
        command: LlmServiceSnapshotCommand,
    ) -> MacacaResult<LlmServiceSnapshot> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk llm client returning unavailable snapshot"
        );
        Ok(LlmServiceSnapshot::unavailable(
            "runtime-backed LLM service is not installed",
        ))
    }
}

/// Runtime-backed LLM client implemented over the generic SDK service client.
#[derive(Clone)]
pub struct ServiceBackedLlmClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedLlmClient {
    /// Create a service-backed client from an existing generic service client.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemLlmClient for ServiceBackedLlmClient {
    async fn chat(&self, command: LlmChatCommand) -> MacacaResult<LlmChatResult> {
        let trace = command.trace.clone();
        let service_command = ServiceCallCommand::new(
            LLM_SERVICE_ID,
            LLM_CHAT_COMMAND,
            serde_json::to_value(command)?,
        )?
        .with_trace(trace);
        let result = self.service.call_service(&service_command).await?;
        serde_json::from_value(result.output).map_err(MacacaError::from)
    }

    async fn select_model(
        &self,
        command: LlmModelSelectionCommand,
    ) -> MacacaResult<LlmModelSelectionResult> {
        let trace = command.trace.clone();
        let service_command = ServiceCallCommand::new(
            LLM_SERVICE_ID,
            LLM_MODEL_SELECTION_COMMAND,
            serde_json::to_value(command)?,
        )?
        .with_trace(trace);
        let result = self.service.call_service(&service_command).await?;
        serde_json::from_value(result.output).map_err(MacacaError::from)
    }

    async fn snapshot(
        &self,
        command: LlmServiceSnapshotCommand,
    ) -> MacacaResult<LlmServiceSnapshot> {
        let trace = command.trace.clone();
        let service_command = ServiceCallCommand::new(
            LLM_SERVICE_ID,
            LLM_SNAPSHOT_COMMAND,
            serde_json::to_value(command)?,
        )?
        .with_trace(trace);
        let result = self.service.call_service(&service_command).await?;
        serde_json::from_value(result.output).map_err(MacacaError::from)
    }
}
