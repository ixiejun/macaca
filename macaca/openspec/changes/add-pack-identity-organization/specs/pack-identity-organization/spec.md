## ADDED Requirements

### Requirement: Macaca SHALL provide Identity Organization Pack as a serviceized capability

Macaca SHALL provide `pack.identity.organization.v1` as a provider-neutral
industrial pack for organization records, identifiers, memberships,
invitations, role bindings, directory-group references, organization policy
references, and audit export handles. The pack SHALL be declared by
applications, resolved by admission/catalog services, and invoked only through
typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.identity.organization.v1` as required and an organization service provider is registered, healthy, entitled, permission-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy template hash, resource limits, approval rules, provider health metadata, compatibility metadata, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets, invite tokens, raw directory-sync data, raw provider payloads, or unbounded member data

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.identity.organization.v1` as required but provider, permission, entitlement, resource, host support, or policy support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact another undeclared provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.identity.organization.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report with unavailable reason codes and command-level availability
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Identity Organization Pack SHALL expose supplier-grade organization contracts

`pack.identity.organization.v1` SHALL expose provider-neutral DTOs for
organization records, identifiers, domains, membership records, membership
states, invitations, role references, role bindings, directory-group
references, policy references, audit references, artifacts, version metadata,
freshness metadata, redaction metadata, and provider capability metadata.

#### Scenario: Provider schema is discovered
- **WHEN** SDK discovery or `organization.discover_schema` inspects the pack
- **THEN** Macaca SHALL return field descriptors, command schemas, permission scopes, lifecycle states, identifier types, membership states, invitation states, role-binding shapes, directory-link support, filter support, pagination support, version support, redaction profile, and compatibility hash
- **AND** the schema SHALL be provider-neutral even when backed by Auth0, Clerk, WorkOS, Okta, Microsoft Graph, Google, SCIM, GitHub-style, built-in, plugin, remote, mock, or unavailable providers

#### Scenario: Organization record is represented
- **WHEN** a provider returns an organization, group, workspace, SCIM group, or developer-platform organization that represents a durable collaboration or administrative container
- **THEN** Macaca SHALL map it to `OrganizationRecord` with stable handle, identifiers, lifecycle state, metadata namespaces, verified-domain references, policy references, version/freshness metadata, and bounded audit references
- **AND** Macaca SHALL NOT copy provider-specific lifecycle policy, tenant isolation policy, billing policy, branding behavior, or application authorization rules into OS semantics

#### Scenario: Directory group reference is represented
- **WHEN** a provider exposes a synced group, nested group, dynamic group, SCIM group, or IdP group-push object
- **THEN** Macaca SHALL represent it as `DirectoryGroupReference` with provider class, group handle, nested/dynamic hints, schema version, freshness, and conflict metadata
- **AND** direct directory synchronization behavior SHALL remain provider-side or in a dedicated service provider, not in the kernel, SDK, shells, or generic application framework

### Requirement: Identity Organization Pack commands SHALL use canonical typed service calls

Every `organization.*` operation SHALL be represented as a typed command/result
DTO and SHALL traverse the canonical service runtime path with trace, policy,
resource, entitlement, approval, health, snapshot, timeout, cancellation,
idempotency, redaction, and structured error behavior.

#### Scenario: Provider is inspected
- **WHEN** `organization.inspect_provider` is invoked for a declared and policy-allowed pack
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and organization service provider
- **AND** the result SHALL report provider class, lifecycle, command availability, organization support, membership support, role support, invitation support, directory-link support, audit-export support, rate-limit state, health, and unavailable diagnostics without raw provider payloads

#### Scenario: Organization is created
- **WHEN** `organization.create` is invoked after `organization.plan_create` validates identifiers, display name, domain references, resource budget, policy, entitlement, and provider capability
- **THEN** Macaca SHALL require an idempotency key, route the command through the canonical service path, return a typed organization result or typed conflict/unavailable/denied result, and emit sanitized trace/audit events
- **AND** the SDK, shell, kernel, and generic application framework SHALL NOT construct concrete providers or branch on provider names

#### Scenario: Organization is searched
- **WHEN** `organization.search` is invoked with filters and field masks
- **THEN** Macaca SHALL enforce permission, tenant/application scope, resource bounds, pagination limits, redaction, and provider capability before returning a bounded page
- **AND** the result SHALL include freshness and continuation metadata without exposing unbounded member lists or private profile fields

