//! Focused tests for hardened `service.llm` behavior.
//!
//! The tests model provider protocol requirements without naming a concrete
//! vendor.  This proves the service catches thinking/tool continuation defects
//! through generic metadata instead of embedding provider-specific branches in
//! OS code.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_llm::{
    LlmBudgetStatusCommand, LlmCatalogReadCommand, LlmContinuationValidateCommand,
    LlmDegradationExplainCommand, LlmProvider, LlmProviderProtocolMetadata, LlmRetryPolicy,
    LlmRouteResolveCommand, LlmServiceScope, LLM_BUDGET_STATUS_COMMAND, LLM_CHAT_COMMAND,
    LLM_CONTINUATION_VALIDATE_COMMAND, LLM_DEGRADATION_EXPLAIN_COMMAND, LLM_MODEL_LIST_COMMAND,
    LLM_PROVIDER_CAPABILITIES_READ_COMMAND, LLM_ROUTE_RESOLVE_COMMAND,
};
use macaca_proto::{
    ApplicationId, LlmMessage, LlmOptions, LlmResponse, MacacaResult, ServiceCommand,
    ServiceCommandName, TokenUsage, ToolCall, TraceContext,
};
use serde::de::DeserializeOwned;
use serde_json::json;

use crate::{LlmProviderProfile, LlmSystemServiceProvider};

struct CountingLlmProvider {
    calls: AtomicUsize,
}

#[async_trait]
impl LlmProvider for CountingLlmProvider {
    fn name(&self) -> &str {
        "generic-provider"
    }

    async fn chat(
        &self,
        _messages: Vec<LlmMessage>,
        options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(LlmResponse {
            content: "ok".into(),
            reasoning_content: None,
            model: options.model.clone(),
            usage: TokenUsage {
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
            },
            finish_reason: "stop".into(),
            tool_calls: None,
        })
    }
}

fn scope() -> LlmServiceScope {
    LlmServiceScope::new(
        ApplicationId::from_name("generic.test.application"),
        "session-1",
        "agent",
    )
    .unwrap()
}

fn trace(name: &str) -> TraceContext {
    TraceContext::new(format!("trace-{name}"))
}

fn command<T: serde::Serialize>(name: &str, payload: T) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        trace(name),
    )
}

