# Commerce Receipt Pack Design

## Context

`pack.commerce.receipt.v1` is Macaca's provider-neutral receipt-evidence
capability. It owns receipt records, receipt issue/reissue requests, read/search,
verification, delivery state, hosted/artifact handles, correction references,
source synchronization, and receipt audit export. It does not own payment
execution, refunds, invoices, settlement, entitlement provisioning, fulfillment,
or application checkout UI.

Receipt APIs are fragmented across providers. Some providers expose a URL on a
payment, some expose terminal receipt data, some send email receipts from gateway
configuration, and some require integrators to build customer-facing documents
from payment/order transaction data. Macaca normalizes the receipt slice and
makes adjacent capabilities explicit through references and boundary checks.

## Supplier Capability Matrix

| Supplier family | Relevant capabilities | Constraints Macaca must model |
| --- | --- | --- |
| Stripe | Payment receipts, charge `receipt_url`, email receipts, refund receipts, invoice receipts, Terminal receipt data | Hosted URL vs generated data, email configuration, refund receipt references, Terminal/card-network-required fields, raw client/provider payload redaction |
| Square | Payment `receipt_url`, tender receipt URLs, Terminal receipt actions for printing/issuing receipts | Split tender variants, existing-payment/cash-payment requirements, POS vs online behavior, delivery separate from payment creation |
| Adyen | Terminal `PaymentReceipt`, shopper/cashier receipt variants, digital receipt generation from terminal response | Multi-audience receipt variants, terminal transaction state, integrator-owned generation/delivery, reconciliation fields |
| PayPal | Orders, captures, authorizations, refunds, transaction details | Receipt evidence is derived from capture/transaction records; approval/capture/refund execution belongs to adjacent packs |
| Braintree | Email receipts for transactions/refunds submitted for settlement, transaction response records | Gateway configuration controls email receipts, settlement state matters, transaction data is sensitive and policy-limited |
| Shopify | Orders, order transactions, POS receipt management, printed/email/SMS/gift receipt behavior | Order/transaction/POS boundaries, line-count constraints for printed receipts, receipt settings/templates are merchant/provider concerns |

## Goals

- Provide provider inspection, schema discovery, receipt issue planning, receipt
  issue/reissue, read/search, source sync, verification, delivery planning,
  delivery request, delivery status, correction/reference linking, audit export,
  and artifact retrieval.
- Preserve receipt source references, amount/currency precision, line/tax/total
  snapshots, audience/variant semantics, delivery state, idempotency, receipt
  artifact retention, provider freshness, and verification evidence.
- Keep payment, refund, invoice, settlement, entitlement, fulfillment, branding
  template, and customer communication workflow as separate capability
  boundaries.
- Route every command through the canonical service runtime path with trace,
  policy, entitlement, resource, approval when required, health, snapshot, and
  structured errors.

## Non-Goals

- Payment authorization/capture, payment-method collection, refund execution,
  invoice generation, settlement reconciliation, payout, dispute handling,
  entitlement provisioning, tax filing, carrier fulfillment, or checkout UI.
- Provider-specific receipt template editing, merchant branding decisions,
  delivery-channel marketing workflow, receipt legal/tax advice, or business
  routing in OS layers.
- Raw buyer PII, payment credentials, raw provider payloads, webhook bodies,
  receipt HTML bodies, printable binary blobs, signatures, private keys, or
  unbounded receipt exports in observability.

## Ownership And Boundaries

- Pack id: `pack.commerce.receipt.v1`.
- Family: `commerce`.
- Backing service owner: receipt service provider family.
- SDK surface: `sdk.packs.commerce.receipt`.
- Command namespace: `receipt.*`.
- Kernel ownership: identity, service-call evidence, policy facade, trace/audit
  primitives, and resource primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective-capability mementos.
- Runtime-host ownership: provider registration, decorators, mock/unavailable
  providers, and adapter composition through approved composition roots.
- Service ownership: capability discovery, receipt state validation, provider
  Strategy dispatch, source normalization, artifact boundary enforcement,
  redaction, and sanitized audit.

## Command Surface

