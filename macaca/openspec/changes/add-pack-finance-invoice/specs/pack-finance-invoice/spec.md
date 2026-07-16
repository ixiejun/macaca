## ADDED Requirements

### Requirement: Macaca SHALL provide Finance Invoice as a serviceized pack

Macaca SHALL provide `pack.finance.invoice.v1` as a provider-neutral,
serviceized finance pack for invoice schema discovery, party/item references,
draft planning and creation, invoice read/list, issue/finalize, delivery,
payment-status sync, reminders, voiding, export, and artifact handles.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.finance.invoice.v1` as required and the invoice service provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with descriptor hash, command schemas, permission scopes, policy template, provider capability, lifecycle support, health, freshness, and replay metadata
- **AND** SDK discovery SHALL mark supported commands as callable without exposing raw PII, payment credentials, tax identifiers, hosted URLs with secrets, invoice PDFs, raw provider payloads, full invoice lines, or unbounded export data

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.finance.invoice.v1` as required but provider, permission, entitlement, policy, resource, host support, or accounting entity access is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, call another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.finance.invoice.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Finance Invoice SHALL expose schema and provider capability discovery

`pack.finance.invoice.v1` SHALL expose provider-neutral discovery for supported
commands, lifecycle transitions, required fields, tax and discount support,
numbering constraints, party and item references, delivery/reminder support,
payment-status support, export formats, freshness, attribution, and limitations.

#### Scenario: Provider schema is inspected
- **WHEN** an application invokes `invoice.inspect_provider` or `invoice.describe_schema`
- **THEN** Macaca SHALL return provider capability and schema metadata with command support, lifecycle support, tax/discount support, required fields, export formats, reminder support, payment-status support, freshness, attribution, and unavailable limitations
- **AND** the response SHALL use provider-neutral metadata rather than raw provider invoice payloads

#### Scenario: Reminder is unsupported
- **WHEN** a provider supports invoice read/write but does not support reminder sending through API
- **THEN** SDK discovery SHALL mark `invoice.send_reminder` as non-callable for the effective capability
- **AND** invoking it SHALL return a typed `unsupported` result without provider side effects

### Requirement: Finance Invoice commands SHALL use typed canonical service calls

Every Finance Invoice operation SHALL be represented as a typed command and
result DTO, and every invocation SHALL traverse the canonical service runtime
path with trace, policy, resource, entitlement, recipient approval when required,
health, snapshot, and structured error behavior.

#### Scenario: Read command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `invoice.read_invoice` is invoked
- **THEN** Macaca SHALL route the command through SDK or facade helpers into the service runtime and invoice service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers

#### Scenario: External delivery is denied before provider call
- **WHEN** policy, recipient approval, permission, entitlement, resource, lifecycle, or provider-capability checks reject `invoice.send_invoice` or `invoice.send_reminder`
- **THEN** Macaca SHALL return a typed `denied`, `unavailable`, `unsupported`, `conflict`, or `quota_exceeded` result before invoking the concrete provider
- **AND** audit evidence SHALL include bounded reason codes and replay pointers without raw recipient data or provider payloads

#### Scenario: Export output is bounded
- **WHEN** `invoice.export_invoice` could return a PDF, HTML, JSON, CSV, or other unbounded artifact
- **THEN** Macaca SHALL produce an `InvoiceArtifactHandle` or bounded metadata response
- **AND** traces and snapshots SHALL store only checksums, handles, expiry, retention, redaction, and sanitized metadata

### Requirement: Finance Invoice SHALL validate invoice structure and lifecycle

Finance Invoice SHALL validate invoice structure and lifecycle transitions before
side effects, including required parties, line totals, tax/discount references,
currency precision, provider concurrency tokens, idempotency keys, lifecycle
state, recipient policy, and provider capability.

#### Scenario: Invoice draft is planned
- **WHEN** an application invokes `invoice.plan_invoice` with party references, line items, tax references, discounts, currency, due dates, and terms
- **THEN** Macaca SHALL return an `InvoiceDraftPlan` with normalized lines, totals, rounding evidence, lifecycle preconditions, idempotency requirements, approval requirements, and sanitized validation evidence
- **AND** the planning command SHALL NOT mutate provider state

