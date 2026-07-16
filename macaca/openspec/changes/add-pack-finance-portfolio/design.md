# Finance Portfolio Pack Design

## Context

`pack.finance.portfolio.v1` is the Macaca capability for portfolio aggregation,
valuation, analytics, and report generation. It is a finance pack, not a trading
or advisory engine. The pack must normalize provider data across account
aggregators, broker APIs, wealth-management platforms, and analytics vendors
while preserving consent, entitlement, freshness, methodology, attribution,
redaction, and audit evidence.

The serviceized design keeps provider-specific schema and analytics engines
behind provider Strategy adapters. Applications see stable command/result DTOs
and explicit unsupported/degraded states.

## Supplier Capability Matrix

| Supplier family | Relevant capabilities | Constraints Macaca must model |
| --- | --- | --- |
| Plaid Investments | User-authorized investment accounts, balances, holdings, securities, investment transactions | Consent scope, limited geography, aggregation freshness, security metadata variance, no trade execution semantics for this pack |
| Yodlee Core APIs | Holdings, securities, investment transactions, data extracts, derived holding summaries, instrument metadata | Field-level variability, option/bond/equity-award attributes, provider account lifecycle, data extract size limits |
| SnapTrade | Connected brokerage accounts, balances, positions, historical data, orders, trading-capable APIs | Trading is out of scope; positions/orders may be read as source evidence only; real-time entitlement varies |
| Alpaca | Positions, account data, portfolio-history time series, trading APIs | Trading/order APIs are excluded; portfolio history assumptions and market-data entitlements must be disclosed |
| Interactive Brokers | Account/portfolio data, intra-day portfolio updates, market data, OAuth/session constraints | Brokerage session, market-data subscriptions, account entitlements, asynchronous updates, regional restrictions |
| Addepar | Portfolio views, dynamic portfolio queries, JSON/CSV/TSV/XLSX exports, performance attributes | Saved-view dependencies, export formats, large-query performance, reporting metadata |
| Morningstar Direct Web Services | Portfolio X-Ray, risk profiler/score, investment data and analytics APIs | Methodology and licensing attribution, analytics availability, model disclosure, no advice semantics |
| BlackRock Aladdin-style analytics | Institutional risk, exposures, stress testing, scenario analytics | Entitlement-heavy analytics, proprietary model disclosure limits, stress scenario metadata |

## Goals

- Provide portfolio-account discovery, holdings, lots, cash balances,
  transactions, valuation snapshots, allocation, exposure, performance, risk
  summary, scenario analysis, rebalance-intent planning, reports, and artifact
  handles.
- Carry explicit assumptions for performance methodology, benchmark, calendar,
  currency, FX source, price source, time weighting, money weighting, and
  valuation timestamp.
- Preserve consent, policy, entitlement, resource budgets, freshness,
  attribution, and redaction across every command.
- Make provider capability differences discoverable before command invocation.
- Require SDK and developer docs that are sufficient for application developers
  and provider implementers.

## Non-Goals

- No trading, order placement, transfers, ACATS, custody, settlement, tax filing,
  suitability determination, personalized investment recommendation, or
  automatic rebalance execution.
- No provider-specific taxonomy, risk model, benchmark, or advice rules in
  generic OS layers.
- No raw credentials, brokerage account numbers, provider payloads, proprietary
  analytics dumps, or unbounded transaction/position rows in observability.

## Ownership And Boundaries

- Pack id: `pack.finance.portfolio.v1`.
- Family: `finance`.
- Backing service owner: portfolio service provider family.
- SDK surface: `sdk.packs.finance.portfolio`.
- Command namespace: `portfolio.*`.
- Kernel ownership: identity, service-call evidence, policy facade, trace/audit
  primitives, and resource primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective-capability mementos.
- Runtime-host ownership: provider registration, decorators, mock/unavailable
  providers, and adapter composition through approved composition roots.
- Service ownership: capability discovery, command dispatch, data normalization,
  provider strategy selection, analytics state machines, redaction, and sanitized
  audit.

## Command Surface