#### Scenario: Organization update is rejected before side effects
- **WHEN** `organization.update`, `organization.archive`, or `organization.restore` fails permission, entitlement, approval, version, lifecycle, resource, or policy validation
- **THEN** Macaca SHALL return a typed denied, approval-required, conflict, stale-version, quota, unavailable, or unsupported result before invoking the concrete provider
- **AND** the audit trail SHALL include only bounded reason codes, hashes, counters, and sanitized references

### Requirement: Identity Organization Pack SHALL manage memberships without owning account or profile lifecycle

`pack.identity.organization.v1` SHALL support membership inspection and
membership changes using account/profile references while preserving boundaries
with account, profile, auth handoff, tenant, entitlement, workflow, and
communication packs.

#### Scenario: Memberships are listed
- **WHEN** `organization.list_members` is invoked for a declared and policy-allowed organization
- **THEN** Macaca SHALL return bounded membership records with account references, profile references, role bindings, membership state, source, invitation reference, directory-group reference, version, freshness, and audit references
- **AND** Macaca SHALL NOT create accounts, mutate profile fields, perform login handoff, or expose private profile data through membership results

#### Scenario: Membership change is planned
- **WHEN** `organization.plan_membership_change` validates add, remove, suspend, reactivate, or update membership behavior
- **THEN** Macaca SHALL check account/profile references, membership uniqueness, directory-managed state, final-owner/admin protection, role constraints, approval requirements, resource limits, entitlement, and provider capability
- **AND** no provider side effect SHALL occur during the plan command

#### Scenario: Membership change conflicts with directory management
- **WHEN** `organization.request_membership_change` targets a membership controlled by an external directory or SCIM group and direct mutation is not supported
- **THEN** Macaca SHALL return a typed conflict or unsupported result with directory-managed diagnostics
- **AND** Macaca SHALL NOT bypass directory control, silently mutate local state, or fake success

### Requirement: Identity Organization Pack SHALL manage invitations without owning communication delivery

`pack.identity.organization.v1` SHALL support invitation creation, inspection,
resend references, and revocation while keeping email, messaging, notification,
and inbox delivery semantics in communication packs or provider internals.

#### Scenario: Invitation is created
- **WHEN** `organization.create_invitation` is invoked for an organization with permitted recipient, role references, expiry, and policy state
- **THEN** Macaca SHALL require idempotency, approval when policy requires it, recipient minimization, no raw invite token exposure, and a typed invitation result with delivery reference metadata
- **AND** Macaca SHALL NOT own provider-specific email templates, message delivery content, or application onboarding workflow

#### Scenario: Invitation is resent
- **WHEN** `organization.resend_invitation` is invoked for a pending invitation
- **THEN** Macaca SHALL route the request through the organization service provider or declared communication handoff reference and emit sanitized audit evidence
- **AND** the result SHALL contain delivery references and state transitions only, not raw message body, raw invite token, credentials, or provider payloads

#### Scenario: Invitation is revoked
- **WHEN** `organization.revoke_invitation` is invoked for an accepted, expired, revoked, missing, or pending invitation
- **THEN** Macaca SHALL return a typed success, conflict, stale-version, unavailable, or unsupported result according to provider state
- **AND** revocation SHALL be idempotent when the provider contract supports idempotency

### Requirement: Identity Organization Pack SHALL manage role bindings without owning product authorization

`pack.identity.organization.v1` SHALL expose role references and role bindings
as identity evidence. It SHALL NOT implement application-specific feature
gating, product authorization decisions, billing entitlement decisions, or
workflow approval policy.

#### Scenario: Role binding is planned
- **WHEN** `organization.plan_role_binding` validates role assignment or removal
- **THEN** Macaca SHALL check role catalog compatibility, privilege class, separation-of-duty policy reference, directory-managed state, final-owner/admin protection, approval requirements, entitlement, and provider support
- **AND** the command SHALL return a plan result without mutating provider state

#### Scenario: Elevated role is assigned
- **WHEN** `organization.request_role_binding` assigns an elevated owner, admin, or privileged role
- **THEN** Macaca SHALL require approval when policy requires it, idempotency, privilege-class audit evidence, and provider capability validation
- **AND** application feature gating SHALL remain outside this pack and SHALL NOT be hardcoded in OS layers

#### Scenario: Role bindings are listed
- **WHEN** `organization.list_role_bindings` is invoked
- **THEN** Macaca SHALL return bounded role bindings with subject references, organization references, role references, source, inherited state, directory-managed state, effective state, version, freshness, and audit references
- **AND** raw provider permission payloads and provider-specific role routing SHALL NOT be exposed as OS semantics

### Requirement: Identity Organization Pack SHALL support audit export and artifact handles safely

`pack.identity.organization.v1` SHALL support bounded audit export and artifact
handle metadata for organization, membership, invitation, role-binding, and
directory-link evidence while preventing observability leaks.

