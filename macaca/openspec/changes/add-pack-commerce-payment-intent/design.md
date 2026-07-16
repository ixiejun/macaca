# Commerce Payment Intent Pack Design

## Context

`pack.commerce.payment.intent.v1` is Macaca's provider-neutral capability for
stateful payment attempts. It owns payment-intent records, authorization/capture
state, cancellation/void state, action requirements, status sync, idempotency,
and sanitized event/audit evidence. It does not own raw credential collection,
refunds, receipts, disputes, settlement reconciliation, payouts, fraud
decisioning, or application checkout UI.

Payment APIs are provider-specific and compliance-sensitive. The serviceized pack
keeps adapters behind Strategy providers and exposes only typed Macaca DTOs with
policy, approval, resource, entitlement, and redaction decorators.

## Supplier Capability Matrix

| Supplier family | Relevant capabilities | Constraints Macaca must model |
| --- | --- | --- |
| Stripe PaymentIntents | Create, confirm, next action, requires capture, capture, cancel, status lifecycle, idempotency | SCA/3DS actions, client-secret redaction, manual capture, cancellation state constraints, async webhooks |
| Adyen Payments/Checkout | Authorization, manual/automatic capture, cancel, cancel-or-refund, payment methods, PSP references, asynchronous notifications | Capture mode, webhook authority, cancel/refund ambiguity, PSP references, payment method and region support |
| PayPal Orders/Payments | Create/approve orders, authorize, capture, void authorizations, show authorization/capture | Orders/Payments split, buyer approval, authorization validity window, capture/void state constraints |
| Braintree Transactions | Sale/authorization, submit for settlement, void, status transitions, partial settlement | Transaction status constraints, settlement vs capture semantics, vault token boundaries |
| Square Payments | Create payment, autocomplete/capture behavior, cancel, payment source token, order linkage, idempotency | Tokenized source only, autocomplete semantics, order linkage, refund outside this pack |

## Goals

- Provide provider inspection, payment-method capability discovery, intent
  planning, intent creation, confirmation/authorization, action inspection,
  capture planning/capture, cancellation/void planning, status sync, idempotency
  inspection, event reference, audit export, and artifact handles.
- Preserve payment state machine correctness, authorization/capture constraints,
  idempotency, amount/currency precision, asynchronous event freshness, and
  approval evidence.
- Reject raw payment credentials at the DTO boundary and keep sensitive provider
  payloads out of SDK diagnostics, traces, and snapshots.
- Keep refunds, receipts, disputes, settlement, payouts, entitlement, and
  checkout UI as separate capabilities.

## Non-Goals

- Raw card/bank data collection, PCI vaulting, refunds, chargebacks/disputes,
  payouts, settlement reconciliation, receipts, entitlement provisioning, fraud
  decisions, tax handling, or application checkout workflow.
- Provider-specific routing, payment-method preference logic, fee policy,
  currency conversion policy, or risk scoring in OS layers.
- Provider secrets, raw webhook bodies, SCA payloads, client secrets, wallet
  cryptograms, or unbounded provider payloads in observability.

## Ownership And Boundaries

- Pack id: `pack.commerce.payment.intent.v1`.
- Family: `commerce`.
- Backing service owner: payment-intent service provider family.
- SDK surface: `sdk.packs.commerce.payment.intent`.
- Command namespace: `payment_intent.*`.
- Kernel ownership: identity, service-call evidence, policy facade, trace/audit
  primitives, and resource primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective-capability mementos.
- Runtime-host ownership: provider registration, decorators, mock/unavailable
  providers, and adapter composition through approved composition roots.
- Service ownership: capability discovery, state-machine validation, provider
  Strategy dispatch, event normalization, redaction, and sanitized audit.

## Command Surface

