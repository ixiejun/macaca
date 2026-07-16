# Change: Add Finance Market Data Pack

## Why

Developers need `pack.finance.market.data.v1` as an industrial market data
capability for provider inspection, instrument discovery, instrument metadata,
latest quotes, latest trades, historical bars/candles, snapshots, market
status, corporate actions, reference data, data freshness, entitlement
diagnostics, attribution, and replay. It must not be a thin wrapper around
Polygon/Massive, Alpaca, Nasdaq Data Link, Finnhub, Alpha Vantage, Tiingo,
Intrinio, exchange direct feeds, or one vendor's symbol model.

Market data is financially sensitive even when it is read-only. Data can be
real-time, delayed, end-of-day, derived, vendor-licensed, exchange-licensed,
region-limited, asset-class-specific, stale, corrected, split-adjusted,
currency-dependent, or unsuitable for trading decisions. Macaca must therefore
expose market data only through provider-neutral typed service commands with
declared permissions, entitlement checks, exchange/vendor attribution,
freshness classes, stale-data diagnostics, rate limits, resource budgets,
trace, audit, health, snapshot, replay, and structured unavailable behavior.

## Research And Supplier/API Baseline

Official and supplier references considered for this pack:

- Polygon/Massive stocks REST APIs expose ticker reference data, quotes, trades,
  aggregates/bars, snapshots, market status, and corporate actions across
  multiple asset classes and plans. Reference:
  https://massive.com/docs/rest/stocks/overview
- Alpaca Market Data API exposes real-time and historical equities, options,
  crypto, bars, quotes, trades, snapshots, and plan/rate-limit behavior.
  References: https://docs.alpaca.markets/us/docs/about-market-data-api and
  https://alpaca.markets/data
- Nasdaq Data Link exposes REST and streaming APIs for real-time, delayed,
  tables, and time-series market datasets with institutional data delivery
  semantics. References: https://docs.data.nasdaq.com/ and
  https://www.nasdaq.com/solutions/data/nasdaq-data-link/api
- Finnhub exposes quote, candles, symbol lookup, market status, fundamentals,
  economic data, and alternative data endpoints. References:
  https://finnhub.io/docs/api and https://finnhub.io/docs/api/quote
- Alpha Vantage, Tiingo, Intrinio, and exchange direct feeds provide additional
  baselines for adjusted bars, rate limits, identifiers, corporate actions,
  reference datasets, and data-license attribution. These are provider
  baselines, not OS semantics.

Macaca maps these supplier concepts into provider-neutral market data scope,
provider capability, instrument handle, instrument identity, exchange/venue
metadata, quote, trade, bar series, snapshot, corporate action, market session,
data freshness, adjustment policy, entitlement class, attribution, request
cursor, artifact/cache handle, and diagnostics DTOs. Concrete market data
vendors, exchange feeds, entitlement systems, cache stores, and historical
backfill providers stay behind replaceable service providers.

## What Changes

- Add provider-neutral `pack.finance.market.data.v1` under the `finance`
  family.
- Define command namespace `market_data.*` for:
  - provider capability inspection
  - instrument search and metadata lookup
  - quote, trade, bar/candle, snapshot, and corporate-action retrieval
  - market status/session inspection
  - data freshness, attribution, entitlement, and provider health diagnostics
  - paged/bounded historical data retrieval and cache/artifact handle
    resolution
- Define DTOs for market data scope, provider capability, instrument handle,
  instrument identity, venue, market session, quote, trade, bar, bar series,
  snapshot, corporate action, adjustment policy, data freshness, attribution,
  entitlement class, request cursor, cache/artifact handle, and diagnostics.
- Define permission scopes, policy defaults, real-time/delayed/end-of-day data
  classes, stale-data behavior, exchange/vendor attribution, licensing
  boundaries, rate limits, SDK discovery, developer documentation, trace/audit
  events, snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/finance/market-data.md` before implementation
  completion.

## Impact

- Affected specs: `pack-finance-market-data`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, market data service
  provider or unavailable provider, runtime-host provider adapters,
  entitlement/licensing/cache support, trace/audit schemas, replay tests,
  dependency-boundary gates, and developer documentation.
- Non-goals: no concrete Polygon/Massive/Alpaca/Nasdaq/Finnhub/Alpha Vantage/
  Tiingo/Intrinio/exchange/cache provider implementation in this proposal; no
  trading, order routing, investment advice, portfolio allocation, tax advice,
  brokerage workflow, alerting workflow, or application-specific finance logic;
  no provider-name, exchange-name, asset-name, symbol-name, dataset-name,
  plan-name, or workflow-name routing in OS layers beyond declarative
  descriptor data; no raw credentials, account identifiers, user holdings,
  raw provider payloads, licensed feed payloads, manifests, package bytes,
  private keys, signatures, or unbounded market datasets in observability; no
  SDK/shell/kernel provider construction; no fake success when provider,
  market, exchange, entitlement, license, symbol, freshness, permission,
  resource, or host support is absent.
