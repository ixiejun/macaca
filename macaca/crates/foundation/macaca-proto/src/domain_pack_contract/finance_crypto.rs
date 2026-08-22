use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::finance_common::{
    define_finance_command_wrappers, finance_pack_definition, finance_stable_hash,
    FinanceCommandEnvelope, FinanceError, FinancePackDescriptor, FinancePage, FinanceProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const FINANCE_CRYPTO_PACK_ID: &str = "pack.finance.crypto.v1";
pub const FINANCE_CRYPTO_SERVICE_ID: &str = "service.finance.crypto";
pub const FINANCE_CRYPTO_COMMANDS: &[&str] = &[
    "crypto.inspect_provider",
    "crypto.search_assets",
    "crypto.get_asset",
    "crypto.get_token_reference",
    "crypto.search_market_pairs",
    "crypto.get_quote",
    "crypto.get_trade",
    "crypto.get_bars",
    "crypto.get_snapshot",
    "crypto.get_supply_metrics",
    "crypto.inspect_market_status",
    "crypto.inspect_public_address_balance",
    "crypto.inspect_freshness",
    "crypto.get_artifact_handle",
];

pub use super::finance_crypto_trace::FINANCE_CRYPTO_TRACE_EVENTS;
const CRYPTO_PERMISSION_SCOPES: &[&str] = &[
    "crypto.provider.inspect",
    "crypto.asset.search",
    "crypto.asset.read",
    "crypto.token.read",
    "crypto.market_pair.search",
    "crypto.quote.read",
    "crypto.trade.read",
    "crypto.bars.read",
    "crypto.snapshot.read",
    "crypto.supply.read",
    "crypto.market_status.read",
    "crypto.public_address.read",
    "crypto.freshness.read",
    "crypto.artifact.read",
];
const CRYPTO_AGGREGATOR_METADATA: &[(&str, &str)] = &[
    ("assets", "true"),
    ("quotes", "true"),
    ("supply", "true"),
    ("attribution", "required"),
];
const CRYPTO_EXCHANGE_METADATA: &[(&str, &str)] = &[
    ("pairs", "true"),
    ("trades", "true"),
    ("bars", "true"),
    ("orders", "false"),
];
const CRYPTO_CHAIN_REFERENCE_METADATA: &[(&str, &str)] = &[
    ("token_references", "true"),
    ("address_diagnostics", "privacy_gated"),
    ("oracle_feeds", "true"),
    ("signing", "false"),
];
const CRYPTO_MOCK_METADATA: &[(&str, &str)] = &[
    ("assets", "synthetic"),
    ("pairs", "synthetic"),
    ("address_diagnostics", "synthetic"),
    ("callable", "false"),
];
const CRYPTO_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("assets", "false"),
    ("pairs", "false"),
    ("address_diagnostics", "false"),
    ("reason", "provider_not_installed"),
];
const CRYPTO_PROVIDER_CLASSES: &[FinanceProviderClass<'_>] = &[
    FinanceProviderClass {
        provider_class: "crypto-aggregator",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CRYPTO_AGGREGATOR_METADATA,
    },
    FinanceProviderClass {
        provider_class: "crypto-exchange-data",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CRYPTO_EXCHANGE_METADATA,
    },
    FinanceProviderClass {
        provider_class: "crypto-chain-reference",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CRYPTO_CHAIN_REFERENCE_METADATA,
    },
    FinanceProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CRYPTO_MOCK_METADATA,
    },
    FinanceProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: CRYPTO_UNAVAILABLE_METADATA,
    },
];

