## ADDED Requirements

### Requirement: Macaca SHALL provide Identity Account as a serviceized pack

Macaca SHALL provide `pack.identity.account.v1` as a provider-neutral,
serviceized identity pack for account records, identifiers, lifecycle state,
linked identities, status synchronization, recovery references, account audit
export, and artifact handles.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.identity.account.v1` as required and the account service provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with descriptor hash, command schemas, permission scopes, policy template, provider capability, schema metadata, lifecycle support, linked identity support, audit support, health, freshness, and replay metadata
- **AND** SDK discovery SHALL mark supported commands as callable without exposing raw credentials, password hashes, reset tokens, recovery codes, MFA secrets, access tokens, refresh tokens, raw provider payloads, identity documents, private keys, signatures, or unbounded audit exports

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.identity.account.v1` as required but provider, permission, entitlement, policy, resource, host support, schema support, lifecycle support, linked identity support, audit support, or tenant/provider scope access is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, call another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.identity.account.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Identity Account SHALL expose provider and schema discovery

`pack.identity.account.v1` SHALL expose provider-neutral discovery for account
schema, identifier types, create/update/search support, lifecycle transitions,
linked identity support, recovery reference support, audit export support,
pagination, versioning, freshness, limits, attribution, entitlement, and
unavailable limitations.

#### Scenario: Provider schema is inspected
- **WHEN** an application invokes `account.inspect_provider` or `account.describe_schema`
- **THEN** Macaca SHALL return `AccountProviderCapability` and schema metadata with command support, identifier support, mutable attribute support, lifecycle transitions, linked identity support, recovery reference support, audit export formats, freshness, attribution, and limits
- **AND** the response SHALL use provider-neutral metadata rather than raw user, credential, token, session, directory, or provider event payloads

#### Scenario: Lifecycle transition is unsupported
- **WHEN** a provider supports account read/search but not a requested lifecycle transition such as unlock, recover, or archive
- **THEN** SDK discovery SHALL mark that transition as non-callable for the effective capability
- **AND** invoking it SHALL return a typed `unsupported` result before provider side effects

### Requirement: Identity Account commands SHALL use typed canonical service calls

Every Identity Account operation SHALL be represented as a typed command and
result DTO, and every invocation SHALL traverse the canonical service runtime
path with trace, policy, resource, entitlement, approval when required, health,
snapshot, redaction, and structured error behavior.

#### Scenario: Account creation succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `account.create_account` is invoked with valid identifiers, source metadata, and idempotency key
- **THEN** Macaca SHALL route the command through SDK or facade helpers into the service runtime and account service provider
- **AND** it SHALL emit sanitized admission, policy, provider-inspection, create-planning, service-call, result, audit, and replay events with stable trace identifiers

