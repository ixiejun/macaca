//! Runtime-host adapter for the Route C LLM Service.
//!
//! This module applies the Adapter/Bridge pattern: `ServiceRuntime` sees a
//! provider-neutral `SystemService`, while the adapter delegates actual model
//! work to an injected `macaca_llm::LlmProvider` strategy.  The runtime host
//! owns lifecycle, trace-aware dispatch, and structured service results; the
//! LLM crate still owns provider semantics and routing.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_llm::{
    llm_service_descriptor, LlmChatCommand, LlmChatResult, LlmModelSelectionCommand,
    LlmModelSelectionResult, LlmProvider, LlmRouteSummary, LlmServiceSnapshot,
    LlmServiceSnapshotCommand, LLM_CHAT_COMMAND, LLM_MODEL_SELECTION_COMMAND, LLM_SNAPSHOT_COMMAND,
};
use macaca_proto::{
    CleanupPolicy, MacacaError, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, TraceContext,
};

/// Host-owned LLM service provider that wraps an injected LLM strategy.
pub struct LlmSystemServiceProvider {
    descriptor: ServiceDescriptor,
    provider: Arc<dyn LlmProvider>,
}

impl LlmSystemServiceProvider {
    /// Create a service adapter from any LLM provider or router.
    ///
    /// The constructor accepts the trait object rather than a concrete router so
    /// applications can replace the implementation with remote, test, or plugin
    /// backed providers without changing runtime-host code.
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            descriptor: llm_service_descriptor(),
            provider,
        }
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn service_result(output: serde_json::Value, trace: TraceContext) -> ServiceCallResult {
        ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        }
    }
}

#[async_trait]
impl SystemService for LlmSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = %self.descriptor.id,
            provider = self.provider.name(),
            "llm service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        tracing::info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "llm service command accepted"
        );

        match command.name.as_str() {
            LLM_CHAT_COMMAND => {
                let typed: LlmChatCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    session_id = %typed.scope.session_id,
                    agent = %typed.scope.agent_name,
                    model = %typed.options.model,
                    "llm service dispatching chat command"
                );
                let response = self
                    .provider
                    .chat(typed.messages, &typed.options)
                    .await
                    .map_err(service_adapter_error)?;
                let route = LlmRouteSummary {
                    provider_id: self.provider.name().into(),
                    model: response.model.clone(),
                    source: "provider".into(),
                    fallbacks: Vec::new(),
                };
                let result = LlmChatResult::new(response, Some(route));
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    "llm service chat command completed"
                );
                Ok(Self::service_result(
                    serde_json::to_value(result).map_err(json_error)?,
                    typed.trace,
                ))
            }
            LLM_MODEL_SELECTION_COMMAND => {
                let typed: LlmModelSelectionCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                let model = typed
                    .request_model
                    .or(typed.agent_model)
                    .or(typed.app_model)
                    .or(typed.system_model)
                    .unwrap_or_else(|| "default".into());
                let result = LlmModelSelectionResult {
                    selected: LlmRouteSummary {
                        provider_id: self.provider.name().into(),
                        model,
                        source: "provider_snapshot".into(),
                        fallbacks: typed.fallbacks,
                    },
                };
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    provider = self.provider.name(),
                    "llm service model selection completed"
                );
                Ok(Self::service_result(
                    serde_json::to_value(result).map_err(json_error)?,
                    typed.trace,
                ))
            }
            LLM_SNAPSHOT_COMMAND => {
                let typed: LlmServiceSnapshotCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                let snapshot = LlmServiceSnapshot::healthy(
                    self.provider.name(),
                    Some("provider_default".into()),
                );
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    provider = self.provider.name(),
                    "llm service snapshot emitted"
                );
                Ok(Self::service_result(
                    serde_json::to_value(snapshot).map_err(json_error)?,
                    typed.trace,
                ))
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported LLM service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "llm service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "llm service provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

fn service_adapter_error(err: MacacaError) -> ServiceError {
    ServiceError::AdapterFailure(err.to_string())
}

fn json_error(err: serde_json::Error) -> ServiceError {
    ServiceError::AdapterFailure(err.to_string())
}
