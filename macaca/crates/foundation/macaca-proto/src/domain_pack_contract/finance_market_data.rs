use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::finance_common::{
    define_finance_command_wrappers, finance_pack_definition, finance_stable_hash,
    FinanceCommandEnvelope, FinanceError, FinancePackDescriptor, FinancePage, FinanceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const FINANCE_MARKET_DATA_PACK_ID: &str = "pack.finance.market.data.v1";
pub const FINANCE_MARKET_DATA_SERVICE_ID: &str = "service.finance.market_data";

/// Descriptor-owned command schema names for read-only market data.
pub const FINANCE_MARKET_DATA_COMMANDS: &[&str] = &[
    "market_data.inspect_provider",
    "market_data.search_instruments",
    "market_data.get_instrument",
    "market_data.get_quote",
    "market_data.get_trade",
    "market_data.get_bars",
    "market_data.get_snapshot",
    "market_data.get_corporate_actions",
    "market_data.inspect_market_status",
    "market_data.inspect_freshness",
    "market_data.get_artifact_handle",
];

const MARKET_DATA_PERMISSION_SCOPES: &[&str] = &[
    "market_data.provider.inspect",
    "market_data.instrument.search",
    "market_data.instrument.read",
    "market_data.quote.read",
    "market_data.trade.read",
    "market_data.bars.read",
    "market_data.snapshot.read",
    "market_data.corporate_actions.read",
    "market_data.market_status.read",
    "market_data.freshness.read",
    "market_data.artifact.read",
];

const REALTIME_FEED_METADATA: &[(&str, &str)] = &[
    ("quotes", "true"),
    ("trades", "true"),
    ("freshness", "real_time"),
    ("attribution", "required"),
];
const HISTORICAL_FEED_METADATA: &[(&str, &str)] = &[
    ("bars", "true"),
    ("corporate_actions", "true"),
    ("adjustments", "true"),
    ("pagination", "cursor"),
];
const MARKET_STATUS_METADATA: &[(&str, &str)] = &[
    ("venues", "true"),
    ("sessions", "true"),
    ("calendars", "true"),
    ("freshness", "delayed_or_realtime"),
];
const MARKET_DATA_MOCK_METADATA: &[(&str, &str)] = &[
    ("quotes", "synthetic"),
    ("bars", "synthetic"),
    ("attribution", "synthetic"),
    ("callable", "false"),
];
const MARKET_DATA_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("quotes", "false"),
    ("bars", "false"),
    ("corporate_actions", "false"),
    ("reason", "provider_not_installed"),
];

const MARKET_DATA_PROVIDER_CLASSES: &[FinanceProviderClass<'_>] = &[
    FinanceProviderClass {
        provider_class: "market-data-realtime",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: REALTIME_FEED_METADATA,
    },
    FinanceProviderClass {
        provider_class: "market-data-history",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: HISTORICAL_FEED_METADATA,
    },
    FinanceProviderClass {
        provider_class: "market-status",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MARKET_STATUS_METADATA,
    },
    FinanceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MARKET_DATA_MOCK_METADATA,
    },
    FinanceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: MARKET_DATA_UNAVAILABLE_METADATA,
    },
];

