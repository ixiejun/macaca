# Commerce Payment Intent Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.commerce.payment.intent.v1`. The payment-intent pack must expose intent
planning, intent creation, confirmation/authorization, next-action inspection,
capture, cancel/void, status sync, idempotency, event references, audit export,
freshness, attribution, and redaction through typed service commands. It must
not collect raw credentials, own PCI vaulting, execute refunds, issue receipts,
provision entitlements, settle payouts, decide fraud outcomes, or encode
application checkout UI/workflows.

## Source Baseline

- Stripe PaymentIntents:
  <https://docs.stripe.com/api/payment_intents>
- Adyen Checkout/Payments, captures, cancels, and webhooks:
  <https://docs.adyen.com/online-payments/build-your-integration>,
  <https://docs.adyen.com/online-payments/capture>, and
  <https://docs.adyen.com/development-resources/webhooks>
- PayPal Orders and Payments:
  <https://developer.paypal.com/docs/api/orders/v2/> and
  <https://developer.paypal.com/docs/api/payments/v2/>
- Braintree Transactions:
  <https://developer.paypal.com/braintree/docs/reference/request/transaction/sale>,
  <https://developer.paypal.com/braintree/docs/reference/request/transaction/submit-for-settlement>, and
  <https://developer.paypal.com/braintree/docs/reference/request/transaction/void>
- Square Payments:
  <https://developer.squareup.com/reference/square/payments-api>

## Supplier API Notes

- Stripe contributes stateful PaymentIntents, confirmation, action-required
  states, manual capture, cancellation, asynchronous events, and idempotency.
  Macaca should redact client secrets and represent next actions as bounded
  handles.
- Adyen contributes authorization, payment methods, manual/automatic capture,
  cancel/cancel-or-refund, PSP references, and webhook authority. Macaca should
  normalize capture and cancellation capabilities while keeping refund
  execution outside this pack.
- PayPal contributes Orders approval, authorization, capture, void, and payment
  record inspection. Macaca should treat buyer approval and authorization/capture
  state as payment-intent semantics without adopting PayPal Orders as order-pack
  behavior.
- Braintree contributes sale/authorization, submit for settlement, void, status
  transitions, partial settlement constraints, and tokenized vault references.
  Macaca should expose tokenized method references only.
- Square contributes source tokens, payment creation, autocomplete/capture
  behavior, cancel, order linkage, and idempotency keys. Macaca should reject
  raw payment credentials and keep order linkage as a reference.

## Macaca-Owned Abstractions

`pack.commerce.payment.intent.v1` should define `PaymentIntentScope`,
`PaymentIntentProviderCapability`, `PaymentIntentFreshness`,
`PaymentIntentAttribution`, `PaymentIntentRedactionPolicy`,
`PaymentIntentPlan`, `PaymentIntentRecord`, `PaymentMethodReference`,
`PaymentActionRequirement`, `PaymentAuthorization`, `PaymentCapture`,
`PaymentCancellation`, `PaymentIntentEventReference`,
`PaymentIntentAuditExportPlan`, and `PaymentIntentArtifactHandle`.

The DTOs must carry merchant/account scope, amount, currency precision, capture
mode, order/cart references, customer/session references, tokenized payment
method references, state transitions, action handles, authorization expiry,
capture evidence, cancellation evidence, idempotency keys, event freshness,
capability hashes, redaction classes, bounded provider reason codes, and replay
pointers. Raw PAN, CVV, bank credentials, wallet cryptograms, provider secrets,
client secrets, raw webhook bodies, SCA payloads, signatures, private keys, and
unbounded provider payloads are rejected.

## Explicit Non-Goals

- Do not implement concrete Stripe, Adyen, PayPal, Braintree, Square,
  fraud-engine, PCI-vault, receipt, refund, settlement, payout, tax, or checkout
  adapters in this research phase.
- Do not define raw credential collection, PCI vaulting, refunds, disputes,
  payouts, settlement reconciliation, receipts, entitlement provisioning, fraud
  decisions, tax handling, or application checkout UI/workflows inside this
  pack.
- Do not expose provider-specific routing, payment-method preference, risk
  scoring, fee policy, currency conversion policy, raw provider payloads, or
  provider-native payment objects as stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` already provides
  descriptor metadata, lifecycle/availability, policy templates, SDK metadata,
  diagnostics, provider snapshots, unavailable diagnostics, and effective
  capability expansion concepts that payment-intent descriptors can reuse.
- `crates/foundation/macaca-proto/src/payment_policy.rs` and
  `crates/runtime/macaca-runtime-host/src/payment_policy.rs` provide existing
  payment-policy vocabulary that future payment-intent descriptors must align
  with without moving gateway logic into kernel, SDK, or shells.
- `crates/facade/macaca-sdk/src/system_facade.rs` and focused SDK clients
  provide the Facade pattern expected for app-facing discovery and command
  construction; payment SDK helpers should only build canonical traced service
  calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics for optional domain-pack
  providers.
- Current evidence does not prove payment-intent-specific DTOs, descriptors,
  command schemas, providers, SDK helpers, WASM ABI metadata, trace schemas,
  replay tests, redaction tests, dependency gates, or developer documentation.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
