## ADDED Requirements

### Requirement: Macaca SHALL provide Commerce Cart as a serviceized pack

Macaca SHALL provide `pack.commerce.cart.v1` as a provider-neutral, serviceized
commerce pack for cart lifecycle, buyer context, line items, discounts, gift
cards, estimates, tax/shipping/duty estimates, validation issues, stale
diagnostics, abandonment diagnostics, handoff intents, export, and artifact
handles.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.commerce.cart.v1` as required and the cart service provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with descriptor hash, command schemas, permission scopes, policy template, provider capability, schema metadata, health, freshness, and replay metadata
- **AND** SDK discovery SHALL mark supported commands as callable without exposing raw buyer PII, payment data, raw provider payloads, secret checkout URLs, provider-specific mutation DSLs, or unbounded cart exports

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.commerce.cart.v1` as required but provider, permission, entitlement, policy, resource, host support, or store/channel access is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, call another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.commerce.cart.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Commerce Cart SHALL expose provider and schema discovery

`pack.commerce.cart.v1` SHALL expose provider-neutral discovery for cart
lifecycle, line support, custom line support, buyer identity support,
discount/gift-card support, tax/shipping estimate support, handoff support,
versioning, stale-price behavior, search/export support, freshness, attribution,
limits, and unavailable limitations.

#### Scenario: Provider schema is inspected
- **WHEN** an application invokes `cart.inspect_provider` or `cart.describe_schema`
- **THEN** Macaca SHALL return `CartProviderCapability` and schema metadata with command support, line/custom-line support, buyer context support, discount/gift-card support, estimate support, handoff support, versioning model, stale-price behavior, export support, freshness, attribution, and limits
- **AND** the response SHALL use provider-neutral metadata rather than raw provider cart payloads

#### Scenario: Handoff is unsupported
- **WHEN** a provider supports cart mutation but does not support checkout URL or order-draft handoff
- **THEN** SDK discovery SHALL mark `cart.handoff_request` as non-callable for the effective capability
- **AND** invoking it SHALL return a typed `unsupported` result without order placement or payment execution

### Requirement: Commerce Cart commands SHALL use typed canonical service calls

Every Commerce Cart operation SHALL be represented as a typed command and result
DTO, and every invocation SHALL traverse the canonical service runtime path with
trace, policy, resource, entitlement, approval when required, health, snapshot,
and structured error behavior.

#### Scenario: Line mutation succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `cart.line_request` is invoked
- **THEN** Macaca SHALL route the command through SDK or facade helpers into the service runtime and cart service provider
- **AND** it SHALL emit sanitized admission, policy, provider-inspection, mutation-planning, service-call, result, and replay events with stable trace identifiers

