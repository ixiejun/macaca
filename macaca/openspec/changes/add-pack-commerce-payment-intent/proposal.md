# Change: Add Commerce Payment Intent Pack

## Why

Macaca applications need `pack.commerce.payment.intent.v1` as an industrial
payment-intent capability for creating, confirming/authorizing, capturing,
canceling, voiding, and inspecting payment attempts. Payment providers expose
stateful flows with strong compliance requirements: SCA/3DS next actions,
authorization windows, partial capture, idempotency, asynchronous webhooks,
multi-method routing, and strict handling of payment credentials.

This proposal defines payment intent as a serviceized, provider-neutral pack. It
lets applications request payment attempts through typed commands while keeping
raw payment credentials, gateway adapters, provider routing, receipts, refunds,
settlement, disputes, payouts, and application checkout workflows outside OS
layers.

## Supplier And API Baseline

The design is based on mature payment APIs:

- Stripe PaymentIntents guide a payment through create, confirm, action-required,
  processing, requires-capture, succeeded, canceled, and failure states, with
  capture/cancel endpoints and idempotency support.
- Adyen Payments/Checkout APIs expose authorization, manual/automatic capture,
  cancel, cancel-or-refund, asynchronous webhooks, payment methods, shopper
  interactions, and PSP references.
- PayPal Orders and Payments APIs expose create, approve, authorize, capture,
  void, and show-authorization/capture flows, with Orders acting as a payment
  abstraction and Payments owning authorization/capture records.
- Braintree Transactions expose sale, authorization, submit-for-settlement,
  void, status transitions, partial settlement constraints, and gateway
  transaction identifiers.
- Square Payments and similar gateway APIs expose payment create/capture/cancel,
  autocomplete, payment source tokens, idempotency keys, and order linkage.

The common denominator is a payment intent with amount, currency, merchant,
payment-method token reference, customer/session context, intent/action state,
authorization reference, capture reference, cancellation/void state,
idempotency, asynchronous events, and sanitized audit evidence.

## Macaca Provider-Neutral Mapping

`pack.commerce.payment.intent.v1` maps supplier concepts into stable contracts:

- Provider PaymentIntent, order-payment, transaction, or payment records become
  `PaymentIntentRecord`.
- Provider next actions, redirects, 3DS/SCA challenge requirements, and approval
  URLs become `PaymentActionRequirement` handles.
- Authorizations, holds, captures, voids, and cancellations become
  `PaymentAuthorization`, `PaymentCapture`, and `PaymentCancellation` references.
- Provider payment-method details become `PaymentMethodReference` only; raw PAN,
  CVV, bank credentials, wallet cryptograms, and full provider payloads are not
  accepted in pack DTOs.
- Provider webhooks and asynchronous events become `PaymentIntentEvent` records
  with bounded provider class, event type, freshness, and replay pointer.
- Refunds, disputes, payouts, settlement reconciliation, and receipts are
  references or separate packs, not payment-intent commands.

## What Changes

- Add provider-neutral `pack.commerce.payment.intent.v1` under the commerce
  family.
- Define commands for provider inspection, payment-method capability discovery,
  intent planning, intent creation, confirmation/authorization, action polling,
  capture planning, capture, cancellation/void planning, cancellation/void,
  status sync, idempotency inspection, event ingestion reference, and audit
  export.
- Define DTOs for payment scope, provider capability, intent records, amount,
  payment-method references, customer/session references, action requirements,
  authorization/capture/cancellation references, state machine, idempotency,
  webhook freshness, attribution, redaction, and audit artifacts.
- Require policy, entitlement, approval, idempotency, amount/currency precision,
  state-transition validation, raw credential rejection, sanitized trace/audit,
  and deterministic unavailable/unsupported behavior.
- Require detailed developer documentation at
  `docs/developer-packs/commerce/payment-intent.md`.

## Impact

- Affected specs: `pack-commerce-payment-intent`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, payment-intent service providers,
  mock/unavailable providers, trace/audit schemas, replay tests, redaction tests,
  and dependency-boundary gates.

## Non-Goals

- No raw card or bank credential collection, PCI vaulting, refund execution,
  dispute management, payout, settlement reconciliation, receipt issuance,
  entitlement provisioning, fraud decisioning, or application checkout workflow.
- No provider-specific routing, payment-method preference, risk scoring,
  currency conversion, fee policy, or merchant business rule in Macaca OS layers.
- No raw payment credentials, secrets, private keys, signatures, provider
  payloads, SCA payloads, webhook bodies, or unbounded output in logs, traces,
  snapshots, or SDK diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
