# Change: Add Finance Crypto Pack

## Why

Developers need `pack.finance.crypto.v1` as an industrial crypto-asset data
capability for crypto asset discovery, asset metadata, spot quotes, market
pairs, exchange market status, historical bars, supply metrics, token reference
metadata, read-only public address balance diagnostics where policy permits,
freshness, attribution, entitlement, and replay. It must not be a thin wrapper
around CoinGecko, CoinMarketCap, Coinbase, Kraken, Binance, OKX, Chainlink,
Etherscan-like explorers, DefiLlama, CCXT, or one exchange's symbol model.

Crypto data can be exchange-specific, chain-specific, pair-specific,
stablecoin-denominated, stale, manipulated, forked, delisted, vendor-licensed,
jurisdiction-restricted, or unsuitable for investment decisions. Public wallet
address lookups can also reveal sensitive behavioral data. Macaca must expose
crypto data only through provider-neutral typed service commands with declared
permissions, entitlement, chain/network scope, address privacy policy, exchange
and provider attribution, freshness diagnostics, rate limits, resource budgets,
trace, audit, health, snapshot, replay, and structured unavailable behavior.

## Research And Supplier/API Baseline

Official and supplier references considered for this pack:

- CoinGecko API exposes coins, asset metadata, market data, historical charts,
  exchanges, categories, NFT/reference data, and rate-limit tiers. Reference:
  https://docs.coingecko.com/reference/introduction
- CoinMarketCap API exposes cryptocurrency listings, quotes, metadata,
  categories, exchange data, and market-pair endpoints with plan/credit
  behavior. Reference: https://coinmarketcap.com/api/documentation/v1/
- Coinbase Exchange and Advanced Trade APIs expose products, market data,
  candles, trades, best bid/ask, and exchange product metadata. References:
  https://docs.cdp.coinbase.com/exchange/reference/exchangerestapi_getproducts
  and https://docs.cdp.coinbase.com/advanced-trade/reference
- Kraken REST API exposes assets, asset pairs, ticker, OHLC, trades, spread,
  and server status. Reference:
  https://docs.kraken.com/api/docs/rest-api/get-asset-info/
- Binance Spot API and OKX APIs expose exchange info, tickers, klines,
  trades, and market status style metadata. References:
  https://developers.binance.com/docs/binance-spot-api-docs/rest-api and
  https://www.okx.com/docs-v5/en/
- Etherscan-like explorers, Chainlink market data feeds, and DefiLlama provide
  baselines for read-only chain/token/address reference data, oracle metadata,
  and DeFi market datasets. These are provider baselines, not OS semantics.

Macaca maps these concepts into provider-neutral crypto scope, provider
capability, crypto asset handle, chain/network reference, token contract
reference, market pair, exchange/venue, crypto quote, trade, bar series, market
snapshot, supply metric, public address balance reference, oracle/feed
reference, freshness, attribution, entitlement class, cursor, artifact handle,
and diagnostics DTOs. Concrete exchanges, aggregators, explorers, oracle feeds,
cache stores, entitlement systems, wallets, signing providers, and chain clients
stay behind their own service providers.

## What Changes

- Add provider-neutral `pack.finance.crypto.v1` under the `finance` family.
- Define command namespace `crypto.*` for:
  - provider capability inspection
  - crypto asset search and metadata lookup
  - token/contract reference lookup
  - exchange product/pair discovery
  - quote, trade, bars, snapshot, supply metric, and market status retrieval
  - optional read-only public address balance diagnostics
  - freshness, attribution, entitlement, and artifact diagnostics
- Define DTOs for crypto scope, provider capability, crypto asset, token
  reference, chain/network reference, exchange venue, market pair, quote, trade,
  bar series, market snapshot, supply metric, public address balance reference,
  oracle/feed reference, freshness, attribution, cursor, artifact handle, and
  diagnostics.
- Define permission scopes, read-only boundaries, address privacy policy,
  market-data handoff rules, Web3/wallet/transfer non-goals, entitlement/
  licensing rules, SDK discovery, developer documentation, trace/audit events,
  snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/finance/crypto.md` before implementation completion.

## Impact

- Affected specs: `pack-finance-crypto`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`, optional Web3/EVM service specs,
  and `pack-finance-market-data`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, crypto finance
  service provider or unavailable provider, market-data/Web3 handoff contracts,
  entitlement/licensing/cache support, trace/audit schemas, replay tests,
  dependency-boundary gates, and developer documentation.
- Non-goals: no concrete CoinGecko/CoinMarketCap/Coinbase/Kraken/Binance/OKX/
  Etherscan/Chainlink/DefiLlama/CCXT/exchange/explorer/cache provider
  implementation in this proposal; no trading, exchange order routing, wallet
  custody, private key management, signing, transfer intent, swap intent,
  staking, bridge, DeFi execution, tax advice, portfolio holdings, investment
  advice, or application-specific crypto workflow; no provider-name,
  exchange-name, chain-name, token-name, contract-address-name, wallet-name,
  dataset-name, plan-name, or workflow-name routing in OS layers beyond
  declarative descriptor data; no raw credentials, private keys, seed phrases,
  signatures, user holdings, raw provider payloads, licensed feed payloads,
  raw chain payloads, manifests, package bytes, or unbounded crypto datasets in
  observability; no SDK/shell/kernel provider construction; no fake success
  when provider, exchange, chain, token, entitlement, license, address policy,
  freshness, permission, resource, Web3, or host support is absent.
