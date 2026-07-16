# Finance Crypto Pack Research

## Purpose

This note records supplier/API research, supplier capability comparison,
Macaca provider-neutral mapping, explicit non-goals, existing platform
inventory, and GitNexus memo evidence for `pack.finance.crypto.v1`. The crypto
pack must expose read-only crypto assets, token references, chain/network
references, venues, market pairs, quotes, trades, bars, snapshots, supply
metrics, public address balance references, oracle/feed references, freshness,
attribution, cursors, and artifacts through typed service commands. It must not
trade, route exchange orders, hold custody, manage keys, sign, transfer, swap,
stake, bridge, execute DeFi actions, or hardcode provider/exchange/chain/token
routing.

## Source Baseline

- CoinGecko API:
  <https://www.coingecko.com/en/api> and
  <https://docs.coingecko.com/reference/endpoint-overview>
- CoinMarketCap API:
  <https://coinmarketcap.com/api/documentation>,
  <https://pro.coinmarketcap.com/api/documentation/pro-api-reference/cryptocurrency>,
  and
  <https://pro.coinmarketcap.com/api/documentation/pro-api-reference/exchange>
- Coinbase Exchange/Advanced Trade market-data endpoints:
  <https://docs.cdp.coinbase.com/api-reference/exchange-api/rest-api/products/get-product-candles>
- Kraken REST market data:
  <https://docs.kraken.com/api/docs/category/rest-api/market-data/>
- Binance API:
  <https://developers.binance.com/docs/binance-spot-api-docs/rest-api/market-data-endpoints>
- OKX market data:
  <https://www.okx.com/docs-v5/en/#order-book-trading-market-data>
- Etherscan API:
  <https://docs.etherscan.io/>
- Chainlink Data Feeds:
  <https://docs.chain.link/data-feeds>
- DefiLlama API:
  <https://defillama.com/docs/api>
- CCXT manual:
  <https://docs.ccxt.com/>

## Supplier API Notes

- CoinGecko contributes coin/token metadata, prices, market charts, exchanges,
  categories, NFTs, global metrics, on-chain DEX data, rate limits, and plan
  boundaries. Macaca should normalize asset metadata, market data, NFTs as
  references, and attribution without adopting CoinGecko ids as universal ids.
- CoinMarketCap contributes listings, quotes, metadata, categories, exchange
  data, market pairs, global metrics, DEX data, plan credits, and attribution.
  Macaca should model credit/quota/license behavior and provider attribution.
- Coinbase, Kraken, Binance, and OKX contribute exchange products/pairs,
  ticker, candles/klines/OHLC, trades, status, precision, lot/tick sizes, rate
  limits, and errors. Macaca should normalize venue/pair/precision/freshness and
  keep trading endpoints out of scope.
- Etherscan-like explorers, Chainlink feeds, and DefiLlama contribute read-only
  token metadata, public address balance references, oracle/feed references,
  DeFi metrics, and chain-specific freshness. Macaca should keep address
  diagnostics privacy-gated and never expose raw chain payloads in observability.
- CCXT contributes useful abstraction vocabulary for symbol normalization,
  pair mapping, rate limits, and capability discovery, but Macaca must not adopt
  CCXT provider runtime semantics as OS contracts.

## Supplier Capability Comparison Memo

Common supplier concepts map to Macaca as follows:

- Aggregator coin ids, exchange asset ids, contract addresses, and symbols
  become `CryptoAsset`, `TokenReference`, and `ChainNetworkReference`.
- Exchanges, venues, products, market pairs, precision, and status become
  `CryptoExchangeVenue` and `CryptoMarketPair`.
- Tickers, quotes, trades, candles/klines/OHLCV, and snapshots become
  `CryptoQuote`, `CryptoTrade`, `CryptoBar`, `CryptoBarSeries`, and
  `CryptoSnapshot`.
- Circulating/total/max supply, FDV, market cap, and protocol metrics become
  `CryptoSupplyMetric`.
- Public address balance diagnostics become `PublicAddressBalanceReference`
  with explicit policy/approval boundaries.
- Oracle feed ids and data-feed references become `CryptoOracleFeedReference`.
- Attribution, rate limits, chain freshness, provider quotas, and cache state
  become `CryptoFreshness`, `CryptoAttribution`, and result diagnostics.

## Macaca-Owned Abstractions

`pack.finance.crypto.v1` should define `CryptoScope`,
`CryptoProviderCapability`, `CryptoAsset`, `TokenReference`,
`ChainNetworkReference`, `CryptoExchangeVenue`, `CryptoMarketPair`,
`CryptoQuote`, `CryptoTrade`, `CryptoBar`, `CryptoBarSeries`,
`CryptoSnapshot`, `CryptoSupplyMetric`,
`PublicAddressBalanceReference`, `CryptoOracleFeedReference`,
`CryptoFreshness`, `CryptoAttribution`, `CryptoCursor`, and
`CryptoArtifactHandle`.

The DTOs must carry asset identity, token contract reference, chain/network,
venue/pair identity, precision, quote currency, interval, trade/bar timestamp,
market status, supply metric source, address-reference redaction, oracle/feed
reference, attribution, license state, freshness, quota diagnostics, redaction
classes, bounded provider reason codes, and replay pointers. Raw chain
payloads, raw wallet data, private keys, signatures, exchange order payloads,
and unbounded datasets are rejected.

## Explicit Non-Goals

- No trading, exchange order routing, wallet custody, private key management,
  signing, transfer intent, swap intent, staking, bridge, DeFi execution,
  portfolio holdings, raw chain payload observability, or provider/exchange/
  chain/token-specific routing.
- No concrete CoinGecko, CoinMarketCap, Coinbase, Kraken, Binance, OKX,
  Etherscan, Chainlink, DefiLlama, CCXT, exchange, wallet, custody, or Web3
  adapters in this research phase.
- No fake prices, stripped attribution, raw address leakage, transaction
  signing, transfer creation, exchange order creation, or provider-native schema
  exposure as stable SDK contracts.

## Existing Macaca Platform Inventory

- Domain-pack descriptors, SDK facade, runtime-host provider registration,
  policy/resource/entitlement gates, trace/audit/redaction helpers, artifact
  handles, mock-provider patterns, unavailable diagnostics, and optional Web3
  module boundaries exist as generic substrate.
- Current evidence does not prove crypto-specific DTOs, descriptors, providers,
  SDK helpers, WASM ABI metadata, replay tests, redaction tests, dependency
  gates, or developer documentation.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
