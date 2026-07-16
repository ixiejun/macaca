# Finance Invoice Pack Design

## Context

`pack.finance.invoice.v1` is the Macaca capability for provider-neutral invoice
documents and invoice lifecycle operations. It is an accounts-receivable pack,
not a payment processor or billing product. The pack normalizes invoice records,
drafting, issuing/finalizing, delivery, payment-status references, reminders,
voiding, and export artifacts across accounting, payment, and SMB invoicing
providers.

The design keeps provider-specific fields and lifecycle quirks behind provider
Strategy adapters. Applications use stable commands and explicit unsupported,
denied, degraded, stale, and unavailable states.

## Supplier Capability Matrix

| Supplier family | Relevant capabilities | Constraints Macaca must model |
| --- | --- | --- |
| Stripe Invoicing | Draft invoice creation, invoice items, finalize/send, hosted pages, PDFs, automatic collection, lifecycle/webhooks | Payment collection is adjacent; auto-advance/finalization semantics, webhook freshness, hosted URL redaction, payment status references |
| QuickBooks Online | Invoice entity, customers, items, sales tax, send, PDF, sync-token concurrency, payment linkage | Accounting-company scope, tax/item dependencies, concurrency tokens, accounting report impact, PDF artifact handling |
| Xero Accounting | Invoices, contacts, items, tax rates, payments, PDF responses, approval/send lifecycle, history | OAuth scopes, approval permissions, invoice reminder API limitations, PDF/export formats, contact/tax references |
| FreshBooks | Invoices, clients, line items, draft/send semantics, payments, gateways, webhooks, pagination | Draft-to-sent accounting implications, request limits, client references, online-payment link behavior |
| Zoho/Square-style SMB APIs | Invoice lifecycle, customers, items, reminders, payment links, tax/discount fields, PDF/export | Provider-specific numbering/templates, reminder cadence, regional tax fields, payment-link boundaries |

## Goals

- Provide invoice schema discovery, party/item reference discovery, draft
  planning, draft creation, invoice list/read, issue/finalize, delivery,
  payment-status sync, reminder planning/sending, voiding, export, and artifact
  retrieval.
- Preserve lifecycle correctness: draft, issued/finalized, sent, viewed,
  partially paid, paid, overdue, voided, cancelled, written off, and provider
  custom states.
- Enforce currency precision, line totals, tax/discount references, idempotency,
  approval, recipient policy, provider capability checks, and lifecycle
  transition validation before side effects.
- Make payment status observable without turning this pack into a payment
  execution capability.
- Require detailed SDK and developer documentation.

## Non-Goals

- Payment processing, payment-intent creation, settlement, refunds, chargebacks,
  tax filing, revenue recognition, subscription billing orchestration,
  collections strategy, and application-specific billing workflow.
- Provider-specific tax calculation, invoice numbering policy, template design,
  email body, reminder cadence, or business terms in Macaca OS layers.
- Raw PII, payment credentials, tax identifiers, invoice PDFs, raw hosted URLs
  with secrets, raw provider payloads, or unbounded invoice lines in
  observability.

## Ownership And Boundaries

- Pack id: `pack.finance.invoice.v1`.
- Family: `finance`.
- Backing service owner: invoice service provider family.
- SDK surface: `sdk.packs.finance.invoice`.
- Command namespace: `invoice.*`.
- Kernel ownership: identity, service-call evidence, policy facade, trace/audit
  primitives, and resource primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective-capability mementos.
- Runtime-host ownership: provider registration, decorators, mock/unavailable
  providers, and adapter composition through approved composition roots.
- Service ownership: capability discovery, lifecycle transition validation,
  provider strategy dispatch, redaction, artifacts, and sanitized audit.

## Command Surface

