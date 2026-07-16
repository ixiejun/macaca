## ADDED Requirements

### Requirement: Macaca SHALL provide Commerce Receipt as a serviceized pack

Macaca SHALL provide `pack.commerce.receipt.v1` as a provider-neutral,
serviceized commerce pack for receipt evidence records, issue/reissue,
read/search, source synchronization, verification, delivery request/status,
correction references, event references, audit export, and artifact handles.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.commerce.receipt.v1` as required and the receipt service provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with descriptor hash, command schemas, permission scopes, policy template, provider capability, schema metadata, health, freshness, artifact support, and replay metadata
- **AND** SDK discovery SHALL mark supported commands as callable without exposing raw buyer PII, payment credentials, raw provider payloads, webhook bodies, receipt HTML bodies, printable binary blobs, private keys, signatures, or unbounded receipt exports

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.commerce.receipt.v1` as required but provider, permission, entitlement, policy, resource, host support, source type, delivery channel, artifact format, or merchant/store/channel access is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, call another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.commerce.receipt.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Commerce Receipt SHALL expose provider and schema discovery

`pack.commerce.receipt.v1` SHALL expose provider-neutral discovery for supported
source types, receipt audiences, receipt variants, delivery channels, artifact
formats, verification modes, reissue support, correction-reference support,
export support, idempotency model, freshness, limits, attribution, entitlement,
and unavailable limitations.

#### Scenario: Provider schema is inspected
- **WHEN** an application invokes `receipt.inspect_provider` or `receipt.describe_schema`
- **THEN** Macaca SHALL return `ReceiptProviderCapability` and schema metadata with command support, source type support, audience and variant support, delivery channel support, artifact support, verification modes, reissue support, correction-reference support, export formats, freshness, attribution, and limits
- **AND** the response SHALL use provider-neutral metadata rather than raw provider receipt, payment, order, invoice, or terminal payloads

#### Scenario: Delivery channel is unsupported
- **WHEN** a provider supports hosted receipt URLs but not SMS, email, print, or terminal delivery through API
- **THEN** SDK discovery SHALL mark unsupported delivery commands or channels as non-callable for the effective capability
- **AND** invoking an unsupported delivery channel SHALL return a typed `unsupported` result before provider side effects

### Requirement: Commerce Receipt commands SHALL use typed canonical service calls

Every Commerce Receipt operation SHALL be represented as a typed command and
result DTO, and every invocation SHALL traverse the canonical service runtime
path with trace, policy, resource, entitlement, approval when required, health,
snapshot, artifact boundary enforcement, and structured error behavior.

#### Scenario: Receipt issue succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `receipt.issue_receipt` is invoked with a valid source reference and idempotency key
- **THEN** Macaca SHALL route the command through SDK or facade helpers into the service runtime and receipt service provider
- **AND** it SHALL emit sanitized admission, policy, provider-inspection, issue-planning, service-call, result, artifact, and replay events with stable trace identifiers

