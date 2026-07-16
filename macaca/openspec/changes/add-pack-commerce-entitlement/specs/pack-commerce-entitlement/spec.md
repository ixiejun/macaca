## ADDED Requirements

### Requirement: Macaca SHALL provide Commerce Entitlement as a serviceized pack

Macaca SHALL provide `pack.commerce.entitlement.v1` as a provider-neutral,
serviceized commerce pack for entitlement grants, checks, batch checks, source
synchronization, suspension/resume, revocation, transfer, seat assignment, usage
metering, proof export, event references, and artifact handles.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.commerce.entitlement.v1` as required and the entitlement service provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with descriptor hash, command schemas, permission scopes, policy template, provider capability, schema metadata, health, freshness, source support, state support, usage support, proof support, and replay metadata
- **AND** SDK discovery SHALL mark supported commands as callable without exposing raw purchase tokens, app-store signed payloads, payment credentials, provider webhook bodies, license secrets, private keys, raw signatures, raw provider payloads, or unbounded exports

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.commerce.entitlement.v1` as required but provider, permission, entitlement, policy, resource, host support, subject scope, source type, state transition, usage dimension, proof format, or merchant/store/channel access is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, call another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.commerce.entitlement.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Commerce Entitlement SHALL expose provider and schema discovery

`pack.commerce.entitlement.v1` SHALL expose provider-neutral discovery for
source types, entitlement states, subject/resource shapes, usage dimensions,
seat support, transfer support, proof export formats, idempotency model,
freshness, limits, attribution, entitlement, and unavailable limitations.

#### Scenario: Provider schema is inspected
- **WHEN** an application invokes `entitlement.inspect_provider` or `entitlement.describe_schema`
- **THEN** Macaca SHALL return `EntitlementProviderCapability` and schema metadata with command support, source type support, state support, subject/resource support, usage dimension support, seat support, transfer support, proof export formats, freshness, attribution, and limits
- **AND** the response SHALL use provider-neutral metadata rather than raw subscription, purchase, store, license, payment, or provider event payloads

#### Scenario: Usage metering is unsupported
- **WHEN** a provider supports entitlement checks but not metered usage or seat assignment
- **THEN** SDK discovery SHALL mark `entitlement.record_usage`, `entitlement.get_usage_balance`, `entitlement.assign_seat`, or `entitlement.release_seat` as non-callable for the effective capability
- **AND** invoking unsupported usage or seat commands SHALL return typed `unsupported` before provider side effects

### Requirement: Commerce Entitlement commands SHALL use typed canonical service calls

Every Commerce Entitlement operation SHALL be represented as a typed command and
result DTO, and every invocation SHALL traverse the canonical service runtime
path with trace, policy, resource, entitlement, approval when required, health,
snapshot, proof boundary enforcement, and structured error behavior.

#### Scenario: Entitlement grant succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `entitlement.grant` is invoked with valid subject, resource, source evidence, validity window, and idempotency key
- **THEN** Macaca SHALL route the command through SDK or facade helpers into the service runtime and entitlement service provider
- **AND** it SHALL emit sanitized admission, policy, provider-inspection, grant-planning, service-call, result, proof, and replay events with stable trace identifiers

