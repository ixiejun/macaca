# Change: Add Finance Stock Pack

## Why

Developers need `pack.finance.stock.v1` as an industrial equity-domain
capability for stock instrument resolution, company profiles, exchange/listing
metadata, equity quote handoff, fundamentals, financial statements, SEC/XBRL
facts, dividends, splits, earnings, analyst/estimate datasets where licensed,
equity screening, stock universe definitions, data freshness, attribution, and
replay diagnostics. It must not be a thin wrapper around SEC EDGAR, Finnhub,
Polygon/Massive, Alpha Vantage, Nasdaq Data Link, Financial Modeling Prep,
Intrinio, Tiingo, exchange feeds, or one provider's ticker model.

Stock data can be real-time, delayed, restated, adjusted, unaudited, vendor
licensed, exchange licensed, jurisdiction-specific, stale, or unsuitable for
investment decisions. Macaca must expose stock data through provider-neutral
typed service commands with entitlement, licensing, attribution, freshness,
source-period, restatement, adjustment, trace, audit, health, snapshot, replay,
and structured unavailable behavior.

## Research And Supplier/API Baseline

Official and supplier references considered for this pack:

- SEC EDGAR APIs expose company submissions and extracted XBRL company facts for
  public-company filings. Reference:
  https://www.sec.gov/search-filings/edgar-application-programming-interfaces
- Finnhub exposes stock company profiles, financials, metrics, recommendation
  trends, earnings, estimates, quote, symbol lookup, and other equity datasets.
  References: https://finnhub.io/docs/api and
  https://finnhub.io/docs/api/company-profile2
- Polygon/Massive Stocks APIs expose ticker reference, dividends, splits,
  financials, quotes, aggregates, snapshots, and market status. Reference:
  https://massive.com/docs/rest/stocks/overview
- Alpha Vantage exposes company overview, income statement, balance sheet, cash
  flow, earnings, and time-series stock endpoints. Reference:
  https://www.alphavantage.co/documentation/
- Nasdaq Data Link, Intrinio, Tiingo, and Financial Modeling Prep provide
  additional baselines for company fundamentals, financial statements,
  corporate actions, analyst/estimate datasets, identifiers, and screening
  APIs. These are provider baselines, not OS semantics.

Macaca maps these supplier concepts into provider-neutral stock scope,
equity instrument handle, company profile, listing, equity quote reference,
financial statement period, financial fact, fundamental metric, earnings event,
dividend, split, analyst dataset reference, screening query, stock universe,
data freshness, attribution, entitlement class, request cursor, artifact handle,
and diagnostics DTOs. Concrete stock data providers, filing systems, exchange
feeds, entitlement systems, cache stores, and historical backfill providers stay
behind replaceable service providers.

## What Changes

- Add provider-neutral `pack.finance.stock.v1` under the `finance` family.
- Define command namespace `stock.*` for:
  - provider capability inspection
  - equity instrument and company profile resolution
  - listing/exchange metadata
  - fundamentals, financial statements, SEC/XBRL-like facts, and metrics
  - dividends, splits, earnings, and corporate-event retrieval
  - equity screening and stock universe creation
  - quote handoff to the market data pack when quote data is required
  - freshness, attribution, entitlement, cache/artifact, and diagnostics
- Define DTOs for stock scope, provider capability, equity instrument, company
  profile, listing, statement period, financial fact, fundamental metric,
  dividend, split, earnings event, analyst dataset reference, screen query,
  stock universe, freshness, attribution, cursor, artifact handle, and
  diagnostics.
- Define permission scopes, no-investment-advice policy, market-data handoff
  boundaries, watchlist/user-state non-goals, entitlement/licensing rules,
  restatement/adjustment semantics, SDK discovery, developer documentation,
  trace/audit events, snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/finance/stock.md` before implementation completion.

## Impact

- Affected specs: `pack-finance-stock`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`, `pack-finance-market-data`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, stock data service
  provider or unavailable provider, market-data handoff contracts,
  entitlement/licensing/cache support, trace/audit schemas, replay tests,
  dependency-boundary gates, and developer documentation.
- Non-goals: no concrete SEC/Finnhub/Polygon/Massive/Alpha Vantage/Nasdaq/
  Intrinio/Tiingo/FMP/exchange/cache provider implementation in this proposal;
  no trading, order routing, investment advice, portfolio holdings, accounting,
  tax advice, brokerage workflow, personal watchlist storage, alerting workflow,
  or application-specific finance logic; no provider-name, exchange-name,
  company-name, ticker-name, CIK-name, dataset-name, metric-name, model-name, or
  workflow-name routing in OS layers beyond declarative descriptor data; no raw
  credentials, account identifiers, user holdings, raw filings, raw provider
  payloads, licensed feed payloads, manifests, package bytes, private keys,
  signatures, or unbounded financial datasets in observability; no SDK/shell/
  kernel provider construction; no fake success when provider, market, equity,
  filing, entitlement, license, symbol, freshness, permission, resource, or host
  support is absent.
