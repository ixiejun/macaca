use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::finance_common::{
    define_finance_command_wrappers, finance_pack_definition, finance_stable_hash,
    FinanceCommandEnvelope, FinanceError, FinancePackDescriptor, FinancePage, FinanceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub use super::finance_portfolio_async::PortfolioReportJob;

pub const FINANCE_PORTFOLIO_PACK_ID: &str = "pack.finance.portfolio.v1";
pub const FINANCE_PORTFOLIO_SERVICE_ID: &str = "service.finance.portfolio";
pub const FINANCE_PORTFOLIO_COMMANDS: &[&str] = &[
    "portfolio.inspect_provider",
    "portfolio.list_accounts",
    "portfolio.get_account",
    "portfolio.list_positions",
    "portfolio.list_lots",
    "portfolio.list_cash_balances",
    "portfolio.list_transactions",
    "portfolio.get_valuation",
    "portfolio.calculate_allocation",
    "portfolio.calculate_exposure",
    "portfolio.calculate_performance",
    "portfolio.summarize_risk",
    "portfolio.run_scenario",
    "portfolio.plan_rebalance_intent",
    "portfolio.rebalance_intent_request",
    "portfolio.generate_report",
    "portfolio.get_artifact_handle",
];

const PORTFOLIO_PERMISSION_SCOPES: &[&str] = &[
    "finance.portfolio.read",
    "finance.portfolio.analytics",
    "finance.portfolio.report",
    "finance.portfolio.intent.write",
];

const PORTFOLIO_AGGREGATION_METADATA: &[(&str, &str)] = &[
    ("accounts", "true"),
    ("positions", "true"),
    ("transactions", "true"),
    ("consent", "required"),
];
const PORTFOLIO_ANALYTICS_METADATA: &[(&str, &str)] = &[
    ("allocation", "true"),
    ("performance", "true"),
    ("risk", "optional"),
    ("advice", "false"),
];
const PORTFOLIO_REPORT_METADATA: &[(&str, &str)] = &[
    ("reports", "true"),
    ("artifacts", "true"),
    ("exports", "approval_required"),
];
const PORTFOLIO_MOCK_METADATA: &[(&str, &str)] = &[
    ("accounts", "synthetic"),
    ("analytics", "synthetic"),
    ("callable", "false"),
];
const PORTFOLIO_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("accounts", "false"),
    ("analytics", "false"),
    ("reports", "false"),
    ("reason", "provider_not_installed"),
];

const PORTFOLIO_PROVIDER_CLASSES: &[FinanceProviderClass<'_>] = &[
    FinanceProviderClass {
        provider_class: "portfolio-aggregation",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PORTFOLIO_AGGREGATION_METADATA,
    },
    FinanceProviderClass {
        provider_class: "portfolio-analytics",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PORTFOLIO_ANALYTICS_METADATA,
    },
    FinanceProviderClass {
        provider_class: "portfolio-reporting",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PORTFOLIO_REPORT_METADATA,
    },
    FinanceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PORTFOLIO_MOCK_METADATA,
    },
    FinanceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: PORTFOLIO_UNAVAILABLE_METADATA,
    },
];

