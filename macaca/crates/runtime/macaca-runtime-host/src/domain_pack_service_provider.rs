//! Built-in domain-pack service adapters for contract-driven applications.
//!
//! Macaca applications must depend on generic OS services instead of forcing
//! the runtime host to know application-specific workflows.  This module keeps
//! that boundary explicit: it exposes provider-neutral `SystemService`
//! implementations for services declared by domain packs, and the Web/CLI host
//! can register those services once during startup.  Applications then request
//! them through manifest service contracts and WASM `service.call` imports.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use macaca_kernel::SystemService;
use macaca_llm::LlmProvider;
use macaca_proto::{
    CleanupPolicy, KernelServiceId, MacacaError, MacacaResult, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, ServiceType, TraceContext,
    TraceSchemaRef,
};
use serde_json::{json, Value};
use tracing::{info, warn};

use crate::{
    finance_live_data::{
        build_finance_http_client, crypto_market_output_from_binance,
        crypto_market_output_from_okx, crypto_news_items_from_rss, crypto_okx_instrument,
        crypto_spot_pair, BinanceTicker24h, OkxTickerEnvelope, CRYPTO_NEWS_RSS_URL,
    },
    finance_llm_analysis_provider::FinanceLlmAnalysisSystemServiceProvider,
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

/// Stable service id exposed by the finance domain pack for quote-like data.
pub const FINANCE_MARKET_DATA_SERVICE_ID: &str = "service.market_data";
/// Stable service id exposed by the finance domain pack for fundamentals.
pub const FINANCE_FINANCIALS_SERVICE_ID: &str = "service.financials";
/// Stable service id exposed by the finance domain pack for summarized news.
pub const FINANCE_NEWS_DIGEST_SERVICE_ID: &str = "service.news_digest";
/// Stable service id exposed by the finance domain pack for model analysis.
pub const FINANCE_LLM_ANALYSIS_SERVICE_ID: &str = "service.llm.analysis";

/// Command accepted by deterministic finance information services.
pub const FINANCE_LOOKUP_COMMAND: &str = "finance.lookup";
/// Command accepted by the LLM-backed finance analysis service.
pub const FINANCE_ANALYZE_COMMAND: &str = "finance.analyze";

/// Common result builder used by all domain-pack service adapters.
///
/// Keeping result construction in one helper makes metadata and cleanup
/// semantics consistent across services.  The metadata is intentionally
/// generic and audit-friendly; it never includes application names or secrets.
pub(crate) fn service_result(
    output: Value,
    trace: TraceContext,
    provider_class: &'static str,
) -> ServiceCallResult {
    let mut metadata = BTreeMap::new();
    metadata.insert("provider_class".into(), provider_class.into());
    metadata.insert("domain_pack".into(), "pack.finance.v1".into());
    ServiceCallResult {
        output,
        trace,
        status: "ok".into(),
        metadata,
        cleanup_hint: Some(CleanupPolicy::None),
    }
}

/// Extract a trace context or fail before provider logic runs.
pub(crate) fn command_trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
    command
        .trace
        .clone()
        .ok_or(ServiceError::MissingTraceContext)
}

/// Extract an already-typed finance symbol from a service payload.
///
/// Macaca deliberately treats symbol extraction as schema validation, not
/// natural-language understanding. Applications, guests, skills, or app-owned
/// coordinators must normalize prompts into typed service arguments before the
/// request crosses the OS service boundary.
pub(crate) fn extract_symbol(payload: &Value) -> ServiceResult<String> {
    let Some((field, raw_symbol)) = ["ticker", "symbol"].into_iter().find_map(|field| {
        payload
            .get(field)
            .and_then(Value::as_str)
            .map(|value| (field, value))
    }) else {
        return Err(ServiceError::InvalidArgument(
            "finance service requires typed `symbol` or `ticker` argument".into(),
        ));
    };
    let symbol = raw_symbol.trim().to_ascii_uppercase();
    if symbol.is_empty()
        || !(1..=12).contains(&symbol.len())
        || !symbol.chars().all(|ch| ch.is_ascii_alphanumeric())
    {
        return Err(ServiceError::InvalidArgument(format!(
            "finance service `{field}` must be a typed alphanumeric symbol, not a prompt"
        )));
    }
    Ok(symbol)
}

/// Return whether the caller is requesting crypto-shaped finance data.
///
/// Crypto remains part of the finance domain pack; this helper only selects a
/// live market-data adapter for generic `asset_class: crypto` requests.  It
/// does not bind the OS to any application, symbol, workflow, or UI surface.
fn is_crypto_asset(payload: &Value) -> bool {
    payload
        .get("asset_class")
        .and_then(Value::as_str)
        .map(|value| value.eq_ignore_ascii_case("crypto"))
        .unwrap_or(false)
}

