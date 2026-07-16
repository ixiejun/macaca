use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::finance_accounting::*;
use super::finance_crypto::*;
use super::finance_invoice::*;
use super::finance_market_data::*;
use super::finance_portfolio::*;
use super::finance_stock::*;
use super::*;

// Finance pack tests validate provider-neutral contract shape only. They do not
// contact exchanges, data vendors, brokers, wallets, explorers, or credential
// stores, and fixtures intentionally use synthetic handles instead of prices or
// licensed provider payloads.

#[test]
fn finance_descriptors_are_discoverable_and_not_callable() {
    let cases = [
        (
            finance_market_data_pack_definition(),
            FINANCE_MARKET_DATA_PACK_ID,
            FINANCE_MARKET_DATA_SERVICE_ID,
            FINANCE_MARKET_DATA_COMMANDS,
            "finance_market_data_provider_not_installed",
            "market-data-realtime",
            "market_data.get_quote",
        ),
        (
            finance_stock_pack_definition(),
            FINANCE_STOCK_PACK_ID,
            FINANCE_STOCK_SERVICE_ID,
            FINANCE_STOCK_COMMANDS,
            "finance_stock_provider_not_installed",
            "stock-fundamentals",
            "stock.get_company_profile",
        ),
        (
            finance_crypto_pack_definition(),
            FINANCE_CRYPTO_PACK_ID,
            FINANCE_CRYPTO_SERVICE_ID,
            FINANCE_CRYPTO_COMMANDS,
            "finance_crypto_provider_not_installed",
            "crypto-aggregator",
            "crypto.get_quote",
        ),
        (
            finance_accounting_pack_definition(),
            FINANCE_ACCOUNTING_PACK_ID,
            FINANCE_ACCOUNTING_SERVICE_ID,
            FINANCE_ACCOUNTING_COMMANDS,
            "finance_accounting_provider_not_installed",
            "accounting-ledger",
            "accounting.post_journal",
        ),
        (
            finance_portfolio_pack_definition(),
            FINANCE_PORTFOLIO_PACK_ID,
            FINANCE_PORTFOLIO_SERVICE_ID,
            FINANCE_PORTFOLIO_COMMANDS,
            "finance_portfolio_provider_not_installed",
            "portfolio-analytics",
            "portfolio.calculate_allocation",
        ),
        (
            finance_invoice_pack_definition(),
            FINANCE_INVOICE_PACK_ID,
            FINANCE_INVOICE_SERVICE_ID,
            FINANCE_INVOICE_COMMANDS,
            "finance_invoice_provider_not_installed",
            "invoice-lifecycle",
            "invoice.issue_invoice",
        ),
    ];

    for (definition, pack_id, service_id, commands, unavailable_reason, provider_class, command) in
        cases
    {
        assert_eq!(definition.pack_id, pack_id);
        assert!(!definition.is_callable());
        assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
        assert_eq!(
            definition.metadata.parent_pack_id.as_deref(),
            Some("pack.finance.v1")
        );
        assert_eq!(
            definition.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(definition
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/finance"));
        assert!(definition
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|schemas| schemas.contains(command)));

        let descriptor_commands = definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .expect("finance descriptor exposes command schemas");
        for expected in commands {
            assert!(
                descriptor_commands.contains(*expected),
                "missing command {expected}"
            );
        }
    }
}

#[test]
fn industrial_catalog_uses_specialized_finance_descriptors() {
    let definitions = industrial_reference_domain_pack_definitions();
    let ids = definitions
        .iter()
        .map(|definition| definition.pack_id.as_str())
        .collect::<BTreeSet<_>>();
    assert!(ids.contains(FINANCE_MARKET_DATA_PACK_ID));
    assert!(!ids.contains("pack.finance.market-data.v1"));

    let market_data = find_pack(&definitions, FINANCE_MARKET_DATA_PACK_ID);
    let stock = find_pack(&definitions, FINANCE_STOCK_PACK_ID);
    let crypto = find_pack(&definitions, FINANCE_CRYPTO_PACK_ID);
    let accounting = find_pack(&definitions, FINANCE_ACCOUNTING_PACK_ID);
    let portfolio = find_pack(&definitions, FINANCE_PORTFOLIO_PACK_ID);
    let invoice = find_pack(&definitions, FINANCE_INVOICE_PACK_ID);

    assert_eq!(
        market_data.metadata.diagnostics.unavailable_reason,
        "finance_market_data_provider_not_installed"
    );
    assert!(market_data
        .metadata
        .service_command_schemas
        .get(FINANCE_MARKET_DATA_SERVICE_ID)
        .is_some_and(|commands| commands.contains("market_data.get_bars")));
    assert_eq!(
        stock
            .metadata
            .provider_descriptors
            .get("stock-fundamentals")
            .and_then(|descriptor| descriptor.metadata.get("fundamentals"))
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        crypto
            .metadata
            .provider_descriptors
            .get("crypto-chain-reference")
            .and_then(|descriptor| descriptor.metadata.get("signing"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        accounting
            .metadata
            .provider_descriptors
            .get("accounting-write")
            .and_then(|descriptor| descriptor.metadata.get("journal_posting"))
            .map(String::as_str),
        Some("approval_required")
    );
    assert_eq!(
        portfolio
            .metadata
            .provider_descriptors
            .get("portfolio-analytics")
            .and_then(|descriptor| descriptor.metadata.get("advice"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        invoice
            .metadata
            .provider_descriptors
            .get("invoice-delivery")
            .and_then(|descriptor| descriptor.metadata.get("recipient_policy"))
            .map(String::as_str),
        Some("required")
    );
}

#[test]
fn finance_command_and_result_dtos_are_serde_compatible() {
    let envelope = FinanceCommandEnvelope {
        subject_ref: "finance:subject".into(),
        parameters: BTreeMap::from([("mode".into(), "synthetic".into())]),
        cursor: None,
        page_size: Some(20),
        idempotency_key: Some("idem-finance".into()),
    };

    let values = [
        serde_json::to_value(MarketDataGetQuoteCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(StockGetCompanyProfileCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(CryptoGetQuoteCommand { request: envelope }).unwrap(),
        serde_json::to_value(AccountingPostJournalCommand {
            request: FinanceCommandEnvelope::default(),
        })
        .unwrap(),
        serde_json::to_value(PortfolioCalculateAllocationCommand {
            request: FinanceCommandEnvelope::default(),
        })
        .unwrap(),
        serde_json::to_value(InvoiceIssueInvoiceCommand {
            request: FinanceCommandEnvelope::default(),
        })
        .unwrap(),
        serde_json::to_value(AccountingReportRequest {
            request_ref: "report-request".into(),
            basis: "accrual".into(),
            period_range: "2026-Q2".into(),
            dimensions: vec![AccountingDimension {
                dimension_ref: "department".into(),
                dimension_kind: "cost_center".into(),
                value_ref: "engineering".into(),
            }],
            currency: "USD".into(),
            pagination: AccountingPaginationMetadata {
                next_cursor: Some("cursor".into()),
                page_size: 100,
                truncated: false,
            },
            async_metadata: Some(AccountingAsyncMetadata {
                job_ref: "job".into(),
                state: "completed".into(),
                submitted_at_epoch_ms: 1,
                result_artifact_ref: Some("artifact".into()),
                replay_pointer: "replay".into(),
            }),
        })
        .unwrap(),
        serde_json::to_value(TrialBalanceReport {
            report_ref: "trial-balance".into(),
            rows: vec![AccountingReportLine::default()],
            basis: "accrual".into(),
            pagination: AccountingPaginationMetadata::default(),
            async_metadata: Some(AccountingAsyncMetadata::default()),
            freshness: AccountingFreshness::default(),
            attribution: AccountingAttribution::default(),
        })
        .unwrap(),
        serde_json::to_value(BalanceSheetReport {
            report_ref: "balance-sheet".into(),
            rows: vec![AccountingReportLine::default()],
            basis: "accrual".into(),
            pagination: AccountingPaginationMetadata::default(),
            async_metadata: Some(AccountingAsyncMetadata::default()),
            freshness: AccountingFreshness::default(),
            attribution: AccountingAttribution::default(),
        })
        .unwrap(),
        serde_json::to_value(ProfitLossReport {
            report_ref: "profit-loss".into(),
            rows: vec![AccountingReportLine::default()],
            basis: "accrual".into(),
            pagination: AccountingPaginationMetadata::default(),
            async_metadata: Some(AccountingAsyncMetadata::default()),
            freshness: AccountingFreshness::default(),
            attribution: AccountingAttribution::default(),
        })
        .unwrap(),
        serde_json::to_value(CashFlowReport {
            report_ref: "cash-flow".into(),
            rows: vec![AccountingReportLine::default()],
            basis: "cash".into(),
            pagination: AccountingPaginationMetadata::default(),
            async_metadata: Some(AccountingAsyncMetadata::default()),
            freshness: AccountingFreshness::default(),
            attribution: AccountingAttribution::default(),
        })
        .unwrap(),
        serde_json::to_value(MarketDataResultEnvelope::<MarketQuote> {
            status: MarketDataResultStatus::LicenseDenied,
            data: None,
            page: None,
            error: Some(FinanceError {
                code: "license_denied".into(),
                message: "synthetic denial".into(),
                retryable: false,
                trace_safe_detail: Some("missing_entitlement".into()),
            }),
        })
        .unwrap(),
        serde_json::to_value(StockResultEnvelope::<CompanyProfile> {
            status: StockResultStatus::RestatementConflict,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(CryptoResultEnvelope::<CryptoQuote> {
            status: CryptoResultStatus::AddressPolicyDenied,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(AccountingResultEnvelope::<JournalEntry> {
            status: AccountingResultStatus::Unavailable,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(PortfolioResultEnvelope::<PortfolioPosition> {
            status: PortfolioResultStatus::StaleData,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(InvoiceResultEnvelope::<InvoiceRecord> {
            status: InvoiceResultStatus::Conflict,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
    ];

    assert!(values.iter().all(|value| value.is_object()));
}

#[test]
fn finance_descriptor_hashes_are_stable_and_distinct() {
    let hash_groups = [
        hash_values(&finance_market_data_descriptor_hashes()),
        hash_values(&finance_stock_descriptor_hashes()),
        hash_values(&finance_crypto_descriptor_hashes()),
        hash_values(&finance_accounting_descriptor_hashes()),
        hash_values(&finance_portfolio_descriptor_hashes()),
        hash_values(&finance_invoice_descriptor_hashes()),
    ];

    for hashes in hash_groups {
        let unique = hashes.into_iter().collect::<BTreeSet<_>>();
        assert!(unique.len() >= 10);
        assert!(unique.iter().all(|hash| !hash.is_empty()));
    }
}

#[test]
fn finance_validation_helpers_are_provider_neutral() {
    let session = MarketSession {
        venue_id: "synthetic".into(),
        session_date: "2026-06-29".into(),
        state: "open".into(),
        opens_at_epoch_ms: 1_000,
        closes_at_epoch_ms: 2_000,
    };
    assert!(session.contains_epoch_ms(1_500));

    let market_bars = MarketBarSeries {
        bars: vec![MarketBar::default()],
        ..Default::default()
    };
    assert!(market_bars.is_bounded(1));

    let screen = StockScreenQuery {
        query_ref: "screen".into(),
        filters: BTreeMap::from([("sector".into(), "synthetic".into())]),
        max_results: 25,
    };
    assert!(screen.is_bounded(50, 2));

    let crypto_bars = CryptoBarSeries {
        bars: vec![CryptoBar::default()],
        ..Default::default()
    };
    assert!(crypto_bars.is_bounded(1));

    let journal_plan = JournalEntryPlan {
        lines: vec![
            JournalLine {
                debit_micros: 100,
                ..Default::default()
            },
            JournalLine {
                credit_micros: 100,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert!(journal_plan.balances());

    let chart = ChartOfAccounts {
        accounts: vec![AccountHandle::default()],
        ..Default::default()
    };
    assert!(chart.is_bounded(5));

    let intent_plan = RebalanceIntentPlan {
        intents: vec![RebalanceIntent::default()],
        ..Default::default()
    };
    assert!(intent_plan.is_bounded(2));

    let invoice_plan = InvoiceDraftPlan {
        lines: vec![InvoiceLine {
            quantity_micros: 2_000_000,
            unit_price_micros: 1_500_000,
            ..Default::default()
        }],
        totals: InvoiceTotals {
            subtotal_micros: 3_000_000,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(invoice_plan.totals_match());
}

#[test]
fn invalid_finance_descriptor_is_rejected() {
    let mut invalid = finance_market_data_pack_definition();
    invalid.pack_id = "pack.finance.market.data.v2".into();
    assert!(DomainPackDefinitionSpec.validate(&invalid).is_err());
}

fn find_pack<'a>(
    definitions: &'a [DomainPackDefinition],
    pack_id: &str,
) -> &'a DomainPackDefinition {
    definitions
        .iter()
        .find(|definition| definition.pack_id == pack_id)
        .expect("industrial catalog includes specialized finance descriptor")
}

fn hash_values<T: Serialize>(value: &T) -> Vec<String> {
    serde_json::to_value(value)
        .expect("descriptor hash DTO is serializable")
        .as_object()
        .expect("descriptor hash DTO serializes as an object")
        .values()
        .map(|value| {
            value
                .as_str()
                .expect("descriptor hash fields are strings")
                .to_string()
        })
        .collect()
}
