//! Live data adapters used by the generic finance domain-pack services.
//!
//! The public finance-pack service contract must stay independent from any one
//! exchange, publication, or application.  This module owns the adapter details
//! for no-key public sources and converts them into the stable JSON shapes that
//! WASM applications already consume through `service.call`.

use std::io::Cursor;
use std::time::Duration;

use chrono::{DateTime, Utc};
use macaca_proto::{ServiceError, ServiceResult};
use rss::Channel;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::warn;

/// No-key public RSS source used for live crypto news summaries.
pub(crate) const CRYPTO_NEWS_RSS_URL: &str =
    "https://www.coindesk.com/arc/outboundfeeds/rss/?outputType=xml";

/// Binance 24-hour ticker response used as the no-key live crypto quote source.
///
/// The external API uses camel-case field names, so the provider keeps a small
/// DTO at the boundary and immediately converts it into the stable finance-pack
/// JSON shape consumed by applications.  Keeping the DTO private prevents the
/// public OS contract from inheriting an exchange-specific schema.
#[derive(Debug, Deserialize)]
pub(crate) struct BinanceTicker24h {
    #[serde(rename = "symbol")]
    pub(crate) pair_symbol: String,
    #[serde(rename = "lastPrice")]
    pub(crate) last_price: String,
    #[serde(rename = "priceChangePercent")]
    pub(crate) price_change_percent: String,
    #[serde(rename = "weightedAvgPrice")]
    pub(crate) weighted_avg_price: String,
    #[serde(rename = "highPrice")]
    pub(crate) high_price: String,
    #[serde(rename = "lowPrice")]
    pub(crate) low_price: String,
    #[serde(rename = "volume")]
    pub(crate) base_volume: String,
    #[serde(rename = "quoteVolume")]
    pub(crate) quote_volume: String,
    #[serde(rename = "closeTime")]
    pub(crate) close_time_millis: i64,
}

/// Build the reusable HTTP client used by live finance-pack adapters.
///
/// Market-data endpoints are public internet resources, so the client honors
/// the host's normal proxy configuration. Local developer networks often need
/// that proxy to reach exchange APIs; disabling it would turn a healthy data
/// source into a false runtime outage.
pub(crate) fn build_finance_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("macaca-finance-domain-pack/0.1")
        .build()
        .unwrap_or_else(|error| {
            warn!(
                error = %error,
                "finance domain-pack HTTP client fell back to default transport"
            );
            reqwest::Client::new()
        })
}

/// Convert a generic finance symbol into the Binance spot pair used for quotes.
pub(crate) fn crypto_spot_pair(symbol: &str) -> String {
    let normalized = symbol.trim().to_ascii_uppercase();
    if normalized.ends_with("USDT") {
        normalized
    } else {
        format!("{normalized}USDT")
    }
}

/// Build the stable finance-pack crypto market snapshot from live exchange data.
pub(crate) fn crypto_market_output_from_binance(
    symbol: &str,
    payload: &Value,
    ticker: BinanceTicker24h,
) -> Value {
    let price = parse_exchange_f64(&ticker.last_price);
    let day_change_percent = parse_exchange_f64(&ticker.price_change_percent);
    let weighted_avg_price = parse_exchange_f64(&ticker.weighted_avg_price);
    let support = parse_exchange_f64(&ticker.low_price);
    let resistance = parse_exchange_f64(&ticker.high_price);
    let base_volume = parse_exchange_f64(&ticker.base_volume);
    let quote_volume = parse_exchange_f64(&ticker.quote_volume);

    json!({
        "symbol": symbol,
        "asset_class": "crypto",
        "currency": "USD",
        "quote_symbol": "USDT",
        "exchange_pair": ticker.pair_symbol,
        "price": price,
        "day_change_percent": day_change_percent,
        "volume_24h": base_volume,
        "volume_24h_usd": quote_volume,
        "weighted_avg_price": weighted_avg_price,
        "technicals": {
            "trend": crypto_trend(day_change_percent),
            "volatility": crypto_volatility(day_change_percent),
            "support": support,
            "resistance": resistance
        },
        "as_of": exchange_timestamp(ticker.close_time_millis),
        "source": "binance.spot.public.24hr",
        "input": payload,
    })
}