| Command | Purpose | Side-effect class |
| --- | --- | --- |
| `payment_intent.inspect_provider` | Return provider capability, payment method support, state support, capture/cancel support, freshness, and attribution | Read-only |
| `payment_intent.describe_schema` | Return amount, payment-method, action, status, idempotency, and event schema | Read-only |
| `payment_intent.plan_intent` | Validate amount, currency, merchant, order/cart references, payment method reference, capture mode, and policy | Planning |
| `payment_intent.create_intent` | Create a provider payment intent/transaction/order-payment record | Mutating |
| `payment_intent.plan_confirmation` | Validate confirmation/authorization requirements and action expectations | Planning |
| `payment_intent.confirm` | Confirm or authorize the intent through approved path | Mutating |
| `payment_intent.inspect_action` | Return next-action or customer-action handle without raw sensitive payloads | Read-only |
| `payment_intent.plan_capture` | Validate capture amount, authorization state, expiry, and provider support | Planning |
| `payment_intent.capture` | Capture an authorized payment intent through approved path | Mutating |
| `payment_intent.plan_cancellation` | Validate cancellation or void eligibility | Planning |
| `payment_intent.cancel` | Cancel or void an uncaptured/unsettled intent when provider supports it | Mutating |
| `payment_intent.get_status` | Refresh payment intent state and references | Read-only or sync |
| `payment_intent.inspect_idempotency` | Inspect idempotency outcome and replay evidence | Read-only |
| `payment_intent.record_event_reference` | Normalize provider event metadata without storing raw webhook body | Mutating metadata |
| `payment_intent.plan_audit_export` | Plan audit export scope, format, redaction, and retention | Planning |
| `payment_intent.audit_export_request` | Produce audit artifact handle | Mutating/export |
| `payment_intent.get_artifact_handle` | Retrieve artifact metadata without raw payload leakage | Read-only |

Every command must define typed command DTOs, success DTOs, partial/async shapes,
denied/unavailable/unsupported/conflict/quota/stale-data/failure results,
idempotency for side effects, redaction policy, and replay metadata.

## Provider-Neutral DTO Model

- `PaymentIntentScope`: application, tenant, session, task, merchant account,
  order/cart reference, payment intent handle, customer/session reference, and
  permission scope.
- `PaymentIntentProviderCapability`: payment method support, capture modes,
  action/redirect support, async event support, cancel/void support, partial
  capture support, idempotency model, status freshness, limits, attribution, and
  entitlement.
- `PaymentIntentPlan`, `PaymentIntentRecord`: amount, currency, capture mode,
  merchant, order/cart reference, customer/session reference, payment-method
  reference, state, action requirements, authorization/capture/cancel references,
  idempotency key, provider version/status, freshness, and redaction class.
- `PaymentMethodReference`: tokenized method handle, type class, region support,
  reusable flag, risk/eligibility metadata, and redaction policy; it must not
  carry raw PAN, CVV, bank credentials, wallet cryptograms, or provider secrets.
- `PaymentActionRequirement`: redirect/action handle, action type, expiry,
  client-facing metadata boundary, return/callback reference, and redaction.
- `PaymentAuthorization`, `PaymentCapture`, `PaymentCancellation`: amount,
  currency, provider reference, state, expiry, partial capture metadata, and
  side-effect evidence.
- `PaymentIntentEventReference`: event type, provider class, event timestamp,
  webhook freshness, delivery id hash, replay pointer, and bounded result code.
- `PaymentIntentAuditExportPlan`, `PaymentIntentArtifactHandle`: export scope,
  format, checksum, expiry, retention, access policy, and redaction.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `commerce.payment.intent.read`
- `commerce.payment.intent.create`
- `commerce.payment.intent.confirm`
- `commerce.payment.intent.capture`
- `commerce.payment.intent.cancel`
- `commerce.payment.intent.audit_export`

Policy defaults:

- Scope every call to application id, tenant id, session id, task id, trace id,
  merchant account, order/cart reference, and payment intent handle.
- Require approval for create, confirm/authorize, capture, cancel/void, and
  retained audit exports.
- Require idempotency keys for all side-effect commands.
- Require tokenized payment method references only; raw credentials are denied
  before service calls.
