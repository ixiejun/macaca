# Finance Portfolio Pack

`pack.finance.portfolio.v1` describes provider-neutral portfolio aggregation
and analytics capabilities. The descriptor is discoverable through SDK catalogs,
but commands remain unavailable until a consent-aware portfolio provider is
installed through the runtime composition root.

## Manifest Declaration

Declare the pack as required only when portfolio data is mandatory for
readiness. Optional declarations degrade with structured unavailable
diagnostics.

```toml
[service_contract]
optional_packs = ["pack.finance.portfolio.v1"]
```

## Permissions

Use the narrowest scope: `finance.portfolio.read`,
`finance.portfolio.analytics`, `finance.portfolio.report`, and
`finance.portfolio.intent.write`.

## Capability Model

Macaca models portfolios as tenant, household, consent, and permission scopes,
provider capability reports, accounts, account groups, masked identifiers,
instrument references, positions, lots, cash balances, transactions,
valuations, allocation buckets, exposure buckets, performance series,
benchmarks, risk summaries, scenario analyses, rebalance-intent plans, reports,
freshness metadata, attribution metadata, redaction policies, and artifact
handles. Credentials, raw account numbers, complete holdings, raw transactions,
provider payloads, proprietary model dumps, and unbounded reports stay behind
provider adapters.

## Commands And Results

`portfolio.inspect_provider`, `portfolio.list_accounts`,
`portfolio.get_account`, `portfolio.list_positions`, `portfolio.list_lots`,
`portfolio.list_cash_balances`, `portfolio.list_transactions`,
`portfolio.get_valuation`, `portfolio.calculate_allocation`,
`portfolio.calculate_exposure`, `portfolio.calculate_performance`,
`portfolio.summarize_risk`, `portfolio.run_scenario`,
`portfolio.plan_rebalance_intent`, `portfolio.rebalance_intent_request`,
`portfolio.generate_report`, and `portfolio.get_artifact_handle` are
descriptor-owned schema names.

Every command uses a `FinanceCommandEnvelope`. Results use
`PortfolioResultEnvelope<T>` with success, partial, denied, unavailable,
unsupported, conflict, quota-exceeded, stale-data, and failure states. Analytics
outputs must carry methodology and no-advice metadata. Rebalance outputs are
intent records only and must never be interpreted as executable orders.

## Supplier Mapping

Plaid Investments, Yodlee Core APIs, SnapTrade, Alpaca, Interactive Brokers,
Addepar, Morningstar Direct Web Services, and Aladdin-style analytics map to
account, position, lot, balance, transaction, valuation, allocation,
performance, risk, scenario, report, artifact, freshness, and attribution DTOs.
Provider account ids, native endpoint names, order placement, transfers,
custody, suitability decisions, and investment advice are not OS semantics.

## App-Facing Examples

- Inspect provider classes and consent state before portfolio reads.
- List accounts, positions, lots, cash balances, transactions, and valuations
  through bounded cursors and opaque references.
- Calculate allocation, exposure, performance, risk, and scenarios only when the
  provider capability descriptor advertises the methodology.
- Generate reports through artifact handles, plan rebalance intent as a
  non-execution artifact, and handle unsupported analytics, stale-data, denied,
  unavailable, quota, and conflict outcomes explicitly.

## Trace And Audit

Traces should record declaration, admission decision, command name, consent ref,
household ref, account ref, instrument ref, request hash, methodology ref,
report ref, artifact id, provider class, capability hash, freshness class,
attribution ref, result status, and redaction profile. They must not record
credentials, raw account numbers, raw provider payloads, full holdings,
transactions, proprietary model dumps, manifests, package bytes, or unbounded
reports.

## Provider Authors

Conformance requires descriptor completeness, consent validation, entitlement
validation, account and instrument normalization, transaction pagination,
valuation freshness, analytics methodology evidence, no-advice labeling,
rebalance-intent non-execution guarantees, resource bounds, timeout and
cancellation handling, policy hooks, unavailable behavior, snapshot and replay
metadata, and redaction tests. Providers must return structured unavailable,
denied, unsupported, conflict, quota, stale-data, timeout, cancellation, and
failure results without executing trades or fabricating advice.
