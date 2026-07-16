# Change: Add Commerce Receipt Pack

## Why

Macaca applications need `pack.commerce.receipt.v1` as an industrial receipt
capability for issuing, retrieving, verifying, reissuing, voiding, referencing,
and exporting transaction receipt evidence. Mature commerce providers expose
receipt behavior in inconsistent places: payment charges, point-of-sale terminal
responses, order transactions, invoice pages, email receipt settings, hosted
receipt URLs, or provider-specific printable payloads. Macaca must normalize
those surfaces without turning receipts into payment execution, refund
execution, invoice generation, settlement, entitlement provisioning, or
application checkout UI.

This proposal defines receipts as a serviceized, provider-neutral pack. It gives
applications typed receipt commands and auditable evidence while keeping
provider adapters, delivery channels, printable formats, compliance metadata,
and unavailable behavior behind replaceable service providers.

## Supplier And API Baseline

The design is based on mature commerce and payment APIs:

- Stripe exposes receipt behavior through receipts for successful payments,
  charge `receipt_url`, email receipts, refund receipt behavior, paid invoice
  receipts, and Terminal receipt data for in-person payments.
- Square exposes `receipt_url` on payment/tender resources and supports Terminal
  receipt actions for issuing or printing receipts for existing payments and
  cash payments.
- Adyen Terminal API returns `PaymentReceipt` data with shopper and cashier
  receipt variants, and Adyen digital-receipt guidance expects integrators to
  generate and deliver receipts from terminal payment responses.
- PayPal Orders and Payments APIs expose order, authorization, capture, refund,
  and transaction details; receipt-like evidence is derived from capture and
  transaction records rather than a single universal receipt endpoint.
- Braintree can send email receipts for transactions and refunds submitted for
  settlement and exposes transaction response records with payment and settlement
  details.
- Shopify order and transaction APIs expose order, transaction, payment, refund,
  and POS receipt context, while POS receipt management includes email, SMS,
  print, gift receipt, and line-count constraints.

The common denominator is not "a provider receipt object." It is a receipt
evidence record tied to payment/order/source references, line/tax/total
snapshots, receipt URL or artifact handles, delivery state, reissue history,
verification status, refund/void references, compliance metadata, and sanitized
audit evidence.

Research references:

- Stripe receipts and charge `receipt_url`: https://docs.stripe.com/receipts and
  https://docs.stripe.com/api/charges/object
- Stripe Terminal receipts: https://docs.stripe.com/terminal/features/receipts
- Square Payment object and Terminal receipt actions:
  https://developer.squareup.com/reference/square/objects/Payment and
  https://developer.squareup.com/docs/terminal-api/advanced-features/issue-receipts
- Adyen receipt data and digital receipts:
  https://docs.adyen.com/point-of-sale/basic-tapi-integration/generate-receipts
  and https://docs.adyen.com/unified-commerce/digital-receipts
- PayPal Orders and Payments:
  https://developer.paypal.com/docs/api/orders/v2/ and
  https://developer.paypal.com/docs/api/payments/v2/
- Braintree email receipts and transaction records:
  https://developer.paypal.com/braintree/articles/control-panel/transactions/email-receipts
  and https://developer.paypal.com/braintree/docs/reference/response/transaction/ruby/
- Shopify orders, transactions, and POS receipt management:
  https://shopify.dev/docs/api/admin-rest/latest/resources/order,
  https://shopify.dev/docs/api/admin-graphql/latest/objects/OrderTransaction,
  and https://help.shopify.com/en/manual/sell-in-person/shopify-pos/receipt-management/managing-receipts

## Macaca Provider-Neutral Mapping

`pack.commerce.receipt.v1` maps supplier concepts into stable Macaca contracts:

- Provider charge receipt URLs, payment receipt URLs, POS receipt data,
  transaction details, order transaction receipts, and invoice receipt links
  become `ReceiptRecord` or `ReceiptArtifactHandle`.
- Provider receipt line items, tax, duty, discount, fee, gratuity, service
  charge, shipping, and total fields become `ReceiptLine`, `ReceiptAdjustment`,
  and `ReceiptTotals` snapshots.
- Customer-facing and merchant-facing receipt variants become
  `ReceiptAudience` and `ReceiptVariant` values.
- Email, SMS, print, hosted URL, PDF, JSON, and artifact retrieval become
  `ReceiptDeliveryRequest`, `ReceiptDeliveryState`, and artifact handles.
- Refund, cancellation, void, and chargeback-related receipt records become
  `ReceiptCorrectionReference` values; refund execution remains outside this
  pack.
- Provider verification, source freshness, signature/hash checks, and
  transaction reconciliation references become `ReceiptVerificationResult`.
- Provider webhook/event references become `ReceiptEventReference` records with
  bounded metadata and replay pointers.

## What Changes

- Add provider-neutral `pack.commerce.receipt.v1` under the commerce family.
- Define commands for provider inspection, schema discovery, receipt planning,
  receipt issue/reissue, read/search, source sync, verification, delivery
  planning and request, delivery status, correction/reference linking, audit
  export, and artifact retrieval.
- Define DTOs for receipt scope, provider capability, receipt records, source
  references, audiences, variants, line snapshots, totals, delivery state,
  verification, correction references, event references, freshness,
  attribution, redaction, and artifact handles.
- Require policy, entitlement, resource bounds, approval for retained/external
  delivery, idempotency for mutating commands, source-state validation, bounded
  receipt artifact handling, and sanitized trace/audit evidence.
- Require detailed developer documentation at
  `docs/developer-packs/commerce/receipt.md`.

## Impact

- Affected specs: `pack-commerce-receipt`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, receipt service providers, mock/unavailable
  providers, trace/audit schemas, replay tests, redaction tests, artifact tests,
  and boundary gates.

## Non-Goals

- No payment authorization/capture, refund execution, dispute management,
  settlement reconciliation, payout, invoice issuance, entitlement
  provisioning, tax filing, carrier fulfillment, or application checkout UI.
- No provider-specific receipt templates, merchant branding rules, delivery
  preference logic, tax/legal advice, payment-method routing, or business policy
  hardcoded into Macaca OS layers.
- No raw payment credentials, full buyer PII, raw provider payloads, private
  keys, signatures, webhook bodies, receipt HTML bodies, printable blobs, or
  unbounded exports in logs, traces, snapshots, or SDK diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
