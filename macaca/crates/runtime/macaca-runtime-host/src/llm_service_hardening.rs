//! Hardened command handlers for `service.llm`.
//!
//! The provider module owns lifecycle and generic service dispatch.  This helper
//! owns the executable Specifications that make LLM routing auditable: model
//! catalog reads, provider capability reads, route resolution, continuation
//! validation, budget status, and degradation explanations.  Keeping these
//! handlers separate prevents the runtime-host adapter from becoming a large
//! God Object while still using the same provider state and trace boundary.

use chrono::Utc;
use macaca_llm::{
    LlmBudgetStatusCommand, LlmBudgetStatusResult, LlmCatalogReadCommand,
    LlmContinuationValidateCommand, LlmContinuationValidateResult, LlmDegradationExplainCommand,
    LlmDegradationExplainResult, LlmDiagnostic, LlmModelCatalogResult, LlmModelSelectionResult,
    LlmProviderCapabilitiesResult, LlmProviderCapabilityRecord, LlmProviderProtocolMetadata,
    LlmRouteResolveCommand, LlmRouteResolveResult, LlmRouteSummary, ModelSelectionRequest,
};
use macaca_proto::{LlmMessage, LlmRole, MacacaError, ServiceError, ServiceResult, ToolCall};
use std::collections::{BTreeMap, BTreeSet};

use crate::llm_service_catalog::{catalog_models, LlmProviderProfile};
use crate::llm_service_provider::LlmSystemServiceProvider;

impl LlmSystemServiceProvider {
    pub(crate) fn profile(&self) -> LlmProviderProfile {
        self.profile.clone()
    }

    pub(crate) fn unavailable_diagnostic(&self) -> Option<LlmDiagnostic> {
        self.unavailable_reason.as_ref().map(|reason| {
            LlmDiagnostic::new(
                "provider_unavailable",
                "error",
                "LLM provider is unavailable for this runtime",
            )
            .with_metadata("reason", reason.clone())
        })
    }

    pub(crate) fn handle_model_list(
        &self,
        command: LlmCatalogReadCommand,
    ) -> ServiceResult<LlmModelCatalogResult> {
        tracing::info!(
            trace_id = %command.trace.trace_id,
            session_id = %command.scope.session_id,
            "llm service reading model catalog"
        );
        let profiles = self.catalog.visible_profiles(command.include_disabled);
        let models = profiles
            .iter()
            .flat_map(|profile| {
                catalog_models(
                    profile,
                    self.catalog.unavailable_reason(&profile.provider_id),
                )
            })
            .collect::<Vec<_>>();
        let unavailable_count = profiles.iter().filter(|profile| !profile.healthy).count();
        tracing::info!(
            trace_id = %command.trace.trace_id,
            session_id = %command.scope.session_id,
            model_count = models.len(),
            provider_count = profiles.len(),
            unavailable_count,
            "llm service model catalog read completed"
        );
        Ok(LlmModelCatalogResult {
            models,
            captured_at: Utc::now(),
        })
    }

    pub(crate) fn handle_provider_capabilities(
        &self,
        command: LlmCatalogReadCommand,
    ) -> ServiceResult<LlmProviderCapabilitiesResult> {
        tracing::info!(
            trace_id = %command.trace.trace_id,
            session_id = %command.scope.session_id,
            "llm service reading provider capabilities"
        );
        let profiles = self.catalog.visible_profiles(command.include_disabled);
        let unavailable_count = profiles.iter().filter(|profile| !profile.healthy).count();
        tracing::info!(
            trace_id = %command.trace.trace_id,
            session_id = %command.scope.session_id,
            provider_count = profiles.len(),
            unavailable_count,
            "llm service provider capability read completed"
        );
        Ok(LlmProviderCapabilitiesResult {
            providers: profiles
                .into_iter()
                .map(|profile| {
                    let unavailable_reason = self.catalog.unavailable_reason(&profile.provider_id);
                    let healthy = profile.healthy
                        && self.unavailable_reason.is_none()
                        && unavailable_reason.is_none();
                    LlmProviderCapabilityRecord {
                        provider_id: profile.provider_id,
                        healthy,
                        availability: if healthy {
                            "healthy".into()
                        } else {
                            "unavailable".into()
                        },
                        unavailable_reason,
                        default_model: profile.default_model,
                        capabilities: profile.capabilities,
                        protocol: profile.protocol,
                    }
                })
                .collect(),
            captured_at: Utc::now(),
        })
    }

