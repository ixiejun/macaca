## ADDED Requirements

### Requirement: Macaca SHALL provide a supplier-grade Foundation Secrets Reference Pack

Macaca SHALL provide `pack.foundation.secrets.reference.v1` as a
provider-neutral, serviceized secret-reference pack for creating/importing
references, inspecting metadata, binding purpose, resolving for providers,
leasing, renewing, revoking, rotating, checking version status, listing
references, and auditing access.

#### Scenario: Application declares secret-reference access
- **WHEN** an application declares `pack.foundation.secrets.reference.v1` with
  reference declarations, allowed purposes, allowed service ids, and permission
  scopes
- **THEN** admission SHALL validate pack id, lifecycle, references, purposes,
  permission scopes, policy bounds, service mappings, command schemas, and
  provider capability requirements
- **AND** admission SHALL produce an effective capability report with callable,
  denied, unsupported, and unavailable command states

#### Scenario: Required secret provider is unavailable
- **WHEN** `pack.foundation.secrets.reference.v1` is required but no admitted
  provider can satisfy declared references and commands
- **THEN** application readiness SHALL be blocked with structured unavailable
  diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to raw environment
  variables, or fake success

#### Scenario: Application requests raw secret value
- **WHEN** an application requests a raw secret value as an ordinary command
  result
- **THEN** Macaca SHALL return a typed `raw_secret_forbidden` or denied result
- **AND** it SHALL NOT place raw secret values in SDK output, WASM memory, traces,
  audit records, snapshots, prompts, diagnostics, or logs

### Requirement: Secret-reference commands SHALL use typed canonical service calls

Every `secrets.*` operation SHALL be represented as a typed command/result DTO
and SHALL traverse the canonical service runtime path with trace, policy,
resource, entitlement, approval, health, snapshot, redaction, and structured
error behavior.

#### Scenario: Provider resolution succeeds without raw app exposure
- **WHEN** a declared and policy-allowed `secrets.resolve_for_provider` command
  is invoked for an admitted service id and purpose
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the
  secret-reference service provider
- **AND** the provider SHALL receive only an approved provider-side injection
  handle or provider-local credential material outside application-visible
  surfaces

#### Scenario: Disabled or expired reference is rejected
- **WHEN** a reference is disabled, destroyed, expired, rotation-required, or has
  an expired lease
- **THEN** Macaca SHALL return a typed diagnostic before provider injection
- **AND** audit evidence SHALL include bounded status and reason codes without
  raw secret values

#### Scenario: Lease is revoked
- **WHEN** `secrets.revoke_lease` is invoked for a valid lease
- **THEN** Macaca SHALL revoke the provider lease when supported or return
  structured unsupported/unavailable diagnostics
- **AND** future provider resolution through that lease SHALL be denied

### Requirement: Secret references SHALL expose metadata and purpose, not raw values

`pack.foundation.secrets.reference.v1` SHALL expose explicit DTOs for references,
external locator hashes, purpose, access policy, leases, resolution handles,
version status, and audit records. It SHALL NOT expose provider-native secret
paths, raw values, credentials, private keys, or signatures to applications.

#### Scenario: Developer inspects reference metadata
- **WHEN** an application invokes `secrets.inspect_reference`
- **THEN** Macaca SHALL return sanitized metadata such as reference id, provider
  class, purpose, version status, expiry, rotation state, and policy summary
- **AND** it SHALL not return raw secret values or provider-private locators

#### Scenario: Purpose binding does not match
- **WHEN** a service requests a reference for a purpose not allowed by the
  reference policy
- **THEN** Macaca SHALL return a typed `invalid_purpose` or denied result
- **AND** provider injection SHALL NOT occur

### Requirement: Secret-reference trace, audit, health, snapshots, and replay SHALL be sanitized

Macaca SHALL bound and sanitize secret-reference metadata, provider resolution
events, lease events, rotation events, snapshots, traces, and audit records.

#### Scenario: Secret-reference snapshot is recorded
- **WHEN** the secret-reference service records a snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class,
  reference count, capability flags, policy template hash, lease metadata hashes,
  rotation status summaries, and sanitized replay references
- **AND** it SHALL exclude raw secret values, external provider locators,
  credentials, private keys, raw signatures, raw provider payloads, prompts, and
  unbounded output

#### Scenario: Audit replay reconstructs a provider resolution decision
- **WHEN** audit replay inspects a secret-reference provider resolution
- **THEN** replay evidence SHALL include command name, reference id hash, service
  id, purpose, version status, lease id hash, policy decision, and trace
  identifiers
- **AND** replay SHALL NOT require or reveal raw secret values

### Requirement: Secret-reference implementation SHALL preserve Macaca boundaries

The secret-reference implementation SHALL remain owned by the secret-reference
system service and replaceable providers. The microkernel, SDK, shells, and
generic application framework SHALL remain provider-neutral and free of
application-specific secret routing.

#### Scenario: Boundary gates scan secret-reference implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path
  gates scan the implementation
- **THEN** they SHALL find no concrete secret provider imports in the
  microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned
  service registrations and typed service commands

#### Scenario: WASM app uses secret-reference host imports
- **WHEN** a WASM application invokes secret-reference host imports
- **THEN** the host imports SHALL route through the same `secrets.*` service
  command path used by SDK and YAML applications
- **AND** WASM code SHALL NOT receive raw secret values, raw provider locators, or
  bypass policy

### Requirement: Secrets reference pack completion SHALL include developer documentation

The `pack.foundation.secrets.reference.v1` proposal SHALL NOT be marked complete
until the detailed developer guide exists and is linked from SDK discovery
metadata.

#### Scenario: Developer reads secrets-reference documentation
- **WHEN** a developer opens
  `docs/developer-packs/foundation/secrets-reference.md`
- **THEN** the guide SHALL document the reference-only model, raw-secret
  prohibition, manifest declaration, purpose binding, permission scopes, policy
  defaults, command DTOs, result DTOs, error DTOs, provider resolution flow,
  leases, rotation, revocation, version status, audit access, unavailable
  diagnostics, provider replacement, trace/audit fields, and examples
- **AND** examples SHALL use generic data and SHALL NOT hardcode application
  business logic, provider names, credentials, raw secret values, or
  workflow-specific secret paths