#### Scenario: Mutation command is denied before provider call
- **WHEN** policy, permission, entitlement, approval, version-token, lifecycle, resource, or provider-capability checks reject `cart.line_request` or `cart.discount_request`
- **THEN** Macaca SHALL return a typed `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, or `stale_data` result before invoking the concrete provider
- **AND** audit evidence SHALL include bounded reason codes and replay pointers without raw buyer data or provider payloads

#### Scenario: Export output is bounded
- **WHEN** `cart.export_cart` could return a large cart, line, or pricing export
- **THEN** Macaca SHALL produce a `CartArtifactHandle` or bounded metadata response
- **AND** traces and snapshots SHALL store only checksums, handles, expiry, retention, redaction, and sanitized metadata

### Requirement: Commerce Cart SHALL normalize cart state, lines, adjustments, and estimates

Commerce Cart SHALL provide normalized DTOs for cart state, buyer context, line
items, custom lines, discounts, gift cards, promotions, totals, estimates,
validation issues, version tokens, freshness, attribution, and redaction.

#### Scenario: Cart is read
- **WHEN** an application invokes `cart.read_cart` with authorized cart scope
- **THEN** Macaca SHALL return `Cart` data with lifecycle state, context, lines, adjustments, estimates, validation issues, version token, freshness, attribution, and redaction metadata
- **AND** provider-specific missing fields SHALL be represented as explicit unavailable or unknown states rather than fabricated values

#### Scenario: Cart estimate is recalculated
- **WHEN** an application invokes `cart.estimate_cart` with a cart handle and estimation context
- **THEN** Macaca SHALL return `CartEstimate` with subtotal, discounts, taxes, duties, shipping, fees, total, currency precision, price-valid timestamp, freshness, attribution, and stale flags
- **AND** estimate calculation SHALL NOT create an order, checkout session, payment intent, receipt, or entitlement

#### Scenario: Cart validation detects unavailable items
- **WHEN** `cart.validate_cart` detects unavailable merchandise, invalid quantity, stale price, expired discount, or unsupported shipping context
- **THEN** Macaca SHALL return line-level or cart-level `CartValidationIssue` records with severity, retriable flag, and bounded remediation metadata
- **AND** validation SHALL NOT mutate provider state unless the provider explicitly requires a read-sync operation that is represented as such

### Requirement: Commerce Cart SHALL separate planning from mutations and handoff

Commerce Cart SHALL provide plan-before-side-effect commands for buyer context,
line items, discounts, handoff, and export so applications can inspect provider
constraints, version tokens, and approval requirements before external state
changes.

#### Scenario: Line mutation is planned
- **WHEN** an application invokes `cart.plan_line_mutation`
- **THEN** Macaca SHALL validate catalog references, quantity, selected options, selling-plan metadata, provider schema, version token, idempotency requirement, and approval requirement
- **AND** the planning command SHALL NOT mutate provider cart state

#### Scenario: Line mutation is applied
- **WHEN** an application invokes `cart.line_request` with an approved plan, valid idempotency key, current provider version token, and supported lifecycle transition
- **THEN** Macaca SHALL call the cart provider through the service runtime and return updated `Cart` state
- **AND** stale version tokens or unsupported transitions SHALL return typed `conflict` or `stale_data` results before side effects when detectable

#### Scenario: Handoff is planned before request
- **WHEN** an application invokes `cart.plan_handoff`
- **THEN** Macaca SHALL validate cart state, provider handoff support, policy, approval, expiry, and no-order/no-payment boundary
- **AND** `cart.handoff_request` SHALL create only a handoff intent or checkout URL handle, not an order, payment, receipt, or entitlement

### Requirement: Commerce Cart SHALL preserve order and payment boundaries

Commerce Cart SHALL expose estimates and handoff intents but SHALL NOT place
orders, complete checkout, create payment intents, capture payments, issue
receipts, provision entitlements, adjust inventory, or fulfill shipments.

#### Scenario: Application attempts order placement through cart pack
- **WHEN** an application attempts to use `pack.commerce.cart.v1` to place an order or complete checkout
- **THEN** Macaca SHALL return `unsupported` or require a separately declared order/checkout capability
- **AND** cart trace evidence SHALL record no payment credential, receipt, entitlement, fulfillment, or settlement payload

#### Scenario: Application attempts payment through cart pack
- **WHEN** an application attempts to use `pack.commerce.cart.v1` to create or capture a payment
- **THEN** Macaca SHALL return `unsupported` or require a separately declared payment capability
- **AND** cart commands SHALL NOT store raw payment data in request DTOs, traces, snapshots, or diagnostics

### Requirement: Commerce Cart SHALL preserve Macaca boundaries

The Commerce Cart implementation SHALL remain owned by the cart service provider
family. The microkernel, SDK, shells, and generic application framework SHALL
remain provider-neutral and SHALL NOT contain concrete provider construction,
provider-name routing, order/payment logic, tax-engine policy, promotion-engine
authoring, inventory adjustment, or application-specific checkout workflow.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete cart provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable cart provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, schema support, capability support, freshness, version/conflict metadata, and bounded result codes

### Requirement: Commerce Cart SHALL provide detailed developer documentation

The Commerce Cart proposal SHALL require a detailed developer guide for
`pack.commerce.cart.v1` that makes the pack usable by application developers and
provider implementers without relying on source-code inspection.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/commerce/cart.md`
- **THEN** the guide SHALL describe purpose, manifest declaration, permission scopes, provider/schema discovery, command DTOs, result DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction guarantees, stale-data semantics, version conflicts, handoff boundaries, and cart/order/payment separation
- **AND** examples SHALL use generic handles and synthetic data instead of raw buyer PII, payment data, credentials, provider routing keys, application-specific checkout workflows, or secret checkout URLs

#### Scenario: SDK discovery links documentation
- **WHEN** SDK discovery returns metadata for `pack.commerce.cart.v1`
- **THEN** the metadata SHALL include the cart developer-guide link and version compatibility information
- **AND** unavailable diagnostics SHALL point developers to the relevant declaration, permission, entitlement, provider, schema, discount, estimate, handoff, freshness, or boundary remediation section
