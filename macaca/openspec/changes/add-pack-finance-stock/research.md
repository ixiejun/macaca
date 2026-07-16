# Finance Stock Pack Research

## Purpose

This note records supplier/API research, supplier capability comparison,
Macaca provider-neutral mapping, explicit non-goals, existing platform
inventory, and GitNexus memo evidence for `pack.finance.stock.v1`. The stock
pack must expose equity identity, company profiles, listings, fundamentals,
filing/fact data, corporate events, screeners, universes, quote handoff,
freshness, attribution, cursors, and artifacts through typed service commands.
It must not trade, route orders, advise, store personal watchlists, own
portfolio holdings, perform tax/accounting, pass through raw filings, or
hardcode provider/exchange/ticker/CIK/dataset routing.

## Source Baseline

- SEC EDGAR APIs:
  <https://www.sec.gov/search-filings/edgar-application-programming-interfaces>
- Finnhub API and fundamentals:
  <https://finnhub.io/docs/api> and
  <https://finnhub.io/docs/api/company-basic-financials>
- Polygon/Massive Stocks REST API:
  <https://massive.com/docs/rest/stocks/overview>
- Alpha Vantage fundamental data:
  <https://www.alphavantage.co/documentation/>
- Nasdaq Data Link:
  <https://docs.data.nasdaq.com/docs/getting-started>
- Intrinio:
  <https://docs.intrinio.com/documentation/web_api>
- Tiingo:
  <https://api.tiingo.com/documentation/general/overview>

## Supplier API Notes

- SEC EDGAR contributes submissions, filing dates, accepted dates, company
  facts, XBRL concepts, taxonomy tags, units, periods, and restatement/source
  behavior. Macaca should model filing/fact provenance and restatement conflict
  semantics without raw filing observability.
- Finnhub contributes company profiles, financials, metrics, recommendations,
  earnings, estimates, symbols, quotas, and errors. Macaca should separate
  analyst/estimate datasets and license requirements from public fundamentals.
- Polygon/Massive contributes ticker reference, dividends, splits, financials,
  snapshots, quote handoff boundaries, pagination, and entitlements. Macaca
  should use stock pack for identity/fundamentals/corporate events and market
  data pack for quotes/trades/bars.
- Alpha Vantage contributes company overview, income statement, balance sheet,
  cash flow, earnings, listing status, and time-series-adjacent APIs. Macaca
  should normalize statement periods, facts, metrics, and source freshness.
- Nasdaq Data Link, Intrinio, Tiingo, and exchange/direct feeds contribute
  premium datasets, identifiers, fundamentals, corporate actions, screeners,
  attribution, and licensing. Macaca should model license denial and
  attribution as first-class result metadata.

## Supplier Capability Comparison Memo

Common supplier concepts map to Macaca as follows:

- Tickers, CIKs, FIGI/ISIN/CUSIP-style ids, and provider ids become
  `EquityInstrument` identity fields.
- Profiles, listings, exchanges, countries, status, and delisting state become
  `CompanyProfile` and `StockListing`.
- XBRL facts, financial statements, metrics, periods, units, and restatements
  become `FinancialStatementPeriod`, `FinancialFact`, and
  `FundamentalMetric`.
- Dividends, splits, earnings, and other corporate events become
  `StockDividend`, `StockSplit`, and `StockEarningsEvent`.
- Analyst estimates, recommendations, and premium datasets become
  `AnalystDatasetReference` with entitlement and attribution metadata.
- Screeners and universes become `StockScreenQuery` and `StockUniverse`.
- Quote handoff remains a plan/reference to market-data pack, not a stock quote
  execution surface.

## Macaca-Owned Abstractions

`pack.finance.stock.v1` should define `StockScope`,
`StockProviderCapability`, `EquityInstrument`, `CompanyProfile`,
`StockListing`, `FinancialStatementPeriod`, `FinancialFact`,
`FundamentalMetric`, `StockDividend`, `StockSplit`, `StockEarningsEvent`,
`AnalystDatasetReference`, `StockScreenQuery`, `StockUniverse`,
`StockFreshness`, `StockAttribution`, `StockCursor`, and
`StockArtifactHandle`.

The DTOs must carry identifier provenance, exchange/country scope, filing
source, statement type, fiscal period, units, restatement status, metric
methodology, corporate-event evidence, screen filters, universe size, quote
handoff references, license state, freshness, attribution, redaction classes,
bounded provider reason codes, and replay pointers. Raw filings, licensed
payloads, provider pass-through schemas, and unbounded datasets are rejected.

## Explicit Non-Goals

- No trading, order routing, investment advice, portfolio holdings, tax or
  accounting workflows, brokerage workflow, personal watchlist storage, raw
  provider pass-through, raw filing observability, or provider/exchange/ticker/
  CIK/dataset-specific routing.
- No concrete SEC, Finnhub, Polygon, Alpha Vantage, Nasdaq, Intrinio, Tiingo,
  exchange, broker, or market-data adapters in this research phase.
- No fake fundamentals, stripped attribution, raw filing leakage,
  recommendation/advice behavior, or provider-native schema exposure as stable
  SDK contracts.

## Existing Macaca Platform Inventory

- Domain-pack descriptors, SDK facade, runtime-host provider registration,
  policy/resource/entitlement gates, trace/audit/redaction helpers, artifact
  handles, mock-provider patterns, and unavailable diagnostics exist as generic
  substrate.
- Current evidence does not prove stock-specific DTOs, descriptors, providers,
  SDK helpers, WASM ABI metadata, replay tests, redaction tests, dependency
  gates, or developer documentation.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
