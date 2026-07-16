# Finance Accounting Pack Design

## Context

`pack.finance.accounting.v1` is the provider-neutral Macaca pack for
general-ledger accounting integrations. It must serve application developers the
way an operating-system capability serves applications: an app declares the
capability and permissions; admission resolves availability; SDK helpers build
typed commands; the service runtime applies trace, policy, resource, entitlement,
approval, metering, and redaction decorators; replaceable providers implement
the actual accounting-system adapter.

Accounting data is regulated, operationally sensitive, and frequently
irreversible after posting. The design therefore treats provider APIs as
strategies behind a Macaca command contract rather than exposing provider-shaped
objects directly.

## Supplier Capability Matrix

| Supplier family | Relevant capabilities | Constraints Macaca must model |
| --- | --- | --- |
| QuickBooks Online | Accounts, journal entries, reports, closed-period checks, object identity, sync tokens, CDC/webhook-style freshness signals | Closed-period rejection, optimistic concurrency, account activation state, report parameter variance, provider error normalization |
| Xero | Accounts, journals, manual-journal/report scopes, reports, attachments, tracking categories | Scope-sensitive access, approval/security constraints for journals, report availability by tier/scope, attachment redaction |
| NetSuite SuiteTalk REST | Journal-entry records, intercompany and period-end variants, subsidiaries, classifications, multi-book accounting, supported-record metadata | Feature-gated records, unavailable tax fields in some REST paths, read-only sublists, subsidiary/book dimensionality |
| Sage Accounting / Sage Active | Ledger accounts, accounting entries, journal resources, dimensions, fiscal exercises, balanced-entry semantics | Balanced entry validation, journal status/draft behavior, fiscal period state, dimension taxonomy |
| Odoo Accounting | Account moves, move lines, partners, journals, taxes, attachments through generic model APIs | ERP model variability, localization modules, posted move immutability/reversal norms, field-level capability discovery |
| Adjacent financial-data providers | Bank feed, statement, and transaction data ingestion | Useful for imported statement lines only; not authoritative general-ledger semantics |

Macaca normalizes these into capability discovery plus command-level support
flags. If a provider cannot post journals, generate a cash-flow report, expose
period locks, or attach audit artifacts, discovery must say so before an app
attempts the command.

## Goals

- Provide industrial accounting operations for chart of accounts, accounting
  entities/books, periods, journals, ledger entries, imported statement lines,
  reconciliation, reports, and audit export.
- Enforce double-entry balance, period-lock checks, idempotency keys, currency
  precision, tax-code references, approval gates, and immutable posted evidence
  before provider side effects.
- Route every command through the canonical service runtime path with trace,
  policy, resource, entitlement, approval, health, snapshot, and structured
  error behavior.
- Expose provider capability differences without leaking provider-specific
  payloads or embedding provider-specific branches in OS layers.
- Require detailed developer documentation and SDK examples using generic
  handles and synthetic data.

## Non-Goals

- Payroll, invoices, payment execution, bank transfers, tax filing, ecommerce
  receipts, portfolio accounting, inventory costing, and investment advice.
- Application-specific bookkeeping workflows, account templates, tax rules,
  regional compliance interpretations, or provider-specific posting recipes.
- Direct provider calls from SDK, shell, kernel, or generic application
  framework code.
- Raw ledgers, attachments, provider responses, credentials, account numbers,
  tax identifiers, or unbounded financial data in observability.

## Ownership And Boundaries

- Pack id: `pack.finance.accounting.v1`.
- Family: `finance`.
- Backing service owner: accounting service provider family.
- SDK surface: `sdk.packs.finance.accounting`.
- Command namespace: `accounting.*`.
- Kernel ownership: identity, service-call evidence, policy facade, trace/audit
  primitives, and resource primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective-capability mementos.
- Runtime-host ownership: provider registration, decorators, unavailable/mock
  providers, and adapter composition through approved composition roots.
- Service ownership: provider-neutral command handling, capability discovery,
  provider strategy dispatch, state-machine enforcement, and sanitized audit.

## Command Surface

