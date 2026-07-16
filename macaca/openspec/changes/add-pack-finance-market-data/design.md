# Finance Market Data Pack Design

## Context

`pack.finance.market.data.v1` exposes market data as a Macaca OS serviceized
capability. It lets applications discover instruments, inspect instrument
metadata, retrieve quotes, trades, bars, snapshots, market status, corporate
actions, and data freshness diagnostics without embedding a vendor SDK,
exchange feed, brokerage adapter, cache backend, or application-specific
finance workflow into generic OS layers.

Market data is not a trading service. It is read-only reference and time-series
data with strict licensing, attribution, freshness, delay, region, asset-class,
and entitlement semantics. The pack treats raw provider payloads, licensed feed
payloads, credentials, cache keys, and unbounded historical datasets as
sensitive. Results return bounded provider-neutral DTOs with explicit
freshness, adjustment, attribution, and entitlement metadata.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Polygon/Massive | Ticker reference, quotes, trades, aggregates, snapshots, market status, corporate actions | Instrument handle, quote/trade DTOs, bar series, snapshot, market status, corporate action |
| Alpaca Market Data | Real-time/historical equities, options, crypto, bars, quotes, trades, snapshots, plan/rate-limit behavior | Asset-class capability, freshness class, paged market data reads, provider quota diagnostics |
| Nasdaq Data Link | REST/streaming APIs for real-time, delayed, tables, and time-series datasets | Dataset capability, table/time-series cursor, entitlement and attribution DTOs |
| Finnhub | Quote, candles, symbol lookup, market status, fundamentals/economic datasets | Symbol search, quote/bar reads, market session, optional dataset descriptors |
| Alpha Vantage / Tiingo / Intrinio / exchange feeds | Adjusted bars, identifiers, corporate actions, reference datasets, rate limits, exchange licensing | Adjustment policy, identifier mapping, corporate action, provider attribution, license diagnostics |

The pack exposes provider-neutral contracts. Provider adapters translate to
vendor REST APIs, streaming feeds, exchange feeds, historical databases, cache
stores, reference datasets, or unavailable providers. OS layers must not branch
on provider names, exchange names, symbols, plan names, dataset names, account
names, or business workflows.

## Goals

- Provide stable pack id `pack.finance.market.data.v1` and command namespace
  `market_data.*`.
- Support provider inspection, instrument search, instrument metadata, quote,
  trade, bars/candles, snapshots, corporate actions, market status/session,
  data freshness, attribution, entitlement diagnostics, cache/artifact handles,
  health, snapshots, and replay diagnostics.
- Preserve financial safety with read-only semantics, data freshness labels,
  stale-data diagnostics, exchange/vendor attribution, entitlement checks,
  delayed/real-time/end-of-day data classes, pagination, bounded output, and
  sanitized audit.
- Keep concrete market data vendors behind replaceable service providers.
- Require developer documentation at
  `docs/developer-packs/finance/market-data.md`.

## Non-Goals

- Do not implement concrete Polygon/Massive, Alpaca, Nasdaq, Finnhub, Alpha
  Vantage, Tiingo, Intrinio, exchange-feed, cache, or entitlement providers in
  this proposal.
- Do not define trading, order routing, investment advice, portfolio allocation,
  tax advice, brokerage workflows, alerting workflows, or application-specific
  finance logic.
- Do not expose raw credentials, account identifiers, user holdings, raw
  provider payloads, licensed feed payloads, manifests, package bytes, private
  keys, signatures, or unbounded market datasets in observability.
- Do not silently substitute providers, remove delay/freshness labels, infer
  investment advice, adjust prices without explicit adjustment policy, or fake
  success when provider, market, exchange, entitlement, license, symbol,
  freshness, permission, resource, or host support is absent.

## Ownership And Boundaries

