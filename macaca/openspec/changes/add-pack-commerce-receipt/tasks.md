## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, umbrella catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API findings for Stripe receipts/charge URLs/Terminal receipts, Square payment receipt URLs/Terminal receipt actions, Adyen Terminal PaymentReceipt/digital receipts, PayPal Orders/Payments transaction evidence, Braintree email receipts/transactions, Shopify orders/transactions/POS receipts, and similar providers.
- [x] 1.3 Confirm the pack scope: receipt evidence records, issue/reissue, read/search, source sync, verification, delivery request/status, correction references, audit export, artifact handles, freshness, attribution, and redaction.
- [x] 1.4 Explicitly exclude payment authorization/capture, payment-method collection, refund execution, invoice generation, settlement reconciliation, payouts, disputes, entitlement provisioning, tax filing, carrier fulfillment, communication workflow ownership, and checkout UI.
- [x] 1.5 Inventory existing descriptors, SDK clients, service runtime hooks, policy gates, approval gates, resource gates, entitlement gates, trace/audit helpers, artifact handles, mock providers, and unavailable providers that can be reused.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define descriptor metadata for `pack.commerce.receipt.v1`, including family, lifecycle, stability, command schemas, scopes, policy template, resource budgets, redaction profile, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define `ReceiptScope`, `ReceiptProviderCapability`, `ReceiptFreshness`, `ReceiptAttribution`, and `ReceiptRedactionPolicy`.
- [x] 2.3 Define `ReceiptSourceReference` for payment intent, capture, charge, transaction, order, invoice, refund, void, cash payment, terminal transaction, external document, and provider event references.
- [x] 2.4 Define `ReceiptRecord`, receipt handle, source references, receipt number/reference, audience, variant, issue state, issued timestamp, correction references, delivery state, artifact handles, verification state, freshness, and redaction class.
- [x] 2.5 Define `ReceiptLine`, item references, descriptions, quantities, unit amounts, source evidence, and redaction metadata.
- [x] 2.6 Define `ReceiptAdjustment` and `ReceiptTotals` for discounts, taxes, duties, fees, shipping, gratuities, service charges, currency precision, and total evidence.
- [x] 2.7 Define `ReceiptAudience` and `ReceiptVariant` for customer, merchant, cashier, gift, regulatory, refund, correction, terminal, hosted, printable, and custom provider variants.
- [x] 2.8 Define `ReceiptDeliveryRequest`, `ReceiptDeliveryState`, delivery channel, destination reference, approval/consent reference, attempt count, provider message/terminal action reference, bounded failure code, and redaction.
- [x] 2.9 Define `ReceiptVerificationResult`, source linkage, totals match, checksum/signature status, provider verification reference, freshness, conflict/stale-data details, and replay pointer.
- [x] 2.10 Define `ReceiptCorrectionReference` for refund, void, cancellation, chargeback, return, replacement, or adjustment references without adjacent side-effect payloads.
- [x] 2.11 Define `ReceiptEventReference`, provider class, event type, event timestamp, delivery id hash, webhook freshness, replay pointer, and bounded result code.
- [x] 2.12 Define `ReceiptAuditExportPlan`, `ReceiptArtifactHandle`, artifact type, hosted URL metadata, checksum, expiry, retention, redaction, access policy, and replay pointer.
- [x] 2.13 Define typed `success`, `partial`, `accepted`, `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, `stale_data`, and `failure` result envelopes for every command family.
- [x] 2.14 Add descriptor hashing and compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema evolution.

## 3. Command Surface And Receipt State Semantics

- [x] 3.1 Implement command schemas for `receipt.inspect_provider` and `receipt.describe_schema`.
- [x] 3.2 Implement command schemas for `receipt.plan_issue` and `receipt.issue_receipt`, including source-state validation and idempotency.
- [x] 3.3 Implement command schemas for `receipt.plan_reissue` and `receipt.reissue_receipt`, including original receipt state, correction references, and provider support.
- [x] 3.4 Implement command schemas for `receipt.read_receipt` and `receipt.search_receipts`, including pagination, filters, freshness, and redaction.
- [x] 3.5 Implement command schemas for `receipt.sync_source` with declared source capability checks and stale-data handling.
- [x] 3.6 Implement command schemas for `receipt.verify_receipt`, including source linkage, totals match, checksum/signature checks, artifact verification, and provider verification references.
- [x] 3.7 Implement command schemas for `receipt.plan_delivery`, `receipt.delivery_request`, and `receipt.get_delivery_status`.
- [x] 3.8 Implement command schemas for `receipt.link_correction_reference` and `receipt.list_correction_references` without refund, void, chargeback, or entitlement side effects.
- [x] 3.9 Implement command schemas for `receipt.record_event_reference` without raw webhook body storage.
- [x] 3.10 Implement command schemas for `receipt.plan_audit_export`, `receipt.audit_export_request`, and `receipt.get_artifact_handle`.
- [x] 3.11 Add validation for source reference visibility, amount/currency precision, line/totals consistency, audience/variant support, delivery channel support, artifact format support, idempotency, approval, retention, export bounds, and stale-data conditions.

## 4. Permission, Policy, Resource, Entitlement, And Approval

- [x] 4.1 Add declaration validation for `commerce.receipt.read`, `commerce.receipt.issue`, `commerce.receipt.reissue`, `commerce.receipt.verify`, `commerce.receipt.deliver`, `commerce.receipt.correction_reference`, and `commerce.receipt.audit_export`.
- [x] 4.2 Require policy decisions before every command and approval before issue/reissue with retained evidence, external delivery, host/terminal printing, and retained audit exports.
- [x] 4.3 Require entitlement checks for provider access, source type support, issue support, reissue support, delivery channel support, verification support, correction-reference support, audit export support, artifact access, and merchant/store/channel access.
- [x] 4.4 Reserve and meter resources for receipt search, source sync, verification, delivery attempts, artifact retrieval, audit export size, provider quotas, storage, and snapshots.
- [x] 4.5 Return typed denied/unavailable/unsupported/conflict/quota/stale-data outcomes before provider calls when preconditions fail.
- [x] 4.6 Add tests proving denied, unavailable, unsupported, conflict, quota, stale-data, and artifact redaction paths do not call concrete providers when preconditions fail.

## 5. Service Provider, Provider Strategy, And Unavailable Behavior

- [x] 5.1 Add the receipt service provider interface with descriptor, lifecycle, health, snapshot, shutdown, timeout, cancellation, async delivery/export support, and command dispatch.
- [x] 5.2 Implement provider Strategy adapters behind the service interface without provider-name routing in OS-layer command logic.
- [x] 5.3 Implement a mock provider with synthetic receipt records, source sync, verification results, delivery states, correction references, artifacts, stale-data states, and configurable capability gaps.
- [x] 5.4 Implement an unavailable provider that returns explicit unavailable diagnostics for every command without fake success.
- [x] 5.5 Normalize provider errors into Macaca result envelopes while preserving sanitized provider class, bounded code, retriable flag, receipt state, delivery state, verification state, freshness, idempotency hash, and replay pointer.
- [x] 5.6 Add provider capability discovery for supported source types, audiences, variants, delivery channels, artifact formats, verification modes, reissue support, correction-reference support, export support, idempotency model, freshness, limits, attribution, and entitlement.

## 6. SDK, Admission, Examples, And Developer Documentation

- [x] 6.1 Extend pack catalog and SDK discovery for `pack.commerce.receipt.v1` with schemas, scopes, examples, availability, health, diagnostics, compatibility, provider class, and docs metadata.
- [x] 6.2 Extend application admission so required declarations block on unavailable/denied states and optional declarations degrade explicitly with effective capability mementos.
- [x] 6.3 Add SDK command helper builders that only construct canonical traced service calls and never construct providers.
- [x] 6.4 Add generic app-facing examples for planning/issuing receipts, reissuing receipts, reading/searching, syncing source metadata, verifying evidence, requesting delivery, checking delivery status, linking correction references, exporting audit evidence, and handling conflicts.
- [x] 6.5 Create `docs/developer-packs/commerce/receipt.md` with purpose, manifest declaration, scopes, commands, DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction, idempotency, artifact retention, delivery semantics, verification semantics, and payment/refund/invoice/communication boundaries.
- [x] 6.6 Cross-link the developer guide from SDK discovery metadata and the industrial pack catalog index.

## 7. Trace, Audit, Replay, And Redaction

- [x] 7.1 Emit sanitized declaration, admission, provider-inspection, policy, entitlement, approval, resource, service-call, issue-planning, delivery-planning, verification, correction-reference, unavailable, health, snapshot, and result events.
- [x] 7.2 Add trace schemas for `receipt_pack_declared`, `receipt_pack_admission_validated`, `receipt_pack_policy_decision`, `receipt_pack_provider_inspected`, `receipt_pack_service_call_requested`, `receipt_pack_service_call_succeeded`, `receipt_pack_service_call_failed`, `receipt_pack_issue_planned`, `receipt_pack_delivery_planned`, `receipt_pack_verification_completed`, `receipt_pack_correction_reference_linked`, `receipt_pack_unavailable`, and `receipt_pack_snapshot_recorded`.
- [x] 7.3 Add replay tests proving every command is trace-addressable through the canonical service runtime path.
- [x] 7.4 Add snapshot tests proving descriptor, provider health, command availability, source/audience/delivery/verification/export support, policy-template hash, redaction profile, resource counters, freshness, and replay pointers are retained without raw payload leakage.
- [x] 7.5 Add redaction tests proving raw buyer PII, payment credentials, raw provider payloads, webhook bodies, receipt HTML bodies, printable binary blobs, private keys, signatures, and unbounded exports never enter logs, traces, snapshots, or SDK diagnostics.
- [x] 7.6 Add artifact-boundary tests proving hosted URLs, PDFs, JSON payloads, print data, and audit exports are represented as bounded handles or checksums in observability surfaces.

## 8. Boundary, Quality, And Validation Gates

- [x] 8.1 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete receipt providers.
- [x] 8.2 Add no-direct-provider-call tests proving all callable operations traverse descriptor-owned service registration and typed service commands.
- [x] 8.3 Add canonical execution-path tests covering read-only, issue, reissue, search, source sync, verification, delivery, correction reference, event reference, audit export, denied, unavailable, unsupported, conflict, quota, stale-data, and artifact-boundary paths.
- [x] 8.4 Add provider replacement tests for built-in, plugin, remote, mock, and unavailable providers.
- [x] 8.5 Add boundary tests proving receipt commands do not authorize/capture payments, execute refunds, issue invoices, provision entitlements, perform settlement, or own communication workflow semantics.
- [x] 8.6 Add file-size and module-ownership checks for any new implementation files.
- [x] 8.7 Run `openspec validate add-pack-commerce-receipt --strict`.
- [x] 8.8 Run targeted cargo checks/tests, dependency-boundary gates, audit replay checks, redaction checks, artifact-boundary checks, and receipt/payment/refund/invoice/communication boundary checks before marking implementation tasks complete.
