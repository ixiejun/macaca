# Finance Portfolio Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.finance.portfolio.v1`. The portfolio pack must expose accounts,
positions, lots, cash balances, transactions, valuation, allocation, exposure,
performance, risk summary, scenario analysis, rebalance intent, reports,
artifacts, freshness, attribution, and redaction through typed service
commands. It must not execute trades, place orders, transfer assets, perform
ACATS, provide custody, settle securities, file taxes, make suitability
decisions, provide investment advice, or execute automatic rebalancing.

## Source Baseline

- Plaid Investments:
  <https://plaid.com/docs/investments/> and
  <https://plaid.com/docs/api/products/investments/>
- Yodlee data aggregation:
  <https://www.yodlee.com/>
- SnapTrade account data:
  <https://docs.snaptrade.com/docs/account-data>,
  <https://docs.snaptrade.com/reference/Account%20Information/AccountInformation_getAccountActivities>
- Alpaca account portfolio history and positions:
  <https://docs.alpaca.markets/us/reference/getaccountportfoliohistory-1>
- Addepar developer platform:
  <https://developers.addepar.com/>
- Morningstar Direct Web Services:
  <https://developer.morningstar.com/>

## Supplier API Notes

- Plaid Investments contributes user-authorized investment accounts, holdings,
  securities, transactions, balances, and transfer-adjacent products. Macaca
  should model consent, holdings, securities, and transactions while excluding
  ACATS/transfer execution.
- Yodlee-style aggregators contribute consumer-permissioned account, balance,
  transaction, and enrichment data. Macaca should model source freshness,
  consent, and attribution rather than aggregator-specific payloads.
- SnapTrade contributes brokerage account connections, balances, positions,
  orders, activities, and trading APIs. Macaca should keep account data,
  positions, and activities in scope while excluding trading/order placement.
- Alpaca contributes positions, account activities, and portfolio history but
  also trading APIs. Macaca should normalize portfolio history and position
  snapshots without exposing order execution.
- Addepar, Morningstar, and Aladdin-style platforms contribute institutional
  portfolio analytics, holdings, performance, risk, exposure, and reporting.
  Macaca should model methodology and attribution but must not provide advice.

## Macaca-Owned Abstractions

`pack.finance.portfolio.v1` should define `PortfolioScope`,
`PortfolioProviderCapability`, `PortfolioAccount`,
`PortfolioInstrumentReference`, `PortfolioPosition`, `PortfolioLot`,
`CashBalance`, `PortfolioValuation`, `PortfolioTransaction`, allocation,
exposure, performance, benchmark, return-series, risk-summary, scenario, and
methodology DTOs, `RebalanceIntentPlan`, `RebalanceIntent`,
`RebalanceConstraint`, `PortfolioReportRequest`, `PortfolioReport`,
`PortfolioArtifactHandle`, `PortfolioFreshness`, `PortfolioAttribution`, and
`PortfolioRedactionPolicy`.

The DTOs must carry account consent, masked identifiers, account grouping,
instrument identity, lots, cost basis, FX and price sources, trade/settle dates,
fees/taxes as data fields, methodology metadata, scenario assumptions,
non-execution rebalance intent, report retention, capability hashes, redaction
classes, bounded provider reason codes, and replay pointers. Raw credentials,
trading payloads, suitability decisions, private account identifiers, and
unbounded reports are rejected.

## Explicit Non-Goals

- Do not implement concrete Plaid, Yodlee, SnapTrade, Alpaca, Interactive
  Brokers, Addepar, Morningstar, Aladdin, broker, transfer, custody, tax, or
  trading adapters in this research phase.
- Do not define trading, order placement, transfers, ACATS, custody,
  settlement, tax filing, suitability decisions, investment advice, or
  automatic rebalancing execution inside this pack.
- Do not expose provider-native portfolio payloads, private account
  credentials, broker order routes, trading workflows, advice text, or
  app-specific allocation policy as stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  portfolio SDK helpers should only build canonical traced service calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics.
- Policy, consent gate, resource, entitlement, trace, audit, artifact,
  mock-provider, and unavailable-provider concepts exist generically, but
  current evidence does not prove portfolio-specific DTOs, descriptors,
  providers, SDK helpers, WASM ABI metadata, tests, or docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