| Command | Purpose | Side-effect class |
| --- | --- | --- |
| `accounting.inspect_provider` | Return provider capability, supported commands, regional/tier limitations, freshness, and attribution | Read-only |
| `accounting.list_entities` | List accounting organizations, subsidiaries, ledger books, and base currencies visible to the caller | Read-only |
| `accounting.inspect_period` | Inspect fiscal period state, lock date, close status, and posting restrictions | Read-only |
| `accounting.get_chart_of_accounts` | Retrieve a normalized chart of accounts with account classes and active state | Read-only |
| `accounting.get_account` | Retrieve one account handle and provider-neutral metadata | Read-only |
| `accounting.plan_account` | Validate account create/update/deactivate request without provider mutation | Planning |
| `accounting.account_request` | Submit account create/update/deactivate through approved side-effect path | Mutating |
| `accounting.plan_journal` | Validate journal lines, dimensions, period, currency, tax references, and balance | Planning |
| `accounting.post_journal` | Post an approved balanced journal or return typed denial/conflict | Mutating |
| `accounting.list_journal_entries` | Search normalized journal headers by period, account, source, status, or cursor | Read-only |
| `accounting.get_ledger_entries` | Retrieve normalized ledger lines with bounded pagination and redaction | Read-only |
| `accounting.import_statement_lines` | Import or register bank/statement rows as reconciliation candidates | Mutating |
| `accounting.plan_reconciliation` | Match statement lines to ledger entries and expose confidence/evidence | Planning |
| `accounting.reconciliation_request` | Apply approved reconciliation actions or return conflict diagnostics | Mutating |
| `accounting.generate_trial_balance` | Generate a typed trial-balance report | Read-only or async read |
| `accounting.generate_balance_sheet` | Generate a typed balance-sheet report | Read-only or async read |
| `accounting.generate_profit_loss` | Generate a typed profit/loss report | Read-only or async read |
| `accounting.generate_cash_flow` | Generate a typed cash-flow report when provider supports it | Read-only or async read |
| `accounting.plan_audit_export` | Plan export scope, redaction, artifact format, and retention | Planning |
| `accounting.audit_export_request` | Produce an audit artifact handle through approved path | Mutating/export |
| `accounting.get_artifact_handle` | Retrieve metadata for an exported artifact without raw payload leakage | Read-only |

Every command must define typed command DTOs, success DTOs, denied/unavailable/
unsupported/conflict/quota/failure DTOs, idempotency behavior for side effects,
pagination/async behavior where relevant, redaction policy, and replay metadata.

## Provider-Neutral DTO Model

- `AccountingScope`: tenant, application, session, task, accounting entity,
  ledger book, period, currency, and permission scope.
- `AccountingProviderCapability`: command support matrix, report support,
  write support, period-lock visibility, attachment/export support, regional
  limitations, freshness model, and attribution requirements.
- `AccountingEntity`, `LedgerBook`, `AccountingPeriod`: organization,
  subsidiary/book, fiscal calendar, open/closed/locked states, base currency,
  and precision.
- `ChartOfAccounts`, `AccountHandle`, `AccountClass`, `AccountMutationPlan`:
  normalized accounts, account categories, active/deprecated state, provider
  concurrency token, and mutation validation.
- `AccountingDimension`: provider-neutral class, department, location, project,
  tracking category, subsidiary, cost center, or custom dimension reference.
- `JournalEntryPlan`, `JournalEntry`, `JournalLine`, `LedgerEntry`: balanced
  double-entry structures, debit/credit amounts, tax-code references, dimensions,
  source references, status, posting evidence, reversal references, and
  idempotency keys.
- `StatementLine`, `ReconciliationCandidate`, `ReconciliationPlan`,
  `ReconciliationResult`: imported statement data, match candidates, confidence,
  conflict reasons, applied actions, and replay evidence.
- `AccountingReportRequest`, `TrialBalanceReport`, `BalanceSheetReport`,
  `ProfitLossReport`, `CashFlowReport`: report basis, period range, currency,
  dimensions, aggregation, generated timestamp, freshness, and provider
  attribution.
- `AuditExportPlan`, `AuditExportResult`, `AccountingArtifactHandle`: export
  scope, format, retention, redaction profile, artifact metadata, checksum, and
  access policy.
- `AccountingFreshness`, `AccountingAttribution`, `AccountingRedactionPolicy`:
  freshness timestamp/source, licensing/attribution text, and field-level
  observability rules.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `finance.accounting.read`