/// Build a generic descriptor for one domain-pack service.
pub(crate) fn finance_descriptor(service_id: &str, service_kind: &str) -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(service_id),
        ServiceType::new(service_kind),
        TraceSchemaRef::new("trace.domain_pack.finance.v1"),
    );
    descriptor
        .metadata
        .insert("domain_pack".into(), "pack.finance.v1".into());
    descriptor
        .metadata
        .insert("contract_version".into(), "v1".into());
    descriptor
}

/// Deterministic finance data provider used for market, financial, and news services.
///
/// This adapter deliberately does not know about any application.  It is a
/// generic domain-pack provider that emits structured fixture-grade data for
/// contract testing and local development.  Real deployments can replace the
/// same service ids with remote data providers without changing WASM apps.
pub struct FinanceDataSystemServiceProvider {
    descriptor: ServiceDescriptor,
    service_kind: &'static str,
    http_client: reqwest::Client,
}

impl FinanceDataSystemServiceProvider {
    /// Create a market-data provider for `pack.finance.v1`.
    pub fn market_data() -> Self {
        Self::new(FINANCE_MARKET_DATA_SERVICE_ID, "finance.market_data")
    }

    /// Create a financials provider for `pack.finance.v1`.
    pub fn financials() -> Self {
        Self::new(FINANCE_FINANCIALS_SERVICE_ID, "finance.financials")
    }

    /// Create a news-digest provider for `pack.finance.v1`.
    pub fn news_digest() -> Self {
        Self::new(FINANCE_NEWS_DIGEST_SERVICE_ID, "finance.news_digest")
    }

    fn new(service_id: &'static str, service_kind: &'static str) -> Self {
        Self {
            descriptor: finance_descriptor(service_id, service_kind),
            service_kind,
            http_client: build_finance_http_client(),
        }
    }

    async fn output_for(&self, symbol: &str, payload: &Value) -> ServiceResult<Value> {
        match self.service_kind {
            "finance.market_data" if is_crypto_asset(payload) => {
                self.live_crypto_market_data(symbol, payload).await
            }
            "finance.market_data" => Ok(json!({
                "symbol": symbol,
                "asset_class": "equity",
                "currency": "USD",
                "price": 197.23,
                "day_change_percent": 0.84,
                "volume_24h_usd": 5_280_000_000_f64,
                "moving_averages": {
                    "ma_20": 194.10,
                    "ma_50": 189.80,
                    "ma_200": 181.40
                },
                "technicals": {
                    "rsi_14": 55.4,
                    "trend": "up",
                    "volatility": "low",
                    "liquidity": "deep",
                    "support": 190.0,
                    "resistance": 205.0
                },
                "as_of": "fixture.realtime",
                "source": "domain_pack.finance.v1.local_fixture",
                "input": payload,
            })),
            "finance.financials" => Ok(json!({
                "symbol": symbol,
                "revenue_growth_yoy_percent": 6.1,
                "gross_margin_percent": 46.6,
                "free_cash_flow_quality": "strong",
                "debt_risk": "low",
                "source": "domain_pack.finance.v1.local_fixture",
                "input": payload,
            })),
            "finance.news_digest" if is_crypto_asset(payload) => {
                self.live_crypto_news_digest(symbol, payload).await
            }
            _ => Ok(json!({
                "symbol": symbol,
                "asset_class": "equity",
                "sentiment": "mixed_positive",
                "risk_level": "medium",
                "items": [
                    "Product demand remains a monitored driver.",
                    "Macro rates and valuation sensitivity remain key risks.",
                    "Recent coverage emphasizes durable services revenue."
                ],
                "source": "domain_pack.finance.v1.local_fixture",
                "input": payload,
            })),
        }
    }

    /// Fetch live crypto market data through the generic finance-pack provider.
    ///
    /// The method intentionally fails closed instead of silently falling back to
    /// fixture data.  A failed quote should be visible in the host-command trace
    /// as `service_unavailable`; otherwise users could mistake stale synthetic
    /// evidence for a real market snapshot.
    async fn live_crypto_market_data(&self, symbol: &str, payload: &Value) -> ServiceResult<Value> {
        let pair_symbol = crypto_spot_pair(symbol);
        let primary_result = self
            .http_client
            .get("https://api.binance.com/api/v3/ticker/24hr")
            .query(&[("symbol", pair_symbol.as_str())])
            .send()
            .await;

        match primary_result {
            Ok(response) if response.status().is_success() => {
                let ticker = response.json::<BinanceTicker24h>().await.map_err(|error| {
                    ServiceError::ServiceUnavailable(format!(
                        "live crypto market data response could not be decoded for {pair_symbol}: {error}"
                    ))
                })?;
                Ok(crypto_market_output_from_binance(symbol, payload, ticker))
            }
            Ok(response) => {
                let primary_error =
                    format!("binance {pair_symbol} returned HTTP {}", response.status());
                self.live_crypto_market_data_from_okx(symbol, payload, primary_error)
                    .await
            }
            Err(error) => {
                let primary_error = format!("binance {pair_symbol} request failed: {error}");
                self.live_crypto_market_data_from_okx(symbol, payload, primary_error)
                    .await
            }
        }
    }

