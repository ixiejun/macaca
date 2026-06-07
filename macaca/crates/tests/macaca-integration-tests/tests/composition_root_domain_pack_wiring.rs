//! Integration tests for web composition-root domain-pack catalog wiring.
//!
//! Verifies that the optional finance package catalog and provider factory are
//! reachable through the same helpers used by web bootstrap, without starting a
//! full HTTP server.

use std::sync::Arc;

use macaca_app::{expand_service_capabilities, AppServiceContractConfig, DomainPackCatalog};
use macaca_domain_pack_finance::{
    finance_domain_pack_registrations, finance_pack_catalog_definition, FINANCE_PACK_ID,
};
use macaca_runtime_host::{bootstrap_domain_pack_services, ServiceRuntime, ServiceRuntimeConfig};

/// Mirror the web composition-root catalog assembly for integration coverage.
fn web_installed_catalog() -> macaca_app::SharedDomainPackCatalog {
    macaca_app::compose_installed_domain_pack_catalog([finance_pack_catalog_definition()])
}

#[test]
fn web_catalog_resolves_finance_pack_for_manifest_use_packs() {
    let catalog = web_installed_catalog();
    assert!(
        catalog.resolve(FINANCE_PACK_ID).is_some(),
        "installed catalog must contain finance pack metadata"
    );

    let declaration = AppServiceContractConfig {
        use_packs: vec![FINANCE_PACK_ID.into()],
        ..Default::default()
    };
    let expanded = expand_service_capabilities(Some(&declaration), catalog.as_ref());
    assert!(
        expanded.unresolved_packs.is_empty(),
        "finance pack should expand to concrete service ids"
    );
    assert!(expanded.services.contains("service.market_data"));
    assert!(expanded.services.contains("service.llm.analysis"));
}

#[tokio::test]
async fn web_provider_registrations_start_all_finance_services() {
    struct FixtureLlm;

    #[async_trait::async_trait]
    impl macaca_sdk::llm::LlmProvider for FixtureLlm {
        fn name(&self) -> &str {
            "fixture-llm"
        }

        async fn chat(
            &self,
            _messages: Vec<macaca_proto::LlmMessage>,
            _options: &macaca_proto::LlmOptions,
        ) -> macaca_proto::MacacaResult<macaca_proto::LlmResponse> {
            Ok(macaca_proto::LlmResponse {
                content: "fixture".into(),
                reasoning_content: None,
                model: "fixture-llm".into(),
                usage: macaca_proto::TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let llm: Arc<dyn macaca_sdk::llm::LlmProvider> = Arc::new(FixtureLlm);
    let registrations = finance_domain_pack_registrations(llm);
    assert_eq!(registrations.len(), 4);

    let bundle = bootstrap_domain_pack_services(
        Arc::clone(&runtime),
        registrations,
        "integration-web-domain-pack-wiring",
    )
    .await
    .expect("finance domain-pack bootstrap should succeed");

    assert_eq!(bundle.started_services.len(), 4);
}
