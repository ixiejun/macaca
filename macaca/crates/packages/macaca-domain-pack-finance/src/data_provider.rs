//! Finance data services for market quotes, fundamentals, and news digests.
//!
//! Implements the Strategy pattern: one `FinanceDataSystemServiceProvider` type
//! serves three service ids (`market_data`, `financials`, `news_digest`) and
//! selects output shape based on `service_kind`.  Live crypto adapters are used
//! only when callers pass typed `asset_class: crypto`; equity paths remain
//! deterministic fixtures suitable for contract testing.

use async_trait::async_trait;
use chrono::Utc;
use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceResult,
};
use serde_json::{json, Value};
use tracing::{info, warn};

use crate::contract::{
    extract_symbol, finance_descriptor, finance_service_result, is_crypto_asset,
    FINANCE_FINANCIALS_SERVICE_ID, FINANCE_LOOKUP_COMMAND, FINANCE_MARKET_DATA_SERVICE_ID,
    FINANCE_NEWS_DIGEST_SERVICE_ID,
};
use crate::live_data::{
    build_finance_http_client, crypto_market_output_from_binance, crypto_market_output_from_okx,
    crypto_news_items_from_rss, crypto_okx_instrument, crypto_spot_pair, BinanceTicker24h,
    CRYPTO_NEWS_RSS_URL, OkxTickerEnvelope,
};

/// Deterministic and live finance data provider for the finance domain pack.
///
/// The adapter is a Bridge between external exchange/RSS sources and the stable
/// JSON contract consumed by WASM applications through `service.call`.
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
        let trace = macaca_runtime_host::domain_pack_service_provider::command_trace(&command)?;
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
        Ok(finance_service_result(
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
