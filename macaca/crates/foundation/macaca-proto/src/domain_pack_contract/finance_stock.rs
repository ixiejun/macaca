use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::finance_common::{
    define_finance_command_wrappers, finance_pack_definition, finance_stable_hash,
    FinanceCommandEnvelope, FinanceError, FinancePackDescriptor, FinancePage, FinanceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const FINANCE_STOCK_PACK_ID: &str = "pack.finance.stock.v1";
pub const FINANCE_STOCK_SERVICE_ID: &str = "service.finance.stock";

pub const FINANCE_STOCK_COMMANDS: &[&str] = &[
    "stock.inspect_provider",
    "stock.search_equities",
    "stock.get_equity",
    "stock.get_company_profile",
    "stock.get_listing",
    "stock.get_fundamentals",
    "stock.get_financial_statements",
    "stock.get_corporate_events",
    "stock.screen_equities",
    "stock.create_universe",
    "stock.plan_quote_handoff",
    "stock.inspect_freshness",
    "stock.get_artifact_handle",
];

const STOCK_PERMISSION_SCOPES: &[&str] = &[
    "stock.provider.inspect",
    "stock.equity.search",
    "stock.equity.read",
    "stock.company.read",
    "stock.listing.read",
    "stock.fundamentals.read",
    "stock.statements.read",
    "stock.corporate_events.read",
    "stock.screen",
    "stock.universe",
    "stock.quote_handoff",
    "stock.freshness.read",
    "stock.artifact.read",
];

const STOCK_FUNDAMENTALS_METADATA: &[(&str, &str)] = &[
    ("profiles", "true"),
    ("fundamentals", "true"),
    ("statements", "true"),
    ("restatements", "true"),
];
const STOCK_FILINGS_METADATA: &[(&str, &str)] = &[
    ("filings", "true"),
    ("facts", "true"),
    ("source_periods", "true"),
    ("raw_filings", "false"),
];
const STOCK_SCREENING_METADATA: &[(&str, &str)] = &[
    ("screeners", "true"),
    ("universes", "true"),
    ("quote_handoff", "true"),
    ("personal_watchlists", "false"),
];
const STOCK_MOCK_METADATA: &[(&str, &str)] = &[
    ("profiles", "synthetic"),
    ("facts", "synthetic"),
    ("screeners", "synthetic"),
    ("callable", "false"),
];
const STOCK_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("profiles", "false"),
    ("facts", "false"),
    ("screeners", "false"),
    ("reason", "provider_not_installed"),
];

const STOCK_PROVIDER_CLASSES: &[FinanceProviderClass<'_>] = &[
    FinanceProviderClass {
        provider_class: "stock-fundamentals",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: STOCK_FUNDAMENTALS_METADATA,
    },
    FinanceProviderClass {
        provider_class: "stock-filings",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: STOCK_FILINGS_METADATA,
    },
    FinanceProviderClass {
        provider_class: "stock-screening",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: STOCK_SCREENING_METADATA,
    },
    FinanceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: STOCK_MOCK_METADATA,
    },
    FinanceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: STOCK_UNAVAILABLE_METADATA,
    },
];

