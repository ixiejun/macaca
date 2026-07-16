## ADDED Requirements

### Requirement: Macaca SHALL provide Finance Accounting as a serviceized pack

Macaca SHALL provide `pack.finance.accounting.v1` as a provider-neutral,
serviceized finance pack for general-ledger accounting operations, including
accounting entities, ledger books, periods, chart of accounts, journals, ledger
entries, statement import, reconciliation, reports, audit export, and artifact
handles.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.finance.accounting.v1` as required and the accounting service provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with descriptor hash, command schemas, permission scopes, policy template, provider capability, health, freshness, and replay metadata
- **AND** SDK discovery SHALL mark supported commands as callable without exposing provider secrets, raw provider payloads, raw ledgers, account numbers, tax identifiers, or attachments

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.finance.accounting.v1` as required but provider, permission, entitlement, policy, resource, host support, or accounting entity access is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, call another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.finance.accounting.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Finance Accounting SHALL expose provider capability discovery

`pack.finance.accounting.v1` SHALL expose provider-neutral capability discovery
for supported commands, accounting entities, ledger books, fiscal periods, write
support, period-lock visibility, report support, export support, dimensions, tax
references, freshness, attribution, and unavailable limitations.

#### Scenario: Provider capabilities are inspected
- **WHEN** an application invokes `accounting.inspect_provider` with a declared accounting scope
- **THEN** Macaca SHALL return `AccountingProviderCapability` with supported command names, unsupported command reasons, provider class, descriptor version, write/report/export capabilities, period-lock visibility, dimension support, tax-reference support, freshness, and attribution metadata
- **AND** the response SHALL use provider-neutral fields rather than provider-specific record payloads

#### Scenario: Provider does not support a command
- **WHEN** a provider descriptor exists but does not support `accounting.generate_cash_flow` or another requested command
- **THEN** SDK discovery SHALL mark that command as non-callable for the effective capability
- **AND** invoking the command SHALL return a typed `unsupported` result without provider side effects

### Requirement: Finance Accounting commands SHALL use typed canonical service calls

Every Finance Accounting operation SHALL be represented as a typed command and
result DTO, and every invocation SHALL traverse the canonical service runtime
path with trace, policy, resource, entitlement, approval when required, health,
snapshot, and structured error behavior.

#### Scenario: Read command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `accounting.get_chart_of_accounts` is invoked
- **THEN** Macaca SHALL route the command through SDK or facade helpers into the service runtime and accounting service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers

#### Scenario: Mutating command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, resource, period-lock, or provider-capability checks reject a mutating command such as `accounting.post_journal`
- **THEN** Macaca SHALL return a typed `denied`, `unavailable`, `unsupported`, `conflict`, or `quota_exceeded` result before invoking the concrete provider
- **AND** the audit trail SHALL include bounded reason codes and replay pointers without raw user data or raw provider payloads

#### Scenario: Command output is bounded
- **WHEN** a ledger, report, or audit export command could return unbounded rows or artifacts
- **THEN** Macaca SHALL require pagination, cursoring, async artifact handles, or resource limits
- **AND** traces and snapshots SHALL store only bounded counters, checksums, handles, and sanitized metadata

### Requirement: Finance Accounting SHALL enforce double-entry and period safety

Finance Accounting SHALL enforce accounting safety checks for mutating commands,
including balanced debit and credit totals per currency, currency precision,
active accounts, valid dimensions, tax-code references, idempotency keys,
provider write support, and accounting-period lock or close state.

#### Scenario: Journal plan is valid
- **WHEN** an application invokes `accounting.plan_journal` with balanced lines, active account handles, valid dimensions, valid tax-code references, supported currency precision, and an open accounting period
- **THEN** Macaca SHALL return a `JournalEntryPlan` containing normalized lines, posting preconditions, idempotency requirements, approval requirements, and sanitized validation evidence
- **AND** the planning command SHALL NOT mutate provider state

#### Scenario: Journal plan is unbalanced
- **WHEN** an application invokes `accounting.plan_journal` with debit and credit totals that do not balance for a currency
- **THEN** Macaca SHALL return a typed validation denial with the affected currency and bounded difference metadata
- **AND** Macaca SHALL NOT build or invoke a provider posting request