| Command | Purpose | Side-effect class |
| --- | --- | --- |
| `invoice.inspect_provider` | Return provider capability, lifecycle support, export formats, reminder support, freshness, and attribution | Read-only |
| `invoice.describe_schema` | Return required fields, supported tax/discount/line types, numbering constraints, and lifecycle transitions | Read-only |
| `invoice.list_parties` | Resolve customers/contacts as provider-neutral party references | Read-only |
| `invoice.list_items` | Resolve item/product/service references where provider supports them | Read-only |
| `invoice.plan_invoice` | Validate invoice draft structure, totals, currency, tax references, parties, and provider constraints | Planning |
| `invoice.create_draft` | Create an invoice draft through approved side-effect path | Mutating |
| `invoice.list_invoices` | Search invoices by party, date, status, amount, or cursor | Read-only |
| `invoice.read_invoice` | Read one normalized invoice and lifecycle/payment state | Read-only |
| `invoice.plan_issue` | Validate finalization/approval requirements and provider lifecycle transition | Planning |
| `invoice.issue_invoice` | Finalize or issue an invoice through approved side-effect path | Mutating |
| `invoice.plan_delivery` | Validate recipients, delivery channel, message policy, and provider delivery support | Planning |
| `invoice.send_invoice` | Send/deliver an issued invoice through external-recipient gated path | External side effect |
| `invoice.sync_payment_status` | Refresh provider payment-status references without collecting payment | Read-only or sync |
| `invoice.plan_reminder` | Validate reminder eligibility, recipients, cadence, and provider support | Planning |
| `invoice.send_reminder` | Send an approved invoice reminder through external-recipient gated path | External side effect |
| `invoice.plan_void` | Validate void/cancel/write-off transition and accounting constraints | Planning |
| `invoice.void_invoice` | Apply an approved void/cancel transition when provider supports it | Mutating |
| `invoice.plan_export` | Plan PDF/HTML/JSON/CSV export, retention, redaction, and resource budget | Planning |
| `invoice.export_invoice` | Produce invoice artifact handle through approved path | Mutating/export |
| `invoice.get_artifact_handle` | Retrieve export artifact metadata without raw payload leakage | Read-only |

Every command must define typed command DTOs, typed success DTOs, typed partial
or async shapes, typed denied/unavailable/unsupported/conflict/quota/stale-data/
failure results, idempotency where side effects exist, redaction policy, and
replay metadata.

## Provider-Neutral DTO Model

- `InvoiceScope`: application, tenant, session, task, accounting entity,
  provider account, invoice handle, party handle, currency, and permission
  scope.
- `InvoiceProviderCapability`: command support, lifecycle support, reminder
  support, export formats, tax/discount support, numbering constraints, payment
  status support, freshness model, attribution, geography, and entitlement.
- `InvoicePartyReference`: customer/contact/vendor handle, redacted display
  name, billing/shipping reference, tax identifier reference, and consent class.
- `InvoiceItemReference`: product/service/item handle, unit metadata, tax class,
  revenue account reference where available, and provider constraints.
- `InvoiceLine`, `InvoiceTaxReference`, `InvoiceDiscount`,
  `InvoiceAdjustment`: line type, quantity, unit amount, currency, tax,
  discount, fees, rounding, service period, and source reference.
- `InvoiceTotals`: subtotal, tax, discount, shipping, fees, amount due, amount
  paid, amount remaining, currency precision, and rounding evidence.
- `InvoiceDraftPlan`, `InvoiceRecord`, `InvoiceLifecycleState`,
  `InvoiceDeliveryState`, `InvoicePaymentStatus`: draft/issued/sent/viewed/paid
  lifecycle, delivery evidence, payment status references, due dates, terms, and
  provider concurrency token.
- `InvoiceReminderPlan`, `InvoiceReminderResult`: recipient list, channel,
  cadence, eligibility, provider support, delivery evidence, and redaction.
- `InvoiceArtifactPlan`, `InvoiceArtifactHandle`: export format, checksum,
  expiry, retention, redaction, access policy, and replay pointer.
- `InvoiceFreshness`, `InvoiceAttribution`, `InvoiceRedactionPolicy`:
  freshness timestamp/source, provider attribution, and observability rules.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `finance.invoice.read`
- `finance.invoice.write`
- `finance.invoice.issue`
- `finance.invoice.deliver`
- `finance.invoice.remind`
- `finance.invoice.export`

Policy defaults:

- Scope every call to application id, tenant id, session id, task id, trace id,
  accounting entity/provider account, invoice handle, and party handle when
  available.
- Require approval for draft creation, issuing/finalizing, external delivery,
  reminder sending, voiding, and retained exports.
- Require recipient policy checks before `send_invoice` and `send_reminder`.
- Require idempotency keys for side-effect commands.
- Validate lifecycle transitions, currency precision, totals, tax references,
  party references, provider concurrency tokens, and provider capability before
  mutation.
- Return typed `denied`, `unavailable`, `unsupported`, `conflict`,
  `quota_exceeded`, `stale_data`, or `failure` before provider calls when
  preconditions fail.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `invoice_pack_declared`
- `invoice_pack_admission_validated`
- `invoice_pack_policy_decision`
- `invoice_pack_provider_inspected`
- `invoice_pack_service_call_requested`
- `invoice_pack_service_call_succeeded`
- `invoice_pack_service_call_failed`
- `invoice_pack_side_effect_planned`
- `invoice_pack_external_delivery_requested`
- `invoice_pack_unavailable`
- `invoice_pack_snapshot_recorded`

Events include pack id, command name, trace id, application/session/task/tenant
identifiers, invoice/party handles, lifecycle transition, policy decision,
provider class, descriptor hash, latency, bounded resource counters, freshness,
result code, and sanitized artifact references. Events must exclude raw PII,
payment credentials, tax identifiers, raw hosted URLs with secrets, raw PDFs,
raw provider payloads, full invoice lines, and unbounded export data.

Snapshots include descriptor version, provider health, command availability,
lifecycle support, reminder/export support, policy-template hash, redaction
profile, resource counters, freshness, and replay pointers.

## SDK And Developer Documentation

SDK discovery must return pack metadata, lifecycle, service mapping, command
schemas, permission scopes, policy templates, examples, availability, health,
provider class, compatibility, diagnostics, and documentation links.

The required developer guide at `docs/developer-packs/finance/invoice.md` must
cover:

- Manifest declaration and permission scopes.
- Capability discovery, schema discovery, and unavailable diagnostics.
- DTO reference for parties, items, lines, taxes, totals, lifecycle, delivery,
  payment status, reminders, and artifacts.
- Examples for planning/creating drafts, issuing, sending, syncing payment
  status, planning reminders, exporting, and handling denied/unsupported states.
- Provider replacement, mock/unavailable provider behavior, external-recipient
  policy, trace/audit interpretation, and redaction guarantees.
- Boundaries: payment collection belongs to payment-intent/commerce; regional
  tax rules, templates, and reminder cadences belong to providers/apps.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while hiding providers.
- **Command**: every invoice operation is a typed command/result DTO.
- **Strategy**: invoice providers adapt lifecycle, tax, reminder, PDF, and
  payment-status differences behind one service contract.
- **Decorator**: trace, policy, entitlement, recipient approval, resource,
  metering, and redaction wrap every service call.
- **State**: invoice lifecycle, delivery, payment status, reminder, export, and
  provider health are explicit states.
- **Specification**: admission validates declarations, scopes, lifecycle
  transitions, recipient policy, provider capability, idempotency, and resource
  limits.
- **Observer**: trace, audit, provider, delivery, and snapshot events are
  subscribable.
- **Memento**: effective capability reports, lifecycle evidence, delivery
  evidence, and artifact handles are replayable bounded records.
- **Abstract Factory**: providers register only through approved runtime-host or
  plugin composition roots.

## Risks And Mitigations

- Risk: invoice pack becomes a payment processor. Mitigation: payment status is
  reference/sync only; collection and settlement live in payment capabilities.
- Risk: external delivery leaks PII or sends unintended email. Mitigation:
  `plan_delivery` and `plan_reminder` require recipient policy and approval.
- Risk: provider lifecycle differences cause invalid transitions. Mitigation:
  schema/capability discovery plus lifecycle Specification checks run before
  side effects.
- Risk: observability leaks invoice PDFs or hosted links. Mitigation: traces and
  snapshots store artifact handles, checksums, bounded codes, and redacted
  metadata only.