    pub(crate) fn handle_route_resolve(
        &self,
        command: LlmRouteResolveCommand,
    ) -> ServiceResult<LlmRouteResolveResult> {
        tracing::info!(
            trace_id = %command.trace.trace_id,
            session_id = %command.scope.session_id,
            "llm service resolving hardened route"
        );
        let result = self.resolve_route(
            command.request_model,
            command.agent_model,
            command.app_model,
            command.app_provider,
            command.system_model,
            command.fallbacks,
        )?;
        let profile = self
            .catalog
            .provider(&result.selected.provider_id)
            .unwrap_or_else(|| self.profile());
        let mut diagnostics: Vec<LlmDiagnostic> =
            self.unavailable_diagnostic().into_iter().collect();
        if let Some(reason) = self
            .catalog
            .unavailable_reason(&result.selected.provider_id)
        {
            diagnostics.push(
                LlmDiagnostic::new(
                    "provider_unavailable",
                    "error",
                    "Selected provider is unavailable for this runtime",
                )
                .with_metadata("provider_id", result.selected.provider_id.clone())
                .with_metadata("reason", reason),
            );
        }
        if !profile.models.is_empty() && !profile.models.contains(&result.selected.model) {
            diagnostics.push(
                LlmDiagnostic::new(
                    "unsupported_model",
                    "warning",
                    "Requested model is not present in the provider catalog",
                )
                .with_metadata("provider_id", result.selected.provider_id.clone()),
            );
        }
        Ok(LlmRouteResolveResult {
            selected: result.selected,
            diagnostics,
        })
    }

    pub(crate) fn handle_continuation_validate(
        &self,
        command: LlmContinuationValidateCommand,
    ) -> ServiceResult<LlmContinuationValidateResult> {
        tracing::info!(
            trace_id = %command.trace.trace_id,
            session_id = %command.scope.session_id,
            "llm service validating continuation protocol"
        );
        let mut diagnostics = validate_continuation(&command.messages, &command.protocol);
        diagnostics.extend(self.unavailable_diagnostic());
        Ok(LlmContinuationValidateResult {
            accepted: diagnostics.iter().all(|d| d.severity != "error"),
            diagnostics,
        })
    }

    pub(crate) fn handle_budget_status(
        &self,
        command: LlmBudgetStatusCommand,
    ) -> ServiceResult<LlmBudgetStatusResult> {
        tracing::info!(
            trace_id = %command.trace.trace_id,
            session_id = %command.scope.session_id,
            "llm service reading budget status"
        );
        let denied = command.policy.max_cost_micros == Some(0);
        let mut diagnostics: Vec<LlmDiagnostic> =
            self.unavailable_diagnostic().into_iter().collect();
        if denied {
            diagnostics.push(LlmDiagnostic::new(
                "budget_denied",
                "error",
                "LLM budget policy denies this request",
            ));
        }
        Ok(LlmBudgetStatusResult {
            status: if denied { "denied" } else { "available" }.into(),
            max_prompt_tokens: command.policy.max_prompt_tokens,
            max_completion_tokens: command.policy.max_completion_tokens,
            max_cost_micros: command.policy.max_cost_micros,
            remaining_cost_micros: command.policy.max_cost_micros,
            diagnostics,
        })
    }

    pub(crate) fn handle_degradation_explain(
        &self,
        command: LlmDegradationExplainCommand,
    ) -> ServiceResult<LlmDegradationExplainResult> {
        tracing::info!(
            trace_id = %command.trace.trace_id,
            session_id = %command.scope.session_id,
            "llm service explaining degradation"
        );
        let failure_code = command.failure_code.unwrap_or_else(|| "none".to_string());
        let retryable = matches!(
            failure_code.as_str(),
            "rate_limited" | "provider_unavailable" | "timeout"
        );
        let degraded = failure_code != "none" || self.unavailable_reason.is_some();
        let mut diagnostics: Vec<LlmDiagnostic> =
            self.unavailable_diagnostic().into_iter().collect();
        if failure_code != "none" {
            diagnostics.push(LlmDiagnostic::new(
                failure_code.clone(),
                if retryable { "warning" } else { "error" },
                "LLM route is degraded by the reported failure code",
            ));
        }
        Ok(LlmDegradationExplainResult {
            degraded,
            reason_code: if degraded {
                failure_code
            } else {
                "healthy".into()
            },
            retryable,
            diagnostics,
        })
    }