#### Scenario: Audit export is requested
- **WHEN** `organization.export_audit` is invoked for organization-scoped evidence
- **THEN** Macaca SHALL enforce permission, entitlement, approval, resource bounds, retention policy, redaction profile, artifact size class, and provider capability
- **AND** the result SHALL return an artifact handle and replay pointers rather than raw unbounded provider audit payloads

#### Scenario: Artifact metadata is retrieved
- **WHEN** `organization.get_artifact` is invoked for an audit/export artifact
- **THEN** Macaca SHALL return artifact id, content class, redaction state, retention deadline, size class, checksum/hash, and retrieval permissions
- **AND** raw provider payloads, raw audit exports, credentials, invite tokens, and private profile fields SHALL remain excluded from SDK diagnostics, traces, and snapshots

### Requirement: Identity Organization Pack SHALL expose health, snapshots, and replayable evidence

`pack.identity.organization.v1` SHALL expose descriptor metadata, service
health, command availability, provider capability hashes, policy template
hashes, snapshots, replay pointers, and sanitized audit events for all
operations.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.identity.organization.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hash, command availability, provider health, role schema hash, policy template hash, resource counters, bounded organization/member/invitation/role summary counts, artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, invite tokens, access tokens, refresh tokens, directory sync secrets, raw provider payloads, full member lists beyond requested pages, private profile fields, raw audit exports, manifests, package bytes, private keys, signatures, and unbounded output

#### Scenario: Trace replay inspects a command
- **WHEN** trace replay inspects any `organization.*` command
- **THEN** replay SHALL prove declaration, admission, policy, resource, entitlement, approval when required, service runtime routing, provider class, result variant, and sanitized audit evidence
- **AND** replay SHALL NOT require provider-specific logs, raw provider responses, or application-specific workflow state

#### Scenario: Provider is unavailable
- **WHEN** the active provider is unavailable, disabled, retired, degraded, command-limited, invitation-limited, role-limited, directory-limited, audit-limited, quota-limited, or rate-limited
- **THEN** SDK discovery, health, snapshots, and command results SHALL expose structured diagnostics with stable reason codes
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact undeclared providers, or fake success

### Requirement: Identity Organization Pack implementation SHALL preserve Macaca boundaries

The `pack.identity.organization.v1` implementation SHALL remain owned by
organization service providers and service-runtime contracts. The microkernel,
SDK, shells, and generic application framework SHALL remain provider-neutral and
free of application-specific, supplier-specific, directory-specific,
role-specific, invitation-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete Auth0, Clerk, WorkOS, Okta, Microsoft Graph, Google, SCIM, GitHub, directory-sync, invitation-delivery, credential, or organization provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.identity.organization.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hashes, and bounded result codes rather than provider-specific business branches

#### Scenario: Adjacent pack boundary is tested
- **WHEN** boundary tests exercise account lifecycle, profile fields, auth handoff, tenant isolation, billing entitlement, communication delivery, workflow approval, and application feature-gating scenarios
- **THEN** `pack.identity.organization.v1` SHALL expose only references or policy decisions for those concerns
- **AND** it SHALL NOT implement those adjacent pack behaviors internally

### Requirement: Identity Organization Pack SHALL include detailed developer documentation

The implementation of `pack.identity.organization.v1` SHALL include detailed
developer documentation under `docs/developer-packs/identity/organization.md`
and SHALL link that documentation from SDK discovery metadata and the industrial
pack catalog index.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/identity/organization.md`
- **THEN** the guide SHALL explain purpose, non-goals, manifest declaration, required versus optional behavior, permission scopes, approval behavior, command DTOs, result DTOs, organization records, identifiers, domains, memberships, invitations, role bindings, directory links, audit exports, artifacts, unavailable diagnostics, provider replacement, and operational limits
- **AND** examples SHALL use synthetic data and generic handles rather than provider names, credentials, application names, raw invite tokens, raw provider payloads, private profile data, or business workflows

#### Scenario: Provider author reads conformance guidance
- **WHEN** a provider author reads the organization pack documentation
- **THEN** the guide SHALL include a supplier/API mapping for Auth0 Organizations, Clerk Organizations, WorkOS Organizations/Directory Sync/RBAC, Okta Groups/Roles, Microsoft Graph groups/directory roles/invitations, Google Admin/Cloud Identity Groups, SCIM Groups, and GitHub Organizations/Teams
- **AND** it SHALL include conformance checks for descriptor completeness, scope validation, idempotency, version handling, directory-managed conflicts, role privilege class mapping, audit redaction, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and no raw payload leakage