| Command | Purpose | Side-effect class |
| --- | --- | --- |
| `receipt.inspect_provider` | Return provider support for source types, audiences, delivery channels, verification, export, freshness, and attribution | Read-only |
| `receipt.describe_schema` | Return receipt, source, line, totals, delivery, verification, correction, event, and artifact schema | Read-only |
| `receipt.plan_issue` | Validate source payment/order/invoice/reference, audience, variant, delivery, idempotency, approval, and provider constraints | Planning |
| `receipt.issue_receipt` | Issue or persist receipt evidence through approved side-effect path | Mutating |
| `receipt.plan_reissue` | Validate reissue target, original receipt state, delivery, correction references, and provider support | Planning |
| `receipt.reissue_receipt` | Reissue or resend receipt evidence when provider supports it | Mutating |
| `receipt.read_receipt` | Read one normalized receipt record | Read-only |
| `receipt.search_receipts` | Search receipts by source, customer/session reference, date, amount, audience, delivery state, or cursor | Read-only |
| `receipt.sync_source` | Refresh source payment/order/transaction/invoice receipt metadata and freshness | Read-only or provider sync |
| `receipt.verify_receipt` | Verify receipt integrity, source linkage, freshness, totals, artifact checksum, or provider verification reference | Read-only |
| `receipt.plan_delivery` | Validate delivery audience, channel, destination reference, approval, redaction, and provider support | Planning |
| `receipt.delivery_request` | Send, print, publish, or enqueue a receipt delivery through approved path | Mutating |
| `receipt.get_delivery_status` | Read delivery state, attempt metadata, and bounded failure code | Read-only |
| `receipt.link_correction_reference` | Link refund, void, cancellation, chargeback, or correction reference without executing adjacent capability | Mutating metadata |
| `receipt.list_correction_references` | Read correction references tied to a receipt | Read-only |
| `receipt.record_event_reference` | Normalize provider event metadata without storing raw webhook body | Mutating metadata |
| `receipt.plan_audit_export` | Plan receipt audit export scope, format, redaction, retention, and artifact bounds | Planning |
| `receipt.audit_export_request` | Produce receipt audit artifact handle | Mutating/export |
| `receipt.get_artifact_handle` | Retrieve hosted URL, PDF, JSON, print, or audit artifact metadata without raw payload leakage | Read-only |

Every command must define typed command DTOs, success DTOs, partial/async shapes,
denied/unavailable/unsupported/conflict/quota/stale-data/failure results,
idempotency for side effects, redaction policy, and replay metadata.

## Provider-Neutral DTO Model

- `ReceiptScope`: application, tenant, session, task, merchant/store/channel,
  receipt handle, source handle, customer/session reference, and permission
  scope.
- `ReceiptProviderCapability`: supported source types, audiences, variants,
  delivery channels, artifact formats, verification modes, reissue support,
  correction-reference support, export support, idempotency model, freshness,
  limits, attribution, and entitlement.
- `ReceiptSourceReference`: payment intent, capture, charge, transaction, order,
  invoice, refund, void, cash payment, terminal transaction, external document,
  and provider event references with redaction metadata.
- `ReceiptRecord`: receipt handle, source references, receipt number/reference,
  audience, variant, issue state, issued timestamp, line snapshots, adjustments,
  totals, payment/order/invoice references, correction references, delivery
  state, artifact handles, verification state, freshness, and redaction class.
- `ReceiptLine`, `ReceiptAdjustment`, `ReceiptTotals`: item references,
  descriptions, quantities, unit amounts, discounts, taxes, duties, fees,
  shipping, gratuities, service charges, currency precision, and evidence
  source.
- `ReceiptAudience` and `ReceiptVariant`: customer, merchant, cashier, gift,
  regulatory, refund, correction, terminal, hosted, printable, and custom
  provider variants with mapping metadata.
- `ReceiptDeliveryRequest` and `ReceiptDeliveryState`: channel, destination
  reference, consent/approval reference, attempt count, provider message
  reference, terminal action reference, state, failure code, and redaction.
- `ReceiptVerificationResult`: source linkage, totals match, checksum/signature
  status, provider verification reference, freshness, conflict/stale-data
  details, and replay pointer.
- `ReceiptCorrectionReference`: refund, void, cancellation, chargeback, return,
  replacement, or adjustment reference without adjacent side-effect payloads.
- `ReceiptEventReference`: provider class, event type, event timestamp, delivery
  id hash, webhook freshness, replay pointer, and bounded result code.
- `ReceiptAuditExportPlan`, `ReceiptArtifactHandle`: export scope, artifact
  type, hosted URL metadata, checksum, expiry, retention, redaction profile,
  access policy, and replay pointer.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `commerce.receipt.read`
- `commerce.receipt.issue`
- `commerce.receipt.reissue`
- `commerce.receipt.verify`
- `commerce.receipt.deliver`
- `commerce.receipt.correction_reference`
- `commerce.receipt.audit_export`

Policy defaults:

- Scope every call to application id, tenant id, session id, task id, trace id,
  merchant/store/channel, receipt handle, source handle, and customer/session
  reference.