    pub(crate) fn resolve_route(
        &self,
        request_model: Option<String>,
        agent_model: Option<String>,
        app_model: Option<String>,
        app_provider: Option<String>,
        system_model: Option<String>,
        fallbacks: Vec<String>,
    ) -> ServiceResult<LlmModelSelectionResult> {
        let profile = self.profile();
        let request_target = request_model.as_deref().and_then(split_provider_model);
        let model = first_non_empty([
            request_target.map(|(_, model)| model),
            request_model.as_deref(),
            agent_model.as_deref(),
            app_model.as_deref(),
            system_model.as_deref(),
            profile.default_model.as_deref(),
        ])
        .ok_or_else(|| ServiceError::AdapterFailure("no model route available".into()))?;
        let selected = LlmRouteSummary {
            provider_id: request_target
                .map(|(provider, _)| provider.to_string())
                .or_else(|| app_provider.filter(|provider| !provider.trim().is_empty()))
                .unwrap_or(profile.provider_id),
            model: model.to_string(),
            source: route_source(&ModelSelectionRequest {
                request_model,
                agent_model,
                app_model,
                app_provider: None,
                system_model,
                fallbacks: fallbacks.clone(),
            })
            .into(),
            fallbacks,
        };
        Ok(LlmModelSelectionResult { selected })
    }
}

pub(crate) fn validate_before_dispatch(
    messages: &[LlmMessage],
    protocol: &LlmProviderProtocolMetadata,
) -> Result<(), MacacaError> {
    let diagnostics = validate_continuation(messages, protocol);
    if diagnostics.iter().any(|d| d.severity == "error") {
        let codes = diagnostics
            .iter()
            .map(|d| d.code.as_str())
            .collect::<Vec<_>>()
            .join(",");
        return Err(MacacaError::Llm(format!(
            "LLM provider protocol validation failed: {codes}"
        )));
    }
    Ok(())
}

fn validate_continuation(
    messages: &[LlmMessage],
    protocol: &LlmProviderProtocolMetadata,
) -> Vec<LlmDiagnostic> {
    let mut diagnostics = Vec::new();
    if !protocol.supports_tool_calls && messages.iter().any(|m| has_tool_calls(m)) {
        diagnostics.push(LlmDiagnostic::new(
            "unsupported_tool_calls",
            "error",
            "Provider protocol does not support assistant tool calls",
        ));
    }
    if !protocol.supports_tool_results && messages.iter().any(|m| m.role == LlmRole::Tool) {
        diagnostics.push(LlmDiagnostic::new(
            "unsupported_tool_results",
            "error",
            "Provider protocol does not support tool-result continuation messages",
        ));
    }
    if protocol.requires_reasoning_continuation {
        diagnostics.extend(validate_reasoning_continuation(messages));
    }
    diagnostics
}

fn validate_reasoning_continuation(messages: &[LlmMessage]) -> Vec<LlmDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut tool_calls_requiring_reasoning = BTreeMap::<String, bool>::new();
    for message in messages {
        if message.role == LlmRole::Assistant {
            for call in message.tool_calls.as_deref().unwrap_or_default() {
                tool_calls_requiring_reasoning.insert(
                    call.id.clone(),
                    message
                        .reasoning_content
                        .as_ref()
                        .map(|value| !value.trim().is_empty())
                        .unwrap_or(false),
                );
            }
        }
    }

    let answered_tool_calls: BTreeSet<&str> = messages
        .iter()
        .filter(|message| message.role == LlmRole::Tool)
        .filter_map(|message| message.tool_call_id.as_deref())
        .collect();

    for (tool_call_id, has_reasoning) in tool_calls_requiring_reasoning {
        if answered_tool_calls.contains(tool_call_id.as_str()) && !has_reasoning {
            diagnostics.push(
                LlmDiagnostic::new(
                    "missing_reasoning_continuation",
                    "error",
                    "Provider requires reasoning metadata when continuing after a tool result",
                )
                .with_metadata("tool_call_id", redact_id(&tool_call_id)),
            );
        }
    }
    diagnostics
}

fn has_tool_calls(message: &LlmMessage) -> bool {
    message
        .tool_calls
        .as_ref()
        .map(|calls: &Vec<ToolCall>| !calls.is_empty())
        .unwrap_or(false)
}

fn first_non_empty<const N: usize>(values: [Option<&str>; N]) -> Option<&str> {
    values
        .into_iter()
        .flatten()
        .find(|value| !value.trim().is_empty())
}

fn split_provider_model(model_ref: &str) -> Option<(&str, &str)> {
    let (provider, model) = model_ref.split_once(':')?;
    (!provider.trim().is_empty() && !model.trim().is_empty())
        .then_some((provider.trim(), model.trim()))
}

fn route_source(request: &ModelSelectionRequest) -> &'static str {
    if request
        .request_model
        .as_ref()
        .is_some_and(|m| !m.is_empty())
    {
        "request"
    } else if request.agent_model.as_ref().is_some_and(|m| !m.is_empty()) {
        "agent"
    } else if request.app_model.as_ref().is_some_and(|m| !m.is_empty()) {
        "app"
    } else {
        "system"
    }
}

fn redact_id(value: &str) -> String {
    if value.len() <= 8 {
        return "<id>".into();
    }
    format!("{}...{}", &value[..4], &value[value.len() - 4..])
}