#### Scenario: Invoice totals are invalid
- **WHEN** an application invokes `invoice.plan_invoice` with totals that do not match line, tax, discount, fee, or rounding rules
- **THEN** Macaca SHALL return a typed validation denial with bounded discrepancy metadata
- **AND** Macaca SHALL NOT build or invoke a provider draft-creation request

#### Scenario: Invoice is issued
- **WHEN** an application invokes `invoice.issue_invoice` with an approved issue plan, valid idempotency key, current provider concurrency token, and supported lifecycle transition
- **THEN** Macaca SHALL call the invoice provider through the service runtime and return updated `InvoiceRecord` lifecycle evidence
- **AND** invalid or stale transitions SHALL return typed `conflict` or `stale_data` results before side effects when detectable

### Requirement: Finance Invoice SHALL separate planning from side effects

Finance Invoice SHALL provide plan-before-side-effect commands for draft
creation, issuing, delivery, reminders, voiding, and export so applications can
inspect provider constraints and approval requirements before external state
changes.

#### Scenario: Delivery is planned before send
- **WHEN** an application invokes `invoice.plan_delivery`
- **THEN** Macaca SHALL validate recipients, delivery channel, message policy, lifecycle state, provider support, and approval requirements
- **AND** the planning command SHALL NOT send an invoice or notify an external recipient

#### Scenario: Reminder is planned before send
- **WHEN** an application invokes `invoice.plan_reminder`
- **THEN** Macaca SHALL validate overdue/eligible state, recipient policy, cadence, provider support, and approval requirements
- **AND** `invoice.send_reminder` SHALL be the only command that sends an approved reminder

#### Scenario: Void is planned before mutation
- **WHEN** an application invokes `invoice.plan_void`
- **THEN** Macaca SHALL validate lifecycle state, provider constraints, accounting implications, idempotency requirements, and approval requirements
- **AND** `invoice.void_invoice` SHALL be the only command that applies a supported void/cancel transition

### Requirement: Finance Invoice SHALL expose payment status without owning payment execution

Finance Invoice SHALL expose invoice payment-status references and sync behavior,
but it SHALL NOT create payment intents, collect payments, settle funds, issue
refunds, or handle chargebacks.

#### Scenario: Payment status is synchronized
- **WHEN** an application invokes `invoice.sync_payment_status`
- **THEN** Macaca SHALL refresh or read provider payment-status references such as amount paid, amount remaining, paid date when available, and provider status
- **AND** the command SHALL NOT initiate collection, settlement, refund, or chargeback behavior

#### Scenario: Payment collection is requested through invoice pack
- **WHEN** an application attempts to use `pack.finance.invoice.v1` to collect payment or create a payment intent
- **THEN** Macaca SHALL return `unsupported` or route only through a separately declared payment capability if one exists
- **AND** invoice traces SHALL record no payment credential or settlement payload

### Requirement: Finance Invoice SHALL preserve Macaca boundaries

The Finance Invoice implementation SHALL remain owned by the invoice service
provider family. The microkernel, SDK, shells, and generic application framework
SHALL remain provider-neutral and SHALL NOT contain concrete provider
construction, provider-name routing, application billing workflows, tax rules,
template logic, reminder cadence, or payment execution.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete invoice provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable invoice provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, lifecycle support, capability support, freshness, and bounded result codes

### Requirement: Finance Invoice SHALL provide detailed developer documentation

The Finance Invoice proposal SHALL require a detailed developer guide for
`pack.finance.invoice.v1` that makes the pack usable by application developers
and provider implementers without relying on source-code inspection.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/finance/invoice.md`
- **THEN** the guide SHALL describe purpose, manifest declaration, permission scopes, capability/schema discovery, command DTOs, result DTOs, lifecycle states, examples, unavailable diagnostics, provider replacement, recipient policy, trace/audit behavior, redaction guarantees, and payment-boundary notes
- **AND** examples SHALL use generic handles and synthetic data instead of raw PII, credentials, provider routing keys, application-specific billing workflows, real invoice data, or regional tax rules

#### Scenario: SDK discovery links documentation
- **WHEN** SDK discovery returns metadata for `pack.finance.invoice.v1`
- **THEN** the metadata SHALL include the invoice developer-guide link and version compatibility information
- **AND** unavailable diagnostics SHALL point developers to the relevant declaration, permission, entitlement, provider, policy, recipient, lifecycle, or payment-boundary remediation section
