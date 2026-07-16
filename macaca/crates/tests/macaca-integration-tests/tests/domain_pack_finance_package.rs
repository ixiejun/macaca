//! Contract tests for optional finance domain-pack registration.
//!
//! Verifies that absent packs leave services unavailable while the optional
//! package can register providers through the generic runtime-host bootstrap.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_domain_pack_finance::finance_domain_pack_registrations;
use macaca_kernel::SystemService;
use macaca_llm::LlmProvider;
use macaca_proto::{
    domain_pack_contract::finance_accounting::{
        FINANCE_ACCOUNTING_COMMANDS, FINANCE_ACCOUNTING_PACK_ID, FINANCE_ACCOUNTING_SERVICE_ID,
    },
    KernelServiceId, LlmMessage, LlmOptions, LlmResponse, MacacaResult, ServiceBusSource,
    ServiceCommand, ServiceCommandName, ServiceDescriptor, ServiceType, TokenUsage, TraceContext,
    TraceSchemaRef,
};
use macaca_runtime_host::{
    bootstrap_domain_pack_services,
    domain_pack_service_provider::DomainPackUnavailableSystemServiceProvider, ServiceRuntime,
    ServiceRuntimeConfig,
};

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

#[tokio::test]
async fn accounting_unavailable_provider_rejects_every_declared_command_without_payload_echo() {
    let provider = DomainPackUnavailableSystemServiceProvider::new(
        ServiceDescriptor::new(
            KernelServiceId::new(FINANCE_ACCOUNTING_SERVICE_ID),
            ServiceType::new("domain_pack.finance.accounting"),
            TraceSchemaRef::new("trace.finance.accounting.v1"),
        ),
        FINANCE_ACCOUNTING_PACK_ID,
        "finance_accounting_provider_not_installed",
    );

    for command in FINANCE_ACCOUNTING_COMMANDS {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({
                    "raw_ledger_row": "must-not-leak",
                    "account_number": "must-not-leak",
                }),
                TraceContext::new(format!("trace-accounting-unavailable-{command}")),
            ))
            .await
            .unwrap();

        assert_eq!(result.status, "unavailable", "{command}");
        assert_eq!(result.output["status"], "unavailable", "{command}");
        assert_eq!(result.output["pack_id"], FINANCE_ACCOUNTING_PACK_ID);
        assert_eq!(result.output["command"], *command);
        assert_eq!(
            result.output["reason_code"],
            "finance_accounting_provider_not_installed"
        );
        assert!(!result.output.to_string().contains("must-not-leak"));
    }
}