#### Scenario: Lifecycle change is denied before provider call
- **WHEN** policy, permission, entitlement, approval, tenant isolation, identifier conflict, version token, lifecycle state, resource, or provider-capability checks reject `account.lifecycle_transition_request`
- **THEN** Macaca SHALL return a typed `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, or `stale_data` result before invoking the concrete provider
- **AND** audit evidence SHALL include bounded reason codes and replay pointers without raw credentials, tokens, provider payloads, or identity documents

#### Scenario: Audit export output is bounded
- **WHEN** `account.audit_export_request` could return a large directory audit export
- **THEN** Macaca SHALL produce an `AccountArtifactHandle` or bounded metadata response
- **AND** traces and snapshots SHALL store only checksums, handles, expiry, retention, redaction profile, and sanitized metadata

### Requirement: Identity Account SHALL normalize account records and lifecycle state

Identity Account SHALL provide normalized DTOs for account records, identifiers,
minimized attributes, lifecycle states, linked identity references, recovery
references, account audit references, version tokens, freshness, attribution,
and redaction.

#### Scenario: Account is read
- **WHEN** an application invokes `account.read_account` with authorized account scope
- **THEN** Macaca SHALL return `AccountRecord` with account handle, stable subject reference, identifiers, minimized attributes, lifecycle state, linked identity references, organization/tenant references, recovery references, audit references, version token, freshness, attribution, and redaction metadata
- **AND** provider-specific missing fields SHALL be represented as explicit unavailable or unknown states rather than fabricated values

#### Scenario: Account status is synchronized
- **WHEN** an application invokes `account.sync_status`
- **THEN** Macaca SHALL refresh lifecycle state, identifier verification states, linked identity state, recovery reference metadata, version token, freshness, and provider attribution
- **AND** the command SHALL NOT perform auth handoff, exchange tokens, bind sessions, verify passwords, execute MFA challenges, update profile preferences, change organization membership, or change tenant policy

### Requirement: Identity Account SHALL separate planning from side effects

Identity Account SHALL provide plan-before-side-effect commands for account
creation, updates, lifecycle transitions, linked identity changes, recovery
reference changes, and audit export so applications can inspect provider
constraints, approval requirements, version tokens, idempotency needs, and
redaction bounds before external state changes.

#### Scenario: Account creation is planned
- **WHEN** an application invokes `account.plan_create`
- **THEN** Macaca SHALL validate identifiers, minimized attributes, provider schema, tenant isolation, identifier uniqueness, idempotency requirement, and approval requirement
- **AND** the planning command SHALL NOT create a provider account

#### Scenario: Linked identity is changed
- **WHEN** an application invokes `account.link_identity` or `account.unlink_identity` with approved scope, valid external identity reference, and provider support
- **THEN** Macaca SHALL call the account provider through the service runtime and return linked identity evidence
- **AND** provider conflicts or stale account state SHALL return typed `conflict` or `stale_data` results before side effects when detectable

#### Scenario: Lifecycle transition is applied
- **WHEN** an application invokes `account.lifecycle_transition_request` with approved plan, valid idempotency key, current version token, supported transition, and valid lifecycle state
- **THEN** Macaca SHALL call the account provider through the service runtime and return updated lifecycle evidence
- **AND** unsupported transitions, stale version tokens, or missing approval SHALL return typed errors before side effects when detectable

### Requirement: Identity Account SHALL preserve profile, auth handoff, organization, tenant, session, and secrets boundaries

Identity Account SHALL expose references to profile, auth handoff, organization,
tenant, session, and secret data when providers include them, but it SHALL NOT
execute or own those adjacent capabilities.

#### Scenario: Auth handoff or credential operation is requested through account pack
- **WHEN** an application attempts to use `pack.identity.account.v1` to perform OAuth/OIDC/SAML handoff, exchange tokens, bind sessions, verify passwords, execute MFA challenges, or store raw credentials
- **THEN** Macaca SHALL return `unsupported` or require separately declared auth handoff, session, or secrets capabilities
- **AND** account traces SHALL record no raw credential, token, MFA secret, or password payload

#### Scenario: Profile or organization mutation is requested through account pack
- **WHEN** an application attempts to manage profile preferences, avatars, organization membership, roles, invitations, or tenant policy through the account pack
- **THEN** Macaca SHALL return `unsupported` or require separately declared profile, organization, or tenant capabilities
- **AND** account commands SHALL only carry sanitized references when available

#### Scenario: Application-specific onboarding workflow is requested
- **WHEN** an application attempts to embed product-specific onboarding, offboarding, HRIS, compliance, or UI workflow inside the account pack
- **THEN** Macaca SHALL reject the behavior as application-owned or provider-owned logic
- **AND** account commands SHALL only return account lifecycle evidence and bounded audit references

### Requirement: Identity Account SHALL preserve Macaca boundaries

The Identity Account implementation SHALL remain owned by the account service
provider family. The microkernel, SDK, shells, and generic application framework
SHALL remain provider-neutral and SHALL NOT contain concrete provider
construction, provider-name routing, auth handoff logic, token handling,
credential storage, profile preference logic, organization membership logic,
tenant policy logic, or application-specific account workflows.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete account provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable account provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, schema support, lifecycle support, linked identity support, freshness, version/conflict metadata, and bounded result codes

### Requirement: Identity Account SHALL provide detailed developer documentation

The Identity Account proposal SHALL require a detailed developer guide for
`pack.identity.account.v1` that makes the pack usable by application developers
and provider implementers without relying on source-code inspection.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/identity/account.md`
- **THEN** the guide SHALL describe purpose, manifest declaration, permission scopes, provider/schema discovery, command DTOs, result DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction guarantees, lifecycle semantics, linked identity semantics, recovery references, idempotency, version tokens, freshness, and profile/auth/organization/tenant/session/secrets boundaries
- **AND** examples SHALL use generic handles and synthetic data instead of raw credentials, tokens, reset secrets, provider routing keys, provider payloads, identity documents, or application-specific onboarding workflows

#### Scenario: SDK discovery links documentation
- **WHEN** SDK discovery returns metadata for `pack.identity.account.v1`
- **THEN** the metadata SHALL include the account developer-guide link and version compatibility information
- **AND** unavailable diagnostics SHALL point developers to the relevant declaration, permission, entitlement, provider, schema, lifecycle, linked identity, tenant isolation, freshness, or boundary remediation section