- Require approval for receipt issue/reissue when it creates retained evidence,
  delivery to external destinations, printing on host/terminal devices, and
  retained audit exports.
- Require idempotency keys for mutating commands and delivery/export requests.
- Require source-state validation before issue/reissue: the referenced payment,
  order, invoice, terminal transaction, cash payment, or correction source must
  be visible through declared capabilities or provider references.
- Return typed `denied`, `unavailable`, `unsupported`, `conflict`,
  `quota_exceeded`, `stale_data`, or `failure` before provider calls when
  preconditions fail.
- Enforce resource budgets for receipt search, source sync, artifact retrieval,
  delivery attempts, audit export size, provider quotas, storage, and snapshots.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `receipt_pack_declared`
- `receipt_pack_admission_validated`
- `receipt_pack_policy_decision`
- `receipt_pack_provider_inspected`
- `receipt_pack_service_call_requested`
- `receipt_pack_service_call_succeeded`
- `receipt_pack_service_call_failed`
- `receipt_pack_issue_planned`
- `receipt_pack_delivery_planned`
- `receipt_pack_verification_completed`
- `receipt_pack_correction_reference_linked`
- `receipt_pack_unavailable`
- `receipt_pack_snapshot_recorded`

Events include pack id, command name, trace id, application/session/task/tenant
identifiers, receipt/source handles, audience, variant, delivery channel,
verification state, policy decision, provider class, descriptor hash, latency,
freshness, idempotency hash, bounded resource counters, result code, and
sanitized artifact references. Events must exclude raw buyer PII, payment
credentials, raw provider payloads, webhook bodies, receipt HTML bodies,
printable binary blobs, private keys, signatures, and unbounded exports.

Snapshots include descriptor version, provider health, command availability,
source/audience/delivery/verification/export support, policy-template hash,
redaction profile, freshness, resource counters, and replay pointers.

## SDK And Developer Documentation

SDK discovery must return pack metadata, lifecycle, service mapping, command
schemas, permission scopes, policy templates, examples, availability, health,
provider class, compatibility, diagnostics, and documentation links.

The required developer guide at `docs/developer-packs/commerce/receipt.md` must
cover:

- Manifest declaration and permission scopes.
- Provider/schema discovery and unavailable diagnostics.
- DTO reference for receipt scopes, source references, receipt records, lines,
  adjustments, totals, audiences, variants, delivery requests, delivery states,
  verification results, correction references, event references, and artifacts.
- Examples for planning/issuing receipts, reissuing receipts, reading/searching,
  syncing source metadata, verifying receipt evidence, requesting delivery,
  checking delivery status, linking correction references, and exporting audit
  evidence.
- Provider replacement, mock/unavailable provider behavior, trace/audit
  interpretation, redaction guarantees, idempotency, artifact retention, and
  boundaries with payment intent, order, invoice, refund, settlement,
  entitlement, communication, and checkout UI.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while hiding providers.
- **Command**: every receipt operation is a typed command/result DTO.
- **Strategy**: Stripe-like, Square-like, Adyen-like, PayPal-like,
  Braintree-like, Shopify-like, and other providers adapt behind one service
  contract.
- **Decorator**: trace, policy, entitlement, approval, resource, idempotency,
  metering, artifact bounds, and redaction wrap every call.
- **State**: issue, reissue, delivery, verification, correction-reference,
  source-sync, audit-export, and provider-health lifecycles are explicit states.
- **Specification**: admission validates declarations, scopes, source
  references, artifact formats, delivery channels, verification modes, and
  resource limits.
- **Observer**: trace, audit, provider, delivery, verification, event-reference,
  and snapshot events are subscribable.
- **Memento**: effective capability reports, receipt artifacts, delivery
  attempts, verification evidence, correction references, event references, and
  export handles are replayable bounded records.
- **Abstract Factory**: providers register only through approved runtime-host or
  plugin composition roots.

## Risks And Mitigations

- Risk: receipt pack becomes a second payment/refund/invoice surface.
  Mitigation: receipt commands carry references only and return `unsupported`
  for adjacent side effects.
- Risk: receipt artifacts leak PII or card/network data. Mitigation: artifact
  handles, redaction profiles, bounded metadata, and redaction tests are
  mandatory; raw receipt bodies are not stored in traces or snapshots.
- Risk: delivery logic becomes a communication workflow. Mitigation: this pack
  owns receipt delivery request/state only; email/SMS/push channel providers
  remain communication packs or provider adapters.
- Risk: provider-specific template behavior enters OS layers. Mitigation:
  template/branding policy stays provider-side or application-side and is
  represented only as opaque template references.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only
  build canonical service-call commands and no-direct-provider-call gates cover
  every command.
