# Finance Stock Pack Design

## Context

`pack.finance.stock.v1` exposes equity-domain stock data as a Macaca OS
serviceized capability. It lets applications resolve equities, inspect company
profiles, listings, fundamentals, financial statements, SEC/XBRL-like facts,
dividends, splits, earnings events, licensed analyst/estimate datasets, equity
screens, stock universes, and stock-specific diagnostics without embedding a
stock data vendor, filing system, exchange feed, brokerage adapter, or
application-specific investment workflow into generic OS layers.

The pack builds on, but does not duplicate, `pack.finance.market.data.v1`.
Real-time quotes, trades, and generic bars remain market-data semantics. The
stock pack may provide quote handoff commands that return typed market-data
requests or references, but it must not bypass market-data freshness,
attribution, entitlement, and licensing policy.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| SEC EDGAR APIs | Company submissions, XBRL company facts, filing metadata | Filing source reference, financial fact, statement period, restatement metadata |
| Finnhub | Company profiles, financials, metrics, recommendations, earnings, estimates, quote/symbol lookup | Company profile, fundamental metric, analyst dataset reference, earnings event |
| Polygon/Massive | Ticker reference, dividends, splits, financials, quotes, snapshots | Equity instrument, listing, dividend, split, market-data handoff |
| Alpha Vantage | Company overview, income statement, balance sheet, cash flow, earnings, time series | Statement period, financial fact, earnings event, stock profile |
| Nasdaq Data Link / Intrinio / Tiingo / FMP | Fundamentals, corporate actions, estimates, identifiers, screeners | Licensed dataset reference, screener, stock universe, attribution |

The pack exposes provider-neutral contracts. Provider adapters translate to
filing APIs, fundamentals APIs, screening APIs, corporate-action datasets,
market-data services, cache stores, entitlement systems, or unavailable
providers. OS layers must not branch on provider names, exchange names,
tickers, companies, CIKs, dataset names, metric names, model names, or business
workflows.

## Goals

- Provide stable pack id `pack.finance.stock.v1` and command namespace
  `stock.*`.
- Support provider inspection, equity search/resolve, company profile, listing
  metadata, fundamentals, financial statements, financial facts, metrics,
  dividends, splits, earnings, analyst dataset references, screening, stock
  universes, quote handoff, freshness, attribution, entitlement diagnostics,
  cache/artifact handles, health, snapshots, and replay diagnostics.
- Preserve financial safety with no-investment-advice semantics, read-only data
  retrieval, explicit source periods, restatement status, adjustment metadata,
  data freshness, attribution, licensing, pagination, bounded output, and
  sanitized audit.
- Keep concrete stock data providers behind replaceable service providers.
- Require developer documentation at `docs/developer-packs/finance/stock.md`.

## Non-Goals

- Do not implement concrete SEC, Finnhub, Polygon/Massive, Alpha Vantage,
  Nasdaq, Intrinio, Tiingo, Financial Modeling Prep, exchange-feed, cache, or
  entitlement providers in this proposal.
- Do not define trading, order routing, investment advice, portfolio holdings,
  accounting, tax advice, brokerage workflows, personal watchlist storage,
  alerts, or application-specific finance logic.
- Do not expose raw credentials, account identifiers, user holdings, raw
  filings, raw provider payloads, licensed feed payloads, manifests, package
  bytes, private keys, signatures, or unbounded financial datasets in
  observability.
- Do not silently substitute providers, infer investment advice, hide
  restatements, apply adjustments without explicit policy, remove attribution,
  or fake success when provider, equity, filing, market, entitlement, license,
  symbol, freshness, permission, resource, or host support is absent.

## Ownership And Boundaries

