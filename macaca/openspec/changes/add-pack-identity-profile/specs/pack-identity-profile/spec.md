## ADDED Requirements

### Requirement: Macaca SHALL provide Identity Profile as a serviceized pack

Macaca SHALL provide `pack.identity.profile.v1` as a provider-neutral,
serviceized identity pack for profile records, fields, schema descriptors,
privacy classifications, profile-owned preferences, avatar references, profile
synchronization, export artifacts, and audit evidence.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.identity.profile.v1` as required and the profile service provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with descriptor hash, command schemas, permission scopes, policy template, provider capability, schema metadata, field support, privacy support, avatar support, export support, health, freshness, and replay metadata
- **AND** SDK discovery SHALL mark supported commands as callable without exposing raw credentials, tokens, identity documents, raw provider payloads, raw avatar/photo bytes, private keys, signatures, or unbounded profile exports

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.identity.profile.v1` as required but provider, permission, entitlement, policy, resource, host support, schema support, field support, avatar support, export support, or tenant/provider scope access is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, call another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.identity.profile.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Identity Profile SHALL expose provider and schema discovery

`pack.identity.profile.v1` SHALL expose provider-neutral discovery for profile
schemas, field masks, metadata namespaces, preference support, privacy classes,
avatar reference support, export formats, versioning, freshness, limits,
attribution, entitlement, and unavailable limitations.

#### Scenario: Provider schema is inspected
- **WHEN** an application invokes `profile.inspect_provider` or `profile.describe_schema`
- **THEN** Macaca SHALL return `ProfileProviderCapability` and schema metadata with command support, field definitions, custom schema extensions, metadata namespace support, preference support, privacy defaults, avatar support, export formats, freshness, attribution, and limits
- **AND** the response SHALL use provider-neutral metadata rather than raw profile, credential, token, identity document, avatar, directory, or provider event payloads

#### Scenario: Avatar update is unsupported
- **WHEN** a provider supports profile read/write but not avatar reference updates
- **THEN** SDK discovery SHALL mark avatar update commands as non-callable for the effective capability
- **AND** invoking `profile.set_avatar_reference` SHALL return a typed `unsupported` result before provider side effects

### Requirement: Identity Profile commands SHALL use typed canonical service calls

Every Identity Profile operation SHALL be represented as a typed command and
result DTO, and every invocation SHALL traverse the canonical service runtime
path with trace, policy, resource, entitlement, approval when required, health,
snapshot, artifact boundary enforcement, field minimization, and structured
error behavior.

#### Scenario: Profile update succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `profile.update_profile` is invoked with valid field patch, version token, and idempotency key
- **THEN** Macaca SHALL route the command through SDK or facade helpers into the service runtime and profile service provider
- **AND** it SHALL emit sanitized admission, policy, provider-inspection, patch-planning, service-call, result, audit, and replay events with stable trace identifiers