| Command | Purpose | Side-effect class |
| --- | --- | --- |
| `portfolio.inspect_provider` | Return provider capability, geography, consent, freshness, analytics, export, and entitlement support | Read-only |
| `portfolio.list_accounts` | List portfolio accounts visible to the caller | Read-only |
| `portfolio.get_account` | Return one normalized portfolio account and consent/freshness metadata | Read-only |
| `portfolio.list_positions` | Return current positions with valuation, quantity, cost basis, and classification | Read-only |
| `portfolio.list_lots` | Return tax/holding lots where provider supports them | Read-only |
| `portfolio.list_cash_balances` | Return cash balances by currency/account | Read-only |
| `portfolio.list_transactions` | Return investment transactions with bounded pagination | Read-only |
| `portfolio.get_valuation` | Return valuation snapshot or time series for accounts/positions | Read-only |
| `portfolio.calculate_allocation` | Calculate allocation by asset class, sector, region, currency, account, or custom bucket | Read-only/async |
| `portfolio.calculate_exposure` | Calculate exposure and look-through metadata when supported | Read-only/async |
| `portfolio.calculate_performance` | Calculate time-weighted, money-weighted, simple, or provider-supported performance | Read-only/async |
| `portfolio.summarize_risk` | Return provider-neutral risk metrics, factor exposure, volatility, drawdown, concentration, or provider risk score | Read-only/async |
| `portfolio.run_scenario` | Run stress/scenario analytics when provider supports it | Read-only/async |
| `portfolio.plan_rebalance_intent` | Produce a non-executing rebalance intent plan with constraints and drift analysis | Planning |
| `portfolio.rebalance_intent_request` | Persist an approved rebalance intent artifact without placing orders | Mutating metadata |
| `portfolio.generate_report` | Generate a portfolio report with methodology and attribution | Read-only/async |
| `portfolio.get_artifact_handle` | Return report/export artifact metadata without raw payload leakage | Read-only |

Every command must define typed command DTOs, typed success DTOs, typed partial
or async result shapes, typed denied/unavailable/unsupported/conflict/quota/
stale-data/failure DTOs, redaction rules, replay metadata, and idempotency where
metadata side effects exist.

## Provider-Neutral DTO Model

- `PortfolioScope`: application, tenant, session, task, portfolio account,
  household/group, currency, time range, consent, and permission scope.
- `PortfolioProviderCapability`: supported account types, instrument classes,
  command matrix, real-time support, lot support, transaction history depth,
  performance/risk/scenario support, export formats, freshness model,
  attribution, geography, and entitlement requirements.
- `PortfolioAccount`: account handle, display label, account type, institution
  class, base currency, consent state, freshness, masked identifiers, and
  ownership metadata.
- `PortfolioInstrumentReference`: provider-neutral instrument id, symbols,
  ISIN/CUSIP/SEDOL/FIGI where available, asset class, security type, currency,
  exchange, maturity/expiry/strike metadata, and classification.
- `PortfolioPosition`, `PortfolioLot`, `CashBalance`: quantity, market value,
  cost basis, unrealized gain/loss, accrued income, price source, valuation time,
  tax-lot metadata, and redaction class.
- `PortfolioTransaction`: normalized activity type, trade date, settlement date,
  amount, quantity, price, fees, taxes, currency, instrument reference, account
  reference, and source evidence.
- `PortfolioValuation`: point-in-time or time-series account/position valuation
  with price source, FX source, freshness, and attribution.
- `AllocationRequest`, `AllocationResult`, `ExposureResult`: grouping strategy,
  look-through flag, bucket rows, residual/unclassified bucket, and methodology.
- `PerformanceRequest`, `PerformanceResult`, `ReturnSeries`,
  `BenchmarkReference`: time-weighted, money-weighted, simple, provider-reported
  return, benchmark, calendar, cash-flow treatment, and methodology disclosure.
- `RiskSummaryRequest`, `RiskSummary`, `ScenarioRequest`, `ScenarioResult`:
  volatility, drawdown, beta, concentration, factor exposure, VaR-style metrics
  when supported, risk score, stress scenario, confidence, assumptions, and
  model attribution.
- `RebalanceIntentPlan`, `RebalanceIntent`, `RebalanceConstraint`: target
  allocations, drift, tolerance bands, excluded instruments, cash constraints,
  tax-awareness flag, approval state, and non-execution disclaimer.
- `PortfolioReportRequest`, `PortfolioReport`, `PortfolioArtifactHandle`:
  report sections, format, checksum, expiry, retention, redaction, and access
  policy.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `finance.portfolio.read`
