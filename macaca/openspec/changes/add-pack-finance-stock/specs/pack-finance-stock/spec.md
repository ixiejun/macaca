## ADDED Requirements

### Requirement: Macaca SHALL provide the Finance Stock Pack as a serviceized capability

Macaca SHALL provide `pack.finance.stock.v1` as a provider-neutral industrial
pack for provider inspection, equity search, equity resolution, company
profiles, listing metadata, fundamentals, financial statements, SEC/XBRL-like
facts, dividends, splits, earnings, licensed analyst/estimate dataset
references, equity screening, stock universe descriptors, quote handoff,
freshness diagnostics, attribution, artifact handles, snapshot, and replay. The
pack SHALL be declared by applications, resolved by application admission and
catalog services, and invoked only through typed service commands owned by the
stock data service provider.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.finance.stock.v1` as required and the stock data service provider is registered, healthy, entitled, licensed, permissioned, resource-admissible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy templates, resource limits, attribution requirements, freshness/restatement semantics, health, compatibility, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider credentials, account identifiers, user holdings, personal watchlists, raw filings, raw provider payloads, licensed feed payloads, or application-specific finance workflow metadata

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.finance.stock.v1` as required but provider registration, entitlement, license, permission, credential reference, resource budget, exchange support, equity support, filing/fundamental support, metric support, quote-handoff support, host capability, or policy admission is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, fabricate stock facts, strip freshness/restatement/attribution, contact a concrete provider, or fake success

#### Scenario: Optional declaration degrades explicitly
- **WHEN** an application declares `pack.finance.stock.v1` as optional and the pack is unavailable or partially available
- **THEN** admission SHALL produce a degraded effective capability memento with unavailable commands, reason codes, provider capability hashes when safe, freshness limitations, restatement limitations, attribution requirements, and remediation metadata
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands while still allowing discovery and diagnostics

### Requirement: Finance Stock Pack commands SHALL use typed canonical service calls

Every `pack.finance.stock.v1` operation SHALL be represented as a typed
`stock.*` command/result DTO and SHALL traverse the canonical service runtime
path with trace context, policy, entitlement, license checks, resource
reservation, attribution, freshness/restatement metadata, metering, health,
snapshot, structured errors, and sanitized audit behavior.

#### Scenario: Provider inspection succeeds through service runtime
- **WHEN** a declared caller invokes `stock.inspect_provider`
- **THEN** Macaca SHALL route the typed command through SDK/facade helpers into the service runtime and stock data service provider
- **AND** the result SHALL include bounded provider capability, supported exchanges/countries, identifier types, profile support, listing support, fundamentals support, filing/fact support, corporate-event support, screener support, universe support, quote-handoff support, attribution requirements, quota class, lifecycle, health, and compatibility diagnostics
- **AND** trace and audit events SHALL contain stable trace identifiers and sanitized descriptor metadata only

#### Scenario: Command is denied before provider invocation
- **WHEN** policy, permission, entitlement, license, attribution, resource, freshness, restatement, equity, exchange, filing, metric, screener, universe, quote-handoff, or artifact checks reject a `stock.*` command
- **THEN** Macaca SHALL return a typed denied, license-denied, quota, stale-data, restatement-conflict, unsupported, or unavailable result before invoking any concrete provider
- **AND** the audit trail SHALL include bounded reason codes without raw filings, raw provider payloads, licensed feed payloads, credentials, account data, or unbounded financial datasets

#### Scenario: Provider does not support a command
- **WHEN** the active provider descriptor does not support a requested command such as `stock.get_financial_statements` or `stock.screen_equities`
- **THEN** Macaca SHALL return a typed unsupported result with descriptor hash, provider capability hash, command name, and safe remediation diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: Finance Stock Pack SHALL expose provider-neutral DTOs and stable hashes

`pack.finance.stock.v1` SHALL define provider-neutral DTOs and deterministic
hashing for `StockScope`, `StockProviderCapability`, `EquityInstrument`,
`CompanyProfile`, `StockListing`, `FinancialStatementPeriod`, `FinancialFact`,
`FundamentalMetric`, `StockDividend`, `StockSplit`, `StockEarningsEvent`,
`AnalystDatasetReference`, `StockScreenQuery`, `StockUniverse`,
`StockFreshness`, `StockAttribution`, `StockCursor`, and
`StockArtifactHandle`. Provider-specific extensions SHALL be bounded as adapter
metadata and SHALL NOT drive OS-layer routing.

#### Scenario: Handles and hashes remain replayable
- **WHEN** Macaca records an equity lookup, company profile, listing, statement period, financial fact, metric, corporate event, screen query, universe, quote handoff, freshness report, cursor, artifact handle, or service snapshot
- **THEN** it SHALL include stable descriptor, capability, equity identity, company, listing, statement period, fact, metric, event, screen, universe, quote-handoff, freshness, attribution, cursor, artifact, event cursor, and redaction hashes
- **AND** replay diagnostics SHALL be able to correlate the bounded evidence chain without reconstructing raw filings, raw provider payloads, licensed feed payloads, or unbounded datasets

