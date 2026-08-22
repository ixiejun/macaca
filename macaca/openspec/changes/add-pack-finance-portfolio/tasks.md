## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, umbrella catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API findings for Plaid Investments, Yodlee Core APIs, SnapTrade, Alpaca, Interactive Brokers, Addepar, Morningstar Direct Web Services, and Aladdin-style analytics.
- [x] 1.3 Confirm the pack scope: accounts, positions, lots, cash balances, transactions, valuation, allocation, exposure, performance, risk summary, scenario analysis, rebalance intent, reports, artifacts, freshness, attribution, and redaction.
- [x] 1.4 Explicitly exclude trading, order placement, transfers, ACATS, custody, settlement, tax filing, suitability decisions, investment advice, and automatic rebalancing execution.
- [x] 1.5 Inventory existing descriptors, SDK clients, service runtime hooks, policy gates, entitlement gates, consent gates, resource gates, trace/audit helpers, artifact handles, mock providers, and unavailable providers that can be reused.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define descriptor metadata for `pack.finance.portfolio.v1`, including family, lifecycle, stability, command schemas, scopes, policy template, resource budgets, redaction profile, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define `PortfolioScope`, `PortfolioProviderCapability`, `PortfolioFreshness`, `PortfolioAttribution`, and `PortfolioRedactionPolicy`.
- [x] 2.3 Define `PortfolioAccount`, consent state, account grouping/household handles, account type, base currency, masked identifiers, and ownership metadata.
- [x] 2.4 Define `PortfolioInstrumentReference` with symbols, identifiers, asset class, security type, currency, exchange, maturity, expiry, strike, and classification metadata.
- [x] 2.5 Define `PortfolioPosition`, `PortfolioLot`, `CashBalance`, `PortfolioValuation`, price source, FX source, valuation timestamp, and cost-basis metadata.
- [x] 2.6 Define `PortfolioTransaction` with activity type, trade date, settle date, amount, quantity, price, fees, taxes, currency, instrument/account references, and source evidence.
- [x] 2.7 Define allocation, exposure, performance, benchmark, return-series, risk-summary, scenario, and methodology DTOs.
- [x] 2.8 Define `RebalanceIntentPlan`, `RebalanceIntent`, `RebalanceConstraint`, drift/tolerance metadata, approval state, and non-execution disclaimer.
- [x] 2.9 Define `PortfolioReportRequest`, `PortfolioReport`, `PortfolioArtifactHandle`, export format, checksum, expiry, retention, redaction, and access policy.
- [x] 2.10 Define typed `success`, `partial`, `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, `stale_data`, and `failure` result envelopes for every command family.
- [x] 2.11 Add descriptor hashing and compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema evolution.

## 3. Command Surface And Analytics Semantics

- [x] 3.1 Implement command schemas for `portfolio.inspect_provider`, `portfolio.list_accounts`, and `portfolio.get_account`.
- [x] 3.2 Implement command schemas for `portfolio.list_positions`, `portfolio.list_lots`, `portfolio.list_cash_balances`, `portfolio.list_transactions`, and `portfolio.get_valuation`.
- [x] 3.3 Implement command schemas for `portfolio.calculate_allocation`, `portfolio.calculate_exposure`, and grouping/look-through options.
- [x] 3.4 Implement command schemas for `portfolio.calculate_performance`, benchmark references, return methodology, cash-flow treatment, and currency/FX assumptions.
- [x] 3.5 Implement command schemas for `portfolio.summarize_risk` and `portfolio.run_scenario` with model availability, assumption, confidence, and no-advice metadata.
- [x] 3.6 Implement command schemas for `portfolio.plan_rebalance_intent` and `portfolio.rebalance_intent_request` without order execution.
- [x] 3.7 Implement command schemas for `portfolio.generate_report` and `portfolio.get_artifact_handle`.
- [x] 3.8 Add pagination, cursor, async-job, timeout, cancellation, bounded-output, freshness, methodology, and attribution rules.

## 4. Permission, Consent, Policy, Resource, Entitlement, And Approval

- [x] 4.1 Add declaration validation for `finance.portfolio.read`, `finance.portfolio.analytics`, `finance.portfolio.report`, and `finance.portfolio.intent.write`.
- [x] 4.2 Require consent and entitlement checks for account data, transaction data, lots, analytics, exports, and retained reports.
- [x] 4.3 Require policy decisions before every command and approval before retained reports or rebalance-intent persistence.
- [x] 4.4 Enforce no-advice metadata for allocation, performance, risk, scenario, and rebalance-intent outputs.
- [x] 4.5 Reserve and meter resources for account fan-out, pagination, analytics jobs, report/export size, storage, provider quotas, and snapshots.
- [x] 4.6 Add tests proving denied, unavailable, unsupported, quota, and stale-data paths do not call concrete providers when preconditions fail.

## 5. Service Provider, Provider Strategy, And Unavailable Behavior

- [x] 5.1 Add the portfolio service provider interface with descriptor, lifecycle, health, snapshot, shutdown, timeout, cancellation, and command dispatch.
- [x] 5.2 Implement provider Strategy adapters behind the service interface without provider-name routing in OS-layer command logic.
- [x] 5.3 Implement a mock provider with synthetic accounts, holdings, transactions, analytics, stale-data states, and configurable capability gaps.
- [x] 5.4 Implement an unavailable provider that returns explicit unavailable diagnostics for every command without fake success.
- [x] 5.5 Normalize provider errors into Macaca result envelopes while preserving sanitized provider class, bounded code, retriable flag, freshness, and replay pointer.
- [x] 5.6 Add provider capability discovery for account types, instrument classes, lots, transaction history depth, performance methods, risk/scenario support, export formats, freshness, and attribution.

## 6. SDK, Admission, Examples, And Developer Documentation

- [x] 6.1 Extend pack catalog and SDK discovery for `pack.finance.portfolio.v1` with schemas, scopes, examples, availability, health, diagnostics, compatibility, provider class, and docs metadata.
- [x] 6.2 Extend application admission so required declarations block on unavailable/denied states and optional declarations degrade explicitly with effective capability mementos.
- [x] 6.3 Add SDK command helper builders that only construct canonical traced service calls and never construct providers.
- [x] 6.4 Add generic app-facing examples for listing positions, calculating allocation, calculating performance, summarizing risk, planning a rebalance intent, and handling unsupported analytics.
- [x] 6.5 Create `docs/developer-packs/finance/portfolio.md` with purpose, manifest declaration, scopes, commands, DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction, methodology, no-advice boundaries, and provider capability differences.
- [x] 6.6 Cross-link the developer guide from SDK discovery metadata and the industrial pack catalog index.

## 7. Trace, Audit, Replay, And Redaction

- [x] 7.1 Emit sanitized declaration, admission, provider-inspection, policy, consent, entitlement, approval, resource, service-call, analytics-job, rebalance-intent, unavailable, health, snapshot, and result events.
- [x] 7.2 Add trace schemas for `portfolio_pack_declared`, `portfolio_pack_admission_validated`, `portfolio_pack_policy_decision`, `portfolio_pack_provider_inspected`, `portfolio_pack_service_call_requested`, `portfolio_pack_service_call_succeeded`, `portfolio_pack_service_call_failed`, `portfolio_pack_analytics_job_started`, `portfolio_pack_rebalance_intent_planned`, `portfolio_pack_unavailable`, and `portfolio_pack_snapshot_recorded`.
- [x] 7.3 Add replay tests proving every command is trace-addressable through the canonical service runtime path.
- [x] 7.4 Add snapshot tests proving descriptor, provider health, command availability, consent, freshness, analytics/export support, redaction profile, resource counters, and replay pointers are retained without raw payload leakage.
- [x] 7.5 Add redaction tests proving credentials, raw account numbers, raw provider payloads, full holdings/transactions, proprietary model dumps, and unbounded report content never enter logs, traces, snapshots, or SDK diagnostics.

## 8. Boundary, Quality, And Validation Gates

- [x] 8.1 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete portfolio providers.
- [x] 8.2 Add no-direct-provider-call tests proving all callable operations traverse descriptor-owned service registration and typed service commands.
- [x] 8.3 Add canonical execution-path tests covering read-only, analytics, report, intent, denied, unavailable, unsupported, conflict, quota, and stale-data paths.
- [x] 8.4 Add provider replacement tests for built-in, plugin, remote, mock, and unavailable providers.
- [x] 8.5 Add file-size and module-ownership checks for any new implementation files.
- [x] 8.6 Run `openspec validate add-pack-finance-portfolio --strict`.
- [x] 8.7 Run targeted cargo checks/tests, dependency-boundary gates, audit replay checks, and redaction checks before marking the implementation tasks complete.