- Pack id: `pack.finance.stock.v1`.
- Family: `finance`.
- Backing service owner: stock data service provider.
- SDK surface: `sdk.packs.finance.stock`.
- Command namespace: `stock.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridges,
  market-data handoff bridges, entitlement/cache bridges, decorators, and
  sanitized diagnostics through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `stock.inspect_provider` | Inspect provider, equity, fundamentals, filing, corporate action, screening, universe, and quote-handoff support | Returns sanitized capability, quota, lifecycle, attribution, health, and compatibility metadata |
| `stock.search_equities` | Search equities by symbol, name, identifier, exchange, country, sector, or universe | Requires bounded query, entitlement, paging, and redaction |
| `stock.get_equity` | Resolve equity instrument metadata | Requires symbol/identifier disambiguation, listing status, attribution, and freshness |
| `stock.get_company_profile` | Retrieve company profile and issuer metadata | Requires equity scope, source period, redaction, and attribution |
| `stock.get_listing` | Retrieve exchange/listing metadata | Requires venue scope, listing state, currency, timezone, and attribution |
| `stock.get_fundamentals` | Retrieve normalized fundamentals or metrics | Requires period, metric set, restatement policy, source attribution, and pagination |
| `stock.get_financial_statements` | Retrieve statement periods and normalized financial facts | Requires statement type, period range, filing source, restatement metadata, and bounded output |
| `stock.get_corporate_events` | Retrieve dividends, splits, earnings, and comparable equity events | Requires event type, date range, adjustment metadata, and entitlement |
| `stock.screen_equities` | Run a bounded equity screener | Requires screen query validation, metric availability, output limit, attribution, and license policy |
| `stock.create_universe` | Create a provider-neutral stock universe descriptor from a bounded screen or explicit handles | Returns universe handle and replay metadata without storing personal watchlists |
| `stock.plan_quote_handoff` | Build a market-data quote request for an equity | Requires market-data pack availability and preserves market-data policy |
| `stock.inspect_freshness` | Inspect source period, filing timestamp, provider timestamp, restatement, cache, and stale diagnostics | Requires equity/provider scope and attribution |
| `stock.get_artifact_handle` | Resolve cached/paged result artifact metadata | Requires artifact permission, retention, and licensing policy |

Every command must define typed command DTOs, typed success results, typed
paged/partial results, typed denied/unavailable/unsupported/conflict/
stale-data/schema-mismatch/provider-attribution-required/license-denied/
symbol-ambiguous/symbol-not-found/equity-unsupported/exchange-unsupported/
filing-unavailable/metric-unsupported/period-unsupported/restatement-conflict/
screen-unsupported/range-too-large/quota/timeout/cancellation/failure results,
redaction profile, cache semantics, idempotency semantics for cache-producing
reads, and replay metadata.

## DTO Model

Core DTOs:

- `StockScope`: provider scope, equity handle, company handle, venue scope,
  filing source, dataset scope, credential reference, entitlement state, license
  policy, freshness policy, permission state, rate-limit profile, and health.
- `StockProviderCapability`: provider class, supported exchanges, countries,
  identifier types, profile support, listing support, fundamentals support,
  filing/fact support, corporate-event support, screener support, universe
  support, quote-handoff support, attribution requirements, auth modes, rate
  limits, lifecycle, and health.
- `EquityInstrument`: equity handle, canonical symbol, identifier set, company
  handle, listing handle, asset class, security type, country, currency,
  exchange, status, entitlement class, attribution class, freshness, and
  redaction class.
- `CompanyProfile`: company handle, issuer name projection, sector/industry
  classes, domicile/locale, fiscal year end, reporting currency, website
  presence class, employee count class, source period, attribution, and
  redaction class.
- `StockListing`: listing handle, equity handle, exchange, mic/code slots,
  currency, timezone, first/listing date class, delisting state, trading status,
  and attribution class.
- `FinancialStatementPeriod`: equity handle, statement type, fiscal period,
  fiscal year, filing date, accepted date, source form class, restatement state,
  currency, units, source handle, and attribution.
- `FinancialFact`: statement period handle, taxonomy/name slot, normalized fact
  key, value class, unit, period, confidence/source class, restatement state,
  and redaction class.
- `FundamentalMetric`: metric handle, equity handle, metric key, value class,
  period, calculation/source class, restatement state, freshness, attribution,
  and redaction class.
- `StockDividend`, `StockSplit`, and `StockEarningsEvent`: equity handle,
  event type, declared/effective/ex-date/report date classes, amount/ratio/eps
  classes, currency, adjustment impact, source timestamp, attribution, and
  redaction class.
- `AnalystDatasetReference`: dataset handle, metric/estimate class,
  provider/license class, period, freshness, attribution, and redaction class.
- `StockScreenQuery`: bounded filter set, metric references, universe scope,
  sort/page policy, license policy, and validation diagnostics.
- `StockUniverse`: universe handle, source query hash, equity count class,
  membership cursor, entitlement class, attribution class, and replay pointer.
- `StockFreshness`, `StockAttribution`, `StockCursor`, and
  `StockArtifactHandle`: freshness, attribution, paging/cache/artifact, license,
  checksum, redaction, and replay metadata.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Licensing Model

Permission scopes:

- `stock.provider.inspect`
- `stock.equity.search`
- `stock.equity.read`
- `stock.company.read`
- `stock.listing.read`
- `stock.fundamentals.read`
- `stock.statements.read`
- `stock.corporate_events.read`
- `stock.screen`
- `stock.universe`
- `stock.quote_handoff`
- `stock.freshness.read`
- `stock.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, equity handle when applicable, company handle when
  applicable, dataset scope when applicable, credential reference, entitlement
  state, license policy, freshness policy, and permission state.
