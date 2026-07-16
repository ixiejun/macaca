# Change: Add Finance Accounting Pack

## Why

Macaca applications need a real `pack.finance.accounting.v1` capability for
general-ledger and accounting-system integrations, not a thin label over a
provider API. Accounting is high-risk operational infrastructure: incorrect
journal posting, period handling, tax-code references, dimensional reporting, or
audit export can corrupt books or produce misleading financial records.

This proposal makes accounting a serviceized, provider-neutral, auditable pack.
Applications declare the pack; admission verifies permissions, policy,
entitlement, and provider capability; SDK helpers build typed canonical service
commands; replaceable providers implement the command contract. The kernel,
SDK, shells, and generic application framework remain free of accounting
workflow logic and provider-specific routing.

## Supplier And API Baseline

The design is based on the public API surfaces and constraints of mature
accounting platforms:

- QuickBooks Online Accounting API exposes account entities, journal entries,
  reports, sync tokens, closed-period concerns, and change tracking/error
  behavior.
- Xero Accounting API exposes accounts, journals, manual-journal/report scopes,
  attachments, and report retrieval, with explicit scope and approval constraints
  around sensitive accounting data.
- Oracle NetSuite SuiteTalk REST records expose journal-entry records,
  intercompany and period-end variants, subsidiaries, classifications, and
  multi-book constraints, while some tax and read-only sublists remain
  unavailable through REST.
- Sage Business Cloud Accounting and Sage Active expose ledger accounts,
  accounting entries, journal resources, dimensions, fiscal periods, and
  balanced-entry expectations.
- Odoo accounting APIs demonstrate ERP-style account move, move line, partner,
  journal, period, tax, and attachment mapping through generic model APIs.
- Stripe Financial Connections and reporting APIs are adjacent inputs for bank
  and financial data ingestion, but they are not a general-ledger system and
  must not define Macaca accounting semantics.

The common denominator is not a single provider schema. It is a double-entry,
period-aware, permission-gated accounting capability with explicit provider
capability discovery, journal validation, immutable posted evidence, report
generation, reconciliation workflows, and auditable exports.

## Macaca Provider-Neutral Mapping

`pack.finance.accounting.v1` maps supplier concepts to stable Macaca contracts:

- Accounting organizations, subsidiaries, books, periods, currencies, and
  lock-state become `AccountingEntity`, `LedgerBook`, and `AccountingPeriod`.
- Provider chart-of-accounts records become `AccountHandle`, `AccountClass`,
  `ChartOfAccounts`, and `AccountMutationPlan`.
- Provider journal/manual-journal/account-move records become
  `JournalEntryPlan`, `JournalEntry`, `JournalLine`, and `LedgerEntry`.
- Tracking categories, classes, departments, cost centers, projects, and
  subsidiaries become provider-neutral `AccountingDimension`.
- Bank feed, statement, and imported transaction rows become `StatementLine`
  and `ReconciliationCandidate`.
- Trial balance, balance sheet, profit/loss, and cash-flow reports become typed
  `AccountingReport` variants with period, book, basis, currency, and dimension
  metadata.
- Attachments and exported audit files become `AccountingArtifactHandle`
  records, never raw unbounded payloads in traces or snapshots.

The pack uses a plan-before-side-effect pattern for mutating operations:
`accounting.plan_account`, `accounting.account_request`,
`accounting.plan_journal`, `accounting.post_journal`,
`accounting.plan_reconciliation`, `accounting.reconciliation_request`,
`accounting.plan_audit_export`, and `accounting.audit_export_request`.
Planning commands normalize provider constraints and return validation evidence;
request commands require policy, entitlement, idempotency, and approval before
calling a provider.

## What Changes

- Add the provider-neutral `pack.finance.accounting.v1` contract under the
  finance family.
- Define an industrial command namespace for provider inspection, entity/book
  discovery, chart of accounts, account mutation planning, journal planning and
  posting, ledger-entry queries, statement import, reconciliation planning,
  report generation, audit export, period inspection, and artifact retrieval.
- Define DTO requirements for accounting scope, provider capability, entities,
  books, periods, accounts, dimensions, journals, lines, ledger entries,
  statement lines, reconciliation plans/results, reports, audit exports,
  freshness, attribution, and redaction metadata.
- Require double-entry validation, period-lock checks, currency precision,
  immutable posted-journal evidence or reversal workflows, tax-code references,
  idempotency, approval, and audit trail on side effects.
- Add SDK discovery metadata, examples, unavailable diagnostics, and detailed
  developer documentation under `docs/developer-packs/finance/accounting.md`.
- Add acceptance gates for canonical execution path, no direct provider calls,
  trace/audit replay, snapshot redaction, provider replacement, and boundary
  purity.

## Impact

- Affected specs: `pack-finance-accounting`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptor and
  admission validators, SDK discovery and command builders, accounting service
  provider interfaces, unavailable/mock provider implementations, trace/audit
  schemas, replay tests, and dependency-boundary gates.
- Developer documentation: a full pack guide must document manifest declaration,
  scopes, commands, DTOs, examples, provider replacement, unavailable states,
  trace/audit behavior, and accounting safety constraints.

## Non-Goals

- No payroll, invoicing, payment execution, bank transfer execution, tax filing,
  inventory costing, ERP workflow automation, portfolio management, or investment
  advice.
- No provider-specific account names, chart templates, tax rules, regional
  bookkeeping policy, or application-specific posting workflows in Macaca OS
  layers.
- No raw provider payloads, credentials, financial account numbers, tax
  identifiers, attachments, or unbounded ledger data in logs, traces, snapshots,
  or SDK diagnostics.
- No SDK, shell, kernel, or generic application-framework provider construction.
- No silent fallback, fake success, or provider-name routing in OS-layer code.
