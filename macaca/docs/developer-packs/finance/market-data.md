# Finance Market Data Pack

`pack.finance.market.data.v1` describes provider-neutral, read-only market data
capabilities. The descriptor is discoverable through SDK catalogs, but commands
remain unavailable until a serviceized provider is installed through the runtime
composition root.

## Manifest Declaration

Declare the pack as required only when market data is mandatory for application
readiness. Optional declarations degrade with structured unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.finance.market.data.v1"]
```

## Permissions

Use the narrowest scope: `market_data.provider.inspect`,
`market_data.instrument.search`, `market_data.instrument.read`,
`market_data.quote.read`, `market_data.trade.read`, `market_data.bars.read`,
`market_data.snapshot.read`, `market_data.corporate_actions.read`,
`market_data.market_status.read`, `market_data.freshness.read`, and
`market_data.artifact.read`.

## Capability Model

Macaca models market data as tenant and dataset scopes, provider capability
reports, opaque instrument handles, normalized instrument identities, venues,
sessions, quotes, trades, bar series, snapshots, corporate actions, freshness
records, attribution records, cursors, and artifact handles. The model carries
identity hashes, freshness labels, redistribution policy, license class, and
bounded pagination metadata. Raw provider payloads, credentials, account data,
holdings, licensed feed payloads, and unbounded historical datasets stay behind
provider adapters.

## Commands And Results

`market_data.inspect_provider`, `market_data.search_instruments`,
`market_data.get_instrument`, `market_data.get_quote`,
`market_data.get_trade`, `market_data.get_bars`,
`market_data.get_snapshot`, `market_data.get_corporate_actions`,
`market_data.inspect_market_status`, `market_data.inspect_freshness`, and
`market_data.get_artifact_handle` are descriptor-owned schema names.

Every command uses a `FinanceCommandEnvelope` with `subject_ref`,
string parameters, optional cursor, optional page size, and optional
idempotency key. Results use `MarketDataResultEnvelope<T>` and may carry a
single DTO, a bounded `FinancePage<T>`, or a sanitized `FinanceError`. Status
values include success, paged, partial, denied, unavailable, unsupported,
conflict, stale-data, schema-mismatch, provider-attribution-required,
license-denied, symbol-ambiguous, symbol-not-found, exchange-unsupported,
asset-class-unsupported, range-too-large, interval-unsupported,
adjustment-unsupported, quota, timeout, cancellation, and failure.

Quotes and trades carry timestamps, freshness, attribution, and instrument
handles. Bars carry bounded time ranges, interval labels, adjustment policy,
volume, and attribution. Snapshots carry hashes to latest quote, trade, and bar
objects instead of embedding unbounded feed payloads. Corporate actions carry
effective dates and adjustment hashes. Cursors and artifacts carry expiry and
retention metadata so replay can prove lineage without retaining raw feeds.

## Supplier Mapping

Polygon/Massive ticker reference, quotes, trades, aggregates, snapshots,
market status, and corporate actions map to instruments, quotes, trades, bars,
snapshots, sessions, and corporate actions. Alpaca real-time and historical
market data maps to provider capability, quote, trade, bar, and snapshot
support. Nasdaq Data Link table and time-series datasets map to dataset scopes,
cursors, and artifact handles. Finnhub symbols, quotes, candles, market status,
and quotas map to instrument identity, quote, bar, session, freshness, and quota
diagnostics. Alpha Vantage, Tiingo, Intrinio, exchange direct feeds, caches,
entitlement systems, and attribution systems provide comparison points only.
Provider-specific endpoints, symbols, native error payloads, pricing terms,
subscription names, and routing rules are not OS semantics.

## App-Facing Examples

- Inspect provider classes and unavailable diagnostics before issuing requests.
- Search instruments using bounded parameters and disambiguate with instrument
  identity hashes.
- Read an instrument, quote, trade, bar series, snapshot, corporate action list,
  market status, or freshness report through opaque handles.
- Use cursors for large history requests and artifact handles for retained
  exports.
- Treat missing provider, missing entitlement, license-denied, stale data,
  delayed-only, symbol-ambiguous, symbol-not-found, exchange-unsupported,
  asset-class-unsupported, range-too-large, interval-unsupported,
  adjustment-unsupported, attribution-required, provider-quota,
  network-denied, timeout, and artifact-denied states as structured results.
  Synthetic examples must use synthetic instruments and prices only.

## Trace And Audit

Traces should record declaration, admission decision, command name, instrument
id, venue id, request hash, provider class, provider capability hash, freshness
class, attribution reference, result status, cursor hash, artifact id, and
redaction profile. They must not record credentials, account identifiers, user
holdings, raw provider payloads, licensed feed payloads, manifests, package
bytes, private keys, signatures, or unbounded market datasets.

## Provider Authors

Conformance requires descriptor completeness, provider and dataset scope
validation, asset-class support, venue and session validation, quote validation,
trade validation, bar range and interval validation, corporate-action
validation, freshness labeling, attribution enforcement, license checks,
pagination, resource bounds, timeout and cancellation handling, policy hooks,
trace and audit events, unavailable behavior, snapshot and replay metadata, and
redaction tests. Providers must return structured unavailable, denied,
unsupported, conflict, stale-data, schema-mismatch, license-denied,
range-too-large, quota, timeout, cancellation, and failure results without
fabricating prices or stripping attribution.
