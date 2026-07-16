## ADDED Requirements

### Requirement: Macaca SHALL provide Finance Portfolio as a serviceized pack

Macaca SHALL provide `pack.finance.portfolio.v1` as a provider-neutral,
serviceized finance pack for portfolio accounts, holdings, lots, balances,
transactions, valuations, allocation, exposure, performance, risk, scenario,
rebalance-intent planning, reports, and artifact handles.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.finance.portfolio.v1` as required and the portfolio service provider is registered, healthy, entitled, consented, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with descriptor hash, command schemas, permission scopes, policy template, provider capability, health, consent, freshness, and replay metadata
- **AND** SDK discovery SHALL mark supported commands as callable without exposing credentials, raw account numbers, raw provider payloads, proprietary model output, full holdings, or full transaction rows

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.finance.portfolio.v1` as required but provider, consent, permission, entitlement, policy, resource, host support, or account access is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, call another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.finance.portfolio.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Finance Portfolio SHALL expose provider capability discovery

`pack.finance.portfolio.v1` SHALL expose provider-neutral capability discovery
for account types, instrument classes, command support, lot support, transaction
history depth, performance methods, risk/scenario analytics, report/export
formats, consent state, freshness, attribution, geography, entitlement, and
unsupported limitations.

#### Scenario: Provider capabilities are inspected
- **WHEN** an application invokes `portfolio.inspect_provider` with a declared portfolio scope
- **THEN** Macaca SHALL return `PortfolioProviderCapability` with supported command names, unsupported command reasons, provider class, descriptor version, account/instrument coverage, analytics support, export support, freshness, consent, attribution, and entitlement metadata
- **AND** the response SHALL use provider-neutral fields rather than provider-specific account or analytics payloads

#### Scenario: Analytics are unsupported
- **WHEN** a provider supports positions but not scenario analytics
- **THEN** SDK discovery SHALL mark `portfolio.run_scenario` as non-callable for the effective capability
- **AND** invoking it SHALL return a typed `unsupported` result without provider side effects

### Requirement: Finance Portfolio commands SHALL use typed canonical service calls

Every Finance Portfolio operation SHALL be represented as a typed command and
result DTO, and every invocation SHALL traverse the canonical service runtime
path with trace, policy, consent, resource, entitlement, approval when required,
health, snapshot, and structured error behavior.

#### Scenario: Position command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `portfolio.list_positions` is invoked
- **THEN** Macaca SHALL route the command through SDK or facade helpers into the service runtime and portfolio service provider
- **AND** it SHALL emit sanitized admission, policy, provider-inspection, service-call, result, and replay events with stable trace identifiers

#### Scenario: Analytics command is denied before provider call
- **WHEN** policy, consent, permission, entitlement, resource, freshness, or provider-capability checks reject a command such as `portfolio.calculate_performance`
- **THEN** Macaca SHALL return a typed `denied`, `unavailable`, `unsupported`, `quota_exceeded`, or `stale_data` result before invoking the concrete provider
- **AND** audit evidence SHALL include bounded reason codes and replay pointers without raw account data or provider payloads

#### Scenario: Output is bounded
- **WHEN** a holdings, transaction, analytics, report, or export command could return unbounded data
- **THEN** Macaca SHALL require pagination, cursoring, async jobs, artifact handles, or resource limits
- **AND** traces and snapshots SHALL store only bounded counters, checksums, handles, methodology hashes, and sanitized metadata

### Requirement: Finance Portfolio SHALL expose normalized portfolio data

Finance Portfolio SHALL provide normalized DTOs for accounts, instruments,
positions, lots, cash balances, investment transactions, and valuations with
freshness, source evidence, attribution, redaction, and provider capability
metadata.

#### Scenario: Positions are listed
- **WHEN** an application invokes `portfolio.list_positions` with authorized account scope
- **THEN** Macaca SHALL return `PortfolioPosition` records with instrument references, quantity, market value, cost basis when available, unrealized gain/loss when available, price source, valuation timestamp, freshness, attribution, and redaction class
- **AND** missing provider fields SHALL be represented as explicit unavailable or unknown states rather than fabricated values

#### Scenario: Transactions are paginated
- **WHEN** an application invokes `portfolio.list_transactions` for a broad date range
- **THEN** Macaca SHALL apply pagination, cursor, resource budget, and freshness rules
- **AND** each `PortfolioTransaction` SHALL include normalized activity type, trade date, settlement date when available, amount, quantity, fees/taxes when available, currency, source evidence, and attribution

### Requirement: Finance Portfolio SHALL expose analytics with methodology disclosure

Finance Portfolio SHALL expose allocation, exposure, performance, risk summary,
and scenario analytics only with explicit methodology, benchmark, time range,
currency, FX source, price source, freshness, provider attribution, and
no-investment-advice metadata.