- `finance.accounting.write`
- `finance.accounting.reconcile`
- `finance.accounting.report`
- `finance.accounting.audit_export`

Policy defaults:

- Scope every call to application id, tenant id, session id, task id, trace id,
  accounting entity, ledger book, and period when available.
- Require approval for account mutation, journal posting, statement import,
  reconciliation application, and audit export.
- Deny journal posting unless debit and credit totals balance per currency and
  provider precision.
- Deny mutation when a period is locked or provider capability reports that the
  target field/record is read-only.
- Require idempotency keys for mutating commands and export requests.
- Require resource reservations for pagination, report generation, export size,
  provider quotas, retained artifacts, and async jobs.
- Return typed `denied`, `unavailable`, `unsupported`, `conflict`,
  `quota_exceeded`, `stale_data`, or `failure` results before side effects when
  preconditions fail.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `accounting_pack_declared`
- `accounting_pack_admission_validated`
- `accounting_pack_policy_decision`
- `accounting_pack_provider_inspected`
- `accounting_pack_service_call_requested`
- `accounting_pack_service_call_succeeded`
- `accounting_pack_service_call_failed`
- `accounting_pack_side_effect_planned`
- `accounting_pack_side_effect_approved`
- `accounting_pack_unavailable`
- `accounting_pack_snapshot_recorded`

Events include pack id, command name, trace id, application/session/task/tenant
identifiers, accounting entity/book/period handles, policy decision, provider
class, descriptor hash, latency, bounded resource counters, result code,
freshness marker, and sanitized artifact references. Events must exclude raw
credentials, account numbers, tax identifiers, attachments, raw ledger payloads,
raw provider responses, and unbounded report rows.

Snapshots include descriptor version, provider health, command availability,
policy-template hash, report/export support, freshness, redaction profile,
resource counters, and replay pointers.

## SDK And Developer Documentation

SDK discovery must expose pack metadata, lifecycle, service mapping, command
schemas, permission scopes, policy templates, examples, availability, health,
provider class, compatibility, diagnostics, and documentation links.

The required developer guide at
`docs/developer-packs/finance/accounting.md` must cover:

- Manifest declaration and permission scopes.
- Capability discovery and unavailable diagnostics.
- DTO reference for every command/result family.
- Examples for reading accounts, planning/posting a balanced journal, generating
  reports, planning reconciliation, and handling denied/unavailable results.
- Provider replacement rules and mock/unavailable provider behavior.
- Trace/audit event interpretation and redaction guarantees.
- Accounting safety notes: double entry, period locks, idempotency, immutable
  posted evidence, reversal, tax-code references, and provider capability
  differences.

Examples must use generic handles and synthetic data. They must not contain real
provider names as routing keys, real credentials, hardcoded application logic, or
regional bookkeeping rules.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while hiding provider
  construction.
- **Command**: every cross-boundary accounting operation is a typed
  command/result DTO.
- **Strategy**: provider adapters implement the same accounting service contract
  with capability-discovery differences.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and
  redaction wrap every service call.
- **State**: journal planning, posting, reconciliation, export, provider health,
  and period availability use explicit states.
- **Specification**: admission validates declarations, scopes, command support,
  provider health, period locks, and mutation preconditions.
- **Observer**: trace, audit, health, and service events are subscribable.
- **Memento**: effective capability reports, snapshots, journal posting
  evidence, and export handles are replayable bounded records.
- **Abstract Factory**: providers register only through approved runtime-host or
  plugin composition roots.

## Risks And Mitigations

- Risk: accounting pack becomes a provider-specific workflow engine.
  Mitigation: keep provider behavior behind Strategy adapters and expose
  differences through capability descriptors.
- Risk: a mutating helper bypasses planning, approval, or period locks.
  Mitigation: enforce plan-before-side-effect and policy decorators with
  no-direct-provider-call gates.
- Risk: provider report output differs enough to confuse apps.
  Mitigation: typed report DTOs carry basis, period, dimensions, freshness, and
  unsupported/degraded metadata.
- Risk: observability leaks sensitive accounting data.
  Mitigation: redaction policy is part of every DTO and trace schema; snapshots
  store handles, hashes, and bounded counters only.
- Risk: posted entries need correction.
  Mitigation: posted journals are immutable in Macaca semantics; corrections use
  provider-supported reversal or adjustment flows through new planned commands.