/// Build the portfolio descriptor without binding brokers, custodians, aggregators, or advisers.
pub fn finance_portfolio_pack_definition() -> DomainPackDefinition {
    finance_pack_definition(FinancePackDescriptor {
        pack_id: FINANCE_PORTFOLIO_PACK_ID,
        child_change_id: "openspec:add-pack-finance-portfolio",
        docs_slug: "portfolio",
        sdk_slug: "portfolio",
        service_id: FINANCE_PORTFOLIO_SERVICE_ID,
        commands: FINANCE_PORTFOLIO_COMMANDS,
        permission_scopes: PORTFOLIO_PERMISSION_SCOPES,
        provider_classes: PORTFOLIO_PROVIDER_CLASSES,
        health_probe: "portfolio.inspect_provider",
        unavailable_reason: "finance_portfolio_provider_not_installed",
        replay_schema: "finance.portfolio.replay.v1",
        data_classification: "regulated_portfolio_reference_metadata",
        retention_policy: "account_position_transaction_analytics_report_and_artifact_metadata_by_reference",
        redaction_policy: "credentials_raw_account_numbers_holdings_transactions_provider_payloads_model_dumps_and_unbounded_reports_redacted",
        timeout_ms: 120_000,
        budget_units: 4,
        examples: &[
            "Declare `pack.finance.portfolio.v1` as optional until a portfolio provider is installed.",
            "Use consent-scoped account and analytics references; rebalance intents are plans, not order execution.",
        ],
        migration_notes: &[
            "Portfolio commands become callable only after an approved provider registers matching schemas.",
            "Trading, transfers, custody, suitability decisions, investment advice, and automatic rebalancing execution remain outside this pack.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioScope {
    pub tenant_scope: String,
    pub household_ref: String,
    pub consent_ref: String,
    pub permission_scope: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioProviderCapability {
    pub provider_class: String,
    pub account_types: BTreeSet<String>,
    pub analytics: BTreeSet<String>,
    pub export_formats: BTreeSet<String>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub valuation_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioAttribution {
    pub source_ref: String,
    pub license_class: String,
    pub required_display_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioRedactionPolicy {
    pub policy_ref: String,
    pub redacted_fields: BTreeSet<String>,
    pub export_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioAccount {
    pub account_ref: String,
    pub group_ref: String,
    pub household_ref: String,
    pub account_type: String,
    pub base_currency: String,
    pub masked_identifier_ref: String,
    pub ownership_ref: String,
    pub consent_state: PortfolioConsentState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioConsentState {
    pub state: String,
    pub granted_at_epoch_ms: Option<u64>,
    pub expires_at_epoch_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioInstrumentReference {
    pub instrument_ref: String,
    pub symbol: String,
    pub identifier_hashes: BTreeSet<String>,
    pub asset_class: String,
    pub security_type: String,
    pub currency: String,
    pub exchange_ref: Option<String>,
    pub maturity_date: Option<String>,
    pub expiry_date: Option<String>,
    pub strike_price_micros: Option<i64>,
    pub classification_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioPosition {
    pub position_ref: String,
    pub account_ref: String,
    pub instrument: PortfolioInstrumentReference,
    pub quantity_micros: i64,
    pub market_value_micros: i64,
    pub cost_basis_ref: String,
    pub freshness: PortfolioFreshness,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioLot {
    pub lot_ref: String,
    pub position_ref: String,
    pub opened_at_date: String,
    pub quantity_micros: i64,
    pub cost_basis_micros: i64,
    pub source_evidence_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashBalance {
    pub account_ref: String,
    pub currency: String,
    pub amount_micros: i64,
    pub available_amount_micros: i64,
    pub freshness: PortfolioFreshness,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioValuation {
    pub valuation_ref: String,
    pub account_ref: String,
    pub total_value_micros: i64,
    pub currency: String,
    pub price_source_ref: String,
    pub fx_source_ref: Option<String>,
    pub valued_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioTransaction {
    pub transaction_ref: String,
    pub account_ref: String,
    pub instrument_ref: Option<String>,
    pub activity_type: String,
    pub trade_date: Option<String>,
    pub settle_date: Option<String>,
    pub amount_micros: i64,
    pub quantity_micros: Option<i64>,
    pub price_micros: Option<i64>,
    pub fees_micros: i64,
    pub taxes_micros: i64,
    pub currency: String,
    pub source_evidence_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllocationBucket {
    pub bucket_ref: String,
    pub classification_ref: String,
    pub value_micros: i64,
    pub weight_basis_points: i32,
    pub no_advice_disclaimer_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExposureBucket {
    pub bucket_ref: String,
    pub exposure_class: String,
    pub notional_micros: i64,
    pub methodology: PortfolioMethodology,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioPerformance {
    pub performance_ref: String,
    pub benchmark: BenchmarkReference,
    pub returns: Vec<ReturnPoint>,
    pub methodology: PortfolioMethodology,
    pub no_advice_disclaimer_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkReference {
    pub benchmark_ref: String,
    pub currency: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnPoint {
    pub period_ref: String,
    pub return_basis_points: i32,
    pub cash_flow_treatment: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioRiskSummary {
    pub risk_ref: String,
    pub volatility_basis_points: i32,
    pub drawdown_basis_points: i32,
    pub confidence_class: String,
    pub no_advice_disclaimer_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioScenarioAnalysis {
    pub scenario_ref: String,
    pub assumption_hash: String,
    pub impact_micros: i64,
    pub confidence_class: String,
    pub no_advice_disclaimer_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioMethodology {
    pub methodology_ref: String,
    pub calculation_class: String,
    pub assumption_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalanceIntentPlan {
    pub plan_ref: String,
    pub intents: Vec<RebalanceIntent>,
    pub constraints: Vec<RebalanceConstraint>,
    pub approval_state: String,
    pub no_advice_disclaimer_ref: String,
}

impl RebalanceIntentPlan {
    /// Rebalance plans are bounded intent records and never executable order lists.
    pub fn is_bounded(&self, max_intents: usize) -> bool {
        !self.intents.is_empty() && self.intents.len() <= max_intents
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalanceIntent {
    pub intent_ref: String,
    pub target_ref: String,
    pub drift_basis_points: i32,
    pub tolerance_basis_points: i32,
    pub non_execution_disclaimer_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalanceConstraint {
    pub constraint_ref: String,
    pub constraint_kind: String,
    pub limit_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioReportRequest {
    pub request_ref: String,
    pub report_kind: String,
    pub period_range: String,
    pub currency: String,
    pub redaction: PortfolioRedactionPolicy,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioReport {
    pub report_ref: String,
    pub sections: BTreeSet<String>,
    pub artifact: Option<PortfolioArtifactHandle>,
    pub freshness: PortfolioFreshness,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioArtifactHandle {
    pub artifact_id: String,
    pub export_format: String,
    pub checksum: String,
    pub expires_at_epoch_ms: u64,
    pub retention_policy: String,
    pub access_policy: String,
}

define_finance_command_wrappers!(
    PortfolioInspectProviderCommand,
    PortfolioListAccountsCommand,
    PortfolioGetAccountCommand,
    PortfolioListPositionsCommand,
    PortfolioListLotsCommand,
    PortfolioListCashBalancesCommand,
    PortfolioListTransactionsCommand,
    PortfolioGetValuationCommand,
    PortfolioCalculateAllocationCommand,
    PortfolioCalculateExposureCommand,
    PortfolioCalculatePerformanceCommand,
    PortfolioSummarizeRiskCommand,
    PortfolioRunScenarioCommand,
    PortfolioPlanRebalanceIntentCommand,
    PortfolioRebalanceIntentRequestCommand,
    PortfolioGenerateReportCommand,
    PortfolioGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    StaleData,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioResultEnvelope<T> {
    pub status: PortfolioResultStatus,
    pub data: Option<T>,
    pub page: Option<FinancePage<T>>,
    pub error: Option<FinanceError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortfolioDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub account_hash: String,
    pub instrument_hash: String,
    pub position_hash: String,
    pub transaction_hash: String,
    pub analytics_hash: String,
    pub intent_hash: String,
    pub artifact_hash: String,
}

pub fn finance_portfolio_descriptor_hashes() -> PortfolioDescriptorHashes {
    let freshness = PortfolioFreshness {
        source_timestamp_epoch_ms: 1,
        valuation_timestamp_epoch_ms: Some(1),
        freshness_class: "current".into(),
    };
    let instrument = PortfolioInstrumentReference {
        instrument_ref: "instrument".into(),
        symbol: "MACA".into(),
        identifier_hashes: BTreeSet::from(["identity".into()]),
        asset_class: "equity".into(),
        security_type: "common_stock".into(),
        currency: "USD".into(),
        classification_ref: "sector".into(),
        ..Default::default()
    };
    PortfolioDescriptorHashes {
        command_schema_hash: portfolio_stable_hash(&FINANCE_PORTFOLIO_COMMANDS),
        result_schema_hash: portfolio_stable_hash(&PortfolioResultStatus::Success),
        descriptor_hash: portfolio_stable_hash(&finance_portfolio_pack_definition()),
        provider_capability_hash: portfolio_stable_hash(&PortfolioProviderCapability {
            provider_class: "mock".into(),
            account_types: BTreeSet::from(["brokerage".into()]),
            analytics: BTreeSet::from(["allocation".into(), "performance".into()]),
            export_formats: BTreeSet::from(["json".into()]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        account_hash: portfolio_stable_hash(&PortfolioAccount::default()),
        instrument_hash: portfolio_stable_hash(&instrument),
        position_hash: portfolio_stable_hash(&PortfolioPosition {
            instrument,
            freshness: freshness.clone(),
            ..Default::default()
        }),
        transaction_hash: portfolio_stable_hash(&PortfolioTransaction::default()),
        analytics_hash: portfolio_stable_hash(&PortfolioPerformance {
            returns: vec![ReturnPoint::default()],
            ..Default::default()
        }),
        intent_hash: portfolio_stable_hash(&RebalanceIntentPlan {
            intents: vec![RebalanceIntent::default()],
            ..Default::default()
        }),
        artifact_hash: portfolio_stable_hash(&PortfolioArtifactHandle::default()),
    }
}

pub fn portfolio_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    finance_stable_hash(value)
}