/// Build the descriptor without binding Polygon, Alpaca, Nasdaq, Finnhub, or exchange feeds.
pub fn finance_market_data_pack_definition() -> DomainPackDefinition {
    finance_pack_definition(FinancePackDescriptor {
        pack_id: FINANCE_MARKET_DATA_PACK_ID,
        child_change_id: "openspec:add-pack-finance-market-data",
        docs_slug: "market-data",
        sdk_slug: "market_data",
        service_id: FINANCE_MARKET_DATA_SERVICE_ID,
        commands: FINANCE_MARKET_DATA_COMMANDS,
        permission_scopes: MARKET_DATA_PERMISSION_SCOPES,
        provider_classes: MARKET_DATA_PROVIDER_CLASSES,
        health_probe: "market_data.inspect_provider",
        unavailable_reason: "finance_market_data_provider_not_installed",
        replay_schema: "finance.market_data.replay.v1",
        data_classification: "licensed_market_reference_metadata",
        retention_policy: "handles_hashes_attribution_freshness_cursors_and_artifacts_by_reference",
        redaction_policy: "credentials_accounts_holdings_raw_provider_payloads_licensed_feeds_and_unbounded_market_data_redacted",
        timeout_ms: 60_000,
        budget_units: 3,
        examples: &[
            "Declare `pack.finance.market.data.v1` as optional until an entitled market data provider is installed.",
            "Use instrument handles, freshness, attribution, cursors, and artifacts instead of provider-native payloads.",
        ],
        migration_notes: &[
            "Market data commands become callable only after an approved service provider registers matching schemas.",
            "Trading, investment advice, brokerage workflow, and provider-native pagination remain outside this read-only pack.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketDataScope {
    pub tenant_scope: String,
    pub dataset_scope: String,
    pub entitlement_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketDataProviderCapability {
    pub provider_class: String,
    pub asset_classes: BTreeSet<String>,
    pub venues: BTreeSet<String>,
    pub interval_support: BTreeSet<String>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentHandle {
    pub instrument_id: String,
    pub identity_hash: String,
    pub scope: MarketDataScope,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentIdentity {
    pub symbol: String,
    pub asset_class: String,
    pub venue_id: String,
    pub currency: String,
    pub identifiers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketVenue {
    pub venue_id: String,
    pub venue_class: String,
    pub country_code: String,
    pub timezone: String,
    pub license_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSession {
    pub venue_id: String,
    pub session_date: String,
    pub state: String,
    pub opens_at_epoch_ms: u64,
    pub closes_at_epoch_ms: u64,
}

impl MarketSession {
    /// Return whether a timestamp is inside the bounded session window.
    pub fn contains_epoch_ms(&self, epoch_ms: u64) -> bool {
        self.opens_at_epoch_ms <= epoch_ms && epoch_ms <= self.closes_at_epoch_ms
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketQuote {
    pub instrument: InstrumentHandle,
    pub bid_price_micros: i64,
    pub ask_price_micros: i64,
    pub bid_size: u64,
    pub ask_size: u64,
    pub quote_timestamp_epoch_ms: u64,
    pub freshness: MarketDataFreshness,
    pub attribution: MarketDataAttribution,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketTrade {
    pub instrument: InstrumentHandle,
    pub price_micros: i64,
    pub size: u64,
    pub trade_timestamp_epoch_ms: u64,
    pub correction_state: String,
    pub attribution: MarketDataAttribution,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketBar {
    pub start_epoch_ms: u64,
    pub end_epoch_ms: u64,
    pub open_micros: i64,
    pub high_micros: i64,
    pub low_micros: i64,
    pub close_micros: i64,
    pub volume: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketBarSeries {
    pub instrument: InstrumentHandle,
    pub interval: String,
    pub adjustment_policy: String,
    pub bars: Vec<MarketBar>,
    pub freshness: MarketDataFreshness,
    pub attribution: MarketDataAttribution,
}

impl MarketBarSeries {
    pub fn is_bounded(&self, max_bars: usize) -> bool {
        !self.bars.is_empty() && self.bars.len() <= max_bars
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSnapshot {
    pub instrument: InstrumentHandle,
    pub quote_hash: String,
    pub trade_hash: String,
    pub latest_bar_hash: String,
    pub freshness: MarketDataFreshness,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorporateAction {
    pub action_id: String,
    pub instrument: InstrumentHandle,
    pub action_kind: String,
    pub effective_date: String,
    pub adjustment_hash: String,
    pub attribution: MarketDataAttribution,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketDataFreshness {
    pub provider_timestamp_epoch_ms: u64,
    pub exchange_timestamp_epoch_ms: Option<u64>,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub freshness_class: String,
    pub stale_reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketDataAttribution {
    pub source_ref: String,
    pub license_class: String,
    pub redistribution_policy: String,
    pub required_display_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketDataCursor {
    pub cursor_hash: String,
    pub request_hash: String,
    pub expires_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketDataArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub retention_class: String,
    pub expires_at_epoch_ms: u64,
}

define_finance_command_wrappers!(
    MarketDataInspectProviderCommand,
    MarketDataSearchInstrumentsCommand,
    MarketDataGetInstrumentCommand,
    MarketDataGetQuoteCommand,
    MarketDataGetTradeCommand,
    MarketDataGetBarsCommand,
    MarketDataGetSnapshotCommand,
    MarketDataGetCorporateActionsCommand,
    MarketDataInspectMarketStatusCommand,
    MarketDataInspectFreshnessCommand,
    MarketDataGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketDataResultStatus {
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
    ExchangeUnsupported,
    AssetClassUnsupported,
    RangeTooLarge,
    IntervalUnsupported,
    AdjustmentUnsupported,
    Quota,
    Timeout,
    Cancellation,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketDataResultEnvelope<T> {
    pub status: MarketDataResultStatus,
    pub data: Option<T>,
    pub page: Option<FinancePage<T>>,
    pub error: Option<FinanceError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketDataDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub instrument_identity_hash: String,
    pub venue_session_hash: String,
    pub request_hash: String,
    pub quote_trade_hash: String,
    pub bar_snapshot_hash: String,
    pub corporate_action_hash: String,
    pub freshness_hash: String,
    pub attribution_hash: String,
    pub cursor_hash: String,
    pub artifact_handle_hash: String,
    pub event_cursor_hash: String,
    pub redaction_metadata_hash: String,
}

pub fn finance_market_data_descriptor_hashes() -> MarketDataDescriptorHashes {
    let scope = MarketDataScope {
        tenant_scope: "tenant".into(),
        dataset_scope: "delayed".into(),
        entitlement_class: "synthetic".into(),
    };
    let handle = InstrumentHandle {
        instrument_id: "instrument".into(),
        identity_hash: "identity".into(),
        scope,
    };
    let attribution = MarketDataAttribution {
        source_ref: "source:synthetic".into(),
        license_class: "synthetic".into(),
        redistribution_policy: "none".into(),
        required_display_ref: Some("display:synthetic".into()),
    };
    let freshness = MarketDataFreshness {
        provider_timestamp_epoch_ms: 1,
        exchange_timestamp_epoch_ms: Some(1),
        cache_timestamp_epoch_ms: Some(2),
        freshness_class: "delayed".into(),
        stale_reason: None,
    };
    let bar = MarketBar {
        start_epoch_ms: 1,
        end_epoch_ms: 2,
        open_micros: 100,
        high_micros: 110,
        low_micros: 90,
        close_micros: 105,
        volume: 10,
    };
    MarketDataDescriptorHashes {
        command_schema_hash: market_data_stable_hash(&FINANCE_MARKET_DATA_COMMANDS),
        result_schema_hash: market_data_stable_hash(&MarketDataResultStatus::Success),
        descriptor_hash: market_data_stable_hash(&finance_market_data_pack_definition()),
        provider_capability_hash: market_data_stable_hash(&MarketDataProviderCapability {
            provider_class: "mock".into(),
            asset_classes: BTreeSet::from(["equity".into(), "fund".into()]),
            venues: BTreeSet::from(["synthetic".into()]),
            interval_support: BTreeSet::from(["1d".into(), "1h".into()]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        instrument_identity_hash: market_data_stable_hash(&InstrumentIdentity {
            symbol: "MACA".into(),
            asset_class: "equity".into(),
            venue_id: "synthetic".into(),
            currency: "USD".into(),
            identifiers: BTreeMap::from([("isin".into(), "SYNTHETIC".into())]),
        }),
        venue_session_hash: market_data_stable_hash(&MarketSession {
            venue_id: "synthetic".into(),
            session_date: "2026-06-29".into(),
            state: "open".into(),
            opens_at_epoch_ms: 1,
            closes_at_epoch_ms: 10,
        }),
        request_hash: market_data_stable_hash(&FinanceCommandEnvelope::default()),
        quote_trade_hash: market_data_stable_hash(&MarketTrade {
            instrument: handle.clone(),
            price_micros: 105,
            size: 10,
            trade_timestamp_epoch_ms: 2,
            correction_state: "official".into(),
            attribution: attribution.clone(),
        }),
        bar_snapshot_hash: market_data_stable_hash(&MarketBarSeries {
            instrument: handle.clone(),
            interval: "1d".into(),
            adjustment_policy: "split_adjusted".into(),
            bars: vec![bar],
            freshness: freshness.clone(),
            attribution: attribution.clone(),
        }),
        corporate_action_hash: market_data_stable_hash(&CorporateAction {
            action_id: "action".into(),
            instrument: handle,
            action_kind: "split".into(),
            effective_date: "2026-06-29".into(),
            adjustment_hash: "adjustment".into(),
            attribution,
        }),
        freshness_hash: market_data_stable_hash(&freshness),
        attribution_hash: market_data_stable_hash(&"attribution:synthetic"),
        cursor_hash: market_data_stable_hash(&MarketDataCursor {
            cursor_hash: "cursor".into(),
            request_hash: "request".into(),
            expires_at_epoch_ms: 10,
        }),
        artifact_handle_hash: market_data_stable_hash(&MarketDataArtifactHandle {
            artifact_id: "artifact".into(),
            artifact_kind: "page".into(),
            retention_class: "short".into(),
            expires_at_epoch_ms: 10,
        }),
        event_cursor_hash: market_data_stable_hash(&"event:market-data"),
        redaction_metadata_hash: market_data_stable_hash(&FinanceError {
            code: "unavailable".into(),
            message: "finance market data provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("finance_market_data_provider_not_installed".into()),
        }),
    }
}

pub fn market_data_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    finance_stable_hash(value)
}
