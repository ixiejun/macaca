## ADDED Requirements

### Requirement: Macaca SHALL provide Commerce Order as a serviceized pack

Macaca SHALL provide `pack.commerce.order.v1` as a provider-neutral,
serviceized commerce pack for order records, source conversion, lifecycle state,
line items, totals, payment-status references, fulfillment-intent references,
cancellation, return references, status sync, audit export, and artifact handles.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.commerce.order.v1` as required and the order service provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with descriptor hash, command schemas, permission scopes, policy template, provider capability, schema metadata, health, freshness, and replay metadata
- **AND** SDK discovery SHALL mark supported commands as callable without exposing raw buyer PII, payment credentials, raw provider payloads, shipping labels, receipts, invoices, refund payloads, or unbounded order exports

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.commerce.order.v1` as required but provider, permission, entitlement, policy, resource, host support, or store/channel access is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, call another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.commerce.order.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Commerce Order SHALL expose provider and schema discovery

`pack.commerce.order.v1` SHALL expose provider-neutral discovery for order
creation support, source conversion support, lifecycle support, cancellation
support, fulfillment-intent support, return-reference support, export support,
versioning, status freshness, limits, attribution, entitlement, and unavailable
limitations.

#### Scenario: Provider schema is inspected
- **WHEN** an application invokes `order.inspect_provider` or `order.describe_schema`
- **THEN** Macaca SHALL return `OrderProviderCapability` and schema metadata with command support, lifecycle transitions, source conversion rules, cancellation support, fulfillment-intent support, return-reference support, export formats, freshness, attribution, and limits
- **AND** the response SHALL use provider-neutral metadata rather than raw provider order payloads

#### Scenario: Fulfillment intent is unsupported
- **WHEN** a provider supports order read/search but does not support fulfillment intent updates through API
- **THEN** SDK discovery SHALL mark `order.fulfillment_intent_request` as non-callable for the effective capability
- **AND** invoking it SHALL return a typed `unsupported` result without shipment, carrier, inventory, or payment side effects

### Requirement: Commerce Order commands SHALL use typed canonical service calls

Every Commerce Order operation SHALL be represented as a typed command and
result DTO, and every invocation SHALL traverse the canonical service runtime
path with trace, policy, resource, entitlement, approval when required, health,
snapshot, and structured error behavior.

#### Scenario: Order creation succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `order.create_order` is invoked
- **THEN** Macaca SHALL route the command through SDK or facade helpers into the service runtime and order service provider
- **AND** it SHALL emit sanitized admission, policy, provider-inspection, lifecycle-planning, service-call, result, and replay events with stable trace identifiers