#### Scenario: Allocation is calculated
- **WHEN** an application invokes `portfolio.calculate_allocation` with authorized account scope and grouping strategy
- **THEN** Macaca SHALL return allocation buckets, unclassified residual bucket, valuation timestamp, grouping methodology, freshness, and attribution
- **AND** results SHALL NOT imply investment advice or suitability

#### Scenario: Performance is calculated
- **WHEN** an application invokes `portfolio.calculate_performance` with time range, benchmark, return methodology, and currency assumptions
- **THEN** Macaca SHALL return a `PerformanceResult` containing return series, cash-flow treatment, benchmark metadata, methodology disclosure, freshness, and attribution
- **AND** unsupported methodology SHALL produce a typed `unsupported` or degraded result instead of silently switching methods

#### Scenario: Risk summary is generated
- **WHEN** an application invokes `portfolio.summarize_risk` or `portfolio.run_scenario`
- **THEN** Macaca SHALL return risk/scenario metrics with model assumptions, confidence where available, provider capability, methodology, freshness, and no-advice metadata
- **AND** proprietary model internals SHALL NOT be written to traces, snapshots, or SDK diagnostics

### Requirement: Finance Portfolio SHALL keep rebalance operations non-executing

Finance Portfolio SHALL support rebalance-intent planning and optional
persistence, but it SHALL NOT place orders, initiate transfers, execute trades,
or automate portfolio changes.

#### Scenario: Rebalance intent is planned
- **WHEN** an application invokes `portfolio.plan_rebalance_intent` with target allocation, tolerance bands, constraints, and account scope
- **THEN** Macaca SHALL return a `RebalanceIntentPlan` with drift analysis, proposed non-executing intent rows, assumptions, required approvals, no-advice metadata, and unsupported constraints
- **AND** the planning command SHALL NOT mutate provider state or place orders

#### Scenario: Rebalance intent is persisted
- **WHEN** an application invokes `portfolio.rebalance_intent_request` with an approved plan and idempotency key
- **THEN** Macaca SHALL persist or export a `RebalanceIntent` artifact through the service runtime
- **AND** the result SHALL state that it is not an order, not a transfer, not advice, and not an execution instruction

### Requirement: Finance Portfolio SHALL provide reports and artifact handles

Finance Portfolio SHALL provide report generation and artifact-handle commands
for portfolio data and analytics while preserving bounded output and redaction.

#### Scenario: Portfolio report is generated
- **WHEN** an application invokes `portfolio.generate_report` with authorized scope, report sections, format, time range, methodology, and resource budget
- **THEN** Macaca SHALL return a `PortfolioReport` or `PortfolioArtifactHandle` with checksum, expiry, retention, redaction profile, freshness, attribution, and replay pointer
- **AND** traces and snapshots SHALL NOT contain the raw exported report

#### Scenario: Artifact metadata is retrieved
- **WHEN** an application invokes `portfolio.get_artifact_handle`
- **THEN** Macaca SHALL return artifact metadata, access policy, checksum, expiry, and redaction class
- **AND** it SHALL NOT expose raw report bytes unless a separate authorized artifact retrieval path exists

### Requirement: Finance Portfolio SHALL preserve Macaca boundaries

The Finance Portfolio implementation SHALL remain owned by the portfolio service
provider family. The microkernel, SDK, shells, and generic application framework
SHALL remain provider-neutral and SHALL NOT contain concrete provider
construction, provider-name routing, investment-advice logic, trading logic, or
application-specific portfolio workflows.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete portfolio provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable portfolio provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability support, freshness, methodology hash, and bounded result codes

### Requirement: Finance Portfolio SHALL provide detailed developer documentation

The Finance Portfolio proposal SHALL require a detailed developer guide for
`pack.finance.portfolio.v1` that makes the pack usable by application developers
and provider implementers without relying on source-code inspection.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/finance/portfolio.md`
- **THEN** the guide SHALL describe purpose, manifest declaration, permission scopes, consent, capability discovery, command DTOs, result DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction guarantees, methodology disclosure, and no-advice/no-trading boundaries
- **AND** examples SHALL use generic handles and synthetic data instead of credentials, provider routing keys, application-specific workflows, real account data, or investment recommendations

#### Scenario: SDK discovery links documentation
- **WHEN** SDK discovery returns metadata for `pack.finance.portfolio.v1`
- **THEN** the metadata SHALL include the portfolio developer-guide link and version compatibility information
- **AND** unavailable diagnostics SHALL point developers to the relevant declaration, consent, permission, entitlement, provider, policy, freshness, or methodology remediation section
