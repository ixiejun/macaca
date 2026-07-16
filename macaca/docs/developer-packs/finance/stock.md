# Finance Stock Pack

`pack.finance.stock.v1` describes provider-neutral, read-only equity-domain
capabilities. The descriptor is discoverable through SDK catalogs, but commands
remain unavailable until a stock data provider is installed through the runtime
composition root.

## Manifest Declaration

Declare the pack as required only when stock reference or fundamentals data is
mandatory for readiness. Optional declarations degrade with structured
unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.finance.stock.v1"]
```

## Permissions

Use the narrowest scope: `stock.provider.inspect`, `stock.equity.search`,
`stock.equity.read`, `stock.company.read`, `stock.listing.read`,
`stock.fundamentals.read`, `stock.statements.read`,
`stock.corporate_events.read`, `stock.screen`, `stock.universe`,
`stock.quote_handoff`, `stock.freshness.read`, and `stock.artifact.read`.

## Capability Model

Macaca models stocks as tenant and region scopes, provider capability reports,
equity instruments, company profile references, listings, financial statement
periods, financial facts, fundamental metrics, dividends, splits, earnings
events, analyst dataset references, screen queries, stock universes, freshness
records, attribution records, cursors, and artifact handles. The model carries
identity hashes, restatement state, source form class, unit metadata, license
class, freshness class, and bounded pagination metadata. Raw filings, raw
provider payloads, personal watchlists, account identifiers, holdings, licensed
datasets, and unbounded financial records stay behind provider adapters.

## Commands And Results

`stock.inspect_provider`, `stock.search_equities`, `stock.get_equity`,
`stock.get_company_profile`, `stock.get_listing`, `stock.get_fundamentals`,
`stock.get_financial_statements`, `stock.get_corporate_events`,
`stock.screen_equities`, `stock.create_universe`,
`stock.plan_quote_handoff`, `stock.inspect_freshness`, and
`stock.get_artifact_handle` are descriptor-owned schema names.

Every command uses a `FinanceCommandEnvelope` with `subject_ref`,
string parameters, optional cursor, optional page size, and optional
idempotency key. Results use `StockResultEnvelope<T>` and may carry a single
DTO, a bounded `FinancePage<T>`, or a sanitized `FinanceError`. Status values
include success, paged, partial, denied, unavailable, unsupported, conflict,
stale-data, schema-mismatch, provider-attribution-required, license-denied,
symbol-ambiguous, symbol-not-found, equity-unsupported, exchange-unsupported,
filing-unavailable, metric-unsupported, period-unsupported,
restatement-conflict, screen-unsupported, range-too-large, quota, timeout,
cancellation, and failure.

Company profiles use redacted display and sector references. Financial facts
use concept references, period references, units, value references, and source
handles instead of raw filings. Screen queries carry bounded filter maps and
maximum result counts. Universes describe membership through counts and cursor
hashes rather than storing user-owned watchlists. Quote handoff creates typed
references for `pack.finance.market.data.v1` and never bypasses market-data
policy, entitlement, freshness, or attribution gates.

## Supplier Mapping

SEC EDGAR submissions, company facts, XBRL concepts, filing dates, accepted
dates, fiscal periods, units, and restatements map to statement periods,
financial facts, metrics, and source handles. Finnhub company profiles,
financials, metrics, recommendations, earnings, estimates, and symbols map to
profiles, fundamentals, analyst dataset references, earnings events, and equity
search. Polygon/Massive ticker reference, dividends, splits, financials, and
snapshots map to equity instruments, corporate events, fundamentals, and quote
handoff references. Alpha Vantage, Nasdaq Data Link, Intrinio, Tiingo, FMP,
exchange feeds, caches, entitlement systems, and attribution systems provide
comparison points only. Provider-specific company identifiers, native filings,
endpoint names, analyst dataset names, and routing rules are not OS semantics.

## App-Facing Examples

- Inspect provider classes and unavailable diagnostics before stock reads.
- Search equities and resolve ambiguity through equity identity hashes.
- Read equity metadata, company profile, listing, fundamentals, statement
  facts, dividends, splits, earnings events, freshness, and artifacts by
  reference.
- Screen equities with bounded filters and create a stock universe descriptor
  without creating personal watchlist or portfolio state.
- Plan a quote handoff to market data through a typed reference instead of
  reading prices through the stock pack.
- Treat missing provider, missing entitlement, license-denied, stale data,
  restatement-conflict, symbol-ambiguous, symbol-not-found, equity-unsupported,
  exchange-unsupported, filing-unavailable, metric-unsupported,
  period-unsupported, screen-unsupported, quote-handoff-unavailable,
  attribution-required, provider-quota, network-denied, timeout, and
  artifact-denied states as structured results. Synthetic examples must use
  synthetic companies and facts only.

## Trace And Audit

Traces should record declaration, admission decision, command name, equity id,
company reference hash, listing id, period id, metric class, screen query hash,
universe id, quote handoff hash, provider class, capability hash, freshness
class, restatement state, attribution reference, result status, cursor hash,
artifact id, and redaction profile. They must not record credentials, account
identifiers, user holdings, personal watchlists, raw filings, raw provider
payloads, licensed datasets, manifests, package bytes, private keys,
signatures, or unbounded financial records.

## Provider Authors

Conformance requires descriptor completeness, equity and company scope
validation, listing and exchange support, identifier support, filing and fact
normalization, fundamental metric validation, corporate-event validation,
screener validation, universe validation, quote-handoff validation,
freshness and restatement labeling, attribution enforcement, license checks,
pagination, resource bounds, timeout and cancellation handling, policy hooks,
trace and audit events, unavailable behavior, snapshot and replay metadata, and
redaction tests. Providers must return structured unavailable, denied,
unsupported, conflict, stale-data, schema-mismatch, license-denied,
restatement-conflict, range-too-large, quota, timeout, cancellation, and failure
results without fabricating fundamentals or storing personal watchlists.
