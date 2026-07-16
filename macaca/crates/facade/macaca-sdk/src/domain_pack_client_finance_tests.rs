use macaca_proto::domain_pack_contract::{
    finance_accounting::{FINANCE_ACCOUNTING_PACK_ID, FINANCE_ACCOUNTING_SERVICE_ID},
    finance_crypto::{FINANCE_CRYPTO_PACK_ID, FINANCE_CRYPTO_SERVICE_ID},
    finance_invoice::{FINANCE_INVOICE_PACK_ID, FINANCE_INVOICE_SERVICE_ID},
    finance_market_data::{FINANCE_MARKET_DATA_PACK_ID, FINANCE_MARKET_DATA_SERVICE_ID},
    finance_portfolio::{FINANCE_PORTFOLIO_PACK_ID, FINANCE_PORTFOLIO_SERVICE_ID},
    finance_stock::{FINANCE_STOCK_PACK_ID, FINANCE_STOCK_SERVICE_ID},
};
use macaca_proto::{
    compose_installed_domain_pack_catalog, reference_domain_pack_definitions,
    AppServiceContractConfig,
};

use super::*;

// These tests keep Finance SDK discovery provider-neutral. The SDK reads catalog
// metadata and unavailable diagnostics; it never constructs market data, stock,
// crypto, broker, wallet, exchange, explorer, credential, or entitlement clients.

#[tokio::test]
async fn catalog_client_discovers_finance_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let cases = [
        (
            FINANCE_MARKET_DATA_PACK_ID,
            FINANCE_MARKET_DATA_SERVICE_ID,
            "market_data.get_quote",
            "finance_market_data_provider_not_installed",
            "market-data-realtime",
        ),
        (
            FINANCE_STOCK_PACK_ID,
            FINANCE_STOCK_SERVICE_ID,
            "stock.get_company_profile",
            "finance_stock_provider_not_installed",
            "stock-fundamentals",
        ),
        (
            FINANCE_CRYPTO_PACK_ID,
            FINANCE_CRYPTO_SERVICE_ID,
            "crypto.get_quote",
            "finance_crypto_provider_not_installed",
            "crypto-aggregator",
        ),
        (
            FINANCE_ACCOUNTING_PACK_ID,
            FINANCE_ACCOUNTING_SERVICE_ID,
            "accounting.post_journal",
            "finance_accounting_provider_not_installed",
            "accounting-ledger",
        ),
        (
            FINANCE_PORTFOLIO_PACK_ID,
            FINANCE_PORTFOLIO_SERVICE_ID,
            "portfolio.calculate_allocation",
            "finance_portfolio_provider_not_installed",
            "portfolio-analytics",
        ),
        (
            FINANCE_INVOICE_PACK_ID,
            FINANCE_INVOICE_SERVICE_ID,
            "invoice.issue_invoice",
            "finance_invoice_provider_not_installed",
            "invoice-lifecycle",
        ),
    ];

    for (pack_id, service_id, command, unavailable_reason, provider_class) in cases {
        let inspect = client
            .inspect_pack(&DomainPackInspectCommand::new(pack_id).expect("valid finance id"))
            .await
            .unwrap();
        let pack = inspect.pack.expect("finance descriptor exists");

        assert!(!pack.is_callable());
        assert_eq!(
            pack.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(pack
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|commands| commands.contains(command)));
        assert!(pack
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(pack
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/finance"));
    }
}

#[tokio::test]
async fn catalog_client_reports_finance_unavailable_reasons() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let command = DomainPackResolveCommand {
        declaration: AppServiceContractConfig {
            optional_packs: vec![
                FINANCE_MARKET_DATA_PACK_ID.into(),
                FINANCE_STOCK_PACK_ID.into(),
                FINANCE_CRYPTO_PACK_ID.into(),
                FINANCE_ACCOUNTING_PACK_ID.into(),
                FINANCE_PORTFOLIO_PACK_ID.into(),
                FINANCE_INVOICE_PACK_ID.into(),
            ],
            ..Default::default()
        },
    };

    let result = client.resolve_declaration(&command).await.unwrap();

    for (pack_id, reason) in [
        (
            FINANCE_MARKET_DATA_PACK_ID,
            "finance_market_data_provider_not_installed",
        ),
        (
            FINANCE_STOCK_PACK_ID,
            "finance_stock_provider_not_installed",
        ),
        (
            FINANCE_CRYPTO_PACK_ID,
            "finance_crypto_provider_not_installed",
        ),
        (
            FINANCE_ACCOUNTING_PACK_ID,
            "finance_accounting_provider_not_installed",
        ),
        (
            FINANCE_PORTFOLIO_PACK_ID,
            "finance_portfolio_provider_not_installed",
        ),
        (
            FINANCE_INVOICE_PACK_ID,
            "finance_invoice_provider_not_installed",
        ),
    ] {
        assert!(result
            .effective
            .unresolved_optional_packs
            .contains(&pack_id.to_string()));
        assert_eq!(
            result.effective.unavailable_pack_reasons.get(pack_id),
            Some(&reason.to_string())
        );
    }
}
