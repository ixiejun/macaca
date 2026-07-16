# Commerce Receipt Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.commerce.receipt.v1`. The receipt pack must expose receipt evidence
records, issue/reissue, read/search, source sync, verification, delivery request
and status, correction references, audit export, artifact handles, freshness,
attribution, and redaction through typed service commands. It must not execute
payments, collect payment methods, execute refunds, generate invoices, reconcile
settlement, provision entitlements, file tax, own carrier fulfillment, or encode
application checkout UI/workflows.

## Source Baseline

- Stripe receipts, charge receipt URLs, and Terminal receipts:
  <https://docs.stripe.com/receipts>,
  <https://docs.stripe.com/api/charges/object>, and
  <https://docs.stripe.com/terminal/features/receipts>
- Square Payment receipt URLs and Terminal receipt actions:
  <https://developer.squareup.com/reference/square/objects/Payment> and
  <https://developer.squareup.com/docs/terminal-api/advanced-features/issue-receipts>
- Adyen Terminal receipt data and digital receipts:
  <https://docs.adyen.com/point-of-sale/basic-tapi-integration/generate-receipts>
  and <https://docs.adyen.com/unified-commerce/digital-receipts>
- PayPal Orders and Payments transaction evidence:
  <https://developer.paypal.com/docs/api/orders/v2/> and
  <https://developer.paypal.com/docs/api/payments/v2/>
- Braintree email receipts and transaction response records:
  <https://developer.paypal.com/braintree/articles/control-panel/transactions/email-receipts>
  and <https://developer.paypal.com/braintree/docs/reference/response/transaction/ruby/>
- Shopify orders, transactions, and POS receipt management:
  <https://shopify.dev/docs/api/admin-rest/latest/resources/order>,
  <https://shopify.dev/docs/api/admin-graphql/latest/objects/OrderTransaction>,
  and
  <https://help.shopify.com/en/manual/sell-in-person/shopify-pos/receipt-management/managing-receipts>

## Supplier API Notes

- Stripe contributes hosted receipt URLs, email receipt settings, charge and
  invoice receipt evidence, refund receipt behavior, and Terminal receipt data.
  Macaca should model hosted URLs as redacted artifact handles and keep payment
  execution outside the receipt pack.
- Square contributes payment receipt URLs and terminal actions for issuing or
  printing receipts for existing payments or cash payments. Macaca should model
  terminal printing as approved delivery, not as payment creation.
- Adyen contributes shopper and cashier receipt variants from terminal
  responses and digital receipt guidance. Macaca should normalize audiences,
  variants, source state, and delivery boundaries.
- PayPal and Braintree contribute transaction/capture evidence and email receipt
  behavior rather than one uniform receipt object. Macaca should treat receipt
  records as normalized evidence tied to source references.
- Shopify contributes order, transaction, POS, gift receipt, email/SMS/print,
  and receipt management behavior. Macaca should keep templates and merchant
  branding outside OS semantics.

## Macaca-Owned Abstractions

`pack.commerce.receipt.v1` should define `ReceiptScope`,
`ReceiptProviderCapability`, `ReceiptFreshness`, `ReceiptAttribution`,
`ReceiptRedactionPolicy`, `ReceiptSourceReference`, `ReceiptRecord`,
`ReceiptLine`, `ReceiptAdjustment`, `ReceiptTotals`, `ReceiptAudience`,
`ReceiptVariant`, `ReceiptDeliveryRequest`, `ReceiptDeliveryState`,
`ReceiptVerificationResult`, `ReceiptCorrectionReference`,
`ReceiptEventReference`, `ReceiptAuditExportPlan`, and
`ReceiptArtifactHandle`.

The DTOs must carry source payment/order/invoice/refund/void references, receipt
numbers, audience and variant, issued timestamps, line/tax/fee/tip/total
snapshots, delivery channel, destination reference, consent/approval reference,
verification metadata, correction references, event freshness, artifact
checksums, capability hashes, redaction classes, bounded provider reason codes,
and replay pointers. Raw buyer PII, payment credentials, raw provider payloads,
raw webhook bodies, receipt HTML bodies, printable binary blobs, signatures,
private keys, and unbounded receipt exports are rejected.

## Explicit Non-Goals

- Do not implement concrete Stripe, Square, Adyen, PayPal, Braintree, Shopify,
  email/SMS, terminal-print, receipt-template, invoice, refund, tax, or
  settlement adapters in this research phase.
- Do not define payment authorization/capture, payment-method collection,
  refund execution, invoice generation, settlement reconciliation, payouts,
  disputes, entitlement provisioning, tax filing, carrier fulfillment,
  communication workflow ownership, or checkout UI semantics inside this pack.
- Do not expose provider-specific receipt templates, merchant branding rules,
  delivery marketing workflows, legal/tax advice, raw hosted receipt URLs, or
  provider-native receipt payloads as stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` already provides
  descriptor metadata, lifecycle/availability, policy templates, SDK metadata,
  diagnostics, provider snapshots, unavailable diagnostics, and effective
  capability expansion concepts that receipt descriptors can reuse.
- `crates/facade/macaca-sdk/src/system_facade.rs` and focused SDK clients
  provide the Facade pattern expected for app-facing discovery and command
  construction; receipt SDK helpers should only build canonical traced service
  calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics for optional domain-pack
  providers.
- `crates/kernel/macaca-kernel/src/policy.rs`,
  `crates/runtime/macaca-runtime-host/src/service_policy_engine.rs`,
  `crates/kernel/macaca-kernel/src/audit.rs`,
  `crates/foundation/macaca-proto/src/audit_redaction.rs`, and
  `crates/runtime/macaca-runtime-host/src/service_call_audit.rs` provide
  reusable policy, redaction, trace, and audit substrate.
- Current evidence does not prove receipt-specific DTOs, descriptors, command
  schemas, providers, SDK helpers, WASM ABI metadata, trace schemas, replay
  tests, redaction tests, dependency gates, or developer documentation.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
