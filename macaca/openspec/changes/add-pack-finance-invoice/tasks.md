## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, umbrella catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API findings for Stripe Invoicing, QuickBooks Online, Xero Accounting, FreshBooks, Zoho Books, Square Invoices, and similar SMB invoicing APIs.
- [x] 1.3 Confirm the pack scope: schema discovery, party/item references, draft planning/creation, invoice read/list, issue/finalize, delivery, payment-status sync, reminders, void/cancel, export, artifacts, freshness, attribution, and redaction.
- [x] 1.4 Explicitly exclude payment processing, payment-intent creation, settlement, refunds, chargebacks, tax filing, revenue recognition, subscription billing orchestration, collections strategy, templates, and application-specific billing workflow.
- [x] 1.5 Inventory existing descriptors, SDK clients, service runtime hooks, policy gates, recipient gates, resource gates, entitlement gates, trace/audit helpers, artifact handles, mock providers, and unavailable providers that can be reused.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define descriptor metadata for `pack.finance.invoice.v1`, including family, lifecycle, stability, command schemas, scopes, policy template, resource budgets, redaction profile, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define `InvoiceScope`, `InvoiceProviderCapability`, `InvoiceFreshness`, `InvoiceAttribution`, and `InvoiceRedactionPolicy`.
- [x] 2.3 Define `InvoicePartyReference`, `InvoiceItemReference`, tax identifier references, billing/shipping references, and consent/redaction classes.
- [x] 2.4 Define `InvoiceLine`, `InvoiceTaxReference`, `InvoiceDiscount`, `InvoiceAdjustment`, service-period fields, quantity/unit metadata, and rounding evidence.
- [x] 2.5 Define `InvoiceTotals`, currency precision, amount due, amount paid, amount remaining, tax/discount totals, and validation diagnostics.
- [x] 2.6 Define `InvoiceDraftPlan`, `InvoiceRecord`, `InvoiceLifecycleState`, `InvoiceDeliveryState`, `InvoicePaymentStatus`, provider concurrency token, and lifecycle evidence.
- [x] 2.7 Define `InvoiceReminderPlan`, `InvoiceReminderResult`, recipient policy metadata, delivery channel, cadence, eligibility, and provider support fields.
- [x] 2.8 Define `InvoiceArtifactPlan`, `InvoiceArtifactHandle`, export format, checksum, expiry, retention, redaction, and access policy.
- [x] 2.9 Define typed `success`, `partial`, `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, `stale_data`, and `failure` result envelopes for every command family.
- [x] 2.10 Add descriptor hashing and compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema evolution.

## 3. Command Surface And Lifecycle Semantics

- [x] 3.1 Implement command schemas for `invoice.inspect_provider`, `invoice.describe_schema`, `invoice.list_parties`, and `invoice.list_items`.
- [x] 3.2 Implement command schemas for `invoice.plan_invoice`, `invoice.create_draft`, `invoice.list_invoices`, and `invoice.read_invoice`.
- [x] 3.3 Implement command schemas for `invoice.plan_issue` and `invoice.issue_invoice` with lifecycle transition validation.
- [x] 3.4 Implement command schemas for `invoice.plan_delivery` and `invoice.send_invoice` with external-recipient policy and approval.
- [x] 3.5 Implement command schemas for `invoice.sync_payment_status` without payment collection or settlement.
- [x] 3.6 Implement command schemas for `invoice.plan_reminder` and `invoice.send_reminder` with provider capability and cadence checks.
- [x] 3.7 Implement command schemas for `invoice.plan_void` and `invoice.void_invoice` with provider lifecycle and accounting constraints.
- [x] 3.8 Implement command schemas for `invoice.plan_export`, `invoice.export_invoice`, and `invoice.get_artifact_handle`.
- [x] 3.9 Add validation for required parties, line totals, tax references, currency precision, lifecycle state, provider concurrency tokens, idempotency keys, recipient policy, pagination, async jobs, and bounded output.

## 4. Permission, Policy, Resource, Entitlement, Recipient Gates, And Approval

- [x] 4.1 Add declaration validation for `finance.invoice.read`, `finance.invoice.write`, `finance.invoice.issue`, `finance.invoice.deliver`, `finance.invoice.remind`, and `finance.invoice.export`.
- [ ] 4.2 Require policy decisions before every command and approval before draft creation, issuing, delivery, reminder sending, voiding, and retained exports.
- [ ] 4.3 Require recipient policy checks before `invoice.send_invoice` and `invoice.send_reminder`.
- [ ] 4.4 Require entitlement checks for provider access, write support, delivery support, reminder support, export support, and accounting entity access.
- [ ] 4.5 Reserve and meter resources for invoice search, export size, retained artifacts, provider quotas, network delivery, storage, and snapshots.
- [ ] 4.6 Add tests proving denied, unavailable, unsupported, conflict, quota, and stale-data paths do not call concrete providers when preconditions fail.

## 5. Service Provider, Provider Strategy, And Unavailable Behavior

- [ ] 5.1 Add the invoice service provider interface with descriptor, lifecycle, health, snapshot, shutdown, timeout, cancellation, and command dispatch.
- [ ] 5.2 Implement provider Strategy adapters behind the service interface without provider-name routing in OS-layer command logic.
- [ ] 5.3 Implement a mock provider with synthetic parties, items, invoices, lifecycle transitions, reminder support gaps, PDF/export handles, and stale-data states.
- [x] 5.4 Implement an unavailable provider that returns explicit unavailable diagnostics for every command without fake success.
- [ ] 5.5 Normalize provider errors into Macaca result envelopes while preserving sanitized provider class, bounded code, retriable flag, lifecycle state, freshness, and replay pointer.
- [ ] 5.6 Add provider capability discovery for lifecycle transitions, tax/discount support, numbering constraints, delivery/reminder support, export formats, payment-status support, freshness, and attribution.

## 6. SDK, Admission, Examples, And Developer Documentation

- [x] 6.1 Extend pack catalog and SDK discovery for `pack.finance.invoice.v1` with schemas, scopes, examples, availability, health, diagnostics, compatibility, provider class, and docs metadata.
- [ ] 6.2 Extend application admission so required declarations block on unavailable/denied states and optional declarations degrade explicitly with effective capability mementos.
- [x] 6.3 Add SDK command helper builders that only construct canonical traced service calls and never construct providers.
- [x] 6.4 Add generic app-facing examples for planning/creating drafts, issuing, sending, syncing payment status, planning reminders, exporting, and handling unsupported reminder providers.
- [x] 6.5 Create `docs/developer-packs/finance/invoice.md` with purpose, manifest declaration, scopes, commands, DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction, external-recipient policy, lifecycle semantics, and payment-boundary notes.
- [x] 6.6 Cross-link the developer guide from SDK discovery metadata and the industrial pack catalog index.

## 7. Trace, Audit, Replay, And Redaction

- [ ] 7.1 Emit sanitized declaration, admission, provider-inspection, policy, recipient-gate, entitlement, approval, resource, service-call, side-effect-planning, external-delivery, unavailable, health, snapshot, and result events.
- [ ] 7.2 Add trace schemas for `invoice_pack_declared`, `invoice_pack_admission_validated`, `invoice_pack_policy_decision`, `invoice_pack_provider_inspected`, `invoice_pack_service_call_requested`, `invoice_pack_service_call_succeeded`, `invoice_pack_service_call_failed`, `invoice_pack_side_effect_planned`, `invoice_pack_external_delivery_requested`, `invoice_pack_unavailable`, and `invoice_pack_snapshot_recorded`.
- [ ] 7.3 Add replay tests proving every command is trace-addressable through the canonical service runtime path.
- [ ] 7.4 Add snapshot tests proving descriptor, provider health, command availability, lifecycle/reminder/export support, policy-template hash, redaction profile, resource counters, freshness, and replay pointers are retained without raw payload leakage.
- [ ] 7.5 Add redaction tests proving raw PII, payment credentials, tax identifiers, hosted URLs with secrets, invoice PDFs, raw provider payloads, full invoice lines, and unbounded export data never enter logs, traces, snapshots, or SDK diagnostics.

## 8. Boundary, Quality, And Validation Gates

- [ ] 8.1 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete invoice providers.
- [x] 8.2 Add no-direct-provider-call tests proving all callable operations traverse descriptor-owned service registration and typed service commands.
- [ ] 8.3 Add canonical execution-path tests covering read-only, planning, mutation, external delivery, export, denied, unavailable, unsupported, conflict, quota, and stale-data paths.
- [ ] 8.4 Add provider replacement tests for built-in, plugin, remote, mock, and unavailable providers.
- [ ] 8.5 Add file-size and module-ownership checks for any new implementation files.
- [x] 8.6 Run `openspec validate add-pack-finance-invoice --strict`.
- [ ] 8.7 Run targeted cargo checks/tests, dependency-boundary gates, audit replay checks, recipient-policy checks, and redaction checks before marking implementation tasks complete.