- Pack id: `pack.finance.market.data.v1`.
- Family: `finance`.
- Backing service owner: market data service provider.
- SDK surface: `sdk.packs.finance.market_data`.
- Command namespace: `market_data.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridges,
  entitlement/cache bridges, decorators, and sanitized diagnostics through
  approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `market_data.inspect_provider` | Inspect provider, asset-class, exchange, freshness, corporate-action, history, snapshot, and entitlement support | Returns sanitized capability, quota, lifecycle, health, attribution, and compatibility metadata |
| `market_data.search_instruments` | Search instruments by text, identifier, exchange, asset class, or dataset | Requires bounded query, region/license policy, paging, and redaction |
| `market_data.get_instrument` | Resolve instrument metadata and identifiers | Requires instrument permission, entitlement, and bounded provider-neutral metadata |
| `market_data.get_quote` | Retrieve latest or time-scoped quote | Requires instrument scope, freshness class, entitlement, attribution, and stale-data diagnostics |
| `market_data.get_trade` | Retrieve latest or time-scoped trade | Requires instrument scope, freshness class, entitlement, and bounded result |
| `market_data.get_bars` | Retrieve historical bars/candles | Requires range validation, interval support, adjustment policy, pagination, quota, and attribution |
| `market_data.get_snapshot` | Retrieve provider-neutral instrument snapshot | Requires quote/trade/bar support, entitlement, stale-data metadata, and bounded result |
| `market_data.get_corporate_actions` | Retrieve splits, dividends, symbol changes, mergers, or comparable actions | Requires date range, action type filter, entitlement, and adjustment metadata |
| `market_data.inspect_market_status` | Inspect exchange/venue market session and calendar status | Requires venue scope and bounded market status metadata |
| `market_data.inspect_freshness` | Inspect data delay, correction, source timestamp, cache timestamp, and staleness diagnostics | Requires provider/instrument scope and attribution |
| `market_data.get_artifact_handle` | Resolve cached/paged result artifact metadata | Requires artifact permission, retention, and licensing policy |

Every command must define typed command DTOs, typed success results, typed
paged/partial results, typed denied/unavailable/unsupported/conflict/
stale-data/schema-mismatch/provider-attribution-required/license-denied/
symbol-ambiguous/symbol-not-found/exchange-unsupported/asset-class-unsupported/
range-too-large/interval-unsupported/adjustment-unsupported/quota/timeout/
cancellation/failure results, redaction profile, cache semantics, idempotency
semantics for cache-producing reads, and replay metadata.

## DTO Model

Core DTOs:

- `MarketDataScope`: provider scope, asset class, venue/exchange scope,
  dataset scope, credential reference, entitlement state, license policy,
  freshness policy, permission state, rate-limit profile, and health.
- `MarketDataProviderCapability`: provider class, asset classes, venues,
  real-time/delayed/end-of-day support, quote/trade/bar/snapshot/corporate
  action support, interval support, adjustment support, identifiers, pagination
  model, attribution requirements, auth modes, rate limits, lifecycle, and
  health.
- `InstrumentHandle`: instrument handle, provider scope, canonical symbol,
  identifier set, asset class, venue/exchange, currency, timezone, status,
  listing class, entitlement class, attribution class, and freshness.
- `InstrumentIdentity`: canonical symbol, alternate symbols, FIGI/ISIN/CUSIP/
  SEDOL-like identifier slots when available, exchange codes, dataset codes,
  currency, locale, and redaction class.
- `MarketVenue`: venue handle, exchange code, region, timezone, session model,
  holiday calendar class, and attribution class.
- `MarketSession`: venue handle, session date, state, open/close classes,
  extended-hours support, source timestamp, and freshness class.
- `MarketQuote`: instrument handle, bid/ask price classes, bid/ask size
  classes, exchange/venue handles, quote timestamp, sequence/correction class,
  freshness class, attribution, and redaction class.
- `MarketTrade`: instrument handle, price class, size class, venue, trade
  timestamp, condition classes, correction class, freshness class, attribution,
  and redaction class.
- `MarketBar` and `MarketBarSeries`: instrument handle, interval, range,
  open/high/low/close/volume classes, adjustment policy, currency, timezone,
  source timestamp range, page cursor, freshness class, attribution, and
  redaction class.
- `MarketSnapshot`: instrument handle, latest quote/trade/bar summaries,
  market state, source timestamps, freshness class, attribution, and redaction
  class.
- `CorporateAction`: instrument handle, action type, effective date, declared
  date, ratio/amount class, currency, affected identifiers, adjustment impact,
  source timestamp, attribution, and redaction class.
- `MarketDataFreshness`: provider timestamp, exchange timestamp, cache
  timestamp, delay class, stale reason, correction state, trading-session
  relation, and replay pointer.
- `MarketDataAttribution`: provider class, exchange/license class, display
  requirement, redistribution class, and audit hash.
- `MarketDataCursor` and `MarketDataArtifactHandle`: page/artifact handles,
  request hash, retention, license policy, checksum handle, redaction class,
  and replay pointer.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `market_data.provider.inspect`
- `market_data.instrument.search`
- `market_data.instrument.read`
- `market_data.quote.read`
- `market_data.trade.read`
- `market_data.bars.read`
- `market_data.snapshot.read`
- `market_data.corporate_actions.read`
- `market_data.market_status.read`
- `market_data.freshness.read`
- `market_data.artifact.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, instrument handle when applicable, venue handle
  when applicable, dataset scope when applicable, credential reference,
  entitlement state, license policy, freshness policy, and permission state.
