# Finance Invoice Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.finance.invoice.v1`. The invoice pack must expose schema discovery,
party/item references, draft planning/creation, invoice read/list,
issue/finalize, delivery, payment-status sync, reminders, void/cancel, export,
artifacts, freshness, attribution, and redaction through typed service commands.
It must not execute payment processing, create payment intents, settle funds,
refund charges, file taxes, orchestrate subscription billing, define
collections strategy, or own application-specific billing workflows.

## Source Baseline

- Stripe Invoicing and invoice lifecycle:
  <https://docs.stripe.com/api/invoices>,
  <https://docs.stripe.com/api/invoices/finalize>, and
  <https://docs.stripe.com/invoicing/integration/workflow-transitions>
- QuickBooks Online API:
  <https://developer.intuit.com/app/developer/qbo/docs/develop>
- Xero Accounting invoices:
  <https://developer.xero.com/documentation/api/accounting/invoices>
- FreshBooks API:
  <https://www.freshbooks.com/api/invoices>
- Zoho Books invoices:
  <https://www.zoho.com/books/api/v3/invoices/>
- Square Invoices:
  <https://developer.squareup.com/reference/square/invoices-api> and
  <https://developer.squareup.com/docs/invoices-api/overview>

## Supplier API Notes

- Stripe contributes invoice objects, draft/finalize/send workflow, invoice
  items, automatic advancement, hosted invoice pages, payment status, and
  webhooks. Macaca should model invoice lifecycle and payment-status references
  without executing payment collection.
- QuickBooks and Xero contribute SMB invoice records, line items, tax fields,
  parties, PDF/export behavior, email/send operations, balances, payments as
  linked records, and accounting constraints. Macaca should separate invoice
  lifecycle from accounting and payment execution.
- FreshBooks and Zoho Books contribute estimates/invoices, clients, items,
  reminders, online payment settings, statuses, and exports. Macaca should
  normalize reminders and delivery while keeping collections strategy outside OS
  semantics.
- Square Invoices contribute draft creation for Orders API orders, delivery
  configuration, publishing, payment requests, webhooks, and hosted invoices.
  Macaca should treat Square order/payment behavior as references to adjacent
  commerce packs.

## Macaca-Owned Abstractions

`pack.finance.invoice.v1` should define `InvoiceScope`,
`InvoiceProviderCapability`, `InvoicePartyReference`,
`InvoiceItemReference`, `InvoiceLine`, `InvoiceTaxReference`,
`InvoiceDiscount`, `InvoiceAdjustment`, `InvoiceTotals`,
`InvoiceDraftPlan`, `InvoiceRecord`, `InvoiceLifecycleState`,
`InvoiceDeliveryState`, `InvoicePaymentStatus`, `InvoiceReminderPlan`,
`InvoiceReminderResult`, `InvoiceArtifactPlan`, `InvoiceArtifactHandle`,
`InvoiceFreshness`, `InvoiceAttribution`, and `InvoiceRedactionPolicy`.

The DTOs must carry provider schema support, party/item references, tax
identifier references, service periods, line totals, currency precision,
rounding evidence, lifecycle state, delivery state, payment-status references,
reminder cadence, recipient policy, provider concurrency tokens, artifact
retention, capability hashes, redaction classes, bounded provider reason codes,
and replay pointers. Raw payment credentials, raw recipient data, provider
templates, subscription billing plans, and unbounded invoice exports are
rejected.

## Explicit Non-Goals

- Do not implement concrete Stripe, QuickBooks, Xero, FreshBooks, Zoho, Square,
  tax, payment, subscription, collections, template, or email/SMS adapters in
  this research phase.
- Do not define payment processing, payment-intent creation, settlement,
  refunds, chargebacks, tax filing, revenue recognition, subscription billing
  orchestration, collections strategy, templates, or application-specific
  billing workflows inside this pack.
- Do not expose provider-native invoice payloads, payment credentials, provider
  templates, dunning rules, tax filing decisions, or app-specific billing logic
  as stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  invoice SDK helpers should only build canonical traced service calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics.
- Policy, recipient gate, resource, entitlement, trace, audit, artifact,
  mock-provider, and unavailable-provider concepts exist generically, but
  current evidence does not prove invoice-specific DTOs, descriptors, providers,
  SDK helpers, WASM ABI metadata, tests, or docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