/// Build the crypto descriptor without binding exchanges, explorers, wallets, or signing clients.
pub fn finance_crypto_pack_definition() -> DomainPackDefinition {
    finance_pack_definition(FinancePackDescriptor {
        pack_id: FINANCE_CRYPTO_PACK_ID,
        child_change_id: "openspec:add-pack-finance-crypto",
        docs_slug: "crypto",
        sdk_slug: "crypto",
        service_id: FINANCE_CRYPTO_SERVICE_ID,
        commands: FINANCE_CRYPTO_COMMANDS,
        permission_scopes: CRYPTO_PERMISSION_SCOPES,
        provider_classes: CRYPTO_PROVIDER_CLASSES,
        health_probe: "crypto.inspect_provider",
        unavailable_reason: "finance_crypto_provider_not_installed",
        replay_schema: "finance.crypto.replay.v1",
        data_classification: "licensed_crypto_reference_metadata",
        retention_policy: "asset_token_chain_pair_freshness_address_reference_and_artifact_metadata_by_reference",
        redaction_policy: "credentials_private_keys_seed_phrases_signatures_wallets_holdings_raw_chain_payloads_provider_payloads_and_unbounded_crypto_data_redacted",
        timeout_ms: 90_000,
        budget_units: 3,
        examples: &[
            "Declare `pack.finance.crypto.v1` as optional until a crypto data provider is installed.",
            "Use asset handles, token references, privacy-gated address references, freshness, attribution, and artifacts instead of raw chain or provider payloads.",
        ],
        migration_notes: &[
            "Crypto commands become callable only after an approved crypto data provider registers matching schemas.",
            "Wallet custody, signing, transfers, swaps, staking, bridges, orders, and DeFi execution belong to other service boundaries.",
        ],
    })
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoScope {
    pub tenant_scope: String,
    pub chain_scope: String,
    pub privacy_profile: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoProviderCapability {
    pub provider_class: String,
    pub chains: BTreeSet<String>,
    pub exchanges: BTreeSet<String>,
    pub feature_flags: BTreeSet<String>,
    pub state: DomainPackProviderCapabilityState,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoAsset {
    pub asset_id: String,
    pub symbol: String,
    pub asset_class: String,
    pub identity_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenReference {
    pub token_ref: String,
    pub chain_id: String,
    pub contract_address_hash: String,
    pub decimals: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainNetworkReference {
    pub chain_id: String,
    pub network_class: String,
    pub finality_class: String,
    pub explorer_available: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoExchangeVenue {
    pub venue_id: String,
    pub venue_class: String,
    pub region_class: String,
    pub license_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoMarketPair {
    pub pair_id: String,
    pub base_asset_id: String,
    pub quote_asset_id: String,
    pub venue_id: String,
    pub precision_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoQuote {
    pub pair_id: String,
    pub bid_price_micros: i64,
    pub ask_price_micros: i64,
    pub source_timestamp_epoch_ms: u64,
    pub freshness: CryptoFreshness,
    pub attribution: CryptoAttribution,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoTrade {
    pub pair_id: String,
    pub price_micros: i64,
    pub size_base_micros: i64,
    pub trade_timestamp_epoch_ms: u64,
    pub attribution: CryptoAttribution,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoBar {
    pub start_epoch_ms: u64,
    pub end_epoch_ms: u64,
    pub open_micros: i64,
    pub high_micros: i64,
    pub low_micros: i64,
    pub close_micros: i64,
    pub volume_base_micros: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoBarSeries {
    pub pair_id: String,
    pub interval: String,
    pub bars: Vec<CryptoBar>,
    pub freshness: CryptoFreshness,
    pub attribution: CryptoAttribution,
}

impl CryptoBarSeries {
    pub fn is_bounded(&self, max_bars: usize) -> bool {
        !self.bars.is_empty() && self.bars.len() <= max_bars
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoSnapshot {
    pub asset_id: String,
    pub quote_hash: String,
    pub trade_hash: String,
    pub supply_hash: String,
    pub freshness: CryptoFreshness,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoSupplyMetric {
    pub asset_id: String,
    pub metric_class: String,
    pub value_ref: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicAddressBalanceReference {
    pub address_hash: String,
    pub chain_id: String,
    pub balance_ref: String,
    pub privacy_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoOracleFeedReference {
    pub feed_ref: String,
    pub chain_id: String,
    pub pair_id: String,
    pub freshness_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoFreshness {
    pub source_timestamp_epoch_ms: u64,
    pub cache_timestamp_epoch_ms: Option<u64>,
    pub chain_height: Option<u64>,
    pub finality_class: Option<String>,
    pub stale_reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoAttribution {
    pub source_ref: String,
    pub license_class: String,
    pub required_display_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoCursor {
    pub cursor_hash: String,
    pub request_hash: String,
    pub expires_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub retention_class: String,
    pub expires_at_epoch_ms: u64,
}

define_finance_command_wrappers!(
    CryptoInspectProviderCommand,
    CryptoSearchAssetsCommand,
    CryptoGetAssetCommand,
    CryptoGetTokenReferenceCommand,
    CryptoSearchMarketPairsCommand,
    CryptoGetQuoteCommand,
    CryptoGetTradeCommand,
    CryptoGetBarsCommand,
    CryptoGetSnapshotCommand,
    CryptoGetSupplyMetricsCommand,
    CryptoInspectMarketStatusCommand,
    CryptoInspectPublicAddressBalanceCommand,
    CryptoInspectFreshnessCommand,
    CryptoGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CryptoResultStatus {
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
    AssetAmbiguous,
    AssetNotFound,
    ChainUnsupported,
    ExchangeUnsupported,
    PairUnsupported,
    IntervalUnsupported,
    AddressPolicyDenied,
    Web3Unavailable,
    RangeTooLarge,
    Quota,
    Timeout,
    Cancellation,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoResultEnvelope<T> {
    pub status: CryptoResultStatus,
    pub data: Option<T>,
    pub page: Option<FinancePage<T>>,
    pub error: Option<FinanceError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CryptoDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub asset_identity_hash: String,
    pub token_reference_hash: String,
    pub chain_network_hash: String,
    pub venue_pair_hash: String,
    pub quote_trade_hash: String,
    pub bar_snapshot_hash: String,
    pub supply_metric_hash: String,
    pub public_address_reference_hash: String,
    pub oracle_feed_hash: String,
    pub freshness_hash: String,
    pub attribution_hash: String,
    pub cursor_hash: String,
    pub artifact_handle_hash: String,
    pub event_cursor_hash: String,
    pub redaction_metadata_hash: String,
}

pub fn finance_crypto_descriptor_hashes() -> CryptoDescriptorHashes {
    let freshness = CryptoFreshness {
        source_timestamp_epoch_ms: 1,
        cache_timestamp_epoch_ms: Some(2),
        chain_height: Some(100),
        finality_class: Some("finalized".into()),
        stale_reason: None,
    };
    let attribution = CryptoAttribution {
        source_ref: "source:synthetic".into(),
        license_class: "synthetic".into(),
        required_display_ref: Some("display".into()),
    };
    CryptoDescriptorHashes {
        command_schema_hash: crypto_stable_hash(&FINANCE_CRYPTO_COMMANDS),
        result_schema_hash: crypto_stable_hash(&CryptoResultStatus::Success),
        descriptor_hash: crypto_stable_hash(&finance_crypto_pack_definition()),
        provider_capability_hash: crypto_stable_hash(&CryptoProviderCapability {
            provider_class: "mock".into(),
            chains: BTreeSet::from(["synthetic-chain".into()]),
            exchanges: BTreeSet::from(["synthetic-exchange".into()]),
            feature_flags: BTreeSet::from(["quotes".into(), "tokens".into()]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        asset_identity_hash: crypto_stable_hash(&CryptoAsset {
            asset_id: "asset".into(),
            symbol: "MACA".into(),
            asset_class: "token".into(),
            identity_hash: "identity".into(),
        }),
        token_reference_hash: crypto_stable_hash(&TokenReference {
            token_ref: "token".into(),
            chain_id: "synthetic-chain".into(),
            contract_address_hash: "address".into(),
            decimals: 18,
        }),
        chain_network_hash: crypto_stable_hash(&ChainNetworkReference {
            chain_id: "synthetic-chain".into(),
            network_class: "test".into(),
            finality_class: "finalized".into(),
            explorer_available: false,
        }),
        venue_pair_hash: crypto_stable_hash(&CryptoMarketPair {
            pair_id: "pair".into(),
            base_asset_id: "asset".into(),
            quote_asset_id: "usd".into(),
            venue_id: "synthetic-exchange".into(),
            precision_class: "standard".into(),
        }),
        quote_trade_hash: crypto_stable_hash(&CryptoTrade {
            pair_id: "pair".into(),
            price_micros: 100,
            size_base_micros: 1,
            trade_timestamp_epoch_ms: 1,
            attribution: attribution.clone(),
        }),
        bar_snapshot_hash: crypto_stable_hash(&CryptoBarSeries {
            pair_id: "pair".into(),
            interval: "1d".into(),
            bars: vec![CryptoBar {
                start_epoch_ms: 1,
                end_epoch_ms: 2,
                open_micros: 100,
                high_micros: 110,
                low_micros: 90,
                close_micros: 105,
                volume_base_micros: 10,
            }],
            freshness: freshness.clone(),
            attribution: attribution.clone(),
        }),
        supply_metric_hash: crypto_stable_hash(&CryptoSupplyMetric {
            asset_id: "asset".into(),
            metric_class: "circulating".into(),
            value_ref: "value".into(),
            source_ref: "source".into(),
        }),
        public_address_reference_hash: crypto_stable_hash(&PublicAddressBalanceReference {
            address_hash: "address".into(),
            chain_id: "synthetic-chain".into(),
            balance_ref: "balance".into(),
            privacy_profile: "hashed".into(),
        }),
        oracle_feed_hash: crypto_stable_hash(&CryptoOracleFeedReference {
            feed_ref: "feed".into(),
            chain_id: "synthetic-chain".into(),
            pair_id: "pair".into(),
            freshness_class: "finalized".into(),
        }),
        freshness_hash: crypto_stable_hash(&freshness),
        attribution_hash: crypto_stable_hash(&attribution),
        cursor_hash: crypto_stable_hash(&CryptoCursor {
            cursor_hash: "cursor".into(),
            request_hash: "request".into(),
            expires_at_epoch_ms: 10,
        }),
        artifact_handle_hash: crypto_stable_hash(&CryptoArtifactHandle {
            artifact_id: "artifact".into(),
            artifact_kind: "page".into(),
            retention_class: "short".into(),
            expires_at_epoch_ms: 10,
        }),
        event_cursor_hash: crypto_stable_hash(&"event:crypto"),
        redaction_metadata_hash: crypto_stable_hash(&FinanceError {
            code: "unavailable".into(),
            message: "finance crypto provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("finance_crypto_provider_not_installed".into()),
        }),
    }
}

pub fn crypto_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    finance_stable_hash(value)
}