#### Scenario: Provider metadata is bounded
- **WHEN** a provider returns symbol, exchange, CIK, filing, company, metric, fundamental, corporate-event, screener, estimate, entitlement, attribution, freshness, restatement, or license metadata
- **THEN** the stock service provider SHALL normalize it into provider-neutral DTO fields or bounded `adapter_metadata`
- **AND** the microkernel, SDK, shell, and generic application framework SHALL NOT branch on provider names, exchange names, company names, ticker names, CIK names, dataset names, metric names, model names, or application workflow names

### Requirement: Finance Stock Pack SHALL preserve stock and market-data boundaries

Macaca SHALL treat `pack.finance.stock.v1` as a read-only equity-domain data
capability. Real-time quotes, trades, and generic bars SHALL remain
`pack.finance.market.data.v1` semantics. Stock quote handoff SHALL produce typed
market-data requests or references and SHALL NOT bypass market-data policy.

#### Scenario: Quote handoff preserves market-data policy
- **WHEN** a caller invokes `stock.plan_quote_handoff`
- **THEN** Macaca SHALL validate `pack.finance.market.data.v1` availability, equity handle, venue, symbol disambiguation, entitlement, freshness policy, and attribution policy
- **AND** the result SHALL be a typed market-data request/reference rather than direct provider quote data from the stock service

#### Scenario: No investment advice or trading semantics
- **WHEN** a caller invokes any `stock.*` command
- **THEN** Macaca SHALL return data, handles, diagnostics, or handoff requests only
- **AND** it SHALL NOT place orders, route trades, recommend investments, optimize portfolios, manage holdings, perform tax/accounting logic, or store personal watchlists

#### Scenario: Stock universe is not a personal watchlist
- **WHEN** a caller invokes `stock.create_universe`
- **THEN** Macaca SHALL return a provider-neutral universe descriptor with source query hash, equity count class, membership cursor, entitlement class, attribution class, and replay pointer
- **AND** it SHALL NOT create user-owned watchlist state or portfolio holdings in the stock service

### Requirement: Finance Stock Pack SHALL enforce permissions, entitlement, licensing, freshness, restatement, resource, and attribution gates

Macaca SHALL gate `pack.finance.stock.v1` with explicit permission scopes:
`stock.provider.inspect`, `stock.equity.search`, `stock.equity.read`,
`stock.company.read`, `stock.listing.read`, `stock.fundamentals.read`,
`stock.statements.read`, `stock.corporate_events.read`, `stock.screen`,
`stock.universe`, `stock.quote_handoff`, `stock.freshness.read`, and
`stock.artifact.read`. Reads SHALL also pass entitlement, license, freshness,
restatement, attribution, resource, cache, and output policy checks.

#### Scenario: Financial statements preserve source period and restatement state
- **WHEN** a caller invokes `stock.get_financial_statements`
- **THEN** Macaca SHALL return `FinancialStatementPeriod` and `FinancialFact` data with statement type, fiscal period, filing date, accepted date, source form class, restatement state, currency, units, source handle, attribution, and redaction class
- **AND** it SHALL NOT hide restatement conflicts, unit mismatches, stale source periods, or provider-specific transformations

#### Scenario: License or entitlement is missing
- **WHEN** a caller requests premium fundamentals, analyst estimates, exchange-licensed data, SEC/fundamental data, region-restricted data, redistribution-sensitive data, or quote handoff without the required entitlement
- **THEN** Macaca SHALL return typed `license_denied`, `unavailable`, or `denied` diagnostics before invoking the provider
- **AND** the audit evidence SHALL identify bounded entitlement and license reason codes without exposing raw credentials or provider payloads

#### Scenario: Resource budget is insufficient
- **WHEN** requested period range, page count, equity count, fact count, metric count, corporate-event count, screener complexity, universe size, provider quota, network transfer, timeout, memory, storage, or retained snapshot budget exceeds policy
- **THEN** Macaca SHALL reject the request with a typed quota/resource result
- **AND** the concrete provider SHALL NOT be invoked for rejected requests

### Requirement: Finance Stock Pack SHALL model pagination, cache artifacts, and diagnostics explicitly

Large stock-data reads SHALL return explicit cursors and artifact handles rather
than unbounded payloads. Cache/artifact handles SHALL carry license, freshness,
restatement, retention, attribution, request hash, and replay metadata.

#### Scenario: Screener result is paged
- **WHEN** a `stock.screen_equities` request returns more results than the configured page limit
- **THEN** Macaca SHALL return a bounded page with `StockCursor`, request hash, freshness metadata, attribution metadata, and next-page diagnostics
- **AND** callers SHALL use the cursor through the canonical service path rather than provider-specific pagination APIs

