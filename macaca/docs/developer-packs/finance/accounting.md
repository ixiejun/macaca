# Finance Accounting Pack

`pack.finance.accounting.v1` describes provider-neutral general-ledger
accounting capabilities. The descriptor is discoverable through SDK catalogs,
but commands remain unavailable until an accounting provider is installed
through the runtime composition root.

## Manifest Declaration

Declare the pack as required only when accounting access is mandatory for
application readiness. Optional declarations degrade with structured
unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.finance.accounting.v1"]
```

## Permissions

Use the narrowest scope: `finance.accounting.read`,
`finance.accounting.write`, `finance.accounting.reconcile`,
`finance.accounting.report`, and `finance.accounting.audit_export`.

## Capability Model

Macaca models accounting as tenant, entity, and ledger-book scopes, provider
capability reports, entities, ledger books, periods, period locks, chart of
accounts, account handles, dimensions, journal-entry plans, posted journals,
ledger entries, statement lines, reconciliation plans, reports, audit-export
plans, redaction policies, freshness metadata, attribution metadata, and
artifact handles. Raw ledgers, bank account numbers, tax identifiers,
attachments, credentials, provider payloads, and unbounded reports stay behind
provider adapters.

## Commands And Results

`accounting.inspect_provider`, `accounting.list_entities`,
`accounting.inspect_period`, `accounting.get_chart_of_accounts`,
`accounting.get_account`, `accounting.plan_account`,
`accounting.account_request`, `accounting.plan_journal`,
`accounting.post_journal`, `accounting.list_journal_entries`,
`accounting.get_ledger_entries`, `accounting.import_statement_lines`,
`accounting.plan_reconciliation`, `accounting.reconciliation_request`,
`accounting.generate_trial_balance`, `accounting.generate_balance_sheet`,
`accounting.generate_profit_loss`, `accounting.generate_cash_flow`,
`accounting.plan_audit_export`, `accounting.audit_export_request`, and
`accounting.get_artifact_handle` are descriptor-owned schema names.

Every command uses a `FinanceCommandEnvelope`. Results use
`AccountingResultEnvelope<T>` with success, partial, denied, unavailable,
unsupported, conflict, quota-exceeded, stale-data, and failure states. Planning
commands must produce traceable plans before side effects. Posting and
reconciliation requests must carry idempotency keys and approval evidence.

## Supplier Mapping

QuickBooks Online, Xero, NetSuite SuiteTalk REST, Sage Accounting/Sage Active,
Odoo Accounting, and bank-data APIs map to entity, account, journal, ledger,
statement-line, reconciliation, report, artifact, freshness, and attribution
DTOs. Provider chart templates, native endpoints, account-number formats,
region-specific tax filing, payroll, invoices, and ERP workflows are not OS
semantics.

## App-Facing Examples

- Inspect provider classes and unavailable diagnostics before accounting reads.
- Read entities, periods, chart of accounts, accounts, journals, ledger pages,
  and reports through bounded references.
- Plan account mutations, journals, reconciliation, and audit exports before
  requesting side effects.
- Generate trial balance, balance sheet, profit/loss, and cash-flow reports as
  bounded report references with artifact handles.
- Treat period locks, unbalanced journals, missing write support, unsupported
  reports, stale data, quota, denied, unavailable, and conflict outcomes as
  structured results.

## Trace And Audit

Traces should record declaration, admission decision, command name, entity ref,
ledger-book ref, period ref, plan ref, journal ref, reconciliation ref, report
ref, artifact id, provider class, capability hash, result status, idempotency
key hash, freshness class, attribution ref, and redaction profile. They must not
record credentials, raw account numbers, tax identifiers, attachments, raw
ledgers, raw provider payloads, manifests, package bytes, private keys, or
unbounded reports.

## Provider Authors

Conformance requires descriptor completeness, period-lock validation, account
state validation, balanced debit and credit validation, dimension and tax-code
validation, idempotency handling, reconciliation conflict evidence, report
pagination, export retention, policy hooks, approval hooks, resource bounds,
timeout and cancellation handling, unavailable behavior, snapshot and replay
metadata, and redaction tests. Providers must return structured unavailable,
denied, unsupported, conflict, quota, stale-data, timeout, cancellation, and
failure results without faking ledger success.
