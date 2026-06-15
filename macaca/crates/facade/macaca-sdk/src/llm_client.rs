//! SDK LLM client facade.
//!
//! The SDK owns a caller-friendly Facade, not provider construction.  Clients
//! either translate typed LLM commands into generic service calls or return a
//! structured unavailable error when no runtime dispatcher is installed.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    LlmBudgetStatusCommand, LlmBudgetStatusResult, LlmCatalogReadCommand, LlmChatCommand,
    LlmChatResult, LlmContinuationValidateCommand, LlmContinuationValidateResult,
    LlmDegradationExplainCommand, LlmDegradationExplainResult, LlmModelCatalogResult,
    LlmModelSelectionCommand, LlmModelSelectionResult, LlmProviderCapabilitiesResult,
    LlmRouteResolveCommand, LlmRouteResolveResult, LlmServiceChatClient, LlmServiceSnapshot,
    LlmServiceSnapshotCommand, MacacaError, MacacaResult, LLM_BUDGET_STATUS_COMMAND,
    LLM_CHAT_COMMAND, LLM_CONTINUATION_VALIDATE_COMMAND, LLM_DEGRADATION_EXPLAIN_COMMAND,
    LLM_MODEL_LIST_COMMAND, LLM_MODEL_SELECTION_COMMAND, LLM_PROVIDER_CAPABILITIES_READ_COMMAND,
    LLM_ROUTE_RESOLVE_COMMAND, LLM_SERVICE_ID, LLM_SNAPSHOT_COMMAND,
};
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
    /// List sanitized model catalog entries.
    async fn list_models(
        &self,
        command: LlmCatalogReadCommand,
    ) -> MacacaResult<LlmModelCatalogResult>;
    /// Read provider capability and protocol metadata.
    async fn provider_capabilities(
        &self,
        command: LlmCatalogReadCommand,
    ) -> MacacaResult<LlmProviderCapabilitiesResult>;
    /// Resolve the effective model route with diagnostics.
    async fn resolve_route(
        &self,
        command: LlmRouteResolveCommand,
    ) -> MacacaResult<LlmRouteResolveResult>;
    /// Validate tool-result continuation before provider dispatch.
    async fn validate_continuation(
        &self,
        command: LlmContinuationValidateCommand,
    ) -> MacacaResult<LlmContinuationValidateResult>;
    /// Read provider-neutral budget status.
    async fn budget_status(
        &self,
        command: LlmBudgetStatusCommand,
    ) -> MacacaResult<LlmBudgetStatusResult>;
    /// Explain degradation without leaking raw provider payloads.
    async fn explain_degradation(
        &self,
        command: LlmDegradationExplainCommand,
    ) -> MacacaResult<LlmDegradationExplainResult>;
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

    async fn list_models(
        &self,
        command: LlmCatalogReadCommand,
    ) -> MacacaResult<LlmModelCatalogResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk llm client unavailable for model list");
        Err(MacacaError::Config("LLM service is unavailable".into()))
    }

    async fn provider_capabilities(
        &self,
        command: LlmCatalogReadCommand,
    ) -> MacacaResult<LlmProviderCapabilitiesResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk llm client unavailable for provider capabilities");
        Err(MacacaError::Config("LLM service is unavailable".into()))
    }

    async fn resolve_route(
        &self,
        command: LlmRouteResolveCommand,
    ) -> MacacaResult<LlmRouteResolveResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk llm client unavailable for route resolution");
        Err(MacacaError::Config("LLM service is unavailable".into()))
    }

    async fn validate_continuation(
        &self,
        command: LlmContinuationValidateCommand,
    ) -> MacacaResult<LlmContinuationValidateResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk llm client unavailable for continuation validation");
        Err(MacacaError::Config("LLM service is unavailable".into()))
    }

    async fn budget_status(
        &self,
        command: LlmBudgetStatusCommand,
    ) -> MacacaResult<LlmBudgetStatusResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk llm client unavailable for budget status");
        Err(MacacaError::Config("LLM service is unavailable".into()))
    }

    async fn explain_degradation(
        &self,
        command: LlmDegradationExplainCommand,
    ) -> MacacaResult<LlmDegradationExplainResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk llm client unavailable for degradation explanation");
        Err(MacacaError::Config("LLM service is unavailable".into()))
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

    async fn list_models(
        &self,
        command: LlmCatalogReadCommand,
    ) -> MacacaResult<LlmModelCatalogResult> {
        self.call_typed(LLM_MODEL_LIST_COMMAND, command.trace.clone(), command)
            .await
    }

    async fn provider_capabilities(
        &self,
        command: LlmCatalogReadCommand,
    ) -> MacacaResult<LlmProviderCapabilitiesResult> {
        self.call_typed(
            LLM_PROVIDER_CAPABILITIES_READ_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn resolve_route(
        &self,
        command: LlmRouteResolveCommand,
    ) -> MacacaResult<LlmRouteResolveResult> {
        self.call_typed(LLM_ROUTE_RESOLVE_COMMAND, command.trace.clone(), command)
            .await
    }

    async fn validate_continuation(
        &self,
        command: LlmContinuationValidateCommand,
    ) -> MacacaResult<LlmContinuationValidateResult> {
        self.call_typed(
            LLM_CONTINUATION_VALIDATE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn budget_status(
        &self,
        command: LlmBudgetStatusCommand,
    ) -> MacacaResult<LlmBudgetStatusResult> {
        self.call_typed(LLM_BUDGET_STATUS_COMMAND, command.trace.clone(), command)
            .await
    }

    async fn explain_degradation(
        &self,
        command: LlmDegradationExplainCommand,
    ) -> MacacaResult<LlmDegradationExplainResult> {
        self.call_typed(
            LLM_DEGRADATION_EXPLAIN_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

/// Bridges the full SDK [`SystemLlmClient`] to the narrow [`LlmServiceChatClient`] port.
///
/// Framework adapters depend only on `LlmServiceChatClient` so `macaca-framework`
/// never needs a direct `macaca-sdk` dependency. This **Adapter** wraps the
/// composition-root `SystemLlmClient` handle and forwards chat commands while
/// logging the trace id for audit.
struct LlmServiceChatClientBridge {
    inner: Arc<dyn SystemLlmClient>,
}

impl LlmServiceChatClientBridge {
    /// Wrap a runtime `SystemLlmClient` as the service-contract chat port.
    ///
    /// Returns a trait object so `ServiceChatModelAdapter` can accept the handle
    /// without knowing which concrete SDK client implementation is installed.
    pub fn wrap(inner: Arc<dyn SystemLlmClient>) -> Arc<dyn LlmServiceChatClient> {
        tracing::info!(
            "sdk llm service chat bridge wrapping SystemLlmClient for framework dispatch"
        );
        Arc::new(Self { inner })
    }
}

#[async_trait]
impl LlmServiceChatClient for LlmServiceChatClientBridge {
    async fn chat(&self, command: LlmChatCommand) -> MacacaResult<LlmChatResult> {
        tracing::info!(
            trace_id = %command.trace.trace_id,
            session_id = %command.scope.session_id,
            agent = %command.scope.agent_name,
            "sdk llm service chat bridge forwarding chat command to SystemLlmClient"
        );
        self.inner.chat(command).await
    }
}

/// Public helper used by presentation shells when wiring framework chat models.
///
/// Keeps the bridge construction in one auditable place instead of repeating
/// adapter boilerplate across `macaca-web` agent factory paths.
pub fn llm_service_chat_client_from_system(
    client: Arc<dyn SystemLlmClient>,
) -> Arc<dyn LlmServiceChatClient> {
    LlmServiceChatClientBridge::wrap(client)
}

impl ServiceBackedLlmClient {
    /// Shared typed command bridge for hardened LLM facade methods.
    ///
    /// The helper keeps each facade method small while preserving trace
    /// propagation and the provider-neutral `service.llm` command envelope.
    async fn call_typed<T, R>(
        &self,
        command_name: &'static str,
        trace: macaca_proto::TraceContext,
        command: T,
    ) -> MacacaResult<R>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let service_command =
            ServiceCallCommand::new(LLM_SERVICE_ID, command_name, serde_json::to_value(command)?)?
                .with_trace(trace);
        let result = self.service.call_service(&service_command).await?;
        serde_json::from_value(result.output).map_err(MacacaError::from)
    }
}
