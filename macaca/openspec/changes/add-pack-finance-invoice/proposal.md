# Change: Add Finance Invoice Pack

## Why

Macaca applications need `pack.finance.invoice.v1` as a real industrial
invoicing capability for accounts-receivable and billing-document workflows.
Invoice APIs are deceptively complex: providers differ on draft/finalized
states, tax handling, customer references, line-item modeling, PDF export,
payment status, reminders, voiding, credit notes, and online-payment links.

This proposal defines invoice as a provider-neutral, serviceized pack. It lets
applications create, inspect, issue, send, void, track, remind, and export
invoices through typed commands while preserving Macaca's microkernel rules:
provider adapters live behind services, policy and approval run before side
effects, traces are sanitized, and OS layers do not own application billing
logic.

## Supplier And API Baseline

The design is based on mature invoicing API patterns:

- Stripe Invoicing exposes invoice objects, invoice items, draft invoice
  creation, finalize/send flows, automatic collection, hosted invoice pages,
  payment status, invoice PDFs, and webhook-driven lifecycle changes.
- QuickBooks Online Accounting API exposes invoice entities for create/read/send,
  customers, items, sales tax, payment processing linkage, sync-token style
  concurrency, PDFs, and accounting-report implications.
- Xero Accounting API exposes invoices, invoice payments, PDF response formats,
  approval/send lifecycle semantics, invoice history, contacts, items, tax
  rates, and explicit limitations around invoice reminders.
- FreshBooks API exposes invoices, clients, line items, draft/send semantics,
  payments, gateways, webhooks, pagination, and request-limit behavior.
- Zoho Books, Square Invoices, and similar SMB platforms expose invoice
  lifecycle, customer/item/tax references, payment links, reminders, and PDF
  export with provider-specific fields and regional requirements.

The common denominator is an invoice document with customer, line items, tax and
discount references, lifecycle state, delivery state, payment status, export
artifacts, external-recipient side effects, and audit evidence.

## Macaca Provider-Neutral Mapping

`pack.finance.invoice.v1` maps supplier concepts into stable contracts:

- Provider customers/contacts become `InvoicePartyReference`.
- Product/service items, usage lines, taxes, discounts, fees, and adjustments
  become `InvoiceLine`, `InvoiceTaxReference`, `InvoiceDiscount`, and
  `InvoiceAdjustment`.
- Draft, issued/finalized, sent, viewed, partially paid, paid, overdue, voided,
  written-off, and cancelled states become `InvoiceLifecycleState`.
- Provider payment objects become `InvoicePaymentStatus` references only.
  Payment collection, payment-intent creation, and settlement belong to
  commerce/payment capabilities, not this pack.
- Provider PDFs, hosted invoice pages, and exports become
  `InvoiceArtifactHandle`, never raw unbounded files in traces.
- Mutating and external-recipient operations use planning commands:
  `invoice.plan_invoice`, `invoice.create_draft`,
  `invoice.plan_issue`, `invoice.issue_invoice`,
  `invoice.plan_delivery`, `invoice.send_invoice`,
  `invoice.plan_reminder`, `invoice.send_reminder`,
  `invoice.plan_void`, `invoice.void_invoice`,
  `invoice.plan_export`, and `invoice.export_invoice`.

## What Changes

- Add provider-neutral `pack.finance.invoice.v1` under the finance family.
- Define commands for provider inspection, invoice schema discovery, customer
  and item references, draft planning/creation, invoice read/list, issue/finalize
  planning, delivery, payment-status sync, reminder planning/sending, voiding,
  credit-note references, export, and artifact retrieval.
- Define DTOs for invoice scope, provider capability, parties, lines, tax,
  discounts, totals, lifecycle state, delivery state, payment status, reminders,
  artifacts, freshness, attribution, redaction, and idempotency.
- Require policy, approval, entitlement, external-recipient gating, tax metadata,
  currency precision, idempotency, lifecycle transition validation, and sanitized
  trace/audit evidence.
- Require detailed developer documentation at
  `docs/developer-packs/finance/invoice.md`.

## Impact

- Affected specs: `pack-finance-invoice`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, invoice service providers, unavailable/mock
  providers, trace/audit schemas, replay tests, external-recipient gates,
  redaction tests, and dependency-boundary gates.

## Non-Goals

- No payment processing, settlement, refunds, chargebacks, tax filing, revenue
  recognition, subscription billing orchestration, collections strategy, or
  application-specific billing workflow.
- No provider-specific invoice templates, regional tax calculations, numbering
  policy, email copy, reminder cadence, or business terms hardcoded into Macaca
  OS layers.
- No raw customer PII, payment credentials, tax identifiers, invoice PDFs, hosted
  URLs with secrets, raw provider payloads, or unbounded line/export data in
  logs, traces, snapshots, or SDK diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