    /// Fetch live crypto market data from OKX when the primary no-key quote
    /// source is unavailable.
    ///
    /// This keeps the finance domain pack fail-closed while avoiding a single
    /// exchange as a hard dependency.  If both sources fail, the returned error
    /// includes both causes so audit traces explain why no market snapshot was
    /// available.
    async fn live_crypto_market_data_from_okx(
        &self,
        symbol: &str,
        payload: &Value,
        primary_error: String,
    ) -> ServiceResult<Value> {
        let instrument = crypto_okx_instrument(symbol);
        let response = self
            .http_client
            .get("https://www.okx.com/api/v5/market/ticker")
            .query(&[("instId", instrument.as_str())])
            .send()
            .await
            .map_err(|error| {
                ServiceError::ServiceUnavailable(format!(
                    "live crypto market data failed; primary={primary_error}; fallback okx {instrument} request failed: {error}"
                ))
            })?;

        if !response.status().is_success() {
            return Err(ServiceError::ServiceUnavailable(format!(
                "live crypto market data failed; primary={primary_error}; fallback okx {instrument} returned HTTP {}",
                response.status()
            )));
        }

        let envelope = response.json::<OkxTickerEnvelope>().await.map_err(|error| {
            ServiceError::ServiceUnavailable(format!(
                "live crypto market data failed; primary={primary_error}; fallback okx {instrument} response could not be decoded: {error}"
            ))
        })?;
        if envelope.code != "0" {
            return Err(ServiceError::ServiceUnavailable(format!(
                "live crypto market data failed; primary={primary_error}; fallback okx {instrument} returned code {}",
                envelope.code
            )));
        }
        let Some(ticker) = envelope.data.into_iter().next() else {
            return Err(ServiceError::ServiceUnavailable(format!(
                "live crypto market data failed; primary={primary_error}; fallback okx {instrument} returned no ticker data"
            )));
        };
        Ok(crypto_market_output_from_okx(symbol, payload, ticker))
    }

    /// Fetch a live crypto news digest from a public RSS source.
    ///
    /// The output stays within the existing finance-pack `news_digest` shape so
    /// WASM applications do not need to know which publication or wire format
    /// the host used.  When the source is unavailable the service reports an
    /// outage instead of mixing live market data with synthetic headlines.
    async fn live_crypto_news_digest(&self, symbol: &str, payload: &Value) -> ServiceResult<Value> {
        let response = self
            .http_client
            .get(CRYPTO_NEWS_RSS_URL)
            .send()
            .await
            .map_err(|error| {
                ServiceError::ServiceUnavailable(format!(
                    "live crypto news request failed for {symbol}: {error}"
                ))
            })?;

        if !response.status().is_success() {
            return Err(ServiceError::ServiceUnavailable(format!(
                "live crypto news request failed for {symbol}: HTTP {}",
                response.status()
            )));
        }

        let fetched_at = Utc::now().to_rfc3339();
        let feed = response.text().await.map_err(|error| {
            ServiceError::ServiceUnavailable(format!(
                "live crypto news response could not be read for {symbol}: {error}"
            ))
        })?;
        let items = crypto_news_items_from_rss(symbol, &feed)?;

        Ok(json!({
            "symbol": symbol,
            "asset_class": "crypto",
            "sentiment": "news_driven",
            "risk_level": "medium",
            "items": items,
            "as_of": fetched_at,
            "source": "coindesk.public.rss",
            "input": payload,
        }))
    }
}

#[async_trait]
impl SystemService for FinanceDataSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            service_kind = self.service_kind,
            "finance domain-pack data service started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command_trace(&command)?;
        let symbol = extract_symbol(&command.payload)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            symbol = %symbol,
            "finance domain-pack data service accepted command"
        );
        if command.name.as_str() != FINANCE_LOOKUP_COMMAND {
            warn!(
                service_id = %self.descriptor.id,
                command = %command.name,
                trace_id = %trace.trace_id,
                "finance domain-pack data service rejected unsupported command"
            );
            return Err(ServiceError::UnsupportedCommand(command.name.to_string()));
        }
        Ok(service_result(
            self.output_for(&symbol, &command.payload).await?,
            trace,
            "finance_domain_pack_data",
        ))
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            service_kind = self.service_kind,
            "finance domain-pack data service stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            service_kind = self.service_kind,
            "finance domain-pack data service cleaned up"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

