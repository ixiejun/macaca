## ADDED Requirements

### Requirement: Macaca SHALL provide the Finance Market Data Pack as a serviceized capability

Macaca SHALL provide `pack.finance.market.data.v1` as a provider-neutral
industrial pack for provider inspection, instrument search, instrument metadata,
latest quotes, latest trades, historical bars/candles, snapshots, corporate
actions, market status/session inspection, data freshness diagnostics,
attribution, entitlement diagnostics, artifact handles, snapshot, and replay.
The pack SHALL be declared by applications, resolved by application admission
and catalog services, and invoked only through typed service commands owned by
the market data service provider.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.finance.market.data.v1` as required and the market data service provider is registered, healthy, entitled, licensed, permissioned, resource-admissible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy templates, resource limits, attribution requirements, freshness classes, health, compatibility, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider credentials, account identifiers, user holdings, raw provider payloads, licensed feed payloads, or application-specific finance workflow metadata

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.finance.market.data.v1` as required but provider registration, entitlement, exchange license, permission, credential reference, resource budget, asset class, venue, symbol, freshness support, host capability, or policy admission is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, fabricate market prices, strip freshness labels, strip attribution, contact a concrete provider, or fake success

#### Scenario: Optional declaration degrades explicitly
- **WHEN** an application declares `pack.finance.market.data.v1` as optional and the pack is unavailable or partially available
- **THEN** admission SHALL produce a degraded effective capability memento with unavailable commands, reason codes, provider capability hashes when safe, freshness limitations, attribution requirements, and remediation metadata
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands while still allowing discovery and diagnostics

### Requirement: Finance Market Data Pack commands SHALL use typed canonical service calls

Every `pack.finance.market.data.v1` operation SHALL be represented as a typed
`market_data.*` command/result DTO and SHALL traverse the canonical service
runtime path with trace context, policy, entitlement, license checks, resource
reservation, attribution, metering, health, snapshot, structured errors, and
sanitized audit behavior.

#### Scenario: Provider inspection succeeds through service runtime
- **WHEN** a declared caller invokes `market_data.inspect_provider`
- **THEN** Macaca SHALL route the typed command through SDK/facade helpers into the service runtime and market data service provider
- **AND** the result SHALL include bounded provider capability, asset classes, venues, real-time/delayed/end-of-day support, quote/trade/bar/snapshot/corporate-action support, interval support, adjustment support, identifier support, attribution requirements, quota class, lifecycle, health, and compatibility diagnostics
- **AND** trace and audit events SHALL contain stable trace identifiers and sanitized descriptor metadata only

#### Scenario: Command is denied before provider invocation
- **WHEN** policy, permission, entitlement, exchange license, attribution, resource, freshness, venue, asset class, symbol, range, interval, adjustment, or artifact checks reject a `market_data.*` command
- **THEN** Macaca SHALL return a typed denied, license-denied, quota, stale-data, unsupported, or unavailable result before invoking any concrete provider
- **AND** the audit trail SHALL include bounded reason codes without raw provider payloads, licensed feed payloads, credentials, account data, or unbounded market datasets

#### Scenario: Provider does not support a command
- **WHEN** the active provider descriptor does not support a requested command such as `market_data.get_corporate_actions` or `market_data.get_snapshot`
- **THEN** Macaca SHALL return a typed unsupported result with descriptor hash, provider capability hash, command name, and safe remediation diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: Finance Market Data Pack SHALL expose provider-neutral DTOs and stable hashes

`pack.finance.market.data.v1` SHALL define provider-neutral DTOs and
deterministic hashing for `MarketDataScope`, `MarketDataProviderCapability`,
`InstrumentHandle`, `InstrumentIdentity`, `MarketVenue`, `MarketSession`,
`MarketQuote`, `MarketTrade`, `MarketBar`, `MarketBarSeries`,
`MarketSnapshot`, `CorporateAction`, `MarketDataFreshness`,
`MarketDataAttribution`, `MarketDataCursor`, and
`MarketDataArtifactHandle`. Provider-specific extensions SHALL be bounded as
adapter metadata and SHALL NOT drive OS-layer routing.

#### Scenario: Handles and hashes remain replayable
- **WHEN** Macaca records an instrument lookup, quote read, trade read, bar series, snapshot, corporate action, freshness report, cursor, artifact handle, or service snapshot
- **THEN** it SHALL include stable descriptor, capability, instrument identity, venue/session, request, result, freshness, attribution, cursor, artifact, event cursor, and redaction hashes
- **AND** replay diagnostics SHALL be able to correlate the bounded evidence chain without reconstructing raw provider payloads, licensed feed payloads, or unbounded datasets

