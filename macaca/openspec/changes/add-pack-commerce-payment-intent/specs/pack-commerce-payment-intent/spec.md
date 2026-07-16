## ADDED Requirements

### Requirement: Macaca SHALL provide Commerce Payment Intent as a serviceized pack

Macaca SHALL provide `pack.commerce.payment.intent.v1` as a provider-neutral,
serviceized commerce pack for payment-intent planning, creation,
confirmation/authorization, action inspection, capture, cancellation/void,
status sync, idempotency inspection, event references, audit export, and artifact
handles.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.commerce.payment.intent.v1` as required and the payment-intent service provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with descriptor hash, command schemas, permission scopes, policy template, provider capability, schema metadata, health, freshness, and replay metadata
- **AND** SDK discovery SHALL mark supported commands as callable without exposing raw payment credentials, client secrets, raw provider payloads, SCA payloads, wallet cryptograms, webhook bodies, private keys, signatures, or unbounded output

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.commerce.payment.intent.v1` as required but provider, permission, entitlement, policy, resource, host support, merchant account, or payment method support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, call another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.commerce.payment.intent.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Commerce Payment Intent SHALL expose provider and schema discovery

`pack.commerce.payment.intent.v1` SHALL expose provider-neutral discovery for
payment method support, capture modes, action/redirect support, asynchronous
event support, cancel/void support, partial capture support, idempotency model,
state machine, status freshness, limits, attribution, entitlement, and
unavailable limitations.

#### Scenario: Provider schema is inspected
- **WHEN** an application invokes `payment_intent.inspect_provider` or `payment_intent.describe_schema`
- **THEN** Macaca SHALL return `PaymentIntentProviderCapability` and schema metadata with command support, state support, payment method support, capture/cancel support, partial capture support, action requirements, event support, idempotency model, freshness, attribution, and limits
- **AND** the response SHALL use provider-neutral metadata rather than raw provider payment payloads

#### Scenario: Partial capture is unsupported
- **WHEN** a provider supports full capture but not partial capture
- **THEN** SDK discovery SHALL mark partial-capture command variants as non-callable or degraded for the effective capability
- **AND** invoking a partial capture SHALL return a typed `unsupported` result before provider side effects

### Requirement: Commerce Payment Intent commands SHALL use typed canonical service calls

Every Commerce Payment Intent operation SHALL be represented as a typed command
and result DTO, and every invocation SHALL traverse the canonical service runtime
path with trace, policy, resource, entitlement, approval when required, health,
snapshot, and structured error behavior.

#### Scenario: Intent creation succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `payment_intent.create_intent` is invoked
- **THEN** Macaca SHALL route the command through SDK or facade helpers into the service runtime and payment-intent service provider
- **AND** it SHALL emit sanitized admission, policy, provider-inspection, state-transition-planning, service-call, result, and replay events with stable trace identifiers

#### Scenario: Raw credentials are denied before provider call
- **WHEN** a command DTO contains raw PAN, CVV, bank credentials, wallet cryptogram, client secret, private key, or raw provider payload
- **THEN** Macaca SHALL return a typed `denied` result before invoking the concrete provider
- **AND** trace evidence SHALL record only a bounded sensitive-input rejection code