/// Build the stock descriptor without binding filing, fundamentals, screener, or quote vendors.
pub fn finance_stock_pack_definition() -> DomainPackDefinition {
    finance_pack_definition(FinancePackDescriptor {
        pack_id: FINANCE_STOCK_PACK_ID,
        child_change_id: "openspec:add-pack-finance-stock",
        docs_slug: "stock",
        sdk_slug: "stock",
        service_id: FINANCE_STOCK_SERVICE_ID,
        commands: FINANCE_STOCK_COMMANDS,
        permission_scopes: STOCK_PERMISSION_SCOPES,
        provider_classes: STOCK_PROVIDER_CLASSES,
        health_probe: "stock.inspect_provider",
        unavailable_reason: "finance_stock_provider_not_installed",
        replay_schema: "finance.stock.replay.v1",
        data_classification: "licensed_stock_reference_metadata",
        retention_policy: "equity_company_filing_fact_screen_universe_and_artifact_metadata_by_reference",
        redaction_policy: "credentials_accounts_holdings_watchlists_raw_filings_provider_payloads_and_unbounded_financial_datasets_redacted",
        timeout_ms: 90_000,
        budget_units: 3,
        examples: &[
            "Declare `pack.finance.stock.v1` as optional until a stock provider is installed.",
            "Use equity handles, fact hashes, restatement labels, and quote handoff references instead of raw filings or market prices.",
        ],
        migration_notes: &[
            "Stock commands become callable only after an approved stock data provider registers matching schemas.",
            "Trading, portfolio, accounting, tax, and personal watchlist state remain outside this read-only pack.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockScope {
    pub tenant_scope: String,
    pub region_scope: String,
    pub entitlement_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockProviderCapability {
    pub provider_class: String,
    pub exchanges: BTreeSet<String>,
    pub identifier_types: BTreeSet<String>,
    pub feature_flags: BTreeSet<String>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquityInstrument {
    pub equity_id: String,
    pub symbol: String,
    pub exchange_id: String,
    pub company_ref: String,
    pub identity_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanyProfile {
    pub company_ref: String,
    pub display_name_ref: String,
    pub country_code: String,
    pub sector_ref: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockListing {
    pub listing_id: String,
    pub equity_id: String,
    pub exchange_id: String,
    pub currency: String,
    pub listing_status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinancialStatementPeriod {
    pub period_id: String,
    pub fiscal_year: i32,
    pub fiscal_period: String,
    pub statement_type: String,
    pub source_form_class: String,
    pub restatement_state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinancialFact {
    pub fact_id: String,
    pub concept_ref: String,
    pub period_id: String,
    pub unit: String,
    pub value_ref: String,
    pub source_handle: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FundamentalMetric {
    pub metric_id: String,
    pub metric_class: String,
    pub value_ref: String,
    pub freshness: StockFreshness,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockDividend {
    pub event_id: String,
    pub equity_id: String,
    pub ex_date: String,
    pub cash_amount_micros: i64,
    pub currency: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockSplit {
    pub event_id: String,
    pub equity_id: String,
    pub effective_date: String,
    pub ratio_numerator: u32,
    pub ratio_denominator: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockEarningsEvent {
    pub event_id: String,
    pub equity_id: String,
    pub fiscal_period: String,
    pub reported_at_epoch_ms: u64,
    pub source_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalystDatasetReference {
    pub dataset_ref: String,
    pub license_class: String,
    pub attribution_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockScreenQuery {
    pub query_ref: String,
    pub filters: BTreeMap<String, String>,
    pub max_results: u32,
}

impl StockScreenQuery {
    pub fn is_bounded(&self, max_results: u32, max_filters: usize) -> bool {
        self.max_results > 0 && self.max_results <= max_results && self.filters.len() <= max_filters
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockUniverse {
    pub universe_id: String,
    pub source_query_hash: String,
    pub equity_count: u32,
    pub membership_cursor_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
    pub restatement_state: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockAttribution {
    pub source_ref: String,
    pub license_class: String,
    pub required_display_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockCursor {
    pub cursor_hash: String,
    pub request_hash: String,
    pub expires_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub retention_class: String,
    pub expires_at_epoch_ms: u64,
}

define_finance_command_wrappers!(
    StockInspectProviderCommand,
    StockSearchEquitiesCommand,
    StockGetEquityCommand,
    StockGetCompanyProfileCommand,
    StockGetListingCommand,
    StockGetFundamentalsCommand,
    StockGetFinancialStatementsCommand,
    StockGetCorporateEventsCommand,
    StockScreenEquitiesCommand,
    StockCreateUniverseCommand,
    StockPlanQuoteHandoffCommand,
    StockInspectFreshnessCommand,
    StockGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleData,
    SchemaMismatch,
    ProviderAttributionRequired,
    LicenseDenied,
    SymbolAmbiguous,
    SymbolNotFound,
    EquityUnsupported,
    ExchangeUnsupported,
    FilingUnavailable,
    MetricUnsupported,
    PeriodUnsupported,
    RestatementConflict,
    ScreenUnsupported,
    RangeTooLarge,
    Quota,
    Timeout,
    Cancellation,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockResultEnvelope<T> {
    pub status: StockResultStatus,
    pub data: Option<T>,
    pub page: Option<FinancePage<T>>,
    pub error: Option<FinanceError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StockDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub equity_identity_hash: String,
    pub company_profile_hash: String,
    pub listing_hash: String,
    pub statement_period_hash: String,
    pub financial_fact_hash: String,
    pub metric_hash: String,
    pub corporate_event_hash: String,
    pub analyst_dataset_hash: String,
    pub screen_query_hash: String,
    pub universe_hash: String,
    pub quote_handoff_hash: String,
    pub freshness_hash: String,
    pub attribution_hash: String,
    pub cursor_hash: String,
    pub artifact_handle_hash: String,
    pub event_cursor_hash: String,
    pub redaction_metadata_hash: String,
}

pub fn finance_stock_descriptor_hashes() -> StockDescriptorHashes {
    let freshness = StockFreshness {
        source_timestamp_epoch_ms: 1,
        cache_timestamp_epoch_ms: Some(2),
        freshness_class: "current".into(),
        restatement_state: Some("original".into()),
    };
    StockDescriptorHashes {
        command_schema_hash: stock_stable_hash(&FINANCE_STOCK_COMMANDS),
        result_schema_hash: stock_stable_hash(&StockResultStatus::Success),
        descriptor_hash: stock_stable_hash(&finance_stock_pack_definition()),
        provider_capability_hash: stock_stable_hash(&StockProviderCapability {
            provider_class: "mock".into(),
            exchanges: BTreeSet::from(["synthetic".into()]),
            identifier_types: BTreeSet::from(["ticker".into(), "company_ref".into()]),
            feature_flags: BTreeSet::from(["fundamentals".into(), "screen".into()]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        equity_identity_hash: stock_stable_hash(&EquityInstrument {
            equity_id: "equity".into(),
            symbol: "MACA".into(),
            exchange_id: "synthetic".into(),
            company_ref: "company".into(),
            identity_hash: "identity".into(),
        }),
        company_profile_hash: stock_stable_hash(&CompanyProfile {
            company_ref: "company".into(),
            display_name_ref: "display".into(),
            country_code: "US".into(),
            sector_ref: "sector".into(),
            redaction_class: "public".into(),
        }),
        listing_hash: stock_stable_hash(&StockListing {
            listing_id: "listing".into(),
            equity_id: "equity".into(),
            exchange_id: "synthetic".into(),
            currency: "USD".into(),
            listing_status: "active".into(),
        }),
        statement_period_hash: stock_stable_hash(&FinancialStatementPeriod {
            period_id: "period".into(),
            fiscal_year: 2026,
            fiscal_period: "FY".into(),
            statement_type: "income".into(),
            source_form_class: "annual".into(),
            restatement_state: "original".into(),
        }),
        financial_fact_hash: stock_stable_hash(&FinancialFact {
            fact_id: "fact".into(),
            concept_ref: "revenue".into(),
            period_id: "period".into(),
            unit: "USD".into(),
            value_ref: "value".into(),
            source_handle: "source".into(),
        }),
        metric_hash: stock_stable_hash(&FundamentalMetric {
            metric_id: "metric".into(),
            metric_class: "ratio".into(),
            value_ref: "value".into(),
            freshness: freshness.clone(),
        }),
        corporate_event_hash: stock_stable_hash(&StockDividend {
            event_id: "event".into(),
            equity_id: "equity".into(),
            ex_date: "2026-06-29".into(),
            cash_amount_micros: 1,
            currency: "USD".into(),
        }),
        analyst_dataset_hash: stock_stable_hash(&AnalystDatasetReference {
            dataset_ref: "dataset".into(),
            license_class: "synthetic".into(),
            attribution_required: true,
        }),
        screen_query_hash: stock_stable_hash(&StockScreenQuery {
            query_ref: "query".into(),
            filters: BTreeMap::from([("sector".into(), "synthetic".into())]),
            max_results: 10,
        }),
        universe_hash: stock_stable_hash(&StockUniverse {
            universe_id: "universe".into(),
            source_query_hash: "query".into(),
            equity_count: 10,
            membership_cursor_hash: "cursor".into(),
        }),
        quote_handoff_hash: stock_stable_hash(&"market_data:get_quote"),
        freshness_hash: stock_stable_hash(&freshness),
        attribution_hash: stock_stable_hash(&StockAttribution {
            source_ref: "source:synthetic".into(),
            license_class: "synthetic".into(),
            required_display_ref: Some("display".into()),
        }),
        cursor_hash: stock_stable_hash(&StockCursor {
            cursor_hash: "cursor".into(),
            request_hash: "request".into(),
            expires_at_epoch_ms: 10,
        }),
        artifact_handle_hash: stock_stable_hash(&StockArtifactHandle {
            artifact_id: "artifact".into(),
            artifact_kind: "statement".into(),
            retention_class: "short".into(),
            expires_at_epoch_ms: 10,
        }),
        event_cursor_hash: stock_stable_hash(&"event:stock"),
        redaction_metadata_hash: stock_stable_hash(&FinanceError {
            code: "unavailable".into(),
            message: "finance stock provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("finance_stock_provider_not_installed".into()),
        }),
    }
}

pub fn stock_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    finance_stable_hash(value)
}