- Validate state transitions, amount/currency precision, capture amount, capture
  expiry, cancellation eligibility, provider support, and event freshness.
- Return typed `denied`, `unavailable`, `unsupported`, `conflict`,
  `quota_exceeded`, `stale_data`, or `failure` before provider calls when
  preconditions fail.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `payment_intent_pack_declared`
- `payment_intent_pack_admission_validated`
- `payment_intent_pack_policy_decision`
- `payment_intent_pack_provider_inspected`
- `payment_intent_pack_service_call_requested`
- `payment_intent_pack_service_call_succeeded`
- `payment_intent_pack_service_call_failed`
- `payment_intent_pack_state_transition_planned`
- `payment_intent_pack_sensitive_input_rejected`
- `payment_intent_pack_unavailable`
- `payment_intent_pack_snapshot_recorded`

Events include pack id, command name, trace id, application/session/task/tenant
identifiers, payment intent/order/cart handles, state transition, policy
decision, provider class, descriptor hash, latency, freshness, idempotency hash,
bounded resource counters, result code, and sanitized artifact references.
Events must exclude raw payment credentials, client secrets, raw provider
payloads, SCA payloads, wallet cryptograms, webhook bodies, private keys,
signatures, and unbounded output.

Snapshots include descriptor version, provider health, command availability,
state/capture/cancel/event support, policy-template hash, redaction profile,
freshness, resource counters, and replay pointers.

## SDK And Developer Documentation

SDK discovery must return pack metadata, lifecycle, service mapping, command
schemas, permission scopes, policy templates, examples, availability, health,
provider class, compatibility, diagnostics, and documentation links.

The required developer guide at
`docs/developer-packs/commerce/payment-intent.md` must cover:

- Manifest declaration and permission scopes.
- Provider/schema discovery and unavailable diagnostics.
- DTO reference for intent plans, payment method references, action
  requirements, authorizations, captures, cancellations, status, idempotency,
  event references, and audit artifacts.
- Examples for planning/creating intents, confirming/authorizing, handling
  action-required state, capturing, canceling/voiding, syncing status, inspecting
  idempotency, and handling conflicts.
- Provider replacement, mock/unavailable provider behavior, trace/audit
  interpretation, redaction guarantees, and boundaries with order, receipt,
  refund, settlement, dispute, entitlement, and checkout UI.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while hiding providers.
- **Command**: every payment-intent operation is a typed command/result DTO.
- **Strategy**: Stripe-like, Adyen-like, PayPal-like, Braintree-like,
  Square-like, and other gateway providers adapt behind one service contract.
- **Decorator**: trace, policy, entitlement, approval, resource, idempotency,
  metering, and redaction wrap every call.
- **State**: payment intent lifecycle, action-required state, authorization,
  capture, cancellation, event sync, audit export, and provider health are
  explicit states.
- **Specification**: admission validates declarations, scopes, payment-method
  tokens, state transitions, capture/cancel eligibility, and resource limits.
- **Observer**: trace, audit, provider, state, webhook-reference, and snapshot
  events are subscribable.
- **Memento**: effective capability reports, idempotency evidence, state
  transition evidence, event references, and artifact handles are replayable
  bounded records.
- **Abstract Factory**: providers register only through approved runtime-host or
  plugin composition roots.

## Risks And Mitigations

- Risk: raw payment data enters OS diagnostics. Mitigation: DTOs accept only
  tokenized references; raw credential detection returns denied before service
  call; redaction tests cover traces/snapshots.
- Risk: payment pack grows into refunds/receipts/settlement. Mitigation: those
  operations are references or separate packs and are explicitly unsupported here.
- Risk: asynchronous webhooks become hidden execution paths. Mitigation: webhook
  ingestion stores bounded event references and routes reconciliation through the
  same service command/audit path.
- Risk: capture/cancel state differs by provider. Mitigation: provider
  capability discovery plus state Specification checks run before side effects.