/// Extract a compact set of market-relevant crypto headlines from an RSS feed.
///
/// The finance-pack contract exposes news as a normalized digest rather than an
/// RSS document. This helper keeps the lossy transformation explicit: only the
/// headline text crosses the WASM boundary, while links and provider markup stay
/// outside the portable application payload.
pub(crate) fn crypto_news_items_from_rss(symbol: &str, xml: &str) -> ServiceResult<Vec<String>> {
    let channel = Channel::read_from(Cursor::new(xml.as_bytes())).map_err(|error| {
        ServiceError::ServiceUnavailable(format!(
            "live crypto news feed could not be decoded: {error}"
        ))
    })?;
    let symbol_lc = symbol.to_ascii_lowercase();
    let asset_keyword = match symbol_lc.as_str() {
        "btc" | "btcusdt" => "bitcoin",
        "eth" | "ethusdt" => "ethereum",
        value => value,
    };
    let mut items: Vec<String> = channel
        .items()
        .iter()
        .filter_map(|item| item.title().map(str::trim))
        .filter(|title| {
            let normalized = title.to_ascii_lowercase();
            normalized.contains(asset_keyword) || normalized.contains("crypto")
        })
        .take(5)
        .map(str::to_string)
        .collect();

    if items.is_empty() {
        items = channel
            .items()
            .iter()
            .filter_map(|item| item.title().map(str::trim))
            .take(5)
            .map(str::to_string)
            .collect();
    }

    if items.is_empty() {
        return Err(ServiceError::ServiceUnavailable(
            "live crypto news feed contained no usable headlines".into(),
        ));
    }
    Ok(items)
}

/// Parse exchange numeric strings without leaking stringly-typed data inward.
fn parse_exchange_f64(value: &str) -> Option<f64> {
    value.parse::<f64>().ok()
}

/// Classify short-term trend from the exchange-reported 24-hour percentage move.
fn crypto_trend(change_percent: Option<f64>) -> &'static str {
    match change_percent {
        Some(value) if value > 0.15 => "up",
        Some(value) if value < -0.15 => "down",
        Some(_) => "flat",
        None => "unknown",
    }
}

/// Classify volatility using only observed 24-hour change data from the source.
fn crypto_volatility(change_percent: Option<f64>) -> &'static str {
    match change_percent.map(f64::abs) {
        Some(value) if value >= 4.0 => "high",
        Some(value) if value >= 1.0 => "medium",
        Some(_) => "low",
        None => "unknown",
    }
}

/// Convert Binance's close timestamp into an RFC3339 audit timestamp.
fn exchange_timestamp(close_time_millis: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(close_time_millis)
        .map(|timestamp| timestamp.to_rfc3339())
        .unwrap_or_else(|| "exchange.timestamp_unavailable".into())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{crypto_market_output_from_binance, crypto_news_items_from_rss, BinanceTicker24h};

    #[test]
    fn finance_market_data_maps_live_crypto_exchange_payload() {
        let payload = json!({
            "symbol": "BTC",
            "asset_class": "crypto",
            "lookup": "market_snapshot"
        });
        let output = crypto_market_output_from_binance(
            "BTC",
            &payload,
            BinanceTicker24h {
                pair_symbol: "BTCUSDT".into(),
                last_price: "79727.66000000".into(),
                price_change_percent: "-1.726".into(),
                weighted_avg_price: "79609.38636486".into(),
                high_price: "81152.00000000".into(),
                low_price: "78754.65000000".into(),
                base_volume: "16348.92621000".into(),
                quote_volume: "1301527983.30244020".into(),
                close_time_millis: 1_778_753_908_002,
            },
        );

        assert_eq!(output["symbol"], json!("BTC"));
        assert_eq!(output["asset_class"], json!("crypto"));
        assert_eq!(output["source"], json!("binance.spot.public.24hr"));
        assert_eq!(output["exchange_pair"], json!("BTCUSDT"));
        assert_eq!(output["price"], json!(79727.66));
        assert_eq!(output["day_change_percent"], json!(-1.726));
        assert_eq!(output["technicals"]["trend"], json!("down"));
        assert_eq!(output["technicals"]["volatility"], json!("medium"));
        assert_eq!(output["technicals"]["support"], json!(78754.65));
        assert_eq!(output["technicals"]["resistance"], json!(81152.0));
    }

    #[test]
    fn finance_news_maps_live_crypto_rss_headlines() {
        let feed = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>Crypto News</title>
    <item><title>Bitcoin ETF flows rise as traders watch macro data</title></item>
    <item><title>Ethereum liquidity improves during market rebound</title></item>
    <item><title>Crypto derivatives positioning stays elevated</title></item>
  </channel>
</rss>"#;

        let items = crypto_news_items_from_rss("BTC", feed).unwrap();

        assert_eq!(items.len(), 2);
        assert!(items[0].contains("Bitcoin ETF"));
        assert!(items[1].contains("Crypto derivatives"));
    }
}