- Results must preserve source period, filing/source timestamps, restatement
  state, adjustment policy, attribution, currency, units, and freshness
  metadata.
- Screening and universe creation require bounded filters, output limits,
  pagination, metric availability checks, license checks, and replayable query
  hashes.
- Quote handoff requires `pack.finance.market.data.v1` availability and must
  preserve market-data freshness, entitlement, and attribution semantics.
- Redistribution-sensitive, premium, analyst/estimate, exchange-licensed,
  SEC/fundamental, region-restricted, or large historical requests may require
  entitlement or approval.
- Raw filings, raw provider payloads, licensed feed payloads, and unbounded
  financial datasets must not enter observability.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
supported exchanges/countries, identifier types, profile support, fundamentals
support, statement/fact support, corporate-event support, screener support,
universe support, quote-handoff support, attribution requirements, freshness
classes, permission scopes, policy templates, resource limits, approval rules,
provider capability hashes, health, compatibility, diagnostics, examples,
redaction profiles, and documentation links.

The developer guide at `docs/developer-packs/finance/stock.md` must cover:

- manifest declaration and optional/required behavior
- equity instruments, company profiles, listings, statement periods, financial
  facts, metrics, dividends, splits, earnings, analyst dataset references,
  screen queries, universes, quote handoff, freshness, attribution,
  entitlements, licenses, cursors, artifacts, provider capabilities, and
  unavailable states
- no-investment-advice semantics, stock versus market-data boundaries, source
  period, restatement, adjustment, stale-data diagnostics, provider replacement,
  trace/audit interpretation, and conformance tests

Examples must use synthetic companies, equities, listings, facts, statements,
events, screens, universes, and artifacts. They must not include provider
names, credentials, real account data, holdings, personal watchlists, live
trading strategies, investment advice, raw filings, raw provider payloads, or
workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `stock_pack_declared`
- `stock_pack_admission_validated`
- `stock_provider_inspected`
- `stock_equities_searched`
- `stock_equity_resolved`
- `stock_company_profile_read`
- `stock_listing_read`
- `stock_fundamentals_read`
- `stock_financial_statements_read`
- `stock_corporate_events_read`
- `stock_equities_screened`
- `stock_universe_created`
- `stock_quote_handoff_planned`
- `stock_freshness_inspected`
- `stock_artifact_handle_resolved`
- `stock_pack_policy_decision`
- `stock_pack_service_call_requested`
- `stock_pack_service_call_succeeded`
- `stock_pack_service_call_failed`
- `stock_pack_unavailable`
- `stock_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, command
availability, provider health, policy template hash, resource counters, bounded
equity/company/listing/dataset/request/cursor/artifact summaries, event cursors,
and sanitized replay pointers. Snapshots must exclude raw credentials, account
identifiers, user holdings, raw filings, raw provider payloads, licensed feed
payloads, manifests, package bytes, private keys, signatures, and unbounded
financial datasets.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, filing readers, fundamentals readers,
  corporate-event readers, screeners, universe builders, market-data handoff
  adapters, attribution resolvers, cache readers, and unavailable behavior are
  replaceable.
- **Decorator**: trace, policy, entitlement, resource, metering, licensing,
  attribution, freshness, cache, restatement, and output redaction wrap service
  calls.
- **Specification**: admission validates provider scope, command availability,
  permissions, entitlement, equity, filing, dataset, metric, period, screener,
  quote-handoff, freshness policy, and compatibility.
- **Observer**: provider health, trace, audit, service events, and cache/artifact
  lifecycle events are subscribable.
- **Memento**: capability hashes, source period hashes, request hashes, cursors,
  cache handles, snapshots, and replay pointers preserve recovery state.
- **Abstract Factory**: concrete stock providers are created only by approved
  runtime-host composition roots.

## Risks And Mitigations

- Risk: pack duplicates market-data quote semantics. Mitigation: quote handoff
  returns typed market-data requests/references and never bypasses market-data
  policy.
- Risk: pack becomes investment advice or portfolio logic. Mitigation:
  read-only data commands, explicit non-goals, and no personal watchlist storage.
- Risk: restated or stale fundamentals are hidden. Mitigation: mandatory source
  period, freshness, restatement, and attribution metadata.
- Risk: licensed filings or datasets leak. Mitigation: bounded DTOs, redaction,
  attribution metadata, and strict observability exclusions.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call provider APIs directly.