#### Scenario: State transition is denied before provider call
- **WHEN** policy, permission, entitlement, approval, subject/resource isolation, source authority, state transition, usage limit, seat limit, resource, or provider-capability checks reject a mutating entitlement command
- **THEN** Macaca SHALL return a typed `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, or `stale_data` result before invoking the concrete provider
- **AND** audit evidence SHALL include bounded reason codes and replay pointers without raw purchase tokens, signed payloads, license secrets, or provider payloads

#### Scenario: Proof export output is bounded
- **WHEN** `entitlement.proof_export_request` could return a large proof export or raw source payload
- **THEN** Macaca SHALL produce an `EntitlementArtifactHandle` or bounded metadata response
- **AND** traces and snapshots SHALL store only checksums, handles, expiry, retention, redaction profile, and sanitized metadata

### Requirement: Commerce Entitlement SHALL normalize grants, states, seats, and usage

Commerce Entitlement SHALL provide normalized DTOs for entitlement subjects,
resources, dimensions, source evidence, grant records, states, validity windows,
seat assignments, usage records, usage balances, event references, proof
artifacts, freshness, attribution, and redaction.

#### Scenario: Entitlement is checked
- **WHEN** an application invokes `entitlement.check` with authorized subject, resource, dimension, and point-in-time scope
- **THEN** Macaca SHALL return `EntitlementGrant` or check evidence with state, validity, quantity, source evidence reference, usage balance when relevant, freshness, attribution, and redaction metadata
- **AND** provider-specific missing fields SHALL be represented as explicit unavailable or unknown states rather than fabricated values

#### Scenario: Source state is synchronized
- **WHEN** an application invokes `entitlement.sync_source`
- **THEN** Macaca SHALL refresh grant state, source evidence, validity window, usage/seat metadata, revocation or suspension reason, transfer history, freshness, and provider attribution
- **AND** the command SHALL NOT charge payment, refund payment, issue invoice, issue receipt, settle funds, change pricing, or run application-specific feature gates

#### Scenario: Usage is recorded
- **WHEN** an application invokes `entitlement.record_usage` with an approved dimension, quantity, idempotency key, and source evidence
- **THEN** Macaca SHALL record metered usage through the entitlement service provider and return bounded usage evidence
- **AND** quota, duplicate idempotency, stale source state, or unsupported dimensions SHALL return typed results before provider side effects when detectable

### Requirement: Commerce Entitlement SHALL separate planning from side effects

Commerce Entitlement SHALL provide plan-before-side-effect commands for grants,
suspension/resume, revocation, transfer, seat assignment, usage recording, and
proof export so applications can inspect provider constraints, source authority,
approval requirements, idempotency needs, and proof bounds before external state
changes.

#### Scenario: Grant is planned
- **WHEN** an application invokes `entitlement.plan_grant`
- **THEN** Macaca SHALL validate subject/resource isolation, source evidence visibility, source authority, state, validity window, quantity, provider schema, idempotency requirement, and approval requirement
- **AND** the planning command SHALL NOT create or change an entitlement grant

#### Scenario: Revocation is applied
- **WHEN** an application invokes `entitlement.revoke` with an approved plan, valid idempotency key, current grant state, revocation reason, and provider support
- **THEN** Macaca SHALL call the entitlement provider through the service runtime and return revocation evidence
- **AND** stale grant state, missing approval, unsupported revocation, or source-authority mismatch SHALL return typed errors before side effects when detectable

#### Scenario: Transfer is applied
- **WHEN** an application invokes `entitlement.transfer` with valid source subject, target subject, resource, transfer plan, idempotency key, and provider support
- **THEN** Macaca SHALL call the entitlement provider through the service runtime and return transfer evidence
- **AND** the command SHALL preserve source and target subject isolation in trace/audit metadata

### Requirement: Commerce Entitlement SHALL preserve payment, refund, invoice, receipt, pricing, and application-feature boundaries

Commerce Entitlement SHALL expose references to payment, refund, invoice,
receipt, subscription, order, store, identity, and workflow approval data when
providers include them, but it SHALL NOT execute or own those adjacent
capabilities.

#### Scenario: Payment or refund execution is requested through entitlement pack
- **WHEN** an application attempts to use `pack.commerce.entitlement.v1` to authorize, capture, refund, void, settle, dispute, or bill payment
- **THEN** Macaca SHALL return `unsupported` or require separately declared payment/refund/billing capabilities
- **AND** entitlement traces SHALL record no raw payment credential, refund payload, settlement payload, or dispute payload

#### Scenario: Invoice or receipt issuance is requested through entitlement pack
- **WHEN** an application attempts to issue an invoice or receipt through the entitlement pack
- **THEN** Macaca SHALL return `unsupported` or require separately declared invoice or receipt capabilities
- **AND** entitlement commands SHALL only carry sanitized invoice or receipt references when available

#### Scenario: Application feature gating is requested through entitlement pack
- **WHEN** an application attempts to embed product-specific feature routing, pricing, upgrade/downgrade rules, or UI access behavior inside the entitlement pack
- **THEN** Macaca SHALL reject the behavior as application-owned or provider-owned logic
- **AND** entitlement commands SHALL only return access evidence that applications can interpret through their own declared behavior

### Requirement: Commerce Entitlement SHALL preserve Macaca boundaries

The Commerce Entitlement implementation SHALL remain owned by the entitlement
service provider family. The microkernel, SDK, shells, and generic application
framework SHALL remain provider-neutral and SHALL NOT contain concrete provider
construction, provider-name routing, billing/refund/invoice/receipt logic,
pricing policy, app-store validation internals, or application-specific feature
gating behavior.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete entitlement provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable entitlement provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, source support, state support, usage support, seat support, proof support, freshness, idempotency hash, and bounded result codes

### Requirement: Commerce Entitlement SHALL provide detailed developer documentation

The Commerce Entitlement proposal SHALL require a detailed developer guide for
`pack.commerce.entitlement.v1` that makes the pack usable by application
developers and provider implementers without relying on source-code inspection.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/commerce/entitlement.md`
- **THEN** the guide SHALL describe purpose, manifest declaration, permission scopes, provider/schema discovery, command DTOs, result DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction guarantees, source freshness, grant semantics, state semantics, usage/seat semantics, proof export, idempotency, and payment/refund/invoice/receipt/application-feature boundaries
- **AND** examples SHALL use generic handles and synthetic data instead of raw purchase tokens, signed app-store payloads, payment data, credentials, provider routing keys, provider webhook bodies, license secrets, application-specific feature rules, or real customer identifiers

#### Scenario: SDK discovery links documentation
- **WHEN** SDK discovery returns metadata for `pack.commerce.entitlement.v1`
- **THEN** the metadata SHALL include the entitlement developer-guide link and version compatibility information
- **AND** unavailable diagnostics SHALL point developers to the relevant declaration, permission, entitlement, provider, source, state, usage, seat, proof, freshness, or boundary remediation section