- `finance.portfolio.analytics`
- `finance.portfolio.report`
- `finance.portfolio.intent.write`

Policy defaults:

- Scope every call to application id, tenant id, session id, task id, trace id,
  account/household handle, permission scope, and consent state.
- Require explicit consent/entitlement for account data, transaction data, lots,
  analytics, exports, and retained reports.
- Require approval before persisting rebalance intents or generating retained
  artifacts.
- Require no-advice metadata for allocation, performance, risk, scenario, and
  rebalance-intent outputs.
- Enforce resource budgets for account fan-out, pagination, analytics jobs,
  report/export size, provider quotas, storage, and retained snapshots.
- Return typed `denied`, `unavailable`, `unsupported`, `quota_exceeded`,
  `stale_data`, `conflict`, or `failure` before provider calls when
  preconditions fail.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `portfolio_pack_declared`
- `portfolio_pack_admission_validated`
- `portfolio_pack_policy_decision`
- `portfolio_pack_provider_inspected`
- `portfolio_pack_service_call_requested`
- `portfolio_pack_service_call_succeeded`
- `portfolio_pack_service_call_failed`
- `portfolio_pack_analytics_job_started`
- `portfolio_pack_rebalance_intent_planned`
- `portfolio_pack_unavailable`
- `portfolio_pack_snapshot_recorded`

Events include pack id, command name, trace id, application/session/task/tenant
identifiers, account/household handles, policy decision, provider class,
descriptor hash, freshness, methodology hash, latency, bounded resource counters,
result code, and sanitized artifact references. Events must exclude credentials,
raw account numbers, raw provider payloads, proprietary model dumps, full
holdings/transaction rows, and unbounded report content.

Snapshots include descriptor version, provider health, command availability,
analytics/export support, consent status, freshness, redaction profile, resource
counters, and replay pointers.

## SDK And Developer Documentation

SDK discovery must return pack metadata, lifecycle, service mapping, command
schemas, permission scopes, policy templates, examples, availability, health,
provider class, compatibility, diagnostics, and documentation links.

The required developer guide at
`docs/developer-packs/finance/portfolio.md` must cover:

- Manifest declaration and permission scopes.
- Capability discovery and unavailable/degraded diagnostics.
- DTO reference for accounts, positions, lots, transactions, analytics, reports,
  and rebalance intents.
- Examples for listing positions, calculating allocation, calculating
  performance, summarizing risk, planning a rebalance intent, and handling
  stale-data or unsupported analytics.
- Provider replacement, mock/unavailable provider behavior, trace/audit event
  interpretation, and redaction guarantees.
- Safety notes that this pack does not provide investment advice, suitability,
  order routing, trading, or transfer execution.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while hiding provider
  construction.
- **Command**: all operations are typed command/result DTOs.
- **Strategy**: account aggregators, broker APIs, wealth platforms, and analytics
  engines implement replaceable provider adapters.
- **Decorator**: trace, policy, consent, entitlement, resource, approval,
  metering, and redaction wrap every call.
- **State**: consent, freshness, async analytics, report generation, and
  rebalance-intent persistence use explicit states.
- **Specification**: admission validates declaration, scopes, consent,
  entitlement, provider support, methodology requirements, and resource limits.
- **Observer**: trace, audit, health, analytics-job, and snapshot events are
  subscribable.
- **Memento**: effective capability reports, analytics assumptions, report
  handles, and rebalance-intent artifacts are replayable bounded records.
- **Abstract Factory**: providers register only through approved runtime-host or
  plugin composition roots.

## Risks And Mitigations

- Risk: portfolio analytics are mistaken for investment advice. Mitigation:
  every analytics/rebalance DTO carries no-advice metadata, methodology, and
  attribution; order execution is out of scope.
- Risk: provider data freshness varies by aggregator and account. Mitigation:
  freshness is required on every account, position, transaction, valuation, and
  analytics result.
- Risk: analytics output leaks proprietary models or raw holdings. Mitigation:
  trace and snapshot schemas store bounded methodology hashes, counters, handles,
  and sanitized summaries only.
- Risk: rebalance intent becomes trading. Mitigation: commands only create
  non-executing plans/artifacts; any future execution belongs to a separate
  regulated trading/order capability.