#### Scenario: Artifact handle is resolved safely
- **WHEN** a caller invokes `stock.get_artifact_handle`
- **THEN** Macaca SHALL enforce artifact permission, retention policy, entitlement, license, attribution, restatement metadata, and redaction before returning bounded artifact metadata
- **AND** the result SHALL NOT include raw filings, raw provider payloads, licensed feed payloads, signed provider URLs beyond policy, or unbounded datasets

### Requirement: Finance Stock Pack SHALL provide sanitized trace, audit, health, snapshot, and replay evidence

`pack.finance.stock.v1` SHALL emit sanitized declaration, admission,
provider-inspection, equity-search, equity-read, profile-read, listing-read,
fundamentals-read, statement-read, corporate-event-read, screen, universe,
quote-handoff, freshness, artifact, policy, entitlement, license, resource,
health, unavailable, failure, snapshot, and replay events. Snapshots SHALL be
bounded and replayable.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.finance.stock.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, command availability, provider health, policy template hash, resource counters, bounded equity/company/listing/dataset/request/cursor/artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, account identifiers, user holdings, personal watchlists, raw filings, raw provider payloads, licensed feed payloads, manifests, package bytes, private keys, signatures, and unbounded financial datasets

#### Scenario: Replay follows the canonical path
- **WHEN** audit replay reconstructs a `stock.*` command chain
- **THEN** it SHALL show descriptor admission, SDK/facade service call, policy decision, entitlement/license decision, resource decision, provider dispatch, freshness/restatement/attribution evidence, cursor/artifact state, and result evidence
- **AND** replay SHALL NOT require direct provider APIs, raw filings, raw provider payloads, licensed feed payloads, or shell-owned state

### Requirement: Finance Stock Pack SHALL preserve Macaca architecture boundaries

The `pack.finance.stock.v1` implementation SHALL preserve Macaca's microkernel,
service runtime, application framework, SDK, runtime-host, plugin, and shell
boundaries. Concrete stock providers SHALL be replaceable Strategy adapters
created only by approved runtime-host composition roots. SDK helpers SHALL only
build typed service commands and SHALL NOT create providers, call vendor APIs
directly, store personal watchlists, remove attribution, hide freshness or
restatements, infer advice, or trade.

#### Scenario: Dependency gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, canonical execution-path, and shell-boundary gates scan the implementation
- **THEN** they SHALL find no concrete SEC, Finnhub, Polygon/Massive, Alpha Vantage, Nasdaq, Intrinio, Tiingo, FMP, exchange-feed, market-data provider, cache, entitlement, credential-manager, or provider adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed `stock.*` service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable stock provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract, unavailable behavior, health semantics, trace shape, audit semantics, freshness model, restatement model, and attribution model
- **AND** provider-specific details SHALL appear only as sanitized descriptor/capability data, not as OS-layer routing branches

### Requirement: Finance Stock Pack SHALL include industrial developer documentation

Macaca SHALL include detailed developer documentation for
`pack.finance.stock.v1` at `docs/developer-packs/finance/stock.md` before
implementation completion. The documentation SHALL describe capability
declaration, required versus optional behavior, DTOs, commands, permissions,
entitlement, licensing, freshness, restatement, attribution, stock versus
market-data boundaries, pagination, cache/artifact handling, provider
replacement, unavailable states, trace/audit/replay, conformance tests, and
supplier/API mapping.

#### Scenario: Developer reads the guide
- **WHEN** a developer opens `docs/developer-packs/finance/stock.md`
- **THEN** the guide SHALL explain equity instruments, identifiers, company profiles, listings, statement periods, financial facts, metrics, dividends, splits, earnings, analyst dataset references, screens, universes, quote handoff, freshness, attribution, entitlements, licenses, cursors, artifacts, diagnostics, and operational limits
- **AND** examples SHALL use synthetic companies, equities, listings, facts, statements, events, screens, universes, and artifacts only

#### Scenario: Provider author checks conformance
- **WHEN** a provider author uses the documentation to implement a provider
- **THEN** the guide SHALL include conformance checks for descriptor completeness, DTO compatibility, command support, stable hashing, scope validation, exchange support, identifier support, filing/fact support, fundamentals validation, metric validation, corporate-event validation, screener validation, universe validation, quote-handoff validation, freshness/restatement labeling, attribution enforcement, license checks, pagination, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction
- **AND** the guide SHALL map SEC EDGAR, Finnhub, Polygon/Massive, Alpha Vantage, Nasdaq Data Link, Intrinio, Tiingo, FMP, exchange feed, cache, entitlement, market-data handoff, and attribution concepts to Macaca abstractions without making supplier-specific behavior OS semantics