- Reads must preserve freshness and attribution metadata. Real-time, delayed,
  end-of-day, cached, corrected, and stale data classes must be explicit.
- Historical reads require bounded range, supported interval, pagination,
  adjustment policy, currency/timezone metadata, and quota checks.
- Redistribution-sensitive, real-time, exchange-licensed, premium, region-
  restricted, or large historical requests may require entitlement or approval.
- Raw provider payloads and licensed feed payloads must not enter logs, traces,
  snapshots, SDK diagnostics, examples, or replay evidence.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
asset classes, venue support, quote/trade/bar/snapshot/corporate-action
support, interval support, adjustment support, identifier support, freshness
classes, attribution requirements, permission scopes, policy templates,
resource limits, approval rules, provider capability hashes, health,
compatibility, diagnostics, examples, redaction profiles, and documentation
links.

The developer guide at `docs/developer-packs/finance/market-data.md` must cover:

- manifest declaration and optional/required behavior
- provider scopes, instruments, identifiers, venues, sessions, quotes, trades,
  bars, snapshots, corporate actions, freshness, attribution, entitlements,
  page cursors, artifacts, provider capabilities, and unavailable states
- range limits, interval support, adjustment policies, delayed versus real-time
  data, stale-data diagnostics, corrected data, cache behavior, license and
  redistribution boundaries, provider replacement, trace/audit interpretation,
  and conformance tests

Examples must use synthetic instruments, venues, quotes, trades, bars,
corporate actions, sessions, and artifacts. They must not include provider
names, credentials, real account data, user holdings, live trading strategies,
investment advice, raw provider payloads, or workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `market_data_pack_declared`
- `market_data_pack_admission_validated`
- `market_data_provider_inspected`
- `market_data_instruments_searched`
- `market_data_instrument_resolved`
- `market_data_quote_read`
- `market_data_trade_read`
- `market_data_bars_read`
- `market_data_snapshot_read`
- `market_data_corporate_actions_read`
- `market_data_market_status_inspected`
- `market_data_freshness_inspected`
- `market_data_artifact_handle_resolved`
- `market_data_pack_policy_decision`
- `market_data_pack_service_call_requested`
- `market_data_pack_service_call_succeeded`
- `market_data_pack_service_call_failed`
- `market_data_pack_unavailable`
- `market_data_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, command
availability, provider health, policy template hash, resource counters, bounded
instrument/venue/request/cursor/artifact summaries, event cursors, and
sanitized replay pointers. Snapshots must exclude raw credentials, account
identifiers, user holdings, raw provider payloads, licensed feed payloads,
manifests, package bytes, private keys, signatures, and unbounded market data.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider adapters, symbol resolvers, historical data readers,
  reference data readers, corporate-action readers, cache readers, attribution
  resolvers, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, metering, licensing,
  attribution, freshness, cache, and output redaction wrap service calls.
- **Specification**: admission validates provider scope, command availability,
  permissions, entitlement, asset class, venue, symbol, range, interval,
  adjustment policy, freshness policy, and compatibility.
- **Observer**: provider health, trace, audit, service events, and cache/artifact
  lifecycle events are subscribable.
- **Memento**: capability hashes, request hashes, cursors, cache handles,
  snapshots, and replay pointers preserve recovery state.
- **Abstract Factory**: concrete market data providers are created only by
  approved runtime-host composition roots.

## Risks And Mitigations

- Risk: pack becomes a wrapper around one vendor symbol model. Mitigation:
  provider-neutral instrument identity, venue, freshness, attribution, and
  cursor DTOs.
- Risk: stale data is used as if it were real-time. Mitigation: mandatory
  freshness classes, source timestamps, stale diagnostics, and replay evidence.
- Risk: licensed provider payloads leak. Mitigation: bounded DTOs, attribution
  metadata, redaction, and strict observability exclusions.
- Risk: market data pack becomes trading/advice. Mitigation: read-only command
  surface and explicit non-goals for trading, portfolio, and advice semantics.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call provider APIs directly.