#### Scenario: Capture is denied before provider call
- **WHEN** policy, permission, entitlement, approval, authorization state, capture amount, expiry, resource, or provider-capability checks reject `payment_intent.capture`
- **THEN** Macaca SHALL return a typed `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, or `stale_data` result before invoking the concrete provider
- **AND** audit evidence SHALL include bounded reason codes and replay pointers without raw payment data

### Requirement: Commerce Payment Intent SHALL enforce payment state-machine semantics

Commerce Payment Intent SHALL validate payment state transitions before side
effects, including amount/currency precision, tokenized payment method
reference, action-required state, authorization expiry, capture amount,
cancel/void eligibility, idempotency, and event freshness.

#### Scenario: Intent is planned
- **WHEN** an application invokes `payment_intent.plan_intent` with amount, currency, merchant account, order/cart reference, customer/session reference, payment method reference, and capture mode
- **THEN** Macaca SHALL return a `PaymentIntentPlan` with normalized amount, required approvals, idempotency requirements, provider constraints, state preconditions, and redaction metadata
- **AND** the planning command SHALL NOT create a provider payment intent

#### Scenario: Action is required
- **WHEN** confirmation or authorization requires customer action such as redirect, SCA, approval, or challenge
- **THEN** Macaca SHALL return an `action_required` result with a sanitized `PaymentActionRequirement` handle
- **AND** traces and snapshots SHALL NOT contain raw client secrets, challenge payloads, or provider action bodies

#### Scenario: Capture is applied
- **WHEN** an application invokes `payment_intent.capture` with approved plan, valid idempotency key, authorized state, unexpired authorization, supported capture amount, and provider capability
- **THEN** Macaca SHALL call the payment provider through the service runtime and return `PaymentCapture` evidence
- **AND** stale state, expired authorization, or unsupported partial capture SHALL return typed `conflict`, `stale_data`, or `unsupported` before side effects when detectable

### Requirement: Commerce Payment Intent SHALL preserve refund, receipt, settlement, and dispute boundaries

Commerce Payment Intent SHALL expose references to refunds, receipts, settlement,
disputes, and payouts when providers include them, but it SHALL NOT execute or
own those adjacent capabilities.

#### Scenario: Refund is requested through payment-intent pack
- **WHEN** an application attempts to use `pack.commerce.payment.intent.v1` to issue a refund
- **THEN** Macaca SHALL return `unsupported` or require a separately declared refund/payment capability
- **AND** payment-intent traces SHALL record no refund payload

#### Scenario: Receipt is requested through payment-intent pack
- **WHEN** an application attempts to issue a receipt through the payment-intent pack
- **THEN** Macaca SHALL return `unsupported` or require a separately declared receipt capability
- **AND** payment-intent commands SHALL only carry sanitized receipt references when available

#### Scenario: Settlement or dispute handling is requested
- **WHEN** an application attempts settlement reconciliation, payout, chargeback, or dispute handling through the payment-intent pack
- **THEN** Macaca SHALL return `unsupported` or require separately declared settlement/dispute capabilities
- **AND** traces SHALL not contain raw settlement, dispute, or payout provider payloads

### Requirement: Commerce Payment Intent SHALL preserve Macaca boundaries

The Commerce Payment Intent implementation SHALL remain owned by the
payment-intent service provider family. The microkernel, SDK, shells, and
generic application framework SHALL remain provider-neutral and SHALL NOT contain
concrete provider construction, provider-name routing, raw credential handling,
refund logic, receipt logic, settlement logic, dispute logic, or
application-specific checkout workflow.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete payment-intent provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable payment-intent provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, schema support, state support, capability support, freshness, idempotency hash, and bounded result codes

### Requirement: Commerce Payment Intent SHALL provide detailed developer documentation

The Commerce Payment Intent proposal SHALL require a detailed developer guide for
`pack.commerce.payment.intent.v1` that makes the pack usable by application
developers and provider implementers without relying on source-code inspection.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/commerce/payment-intent.md`
- **THEN** the guide SHALL describe purpose, manifest declaration, permission scopes, provider/schema discovery, command DTOs, result DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction guarantees, state-machine semantics, idempotency, action-required handling, raw-credential rejection, and refund/receipt/settlement boundaries
- **AND** examples SHALL use generic handles and synthetic token references instead of raw payment credentials, secrets, provider routing keys, application-specific checkout workflows, or real payment data

#### Scenario: SDK discovery links documentation
- **WHEN** SDK discovery returns metadata for `pack.commerce.payment.intent.v1`
- **THEN** the metadata SHALL include the payment-intent developer-guide link and version compatibility information
- **AND** unavailable diagnostics SHALL point developers to the relevant declaration, permission, entitlement, provider, payment-method, state-machine, idempotency, freshness, or boundary remediation section