#### Scenario: Lifecycle command is denied before provider call
- **WHEN** policy, permission, entitlement, approval, version-token, lifecycle, resource, or provider-capability checks reject `order.state_transition_request` or `order.cancel_order`
- **THEN** Macaca SHALL return a typed `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, or `stale_data` result before invoking the concrete provider
- **AND** audit evidence SHALL include bounded reason codes and replay pointers without raw buyer data or provider payloads

#### Scenario: Audit export output is bounded
- **WHEN** `order.audit_export_request` could return a large order audit export
- **THEN** Macaca SHALL produce an `OrderArtifactHandle` or bounded metadata response
- **AND** traces and snapshots SHALL store only checksums, handles, expiry, retention, redaction, and sanitized metadata

### Requirement: Commerce Order SHALL normalize order records and lifecycle state

Commerce Order SHALL provide normalized DTOs for order records, order lines,
adjustments, totals, party/address references, lifecycle states, payment-status
references, invoice/receipt references, fulfillment references, return
references, version tokens, freshness, attribution, and redaction.

#### Scenario: Order is read
- **WHEN** an application invokes `order.read_order` with authorized order scope
- **THEN** Macaca SHALL return `OrderRecord` with source reference, lifecycle state, lines, totals, party/address references, payment-status references, invoice/receipt references, fulfillment references, return references, version token, freshness, attribution, and redaction metadata
- **AND** provider-specific missing fields SHALL be represented as explicit unavailable or unknown states rather than fabricated values

#### Scenario: Status is synchronized
- **WHEN** an application invokes `order.sync_status`
- **THEN** Macaca SHALL refresh lifecycle state, payment-status reference, fulfillment-status reference, return reference, freshness, and provider attribution
- **AND** the command SHALL NOT authorize payment, capture payment, issue refund, create receipt, adjust inventory, or execute shipment carrier actions

### Requirement: Commerce Order SHALL separate planning from side effects

Commerce Order SHALL provide plan-before-side-effect commands for order creation,
lifecycle transitions, fulfillment-intent updates, cancellation, and audit export
so applications can inspect provider constraints, version tokens, and approval
requirements before external state changes.

#### Scenario: Order creation is planned
- **WHEN** an application invokes `order.plan_order`
- **THEN** Macaca SHALL validate source cart or quote state, order lines, totals, party/address references, provider schema, idempotency requirement, and approval requirement
- **AND** the planning command SHALL NOT create a provider order

#### Scenario: Lifecycle transition is applied
- **WHEN** an application invokes `order.state_transition_request` with an approved plan, valid idempotency key, current provider version token, and supported lifecycle transition
- **THEN** Macaca SHALL call the order provider through the service runtime and return updated `OrderRecord` lifecycle evidence
- **AND** stale version tokens or unsupported transitions SHALL return typed `conflict` or `stale_data` results before side effects when detectable

#### Scenario: Fulfillment intent is recorded
- **WHEN** an application invokes `order.fulfillment_intent_request` with approved plan and supported provider capability
- **THEN** Macaca SHALL record fulfillment intent or status reference metadata through the service runtime
- **AND** the command SHALL NOT buy labels, call carriers, adjust inventory, or complete fulfillment outside declared fulfillment capabilities

### Requirement: Commerce Order SHALL preserve payment, receipt, invoice, entitlement, and inventory boundaries

Commerce Order SHALL expose references to payment, refund, receipt, invoice,
entitlement, inventory, and fulfillment data when providers include them, but it
SHALL NOT execute those adjacent capabilities.

#### Scenario: Payment execution is requested through order pack
- **WHEN** an application attempts to use `pack.commerce.order.v1` to authorize, capture, refund, or settle payment
- **THEN** Macaca SHALL return `unsupported` or require a separately declared payment capability
- **AND** order traces SHALL record no raw payment credential or settlement payload

#### Scenario: Receipt or entitlement provisioning is requested through order pack
- **WHEN** an application attempts to issue a receipt or provision entitlement through the order pack
- **THEN** Macaca SHALL return `unsupported` or require separately declared receipt or entitlement capabilities
- **AND** order commands SHALL only carry sanitized receipt or entitlement references when available

#### Scenario: Inventory adjustment is requested through order pack
- **WHEN** an application attempts to adjust or reserve inventory through the order pack
- **THEN** Macaca SHALL return `unsupported` or require a separately declared inventory or fulfillment capability
- **AND** order traces SHALL not contain provider inventory mutation payloads

### Requirement: Commerce Order SHALL preserve Macaca boundaries

The Commerce Order implementation SHALL remain owned by the order service
provider family. The microkernel, SDK, shells, and generic application framework
SHALL remain provider-neutral and SHALL NOT contain concrete provider
construction, provider-name routing, payment/refund/receipt logic, inventory
adjustment, carrier execution, or application-specific checkout workflow.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete order provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable order provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, schema support, lifecycle support, capability support, freshness, version/conflict metadata, and bounded result codes

### Requirement: Commerce Order SHALL provide detailed developer documentation

The Commerce Order proposal SHALL require a detailed developer guide for
`pack.commerce.order.v1` that makes the pack usable by application developers
and provider implementers without relying on source-code inspection.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/commerce/order.md`
- **THEN** the guide SHALL describe purpose, manifest declaration, permission scopes, provider/schema discovery, command DTOs, result DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction guarantees, lifecycle semantics, fulfillment-intent boundaries, and payment/receipt/inventory separation
- **AND** examples SHALL use generic handles and synthetic data instead of raw buyer PII, payment data, credentials, provider routing keys, application-specific checkout workflows, or shipping labels

#### Scenario: SDK discovery links documentation
- **WHEN** SDK discovery returns metadata for `pack.commerce.order.v1`
- **THEN** the metadata SHALL include the order developer-guide link and version compatibility information
- **AND** unavailable diagnostics SHALL point developers to the relevant declaration, permission, entitlement, provider, schema, lifecycle, fulfillment, freshness, or boundary remediation section
