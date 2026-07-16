# Finance Market Data Pack Research

## Purpose

This note records supplier/API research, supplier capability comparison,
Macaca provider-neutral mapping, explicit non-goals, existing platform
inventory, and GitNexus memo evidence for `pack.finance.market.data.v1`. The
market-data pack must expose read-only instruments, venues, quotes, trades,
bars, snapshots, corporate actions, market status, freshness, attribution,
cursors, and artifacts through typed service commands. It must not trade, route
orders, advise, allocate portfolios, broker workflows, pass through raw provider
payloads, leak licensed feeds, or hardcode provider/exchange/symbol routing.

## Source Baseline

- Polygon/Massive Stocks REST API:
  <https://massive.com/docs/rest/stocks/overview>
- Alpaca Market Data:
  <https://alpaca.markets/data>
- Nasdaq Data Link:
  <https://docs.data.nasdaq.com/docs/getting-started>
- Finnhub API:
  <https://finnhub.io/docs/api>
- Alpha Vantage:
  <https://www.alphavantage.co/documentation/>
- Tiingo:
  <https://api.tiingo.com/documentation/general/overview>
- Intrinio:
  <https://docs.intrinio.com/documentation/web_api>

## Supplier API Notes

- Polygon/Massive contributes ticker reference, latest quotes, trades,
  aggregates/bars, snapshots, market status, corporate actions, pagination,
  entitlements, and normalized exchange data. Macaca should model venue,
  instrument, quote/trade/bar/snapshot, action, entitlement, and freshness
  separately.
- Alpaca contributes equities/options/crypto bars, quotes, trades, snapshots,
  real-time and historical endpoints, subscription tiers, and asset-class
  boundaries. Macaca should expose asset-class support and licensing states
  through provider capability descriptors.
- Nasdaq Data Link contributes free/premium datasets, real-time/delayed REST,
  streaming, table data, time-series data, attribution, and licensing behavior.
  Macaca should model dataset attribution and redistribution restrictions.
- Finnhub contributes quotes, candles, symbol lookup, market status,
  fundamentals/economic data, quotas, and errors. Macaca should keep market data
  separate from fundamentals and economic datasets unless explicitly declared.
- Alpha Vantage, Tiingo, Intrinio, and exchange direct feeds contribute
  adjusted bars, corporate actions, identifiers, reference datasets,
  entitlements, exchange licensing, and attribution requirements. Macaca should
  normalize adjusted/unadjusted series and license denial.

## Supplier Capability Comparison Memo

Common supplier concepts map to Macaca as follows:

- Vendor symbols and instrument ids become `InstrumentHandle` and
  `InstrumentIdentity`.
- Exchanges, MICs, regions, sessions, and trading calendars become
  `MarketVenue` and `MarketSession`.
- Quotes, trades, OHLCV bars, aggregate bars, and snapshots become
  `MarketQuote`, `MarketTrade`, `MarketBar`, `MarketBarSeries`, and
  `MarketSnapshot`.
- Splits, dividends, symbol changes, and other events become `CorporateAction`.
- Real-time, delayed, end-of-day, adjusted, unadjusted, stale, and cached data
  become `MarketDataFreshness` and provider capability fields.
- Pagination, dataset licenses, exchange entitlements, attribution, and
  redistribution rules become cursor, entitlement, policy, and
  `MarketDataAttribution` metadata.

## Macaca-Owned Abstractions

`pack.finance.market.data.v1` should define `MarketDataScope`,
`MarketDataProviderCapability`, `InstrumentHandle`, `InstrumentIdentity`,
`MarketVenue`, `MarketSession`, `MarketQuote`, `MarketTrade`, `MarketBar`,
`MarketBarSeries`, `MarketSnapshot`, `CorporateAction`,
`MarketDataFreshness`, `MarketDataAttribution`, `MarketDataCursor`, and
`MarketDataArtifactHandle`.

The DTOs must carry provider class, instrument identity, venue/session,
asset class, interval, adjustment state, quote/trade/bar timestamp,
corporate-action type, attribution requirement, license state, pagination,
freshness, quota diagnostics, redaction classes, bounded provider reason codes,
and replay pointers. Raw licensed payloads, raw feed packets, provider
pass-through fields, and unbounded datasets are rejected.

## Explicit Non-Goals

- No trading, order routing, investment advice, portfolio allocation,
  brokerage workflow, raw provider pass-through, licensed feed observability, or
  provider/exchange/symbol/dataset-specific routing.
- No concrete Polygon, Alpaca, Nasdaq, Finnhub, Alpha Vantage, Tiingo,
  Intrinio, exchange direct-feed, broker, or trading adapters in this research
  phase.
- No fake prices, stripped attribution, redistribution-sensitive leakage, or
  provider-native schema exposure as stable SDK contracts.

## Existing Macaca Platform Inventory

- Domain-pack descriptors, SDK facade, runtime-host provider registration,
  policy/resource/entitlement gates, trace/audit/redaction helpers, artifact
  handles, mock-provider patterns, and unavailable diagnostics exist as generic
  substrate.
- Current evidence does not prove market-data-specific DTOs, descriptors,
  providers, SDK helpers, WASM ABI metadata, replay tests, redaction tests,
  dependency gates, or developer documentation.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
