//! Contract tests for optional finance domain-pack registration.
//!
//! Verifies that absent packs leave services unavailable while the optional
//! package can register providers through the generic runtime-host bootstrap.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_domain_pack_finance::finance_domain_pack_registrations;
use macaca_llm::LlmProvider;
use macaca_proto::{
    KernelServiceId, LlmMessage, LlmOptions, LlmResponse, MacacaResult, ServiceBusSource,
    ServiceCommand, ServiceCommandName, TokenUsage, TraceContext,
};
use macaca_runtime_host::{bootstrap_domain_pack_services, ServiceRuntime, ServiceRuntimeConfig};

/// Minimal LLM stub for finance analysis provider registration tests.
struct FixtureLlm;

#[async_trait]
impl LlmProvider for FixtureLlm {
    fn name(&self) -> &str {
        "fixture-llm"
    }

    async fn chat(
        &self,
        _messages: Vec<LlmMessage>,
        _options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        Ok(LlmResponse {
            content: "fixture-analysis".into(),
            reasoning_content: None,
            model: "fixture-llm".into(),
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

fn new_runtime() -> Arc<ServiceRuntime> {
    Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()))
}

#[tokio::test]
async fn absent_finance_pack_leaves_service_unavailable() {
    let runtime = new_runtime();
    let bundle = bootstrap_domain_pack_services(
        Arc::clone(&runtime),
        std::iter::empty(),
        "test-absent-domain-pack",
    )
    .await
    .expect("empty domain-pack bootstrap should succeed");

    assert!(
        bundle.started_services.is_empty(),
        "base bootstrap must not register finance providers"
    );

    let service_id = KernelServiceId::new("service.market_data");
    let trace = TraceContext::new("test-absent-finance-service");
    let command = ServiceCommand::with_trace(
        ServiceCommandName::new("finance.lookup"),
        serde_json::json!({ "symbol": "BTC", "asset_class": "crypto" }),
        trace,
    );

    let result = runtime
        .call(
            &service_id,
            ServiceBusSource::new("domain-pack-finance-test"),
            command,
        )
        .await;

    assert!(
        result.is_err(),
        "absent finance service must return structured unavailable, not success"
    );
}

#[tokio::test]
async fn finance_package_registers_all_contract_services() {
    let runtime = new_runtime();
    let llm: Arc<dyn LlmProvider> = Arc::new(FixtureLlm);
    let registrations = finance_domain_pack_registrations(llm);
    assert_eq!(registrations.len(), 4);

    let bundle = bootstrap_domain_pack_services(
        Arc::clone(&runtime),
        registrations,
        "test-finance-domain-pack",
    )
    .await
    .expect("finance domain-pack bootstrap should succeed");

    assert_eq!(bundle.started_services.len(), 4);
    for service_id in [
        "service.market_data",
        "service.financials",
        "service.news_digest",
        "service.llm.analysis",
    ] {
        assert!(
            bundle
                .started_services
                .iter()
                .any(|started| started.as_str() == service_id),
            "expected finance service `{service_id}` to start"
        );
    }
}