#### Scenario: Sensitive field update is denied before provider call
- **WHEN** policy, permission, entitlement, approval, privacy class, metadata namespace, schema, version token, resource, or provider-capability checks reject a profile update
- **THEN** Macaca SHALL return a typed `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, or `stale_data` result before invoking the concrete provider
- **AND** audit evidence SHALL include bounded reason codes and replay pointers without raw credentials, tokens, provider payloads, identity documents, or avatar bytes

#### Scenario: Export output is bounded
- **WHEN** `profile.export_profile` could return a large profile export or raw avatar bytes
- **THEN** Macaca SHALL produce a `ProfileArtifactHandle` or bounded metadata response
- **AND** traces and snapshots SHALL store only checksums, handles, expiry, retention, redaction profile, field masks, and sanitized metadata

### Requirement: Identity Profile SHALL normalize profile records and privacy fields

Identity Profile SHALL provide normalized DTOs for profile records, fields,
schema descriptors, metadata namespaces, profile-owned preferences, privacy
classes, avatar references, audit references, version tokens, freshness,
attribution, and redaction.

#### Scenario: Profile is read
- **WHEN** an application invokes `profile.read_profile` with authorized profile scope and field mask
- **THEN** Macaca SHALL return `ProfileRecord` with account/subject references, requested fields, metadata namespaces, profile-owned preferences, avatar reference, privacy map, version token, freshness, attribution, and redaction metadata
- **AND** provider-specific missing fields SHALL be represented as explicit unavailable or unknown states rather than fabricated values

#### Scenario: Privacy fields are inspected
- **WHEN** an application invokes `profile.inspect_privacy_fields`
- **THEN** Macaca SHALL return privacy classes, visibility, retention, mutability, redaction, and approval requirements for requested fields
- **AND** the command SHALL NOT expose raw provider profile payloads or expand the authorized field mask

#### Scenario: Profile state is synchronized
- **WHEN** an application invokes `profile.sync_profile`
- **THEN** Macaca SHALL refresh schema version, field freshness, avatar reference metadata, profile version token, and provider attribution
- **AND** the command SHALL NOT create accounts, perform auth handoff, exchange tokens, bind sessions, change organization membership, change tenant policy, or process media bytes

### Requirement: Identity Profile SHALL separate planning from side effects

Identity Profile SHALL provide plan-before-side-effect commands for profile
patches, preference writes, avatar reference changes, and profile exports so
applications can inspect schema constraints, privacy classes, approval
requirements, version tokens, idempotency needs, and artifact bounds before
external state changes.

#### Scenario: Profile patch is planned
- **WHEN** an application invokes `profile.plan_patch`
- **THEN** Macaca SHALL validate field masks, schema constraints, privacy classes, metadata namespace access, version token requirements, idempotency requirement, retention, and approval requirement
- **AND** the planning command SHALL NOT update a provider profile

#### Scenario: Avatar reference is changed
- **WHEN** an application invokes `profile.set_avatar_reference` with approved plan, bounded artifact handle or hosted URL, valid media metadata, and provider support
- **THEN** Macaca SHALL call the profile provider through the service runtime and return avatar reference evidence
- **AND** unsupported media metadata, unbounded bytes, stale profile state, or missing approval SHALL return typed errors before side effects when detectable

#### Scenario: Profile-owned preference is set
- **WHEN** an application invokes `profile.set_preference` for a declared profile-owned namespace
- **THEN** Macaca SHALL validate namespace policy, privacy class, retention, idempotency, and provider support before writing
- **AND** application business preferences, feature flags, marketing workflows, or UI routing preferences SHALL return `unsupported` or remain application-owned

### Requirement: Identity Profile SHALL preserve account, auth handoff, organization, tenant, session, media, and application-preference boundaries

Identity Profile SHALL expose references to account, auth handoff,
organization, tenant, session, media, and secrets data when providers include
them, but it SHALL NOT execute or own those adjacent capabilities.

#### Scenario: Account or auth operation is requested through profile pack
- **WHEN** an application attempts to use `pack.identity.profile.v1` to create accounts, change lifecycle state, perform OAuth/OIDC/SAML handoff, exchange tokens, bind sessions, verify passwords, execute MFA challenges, or store raw credentials
- **THEN** Macaca SHALL return `unsupported` or require separately declared account, auth handoff, session, or secrets capabilities
- **AND** profile traces SHALL record no raw credential, token, MFA secret, or password payload

#### Scenario: Organization or tenant operation is requested through profile pack
- **WHEN** an application attempts to manage organization membership, roles, invitations, tenant policy, or tenant quota through the profile pack
- **THEN** Macaca SHALL return `unsupported` or require separately declared organization or tenant capabilities
- **AND** profile commands SHALL only carry sanitized references when available

#### Scenario: Media processing or application preference workflow is requested
- **WHEN** an application attempts to process avatar media bytes, transform images, run marketing preferences, or store product-specific application settings through the profile pack
- **THEN** Macaca SHALL return `unsupported` or require separately declared media/application capabilities
- **AND** profile commands SHALL only carry bounded avatar artifact references and profile-owned preference namespaces

### Requirement: Identity Profile SHALL preserve Macaca boundaries

The Identity Profile implementation SHALL remain owned by the profile service
provider family. The microkernel, SDK, shells, and generic application framework
SHALL remain provider-neutral and SHALL NOT contain concrete provider
construction, provider-name routing, account lifecycle logic, auth handoff
logic, token handling, credential storage, organization membership logic, tenant
policy logic, media processing, or application-specific preference behavior.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete profile provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable profile provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, schema support, field support, privacy support, avatar support, freshness, version/conflict metadata, and bounded result codes

### Requirement: Identity Profile SHALL provide detailed developer documentation

The Identity Profile proposal SHALL require a detailed developer guide for
`pack.identity.profile.v1` that makes the pack usable by application developers
and provider implementers without relying on source-code inspection.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/identity/profile.md`
- **THEN** the guide SHALL describe purpose, manifest declaration, permission scopes, provider/schema discovery, command DTOs, result DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction guarantees, field minimization, privacy classes, profile-owned preferences, avatar artifact boundaries, idempotency, version tokens, freshness, and account/auth/organization/tenant/session/media/application-preference boundaries
- **AND** examples SHALL use generic handles and synthetic data instead of raw credentials, tokens, provider routing keys, provider payloads, identity documents, raw avatar bytes, or application-specific preference workflows

#### Scenario: SDK discovery links documentation
- **WHEN** SDK discovery returns metadata for `pack.identity.profile.v1`
- **THEN** the metadata SHALL include the profile developer-guide link and version compatibility information
- **AND** unavailable diagnostics SHALL point developers to the relevant declaration, permission, entitlement, provider, schema, field mask, privacy class, avatar, freshness, or boundary remediation section
