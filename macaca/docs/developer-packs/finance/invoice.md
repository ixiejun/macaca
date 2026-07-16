# Finance Invoice Pack

`pack.finance.invoice.v1` describes provider-neutral invoice lifecycle,
delivery, reminder, payment-status sync, and export capabilities. The
descriptor is discoverable through SDK catalogs, but commands remain unavailable
until an invoice provider is installed through the runtime composition root.

## Manifest Declaration

Declare the pack as required only when invoice operations are mandatory for
readiness. Optional declarations degrade with structured unavailable
diagnostics.

```toml
[service_contract]
optional_packs = ["pack.finance.invoice.v1"]
```

## Permissions

Use the narrowest scope: `finance.invoice.read`, `finance.invoice.write`,
`finance.invoice.issue`, `finance.invoice.deliver`,
`finance.invoice.remind`, and `finance.invoice.export`.

## Capability Model

Macaca models invoices as tenant, accounting-entity, recipient-policy, and
permission scopes, provider capability reports, party references, item
references, tax identifier references, invoice lines, tax references,
discounts, adjustments, totals, draft plans, invoice records, lifecycle states,
delivery states, payment-status snapshots, provider concurrency tokens,
reminder plans, artifact plans, freshness metadata, attribution metadata,
redaction policies, and artifact handles. Raw PII, payment credentials, hosted
URLs with secrets, raw PDFs, provider payloads, and unbounded invoice lines stay
behind provider adapters.

## Commands And Results

`invoice.inspect_provider`, `invoice.describe_schema`, `invoice.list_parties`,
`invoice.list_items`, `invoice.plan_invoice`, `invoice.create_draft`,
`invoice.list_invoices`, `invoice.read_invoice`, `invoice.plan_issue`,
`invoice.issue_invoice`, `invoice.plan_delivery`, `invoice.send_invoice`,
`invoice.sync_payment_status`, `invoice.plan_reminder`,
`invoice.send_reminder`, `invoice.plan_void`, `invoice.void_invoice`,
`invoice.plan_export`, `invoice.export_invoice`, and
`invoice.get_artifact_handle` are descriptor-owned schema names.

Every command uses a `FinanceCommandEnvelope`. Results use
`InvoiceResultEnvelope<T>` with success, partial, denied, unavailable,
unsupported, conflict, quota-exceeded, stale-data, and failure states. Planning
commands must produce traceable lifecycle plans before draft creation, issuing,
delivery, reminders, voiding, or retained exports.

## Supplier Mapping

Stripe Invoicing, QuickBooks Online, Xero Accounting, FreshBooks, Zoho Books,
Square Invoices, and similar SMB invoicing APIs map to schema discovery, party
and item references, draft plans, invoice records, lifecycle states, delivery
states, payment-status sync, reminder plans, exports, artifacts, freshness, and
attribution DTOs. Payment processing, payment-intent creation, settlement,
refunds, chargebacks, tax filing, revenue recognition, subscription billing,
collections strategy, templates, and application-specific billing workflows are
not OS semantics.

## App-Facing Examples

- Inspect provider classes and unavailable diagnostics before invoice actions.
- List parties, list items, plan invoices, create drafts, read/list invoices,
  plan issue, issue invoices, plan delivery, send invoices, sync payment status,
  plan reminders, send reminders, plan voids, void invoices, and export through
  descriptor-owned command schemas.
- Enforce recipient policy before delivery and reminders.
- Export invoices through retained artifact handles and keep hosted URLs or raw
  PDFs outside trace evidence.
- Treat unsupported reminders, lifecycle conflicts, stale data, denied,
  unavailable, quota, export-denied, and recipient-policy-denied outcomes as
  structured results.

## Trace And Audit

Traces should record declaration, admission decision, command name, entity ref,
party ref hash, invoice ref, lifecycle state, delivery channel, recipient-policy
hash, payment-status class, artifact id, provider class, capability hash,
freshness class, attribution ref, result status, idempotency key hash, and
redaction profile. They must not record raw PII, payment credentials, tax
identifiers, hosted URLs with secrets, invoice PDFs, raw provider payloads,
manifests, package bytes, private keys, or unbounded invoice lines.

## Provider Authors

Conformance requires descriptor completeness, schema discovery, party and item
normalization, tax and discount validation, line-total validation, lifecycle
transition validation, concurrency-token handling, idempotency handling,
recipient policy checks, reminder cadence checks, export retention, resource
bounds, timeout and cancellation handling, policy hooks, unavailable behavior,
snapshot and replay metadata, and redaction tests. Providers must return
structured unavailable, denied, unsupported, conflict, quota, stale-data,
timeout, cancellation, and failure results without collecting payments or
fabricating delivery success.
