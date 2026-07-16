## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, the industrial catalog umbrella proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API findings for QuickBooks Online, Xero, NetSuite SuiteTalk REST, Sage Accounting/Sage Active, Odoo Accounting, and adjacent bank-data APIs, including unsupported fields, scope gates, report differences, and mutation constraints.
- [x] 1.3 Confirm the pack scope: general-ledger accounting, chart of accounts, periods, journals, ledger entries, statement import, reconciliation, reports, audit export, artifacts, freshness, attribution, and redaction.
- [x] 1.4 Explicitly exclude payroll, invoices, payments, bank transfers, tax filing, ERP workflows, portfolio accounting, investment advice, provider chart templates, and application-specific posting workflows.
- [x] 1.5 Inventory existing descriptors, SDK clients, service runtime hooks, policy gates, resource gates, entitlement gates, trace/audit event helpers, artifact handles, mock providers, and unavailable providers that can be reused.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define the stable descriptor for `pack.finance.accounting.v1`, including family, lifecycle, stability, command schemas, scopes, policy template, resource budgets, redaction profile, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define `AccountingScope`, `AccountingProviderCapability`, `AccountingFreshness`, `AccountingAttribution`, and `AccountingRedactionPolicy`.
- [x] 2.3 Define `AccountingEntity`, `LedgerBook`, `AccountingPeriod`, and period-lock/close-state DTOs.
- [x] 2.4 Define `ChartOfAccounts`, `AccountHandle`, `AccountClass`, `AccountingDimension`, and provider concurrency-token metadata.
- [x] 2.5 Define `AccountMutationPlan` and account mutation result DTOs for create, update, deactivate, deny, conflict, unsupported, unavailable, and failure outcomes.
- [x] 2.6 Define `JournalEntryPlan`, `JournalEntry`, `JournalLine`, `LedgerEntry`, reversal references, source references, tax-code references, dimensions, idempotency keys, posting evidence, and immutable status metadata.
- [x] 2.7 Define `StatementLine`, `ReconciliationCandidate`, `ReconciliationPlan`, `ReconciliationResult`, conflict reasons, confidence metadata, and applied-action evidence.
- [x] 2.8 Define `AccountingReportRequest`, `TrialBalanceReport`, `BalanceSheetReport`, `ProfitLossReport`, `CashFlowReport`, report basis, period range, dimensions, currency, freshness, pagination/async metadata, and provider attribution.
- [x] 2.9 Define `AuditExportPlan`, `AuditExportResult`, `AccountingArtifactHandle`, export format, retention policy, checksum, access policy, and redaction profile.
- [x] 2.10 Define typed `success`, `partial`, `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, `stale_data`, and `failure` result envelopes for every command family.
- [x] 2.11 Add descriptor hash and compatibility tests proving stable schema evolution and rejected incompatible descriptors.

## 3. Command Surface And Validation Rules

- [x] 3.1 Implement command schemas for `accounting.inspect_provider`, `accounting.list_entities`, `accounting.inspect_period`, `accounting.get_chart_of_accounts`, and `accounting.get_account`.
- [x] 3.2 Implement command schemas for `accounting.plan_account` and `accounting.account_request`, including no-side-effect planning and idempotent mutation requests.
- [x] 3.3 Implement command schemas for `accounting.plan_journal`, `accounting.post_journal`, `accounting.list_journal_entries`, and `accounting.get_ledger_entries`.
- [x] 3.4 Implement command schemas for `accounting.import_statement_lines`, `accounting.plan_reconciliation`, and `accounting.reconciliation_request`.
- [x] 3.5 Implement command schemas for `accounting.generate_trial_balance`, `accounting.generate_balance_sheet`, `accounting.generate_profit_loss`, and `accounting.generate_cash_flow`.
- [x] 3.6 Implement command schemas for `accounting.plan_audit_export`, `accounting.audit_export_request`, and `accounting.get_artifact_handle`.
- [x] 3.7 Add validation for balanced debit/credit totals per currency, currency precision, account active state, required dimensions, tax-code reference format, period locks, provider write support, provider report support, and idempotency keys.
- [x] 3.8 Add pagination, cursor, async-job, timeout, cancellation, and bounded-output rules for ledger, report, and export commands.

## 4. Permission, Policy, Resource, Entitlement, And Approval

- [x] 4.1 Add declaration validation for `finance.accounting.read`, `finance.accounting.write`, `finance.accounting.reconcile`, `finance.accounting.report`, and `finance.accounting.audit_export`.
- [x] 4.2 Require policy decisions before every command and require approval before account mutation, journal posting, statement import, reconciliation application, and audit export.
- [x] 4.3 Reserve and meter resources for provider calls, report generation, ledger pagination, export size, retained artifacts, network quota, storage, and async jobs.
- [x] 4.4 Add entitlement checks for provider access, write support, report support, export support, and tenant/accounting-entity access.
- [x] 4.5 Return typed denied/unavailable/unsupported/quota/conflict/stale-data outcomes before provider invocation when preconditions fail.
- [ ] 4.6 Add tests proving denied, unavailable, unsupported, and quota paths do not call concrete providers.

## 5. Service Provider, Provider Strategy, And Unavailable Behavior

- [ ] 5.1 Add the accounting service provider interface with descriptor, lifecycle, health, snapshot, shutdown, timeout, cancellation, and command dispatch.
- [ ] 5.2 Implement provider Strategy adapters behind the service interface; provider selection must be descriptor-driven and must not branch on provider names in OS-layer command logic.
- [ ] 5.3 Implement a mock provider for deterministic tests with synthetic accounting data and configurable capability gaps.
- [x] 5.4 Implement an unavailable provider that returns explicit unavailable diagnostics for every command without fake success.
- [ ] 5.5 Normalize provider errors into Macaca result envelopes while preserving sanitized provider class, bounded code, retriable flag, and replay pointer.
- [ ] 5.6 Add provider capability discovery for period locks, write support, report support, export support, attachments/artifacts, dimensions, tax references, and async operations.

## 6. SDK, Admission, Examples, And Developer Documentation

- [x] 6.1 Extend pack catalog and SDK discovery for `pack.finance.accounting.v1` with command schemas, scopes, examples, availability, health, diagnostics, compatibility, provider class, and docs metadata.
- [ ] 6.2 Extend application admission so required declarations block on unavailable/denied states and optional declarations degrade explicitly with effective capability mementos.
- [x] 6.3 Add SDK command helper builders that only construct canonical traced service calls and never construct providers.
- [x] 6.4 Add generic app-facing examples for reading chart of accounts, planning/posting a balanced journal, generating a report, planning reconciliation, and handling unavailable diagnostics.
- [x] 6.5 Create `docs/developer-packs/finance/accounting.md` with purpose, manifest declaration, scopes, commands, DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction, and accounting safety constraints.
- [x] 6.6 Cross-link the developer guide from SDK discovery metadata and the industrial pack catalog index.

## 7. Trace, Audit, Replay, And Redaction

- [ ] 7.1 Emit sanitized declaration, admission, provider-inspection, policy, entitlement, approval, resource, service-call, health, snapshot, side-effect-planning, side-effect-approval, unavailable, and result events.
- [ ] 7.2 Add trace schemas for `accounting_pack_declared`, `accounting_pack_admission_validated`, `accounting_pack_policy_decision`, `accounting_pack_provider_inspected`, `accounting_pack_service_call_requested`, `accounting_pack_service_call_succeeded`, `accounting_pack_service_call_failed`, `accounting_pack_side_effect_planned`, `accounting_pack_side_effect_approved`, `accounting_pack_unavailable`, and `accounting_pack_snapshot_recorded`.
- [ ] 7.3 Add replay tests proving every command is trace-addressable through the canonical service runtime path.
- [ ] 7.4 Add snapshot tests proving descriptors, health, command availability, policy-template hash, resource counters, freshness, and replay pointers are retained without raw payload leakage.
- [ ] 7.5 Add redaction tests proving credentials, account numbers, tax identifiers, attachments, raw ledgers, raw provider payloads, and unbounded report rows never enter logs, traces, snapshots, or SDK diagnostics.

## 8. Boundary, Quality, And Validation Gates

- [ ] 8.1 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete accounting providers.
- [x] 8.2 Add no-direct-provider-call tests proving all callable operations traverse descriptor-owned service registration and typed service commands.
- [ ] 8.3 Add canonical execution-path tests covering read-only, planning, mutating, report, export, denied, unavailable, unsupported, conflict, quota, and stale-data paths.
- [ ] 8.4 Add provider replacement tests for built-in, plugin, remote, mock, and unavailable providers.
- [x] 8.5 Add file-size and module-ownership checks for any new implementation files.
- [x] 8.6 Run `openspec validate add-pack-finance-accounting --strict`.
- [ ] 8.7 Run targeted cargo checks/tests, dependency-boundary gates, audit replay checks, and redaction checks before marking the implementation tasks complete.