#### Scenario: Delivery is denied before provider call
- **WHEN** policy, permission, entitlement, approval, destination, delivery-channel, resource, artifact, or provider-capability checks reject `receipt.delivery_request`
- **THEN** Macaca SHALL return a typed `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, or `stale_data` result before invoking the concrete provider
- **AND** audit evidence SHALL include bounded reason codes and replay pointers without raw buyer data, destination secrets, receipt bodies, or provider payloads

#### Scenario: Audit export output is bounded
- **WHEN** `receipt.audit_export_request` could return a large receipt export or raw receipt body
- **THEN** Macaca SHALL produce a `ReceiptArtifactHandle` or bounded metadata response
- **AND** traces and snapshots SHALL store only checksums, handles, expiry, retention, redaction profile, and sanitized metadata

### Requirement: Commerce Receipt SHALL normalize receipt records and evidence

Commerce Receipt SHALL provide normalized DTOs for receipt records, source
references, line snapshots, adjustments, totals, audiences, variants, delivery
states, verification results, correction references, event references, artifact
handles, freshness, attribution, and redaction.

#### Scenario: Receipt is read
- **WHEN** an application invokes `receipt.read_receipt` with authorized receipt scope
- **THEN** Macaca SHALL return `ReceiptRecord` with receipt handle, source references, receipt number/reference, audience, variant, issue state, line snapshots, adjustments, totals, delivery state, artifact handles, verification state, correction references, freshness, attribution, and redaction metadata
- **AND** provider-specific missing fields SHALL be represented as explicit unavailable or unknown states rather than fabricated values

#### Scenario: Source metadata is synchronized
- **WHEN** an application invokes `receipt.sync_source`
- **THEN** Macaca SHALL refresh receipt source references, issue state, delivery state, verification state, artifact metadata, correction references, freshness, and provider attribution
- **AND** the command SHALL NOT authorize payment, capture payment, refund payment, issue invoice, provision entitlement, perform settlement, or send unrelated communication workflow messages

### Requirement: Commerce Receipt SHALL separate planning from side effects

Commerce Receipt SHALL provide plan-before-side-effect commands for receipt
issue, reissue, delivery, correction-reference linking, and audit export so
applications can inspect provider constraints, source state, approval
requirements, idempotency needs, and artifact bounds before external state
changes.

#### Scenario: Receipt issue is planned
- **WHEN** an application invokes `receipt.plan_issue`
- **THEN** Macaca SHALL validate source reference visibility, source state, audience, variant, line/totals evidence, provider schema, idempotency requirement, artifact format, delivery constraints, and approval requirement
- **AND** the planning command SHALL NOT issue, send, print, publish, or persist a provider receipt

#### Scenario: Delivery request is applied
- **WHEN** an application invokes `receipt.delivery_request` with an approved plan, valid idempotency key, supported delivery channel, destination reference, and provider capability
- **THEN** Macaca SHALL call the receipt provider through the service runtime and return `ReceiptDeliveryState` evidence
- **AND** stale source state, unsupported channels, missing approval, or destination-policy failures SHALL return typed errors before side effects when detectable

#### Scenario: Verification is performed
- **WHEN** an application invokes `receipt.verify_receipt`
- **THEN** Macaca SHALL validate source linkage, totals match, artifact checksum or signature status when available, provider verification reference, and freshness
- **AND** verification SHALL NOT create payment, refund, invoice, entitlement, or delivery side effects

### Requirement: Commerce Receipt SHALL preserve payment, refund, invoice, communication, and entitlement boundaries

Commerce Receipt SHALL expose references to payment, refund, invoice,
settlement, entitlement, communication, and order data when providers include
them, but it SHALL NOT execute or own those adjacent capabilities.

#### Scenario: Payment or refund execution is requested through receipt pack
- **WHEN** an application attempts to use `pack.commerce.receipt.v1` to authorize, capture, refund, void, settle, or dispute payment
- **THEN** Macaca SHALL return `unsupported` or require a separately declared payment/refund/settlement capability
- **AND** receipt traces SHALL record no raw payment credential, refund payload, settlement payload, or dispute payload

#### Scenario: Invoice or entitlement provisioning is requested through receipt pack
- **WHEN** an application attempts to issue an invoice or provision entitlement through the receipt pack
- **THEN** Macaca SHALL return `unsupported` or require separately declared invoice or entitlement capabilities
- **AND** receipt commands SHALL only carry sanitized invoice or entitlement references when available

#### Scenario: Generic communication workflow is requested through receipt pack
- **WHEN** an application attempts to use the receipt pack for non-receipt email, SMS, inbox, notification, marketing, or calendar communication
- **THEN** Macaca SHALL return `unsupported` or require separately declared communication packs
- **AND** receipt delivery traces SHALL contain only receipt delivery state and bounded destination references

### Requirement: Commerce Receipt SHALL preserve Macaca boundaries

The Commerce Receipt implementation SHALL remain owned by the receipt service
provider family. The microkernel, SDK, shells, and generic application framework
SHALL remain provider-neutral and SHALL NOT contain concrete provider
construction, provider-name routing, payment/refund/invoice logic, entitlement
provisioning, generic communication workflow logic, or application-specific
checkout behavior.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete receipt provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable receipt provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, schema support, source support, delivery support, verification support, artifact support, freshness, idempotency hash, and bounded result codes

### Requirement: Commerce Receipt SHALL provide detailed developer documentation

The Commerce Receipt proposal SHALL require a detailed developer guide for
`pack.commerce.receipt.v1` that makes the pack usable by application developers
and provider implementers without relying on source-code inspection.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/commerce/receipt.md`
- **THEN** the guide SHALL describe purpose, manifest declaration, permission scopes, provider/schema discovery, command DTOs, result DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction guarantees, issue/reissue semantics, delivery semantics, verification semantics, artifact retention, idempotency, and payment/refund/invoice/communication boundaries
- **AND** examples SHALL use generic handles and synthetic data instead of raw buyer PII, payment data, credentials, provider routing keys, receipt HTML bodies, printable blobs, application-specific checkout workflows, or real customer destinations

#### Scenario: SDK discovery links documentation
- **WHEN** SDK discovery returns metadata for `pack.commerce.receipt.v1`
- **THEN** the metadata SHALL include the receipt developer-guide link and version compatibility information
- **AND** unavailable diagnostics SHALL point developers to the relevant declaration, permission, entitlement, provider, source, delivery, verification, artifact, freshness, or boundary remediation section
