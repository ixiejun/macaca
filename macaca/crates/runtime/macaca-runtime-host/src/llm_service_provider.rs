//! Runtime-host adapter for the LLM Service.
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
    llm_service_descriptor, LlmBudgetStatusCommand, LlmCatalogReadCommand, LlmChatCommand,
    LlmChatResult, LlmContinuationValidateCommand, LlmDegradationExplainCommand,
    LlmModelSelectionCommand, LlmProvider, LlmRouteSummary, LlmServiceSnapshot,
    LlmServiceSnapshotCommand, LLM_BUDGET_STATUS_COMMAND, LLM_CHAT_COMMAND,
    LLM_CONTINUATION_VALIDATE_COMMAND, LLM_DEGRADATION_EXPLAIN_COMMAND, LLM_MODEL_LIST_COMMAND,
    LLM_MODEL_SELECTION_COMMAND, LLM_PROVIDER_CAPABILITIES_READ_COMMAND, LLM_ROUTE_RESOLVE_COMMAND,
    LLM_SNAPSHOT_COMMAND,
};
use macaca_proto::{
    CleanupPolicy, MacacaError, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, TraceContext,
};

use crate::llm_service_catalog::{LlmProviderCatalogProfile, LlmProviderProfile};
use crate::llm_service_hardening::validate_before_dispatch;

/// Host-owned LLM service provider that wraps an injected LLM strategy.
pub struct LlmSystemServiceProvider {
    pub(crate) descriptor: ServiceDescriptor,
    pub(crate) provider: Option<Arc<dyn LlmProvider>>,
    pub(crate) profile: LlmProviderProfile,
    /// Sanitized catalog snapshot used by `model.list` and provider capability
    /// commands. Chat dispatch still delegates to the injected provider
    /// strategy, while catalog reads use this bounded Memento so application
    /// UIs can render truthful provider/model choices without reading config.
    pub(crate) catalog: LlmProviderCatalogProfile,
    pub(crate) unavailable_reason: Option<String>,
}

