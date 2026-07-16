# Change: Add Finance Portfolio Pack

## Why

Macaca applications need `pack.finance.portfolio.v1` as an industrial
portfolio-data and portfolio-analytics capability. A useful portfolio pack must
do more than list positions: it must normalize brokerage/wealth data, expose
holdings and transactions with freshness and attribution, calculate allocation
and performance on explicit assumptions, summarize risk without pretending to be
regulated advice, and produce auditable reports.

This proposal defines the pack as a provider-neutral, serviceized capability.
Applications declare the pack; admission validates permissions and entitlement;
SDK helpers build typed canonical service commands; providers implement the
portfolio service contract. Macaca OS layers remain free of provider routing,
investment advice, order execution, and application-specific portfolio logic.

## Supplier And API Baseline

The design is based on current API patterns from portfolio data, brokerage, and
wealth analytics providers:

- Plaid Investments exposes user-authorized investment accounts, holdings,
  securities, balances, and investment transactions for aggregation and personal
  financial-management use cases.
- Yodlee Core APIs expose investment holdings, securities, investment
  transactions, data extracts, security identifiers, option/bond metadata, and
  derived holding summaries.
- SnapTrade exposes connected brokerage accounts, balances, positions,
  historical account data, orders, and trading-capable APIs; Macaca uses the data
  and analysis concepts but does not make trading a portfolio-pack side effect.
- Alpaca exposes account positions and portfolio-history endpoints such as
  account equity and profit/loss time series; trading/order APIs are outside
  this pack.
- Interactive Brokers Client Portal/Web API exposes account, market data,
  portfolio, and intra-day portfolio update concepts with OAuth/session and
  entitlement requirements.
- Addepar Portfolio APIs expose saved/dynamic portfolio queries, analysis views,
  exports, performance attributes, and reporting for wealth-management data.
- Morningstar Direct Web Services exposes portfolio analytics such as Portfolio
  X-Ray and risk-score/risk-profiler style analytics; Macaca treats these as
  optional analytics strategies, not advice semantics.
- BlackRock Aladdin APIs represent institutional risk/analytics and stress-test
  style capability classes; Macaca maps only provider-neutral analytics
  categories and availability flags.

The shared model is account-authorized portfolio data, security normalization,
position/transaction histories, valuation, allocation, performance, risk
analytics, rebalancing analysis, report/export, and strong consent, entitlement,
freshness, and disclosure controls.

## Macaca Provider-Neutral Mapping

`pack.finance.portfolio.v1` maps supplier concepts into stable Macaca contracts:

- Brokerage, wealth, retirement, and simulated accounts become
  `PortfolioAccount`.
- Securities, funds, cash, options, bonds, crypto, and custom assets become
  `PortfolioInstrumentReference` records with identifiers and classification
  metadata.
- Holdings, balances, lots, cost basis, accrued income, and valuation snapshots
  become `PortfolioPosition`, `PortfolioLot`, `CashBalance`, and
  `PortfolioValuation`.
- Investment activity becomes `PortfolioTransaction` with normalized transaction
  type, trade/settle dates, fees, taxes, currency, and source evidence.
- Allocation, exposure, performance, risk, and scenario calculations become
  explicit request/result DTOs carrying assumptions, benchmark, time range,
  methodology, freshness, and provider attribution.
- Rebalance operations produce `RebalanceIntentPlan` and `RebalanceIntent`
  objects only. They do not place orders, transfer assets, or automate trading.
- Reports and exports return `PortfolioReport` or `PortfolioArtifactHandle`
  records, not raw unbounded provider payloads.

## What Changes

- Add provider-neutral `pack.finance.portfolio.v1` under the finance family.
- Define command surfaces for provider inspection, account discovery, positions,
  lots, balances, transactions, valuation, allocation, exposure, performance,
  risk summary, scenario analysis, rebalance intent planning, report generation,
  and artifact retrieval.
- Define DTOs for portfolio scope, provider capability, accounts, instruments,
  positions, lots, balances, transactions, benchmarks, return series,
  allocation/exposure buckets, risk metrics, rebalance constraints, report
  requests, freshness, attribution, and redaction.
- Require consent/permission, entitlement, freshness, methodology disclosure,
  bounded output, idempotency for persisted intents, explicit advice disclaimers,
  and audit evidence.
- Require a detailed developer guide at
  `docs/developer-packs/finance/portfolio.md`.

## Impact

- Affected specs: `pack-finance-portfolio`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, portfolio service providers, mock/unavailable
  providers, trace/audit schemas, replay tests, redaction tests, and
  dependency-boundary gates.

## Non-Goals

- No order placement, broker trading, transfer initiation, ACATS movement,
  custody, settlement, tax filing, investment advice, suitability decisions, or
  automatic rebalancing execution.
- No provider-specific investment taxonomy, proprietary analytics model,
  benchmark, risk score, or advice workflow in Macaca OS layers.
- No raw account numbers, brokerage credentials, full provider payloads,
  unbounded holdings/transactions, or proprietary model output in logs, traces,
  snapshots, or SDK diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