#### Scenario: Provider metadata is bounded
- **WHEN** a provider returns symbol, exchange, dataset, quote, trade, bar, corporate-action, entitlement, attribution, freshness, or license metadata
- **THEN** the market data service provider SHALL normalize it into provider-neutral DTO fields or bounded `adapter_metadata`
- **AND** the microkernel, SDK, shell, and generic application framework SHALL NOT branch on provider names, exchange names, asset names, symbol names, dataset names, plan names, account names, or application workflow names

### Requirement: Finance Market Data Pack SHALL preserve read-only market data semantics

Macaca SHALL treat `pack.finance.market.data.v1` as a read-only data capability.
It SHALL NOT place orders, route trades, provide investment advice, optimize
portfolios, manage brokerage accounts, or infer trading decisions. Data results
SHALL carry explicit freshness, attribution, entitlement, and adjustment
metadata.

#### Scenario: Quote read preserves freshness and attribution
- **WHEN** a caller invokes `market_data.get_quote`
- **THEN** Macaca SHALL return `MarketQuote` with instrument handle, bid/ask classes, venue handles, quote timestamp, sequence/correction class, freshness class, attribution, and redaction class
- **AND** it SHALL NOT omit delayed/stale/corrected status or vendor/exchange attribution required by policy

#### Scenario: Historical bars preserve range and adjustment policy
- **WHEN** a caller invokes `market_data.get_bars`
- **THEN** Macaca SHALL validate range, interval, pagination, asset class, venue, adjustment policy, currency, timezone, freshness policy, and quota before returning `MarketBarSeries`
- **AND** the result SHALL include adjustment and attribution metadata rather than silently applying provider-specific price transformations

#### Scenario: Instrument search is bounded and ambiguous symbols are explicit
- **WHEN** a caller invokes `market_data.search_instruments`
- **THEN** Macaca SHALL return paged bounded instrument matches with identifiers, asset class, venue, currency, listing status, entitlement class, and attribution class
- **AND** ambiguous or missing symbols SHALL return typed `symbol_ambiguous` or `symbol_not_found` diagnostics rather than guessing a default instrument

### Requirement: Finance Market Data Pack SHALL enforce permissions, entitlement, licensing, freshness, resource, and attribution gates

Macaca SHALL gate `pack.finance.market.data.v1` with explicit permission scopes:
`market_data.provider.inspect`, `market_data.instrument.search`,
`market_data.instrument.read`, `market_data.quote.read`,
`market_data.trade.read`, `market_data.bars.read`,
`market_data.snapshot.read`, `market_data.corporate_actions.read`,
`market_data.market_status.read`, `market_data.freshness.read`, and
`market_data.artifact.read`. Reads SHALL also pass entitlement, license,
freshness, attribution, resource, cache, and output policy checks.

#### Scenario: License or entitlement is missing
- **WHEN** a caller requests real-time, exchange-licensed, premium, region-restricted, redistribution-sensitive, or provider-plan-limited market data without the required entitlement
- **THEN** Macaca SHALL return typed `license_denied`, `unavailable`, or `denied` diagnostics before invoking the provider
- **AND** the audit evidence SHALL identify bounded entitlement and license reason codes without exposing raw credentials or provider payloads

#### Scenario: Data is stale or delayed
- **WHEN** provider, exchange, or cache timestamps indicate delayed, cached, corrected, or stale data
- **THEN** Macaca SHALL include `MarketDataFreshness` with provider timestamp, exchange timestamp, cache timestamp, delay class, stale reason, correction state, and replay pointer
- **AND** it SHALL NOT represent delayed or stale data as real-time data

#### Scenario: Resource budget is insufficient
- **WHEN** requested result size, historical range, page count, instrument count, bar count, corporate-action count, provider quota, network transfer, timeout, memory, storage, or retained snapshot budget exceeds policy
- **THEN** Macaca SHALL reject the request with a typed quota/resource result
- **AND** the concrete provider SHALL NOT be invoked for rejected requests

### Requirement: Finance Market Data Pack SHALL model pagination, cache artifacts, and diagnostics explicitly

Large or paged market data reads SHALL return explicit cursors and artifact
handles rather than unbounded payloads. Cache/artifact handles SHALL carry
license, freshness, retention, attribution, request hash, and replay metadata.