/// Started service ids returned by the domain-pack bootstrap boundary.
#[derive(Clone, Debug, Default)]
pub struct DomainPackRuntimeBundle {
    pub started_services: Vec<KernelServiceId>,
}

/// Register and start built-in domain-pack services used by contract apps.
///
/// The function is deliberately generic at the app boundary.  It registers
/// services by domain-pack service id, so any application declaring
/// `pack.finance.v1` can use them without runtime code mentioning that app.
pub async fn bootstrap_builtin_domain_pack_services(
    runtime: Arc<ServiceRuntime>,
    llm: Arc<dyn LlmProvider>,
    trace_prefix: impl Into<String>,
) -> MacacaResult<DomainPackRuntimeBundle> {
    let trace_prefix = trace_prefix.into();
    let services: Vec<(ServiceDescriptor, Arc<dyn SystemService>, &'static str)> = vec![
        (
            finance_descriptor(FINANCE_MARKET_DATA_SERVICE_ID, "finance.market_data"),
            Arc::new(FinanceDataSystemServiceProvider::market_data()),
            "finance-market-data",
        ),
        (
            finance_descriptor(FINANCE_FINANCIALS_SERVICE_ID, "finance.financials"),
            Arc::new(FinanceDataSystemServiceProvider::financials()),
            "finance-financials",
        ),
        (
            finance_descriptor(FINANCE_NEWS_DIGEST_SERVICE_ID, "finance.news_digest"),
            Arc::new(FinanceDataSystemServiceProvider::news_digest()),
            "finance-news-digest",
        ),
        (
            finance_descriptor(FINANCE_LLM_ANALYSIS_SERVICE_ID, "finance.llm_analysis"),
            Arc::new(FinanceLlmAnalysisSystemServiceProvider::new(llm)),
            "finance-llm-analysis",
        ),
    ];
    let mut bundle = DomainPackRuntimeBundle::default();
    for (descriptor, service, trace_suffix) in services {
        let service_id = descriptor.id.clone();
        let trace = TraceContext::new(format!("{trace_prefix}-{trace_suffix}"));
        info!(
            service_id = %service_id,
            trace_id = %trace.trace_id,
            "domain-pack service registering provider"
        );
        runtime
            .register_provider(
                &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                    descriptor, service,
                )),
                ServiceProviderFactoryContext::new(),
            )
            .await
            .map_err(runtime_error)?;
        runtime
            .start(&service_id, trace.clone())
            .await
            .map_err(runtime_error)?;
        info!(
            service_id = %service_id,
            trace_id = %trace.trace_id,
            "domain-pack service started"
        );
        bundle.started_services.push(service_id);
    }
    Ok(bundle)
}

pub(crate) fn service_adapter_error(error: MacacaError) -> ServiceError {
    ServiceError::ServiceUnavailable(error.to_string())
}

fn runtime_error(error: crate::ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}

#[cfg(test)]
mod tests {
    use macaca_proto::ServiceError;
    use serde_json::json;

    use super::extract_symbol;

    #[test]
    fn extract_symbol_accepts_typed_symbol_field() {
        let payload = json!({
            "asset_class": "crypto",
            "symbol": "BTC"
        });

        assert_eq!(extract_symbol(&payload).unwrap(), "BTC");
    }

    #[test]
    fn extract_symbol_rejects_prompt_shaped_symbol_field() {
        let payload = json!({
            "asset_class": "crypto",
            "symbol": "Analyze BTC/USDT now with available skills",
            "input": "ignored"
        });

        assert!(matches!(
            extract_symbol(&payload),
            Err(ServiceError::InvalidArgument(message)) if message.contains("symbol")
        ));
    }

    #[test]
    fn extract_symbol_rejects_untyped_chat_input() {
        let payload = json!({
            "asset_class": "crypto",
            "input": "Analyze SOL signal"
        });

        assert!(matches!(
            extract_symbol(&payload),
            Err(ServiceError::InvalidArgument(message)) if message.contains("symbol")
        ));
    }

    #[test]
    fn extract_symbol_accepts_ticker_alias_as_typed_field() {
        let payload = json!({ "ticker": "AAPL" });

        assert_eq!(extract_symbol(&payload).unwrap(), "AAPL");
    }
}