impl LlmSystemServiceProvider {
    /// Create a service adapter from any LLM provider or router.
    ///
    /// The constructor accepts the trait object rather than a concrete router so
    /// applications can replace the implementation with remote, test, or plugin
    /// backed providers without changing runtime-host code.
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        let profile = LlmProviderProfile::generic(provider.name());
        Self::with_profile(provider, profile)
    }

    /// Create a service adapter with explicit catalog/protocol metadata.
    ///
    /// This constructor is the runtime-host composition point for future
    /// plugin-backed or remote LLM providers.  The service can expose hardened
    /// diagnostics without forcing the low-level `LlmProvider` trait to become a
    /// catalog API.
    pub fn with_profile(provider: Arc<dyn LlmProvider>, profile: LlmProviderProfile) -> Self {
        let catalog = LlmProviderCatalogProfile::single(profile.clone());
        Self::with_catalog(provider, profile, catalog)
    }

    /// Create a service adapter with a sanitized multi-provider catalog.
    ///
    /// Runtime composition roots use this constructor when a router was built
    /// from configuration. The low-level provider remains replaceable, while
    /// catalog commands can report all configured providers, including
    /// unavailable rows, through the same `service.llm` boundary.
    pub fn with_catalog(
        provider: Arc<dyn LlmProvider>,
        profile: LlmProviderProfile,
        catalog: LlmProviderCatalogProfile,
    ) -> Self {
        Self {
            descriptor: llm_service_descriptor(),
            provider: Some(provider),
            profile,
            catalog,
            unavailable_reason: None,
        }
    }

    /// Create a Null Object provider that answers diagnostics but rejects chat.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self {
            descriptor: llm_service_descriptor(),
            provider: None,
            profile: LlmProviderProfile::generic("unavailable"),
            catalog: LlmProviderCatalogProfile::single(LlmProviderProfile::generic("unavailable")),
            unavailable_reason: Some(reason),
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
            command = "start",
            unavailable = self.unavailable_reason.is_some(),
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
                if let Some(reason) = &self.unavailable_reason {
                    tracing::warn!(
                        trace_id = %typed.trace.trace_id,
                        reason = %reason,
                        "llm service rejected chat because provider is unavailable"
                    );
                    return Err(ServiceError::AdapterFailure(format!(
                        "LLM provider unavailable: {reason}"
                    )));
                }
                validate_before_dispatch(&typed.messages, &self.profile.protocol)
                    .map_err(service_adapter_error)?;
                tracing::info!(
                    service_id = %self.descriptor.id,
                    command = LLM_CHAT_COMMAND,
                    trace_id = %typed.trace.trace_id,
                    session_id = %typed.scope.session_id,
                    agent_id = %typed.scope.agent_name,
                    model_hint_present = typed.model_hint.is_some(),
                    "llm service dispatching chat command"
                );
                let provider = self.provider.as_ref().ok_or_else(|| {
                    ServiceError::AdapterFailure("LLM provider unavailable".into())
                })?;
                let response = provider
                    .chat(typed.messages, &typed.options)
                    .await
                    .map_err(service_adapter_error)?;
                let route = LlmRouteSummary {
                    provider_id: provider.name().into(),
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
                let result = self.resolve_route(
                    typed.request_model,
                    typed.agent_model,
                    typed.app_model,
                    typed.app_provider,
                    typed.system_model,
                    typed.fallbacks,
                )?;
                tracing::info!(
                    service_id = %self.descriptor.id,
                    command = LLM_MODEL_SELECTION_COMMAND,
                    trace_id = %typed.trace.trace_id,
                    route_resolved = true,
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
                let snapshot = if let Some(reason) = &self.unavailable_reason {
                    LlmServiceSnapshot::unavailable(reason.clone())
                } else {
                    LlmServiceSnapshot::healthy(
                        self.profile.provider_id.clone(),
                        self.profile.default_model.clone(),
                    )
                };
                tracing::info!(
                    service_id = %self.descriptor.id,
                    command = LLM_SNAPSHOT_COMMAND,
                    trace_id = %typed.trace.trace_id,
                    "llm service snapshot emitted"
                );
                Ok(Self::service_result(
                    serde_json::to_value(snapshot).map_err(json_error)?,
                    typed.trace,
                ))
            }
            LLM_MODEL_LIST_COMMAND => {
                let typed: LlmCatalogReadCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                Ok(Self::service_result(
                    serde_json::to_value(self.handle_model_list(typed.clone())?)
                        .map_err(json_error)?,
                    typed.trace,
                ))
            }
            LLM_PROVIDER_CAPABILITIES_READ_COMMAND => {
                let typed: LlmCatalogReadCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                Ok(Self::service_result(
                    serde_json::to_value(self.handle_provider_capabilities(typed.clone())?)
                        .map_err(json_error)?,
                    typed.trace,
                ))
            }
            LLM_ROUTE_RESOLVE_COMMAND => {
                let typed: macaca_llm::LlmRouteResolveCommand =
                    serde_json::from_value(command.payload)
                        .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                Ok(Self::service_result(
                    serde_json::to_value(self.handle_route_resolve(typed.clone())?)
                        .map_err(json_error)?,
                    typed.trace,
                ))
            }
            LLM_CONTINUATION_VALIDATE_COMMAND => {
                let typed: LlmContinuationValidateCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                Ok(Self::service_result(
                    serde_json::to_value(self.handle_continuation_validate(typed.clone())?)
                        .map_err(json_error)?,
                    typed.trace,
                ))
            }
            LLM_BUDGET_STATUS_COMMAND => {
                let typed: LlmBudgetStatusCommand = serde_json::from_value(command.payload)
                    .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                Ok(Self::service_result(
                    serde_json::to_value(self.handle_budget_status(typed.clone())?)
                        .map_err(json_error)?,
                    typed.trace,
                ))
            }
            LLM_DEGRADATION_EXPLAIN_COMMAND => {
                let typed: LlmDegradationExplainCommand =
                    serde_json::from_value(command.payload)
                        .map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))?;
                Ok(Self::service_result(
                    serde_json::to_value(self.handle_degradation_explain(typed.clone())?)
                        .map_err(json_error)?,
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