fn decode<T: DeserializeOwned>(value: serde_json::Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn strict_protocol() -> LlmProviderProtocolMetadata {
    LlmProviderProtocolMetadata {
        supports_reasoning: true,
        requires_reasoning_continuation: true,
        supports_tool_calls: true,
        supports_tool_results: true,
        supports_service_tiers: true,
        retry_policy: LlmRetryPolicy {
            max_attempts: 3,
            retryable_failure_codes: vec!["rate_limited".into(), "timeout".into()],
            backoff_base_ms: 50,
        },
    }
}

fn provider_with_protocol(
    protocol: LlmProviderProtocolMetadata,
) -> (LlmSystemServiceProvider, Arc<CountingLlmProvider>) {
    let provider = Arc::new(CountingLlmProvider {
        calls: AtomicUsize::new(0),
    });
    let profile = LlmProviderProfile::generic("generic-provider")
        .with_default_model("generic-model")
        .with_protocol(protocol);
    (
        LlmSystemServiceProvider::with_profile(provider.clone(), profile),
        provider,
    )
}

fn tool_transcript(include_reasoning: bool) -> Vec<LlmMessage> {
    let call = ToolCall {
        id: "tool-call-123456".into(),
        name: "generic_tool".into(),
        arguments: json!({"key": "value"}),
    };
    let mut assistant = LlmMessage::assistant_with_tool_calls("", vec![call]);
    if include_reasoning {
        assistant.reasoning_content = Some("bounded reasoning continuation metadata".into());
    }
    vec![
        LlmMessage::user("run generic tool"),
        assistant,
        LlmMessage::tool_result("tool-call-123456", "done"),
    ]
}

#[tokio::test]
async fn continuation_validation_rejects_missing_reasoning_metadata() {
    let (provider, calls) = provider_with_protocol(strict_protocol());
    let result = provider
        .call(command(
            LLM_CONTINUATION_VALIDATE_COMMAND,
            LlmContinuationValidateCommand {
                scope: scope(),
                trace: trace("continuation"),
                route: None,
                protocol: strict_protocol(),
                messages: tool_transcript(false),
            },
        ))
        .await
        .unwrap();
    let validation: macaca_llm::LlmContinuationValidateResult = decode(result.output);

    assert!(!validation.accepted);
    assert_eq!(
        validation.diagnostics[0].code,
        "missing_reasoning_continuation"
    );
    assert_eq!(calls.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn chat_rejects_invalid_continuation_before_provider_dispatch() {
    let (provider, calls) = provider_with_protocol(strict_protocol());
    let chat_command = macaca_llm::LlmChatCommand::new(
        scope(),
        trace("chat"),
        tool_transcript(false),
        LlmOptions {
            model: "generic-model".into(),
            ..LlmOptions::default()
        },
    )
    .unwrap();

    let err = provider
        .call(command(LLM_CHAT_COMMAND, chat_command))
        .await
        .expect_err("invalid continuation must fail before provider dispatch");

    assert!(err.to_string().contains("missing_reasoning_continuation"));
    assert_eq!(calls.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn chat_allows_valid_reasoning_continuation() {
    let (provider, calls) = provider_with_protocol(strict_protocol());
    let chat_command = macaca_llm::LlmChatCommand::new(
        scope(),
        trace("chat-valid"),
        tool_transcript(true),
        LlmOptions {
            model: "generic-model".into(),
            ..LlmOptions::default()
        },
    )
    .unwrap();

    let result = provider
        .call(command(LLM_CHAT_COMMAND, chat_command))
        .await
        .unwrap();
    let chat: macaca_llm::LlmChatResult = decode(result.output);

    assert_eq!(chat.response.model, "generic-model");
    assert_eq!(calls.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn catalog_capabilities_route_budget_and_degradation_are_structured() {
    let (provider, _calls) = provider_with_protocol(strict_protocol());
    let catalog: macaca_llm::LlmModelCatalogResult = decode(
        provider
            .call(command(
                LLM_MODEL_LIST_COMMAND,
                LlmCatalogReadCommand {
                    scope: scope(),
                    trace: trace("models"),
                    include_disabled: false,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert_eq!(catalog.models[0].model, "generic-model");
    assert!(catalog.models[0].protocol.requires_reasoning_continuation);

    let capabilities: macaca_llm::LlmProviderCapabilitiesResult = decode(
        provider
            .call(command(
                LLM_PROVIDER_CAPABILITIES_READ_COMMAND,
                LlmCatalogReadCommand {
                    scope: scope(),
                    trace: trace("caps"),
                    include_disabled: false,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(capabilities.providers[0].healthy);

    let route: macaca_llm::LlmRouteResolveResult = decode(
        provider
            .call(command(
                LLM_ROUTE_RESOLVE_COMMAND,
                LlmRouteResolveCommand {
                    scope: scope(),
                    trace: trace("route"),
                    request_model: None,
                    agent_model: None,
                    app_model: None,
                    app_provider: None,
                    system_model: None,
                    fallbacks: vec!["fallback-model".into()],
                    policy: Default::default(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert_eq!(route.selected.model, "generic-model");
    assert_eq!(route.selected.fallbacks, vec!["fallback-model"]);

    let unsupported_route: macaca_llm::LlmRouteResolveResult = decode(
        provider
            .call(command(
                LLM_ROUTE_RESOLVE_COMMAND,
                LlmRouteResolveCommand {
                    scope: scope(),
                    trace: trace("route-unsupported"),
                    request_model: Some("missing-model".into()),
                    agent_model: None,
                    app_model: None,
                    app_provider: None,
                    system_model: None,
                    fallbacks: Vec::new(),
                    policy: Default::default(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert_eq!(unsupported_route.diagnostics[0].code, "unsupported_model");

    let budget: macaca_llm::LlmBudgetStatusResult = decode(
        provider
            .call(command(
                LLM_BUDGET_STATUS_COMMAND,
                LlmBudgetStatusCommand {
                    scope: scope(),
                    trace: trace("budget"),
                    policy: macaca_llm::LlmPolicyHints {
                        max_cost_micros: Some(0),
                        ..Default::default()
                    },
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert_eq!(budget.status, "denied");
    assert_eq!(budget.diagnostics[0].code, "budget_denied");

    let degradation: macaca_llm::LlmDegradationExplainResult = decode(
        provider
            .call(command(
                LLM_DEGRADATION_EXPLAIN_COMMAND,
                LlmDegradationExplainCommand {
                    scope: scope(),
                    trace: trace("degradation"),
                    route: Some(route.selected),
                    failure_code: Some("rate_limited".into()),
                    policy: Default::default(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(degradation.degraded);
    assert!(degradation.retryable);
}

#[tokio::test]
async fn unavailable_provider_returns_diagnostics_without_provider_payload() {
    let provider = LlmSystemServiceProvider::unavailable("not configured");
    let result: macaca_llm::LlmProviderCapabilitiesResult = decode(
        provider
            .call(command(
                LLM_PROVIDER_CAPABILITIES_READ_COMMAND,
                LlmCatalogReadCommand {
                    scope: scope(),
                    trace: trace("unavailable"),
                    include_disabled: true,
                },
            ))
            .await
            .unwrap()
            .output,
    );

    assert!(!result.providers[0].healthy);
    assert_eq!(result.providers[0].provider_id, "unavailable");
}