#### Scenario: Historical bars are paged
- **WHEN** a `market_data.get_bars` request spans more data than the configured page limit
- **THEN** Macaca SHALL return a bounded page with `MarketDataCursor`, request hash, freshness metadata, attribution metadata, and next-page diagnostics
- **AND** callers SHALL use the cursor through the canonical service path rather than provider-specific pagination APIs

#### Scenario: Artifact handle is resolved safely
- **WHEN** a caller invokes `market_data.get_artifact_handle`
- **THEN** Macaca SHALL enforce artifact permission, retention policy, entitlement, license, attribution, and redaction before returning bounded artifact metadata
- **AND** the result SHALL NOT include raw provider payloads, licensed feed payloads, signed provider URLs beyond policy, or unbounded datasets

### Requirement: Finance Market Data Pack SHALL provide sanitized trace, audit, health, snapshot, and replay evidence

`pack.finance.market.data.v1` SHALL emit sanitized declaration, admission,
provider-inspection, instrument-search, instrument-read, quote-read, trade-read,
bars-read, snapshot-read, corporate-action-read, market-status, freshness,
artifact, policy, entitlement, license, resource, health, unavailable, failure,
snapshot, and replay events. Snapshots SHALL be bounded and replayable.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.finance.market.data.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, command availability, provider health, policy template hash, resource counters, bounded instrument/venue/request/cursor/artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, account identifiers, user holdings, raw provider payloads, licensed feed payloads, manifests, package bytes, private keys, signatures, and unbounded market data

#### Scenario: Replay follows the canonical path
- **WHEN** audit replay reconstructs a `market_data.*` command chain
- **THEN** it SHALL show descriptor admission, SDK/facade service call, policy decision, entitlement/license decision, resource decision, provider dispatch, freshness/attribution evidence, cursor/artifact state, and result evidence
- **AND** replay SHALL NOT require direct provider APIs, raw provider payloads, licensed feed payloads, or shell-owned state

### Requirement: Finance Market Data Pack SHALL preserve Macaca architecture boundaries

The `pack.finance.market.data.v1` implementation SHALL preserve Macaca's
microkernel, service runtime, application framework, SDK, runtime-host, plugin,
and shell boundaries. Concrete market data providers SHALL be replaceable
Strategy adapters created only by approved runtime-host composition roots. SDK
helpers SHALL only build typed service commands and SHALL NOT create providers,
call vendor APIs directly, remove attribution, hide freshness, infer advice, or
trade.

#### Scenario: Dependency gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, canonical execution-path, and shell-boundary gates scan the implementation
- **THEN** they SHALL find no concrete Polygon/Massive, Alpaca, Nasdaq, Finnhub, Alpha Vantage, Tiingo, Intrinio, exchange-feed, cache, entitlement, credential-manager, or provider adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed `market_data.*` service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable market data provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract, unavailable behavior, health semantics, trace shape, audit semantics, freshness model, and attribution model
- **AND** provider-specific details SHALL appear only as sanitized descriptor/capability data, not as OS-layer routing branches

### Requirement: Finance Market Data Pack SHALL include industrial developer documentation

Macaca SHALL include detailed developer documentation for
`pack.finance.market.data.v1` at
`docs/developer-packs/finance/market-data.md` before implementation completion.
The documentation SHALL describe capability declaration, required versus
optional behavior, DTOs, commands, permissions, entitlement, licensing,
freshness, attribution, pagination, cache/artifact handling, provider
replacement, unavailable states, trace/audit/replay, conformance tests, and
supplier/API mapping.

#### Scenario: Developer reads the guide
- **WHEN** a developer opens `docs/developer-packs/finance/market-data.md`
- **THEN** the guide SHALL explain provider scopes, instruments, identifiers, venues, sessions, quotes, trades, bars, snapshots, corporate actions, freshness, attribution, entitlements, licenses, cursors, artifacts, diagnostics, and operational limits
- **AND** examples SHALL use synthetic instruments, venues, quotes, trades, bars, corporate actions, sessions, and artifacts only

#### Scenario: Provider author checks conformance
- **WHEN** a provider author uses the documentation to implement a provider
- **THEN** the guide SHALL include conformance checks for descriptor completeness, DTO compatibility, command support, stable hashing, scope validation, asset-class support, venue support, freshness labeling, attribution enforcement, license checks, pagination, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction
- **AND** the guide SHALL map Polygon/Massive, Alpaca, Nasdaq Data Link, Finnhub, Alpha Vantage, Tiingo, Intrinio, exchange feed, cache, entitlement, and attribution concepts to Macaca abstractions without making supplier-specific behavior OS semantics
