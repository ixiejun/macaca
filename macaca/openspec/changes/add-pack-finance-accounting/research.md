# Finance Accounting Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.finance.accounting.v1`. The accounting pack must expose general-ledger
accounting, chart of accounts, periods, journals, ledger entries, statement
import, reconciliation, reports, audit export, artifacts, freshness,
attribution, and redaction through typed service commands. It must not own
payroll, invoices, payments, bank transfers, tax filing, ERP workflows,
portfolio accounting, investment advice, provider chart templates, or
application-specific posting workflows.

## Source Baseline

- QuickBooks Online Accounting API and reports:
  <https://developer.intuit.com/app/developer/qbo/docs/learn/explore-the-quickbooks-online-api>
  and <https://developer.intuit.com/app/developer/qbo/docs/workflows/run-reports>
- Xero Accounting API accounts, journals, and reports:
  <https://developer.xero.com/documentation/api/accounting/overview>,
  <https://developer.xero.com/documentation/api/accounting/accounts>,
  <https://developer.xero.com/documentation/api/accounting/journals>, and
  <https://developer.xero.com/documentation/api/accounting/reports>
- NetSuite SuiteTalk REST records and journal entries:
  <https://docs.oracle.com/en/cloud/saas/netsuite/ns-online-help/chapter_1558962745.html>
  and
  <https://docs.oracle.com/en/cloud/saas/netsuite/ns-online-help/section_159886587653.html>
- Sage Accounting and Sage Intacct general ledger:
  <https://developer.sage.com/accounting/docs/v1.0.0/guides/learning/key-concepts/overview>,
  <https://developer.sage.com/accounting/apis/sagebusinesscloudaccounting/3.1.0/accounting>,
  and <https://developer.intacct.com/api/>
- Odoo Accounting:
  <https://www.odoo.com/documentation/latest/applications/finance/accounting.html>

## Supplier API Notes

- QuickBooks Online contributes accounts, journal entries, classes, customers,
  vendors, items, attachments, and financial reports. Macaca should normalize
  reporting basis, entity scope, period range, and provider-specific report
  differences instead of exposing QuickBooks report payloads directly.
- Xero contributes accounts, journals with security/use-case gates, tracking
  categories, bank transactions, reconciliation-related surfaces, and reports.
  Macaca should model scope gates, report availability, and approval-sensitive
  journal access explicitly.
- NetSuite contributes broad ERP records, journal entries, accounting books,
  custom segments, and SuiteTax constraints. Macaca should isolate general
  ledger behavior and keep ERP workflow semantics outside this pack.
- Sage and Sage Intacct contribute chart of accounts, transactions, journals,
  ledger entries, reporting, dimensions, and budget/reporting structures. Macaca
  should map dimensions and report formats without provider chart templates.
- Odoo contributes open accounting models for accounts, journals, moves, taxes,
  reconciliation, and reports. Macaca should treat it as another provider model
  behind descriptors and capability discovery.

## Macaca-Owned Abstractions

`pack.finance.accounting.v1` should define `AccountingScope`,
`AccountingProviderCapability`, `AccountingEntity`, `LedgerBook`,
`AccountingPeriod`, `ChartOfAccounts`, `AccountHandle`, `AccountClass`,
`AccountingDimension`, `AccountMutationPlan`, `JournalEntryPlan`,
`JournalEntry`, `JournalLine`, `LedgerEntry`, `StatementLine`,
`ReconciliationCandidate`, `ReconciliationPlan`, `ReconciliationResult`,
`AccountingReportRequest`, `TrialBalanceReport`, `BalanceSheetReport`,
`ProfitLossReport`, `CashFlowReport`, `AuditExportPlan`,
`AccountingArtifactHandle`, `AccountingFreshness`, `AccountingAttribution`, and
`AccountingRedactionPolicy`.

The DTOs must carry entity/book scope, period locks, account classes,
dimensions, debit/credit balance constraints, currency precision, tax-code
references, source references, statement line provenance, reconciliation
confidence, report basis, pagination/async metadata, export retention,
capability hashes, redaction classes, bounded provider reason codes, and replay
pointers. Raw bank credentials, unbounded ledger exports, provider-specific ERP
workflows, and raw provider payloads are rejected.

## Explicit Non-Goals

- Do not implement concrete QuickBooks, Xero, NetSuite, Sage, Odoo, bank-data,
  tax, payroll, payment, or ERP adapters in this research phase.
- Do not define payroll, invoices, payments, bank transfers, tax filing, ERP
  workflows, portfolio accounting, investment advice, provider chart templates,
  or application-specific posting workflows inside this pack.
- Do not expose provider-native ledger payloads, credentials, raw bank data,
  chart templates, tax filing decisions, or business-specific posting rules as
  stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  accounting SDK helpers should only build canonical traced service calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics for optional domain-pack
  providers.
- Policy, resource, entitlement, trace, audit, artifact, mock-provider, and
  unavailable-provider concepts exist generically, but current evidence does
  not prove accounting-specific DTOs, descriptors, providers, SDK helpers, WASM
  ABI metadata, tests, or docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