#### Scenario: Accounting period is locked
- **WHEN** an application attempts `accounting.account_request`, `accounting.post_journal`, `accounting.import_statement_lines`, or `accounting.reconciliation_request` against a locked or closed period
- **THEN** Macaca SHALL return a typed `denied` or `conflict` result before provider side effects unless a provider-supported correction workflow is explicitly planned and approved
- **AND** trace evidence SHALL record the period handle and bounded lock-state reason

### Requirement: Finance Accounting SHALL separate planning from side effects

Finance Accounting SHALL provide plan-before-side-effect commands for account
mutation, journal posting, reconciliation, and audit export so applications can
inspect normalized provider constraints and approval requirements before any
external state changes.

#### Scenario: Account mutation is planned before request
- **WHEN** an application invokes `accounting.plan_account`
- **THEN** Macaca SHALL validate account class, parent account, active state, provider concurrency token, period restrictions, and permission requirements
- **AND** it SHALL return an `AccountMutationPlan` without mutating provider state

#### Scenario: Planned journal is posted
- **WHEN** an application submits `accounting.post_journal` with an approved `JournalEntryPlan`, valid idempotency key, open period, and unchanged provider preconditions
- **THEN** Macaca SHALL call the accounting provider through the service runtime and return `JournalEntry` posting evidence
- **AND** posted evidence SHALL be immutable in Macaca semantics; corrections SHALL require a provider-supported reversal or adjustment workflow

#### Scenario: Reconciliation is planned before applying
- **WHEN** an application invokes `accounting.plan_reconciliation`
- **THEN** Macaca SHALL return candidate matches, confidence metadata, conflict reasons, required approvals, and sanitized evidence
- **AND** `accounting.reconciliation_request` SHALL be the only command that applies approved reconciliation side effects

### Requirement: Finance Accounting SHALL expose industrial report and export DTOs

Finance Accounting SHALL expose typed report and export DTOs for trial balance,
balance sheet, profit/loss, cash flow when supported, audit export planning,
audit export requests, and accounting artifact handles.

#### Scenario: Trial balance is generated
- **WHEN** an application invokes `accounting.generate_trial_balance` with authorized scope, period range, book, basis, currency, and dimension filters
- **THEN** Macaca SHALL return a `TrialBalanceReport` or async report handle containing normalized report rows, basis metadata, freshness, attribution, and bounded pagination metadata
- **AND** the report SHALL NOT imply tax filing, investment advice, or provider-specific accounting policy beyond the selected provider data

#### Scenario: Audit export is planned and requested
- **WHEN** an application invokes `accounting.plan_audit_export` and then `accounting.audit_export_request` with approval, retention policy, and resource budget
- **THEN** Macaca SHALL produce an `AccountingArtifactHandle` with format, checksum, expiry, access policy, redaction profile, and replay pointer
- **AND** traces and snapshots SHALL NOT contain the raw exported artifact

### Requirement: Finance Accounting SHALL preserve Macaca boundaries

The Finance Accounting implementation SHALL remain owned by the accounting
service provider family. The microkernel, SDK, shells, and generic application
framework SHALL remain provider-neutral and SHALL NOT contain concrete provider
construction, provider-name routing, accounting workflow hardcoding, or
application-specific business logic.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete accounting provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable accounting provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability support, and bounded result codes

### Requirement: Finance Accounting SHALL provide detailed developer documentation

The Finance Accounting proposal SHALL require a detailed developer guide for
`pack.finance.accounting.v1` that makes the pack usable by application
developers and provider implementers without relying on source-code inspection.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/finance/accounting.md`
- **THEN** the guide SHALL describe purpose, manifest declaration, permission scopes, capability discovery, command DTOs, result DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction guarantees, and accounting safety constraints
- **AND** examples SHALL use generic handles and synthetic data instead of real credentials, provider routing keys, application-specific workflows, regional tax rules, or production financial records

#### Scenario: SDK discovery links documentation
- **WHEN** SDK discovery returns metadata for `pack.finance.accounting.v1`
- **THEN** the metadata SHALL include the accounting developer-guide link and version compatibility information
- **AND** unavailable diagnostics SHALL point developers to the relevant declaration, permission, entitlement, provider, or policy remediation section
