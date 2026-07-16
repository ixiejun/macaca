## ADDED Requirements

### Requirement: Macaca SHALL provide a supplier-grade Foundation Session State Pack

Macaca SHALL provide `pack.foundation.session.state.v1` as a provider-neutral,
serviceized session-state pack for scoped get, put, delete, merge patch, list,
checkpoint, checkpoint listing, restore, checkpoint comparison, history
compaction, session clearing, redacted export, and recovery inspection.

#### Scenario: Application declares session-state access
- **WHEN** an application declares `pack.foundation.session.state.v1` with session
  scopes, checkpoint requirements, retention policy, and permission scopes
- **THEN** admission SHALL validate pack id, lifecycle, session scope, permission
  scopes, policy bounds, service mappings, command schemas, and provider
  capability requirements
- **AND** admission SHALL produce an effective capability report with callable,
  denied, unsupported, and unavailable command states

#### Scenario: Required session-state provider is unavailable
- **WHEN** `pack.foundation.session.state.v1` is required but no admitted provider
  can satisfy declared session-state commands
- **THEN** application readiness SHALL be blocked with structured unavailable
  diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to shell-owned state
  repair, or fake success

#### Scenario: Optional restore support is unavailable
- **WHEN** session-state is available but checkpoint restore is optional and not
  supported by the active provider
- **THEN** admission and SDK discovery SHALL report restore as unsupported or
  degraded
- **AND** restore helpers SHALL refuse to build callable restore service calls

### Requirement: Session-state commands SHALL use typed canonical service calls

Every `session_state.*` operation SHALL be represented as a typed command/result
DTO and SHALL traverse the canonical service runtime path with trace, policy,
resource, entitlement, approval, health, snapshot, redaction, and structured
error behavior.

#### Scenario: Checkpoint command succeeds
- **WHEN** a declared and policy-allowed `session_state.create_checkpoint`
  command is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the
  session-state service provider
- **AND** it SHALL emit sanitized policy, service-call, checkpoint, result, and
  replay events with state hash, schema version, provider class, and stable trace
  identifiers

#### Scenario: Restore is denied before mutation
- **WHEN** `session_state.restore_checkpoint`, `session_state.clear_session`, or
  `session_state.compact_history` is rejected by permission, policy, approval,
  retention, conflict, or resource checks
- **THEN** Macaca SHALL return a typed denied, conflict, or quota result before
  mutating provider state
- **AND** audit evidence SHALL include bounded reason codes and checkpoint/session
  metadata without raw state values

#### Scenario: Schema mismatch is detected
- **WHEN** a checkpoint restore targets a session with incompatible schema version
- **THEN** Macaca SHALL return a typed schema_mismatch result or dry-run report
- **AND** it SHALL NOT overwrite live state unless a policy-approved migration
  path is declared

### Requirement: Session state SHALL be scoped, revisioned, checkpointed, and replayable

`pack.foundation.session.state.v1` SHALL expose explicit DTOs for session refs,
state keys, typed values, revisions, checkpoint refs, restore plans, recovery
metadata, retention policy, redaction summaries, and provider capability
reports. It SHALL NOT expose raw provider session-store handles to applications.

#### Scenario: Application saves transient state
- **WHEN** an application invokes `session_state.put` with a valid session ref,
  key, typed value, and optional expected revision
- **THEN** Macaca SHALL validate scope, size, schema, revision, and policy before
  provider mutation
- **AND** the result SHALL include the new revision and replay metadata

#### Scenario: Application attempts to store raw secret state
- **WHEN** an application attempts to store raw secret material in session state
- **THEN** Macaca SHALL reject the command unless the value is a permitted secret
  reference
- **AND** traces, audit records, snapshots, diagnostics, and developer examples
  SHALL NOT include raw secret values

### Requirement: Session-state snapshots and recovery metadata SHALL be bounded and sanitized

Macaca SHALL bound and sanitize session state, checkpoint summaries, restore
diagnostics, recovery metadata, traces, and audit records for
`pack.foundation.session.state.v1`.

#### Scenario: Recovery metadata is inspected
- **WHEN** `session_state.inspect_recovery` is invoked
- **THEN** Macaca SHALL return last checkpoint metadata, provider health,
  compaction state, retention state, replay refs, and unavailable reasons
- **AND** it SHALL exclude raw state values, raw secrets, provider payloads, and
  unbounded output

#### Scenario: History is compacted
- **WHEN** `session_state.compact_history` compacts old session history
- **THEN** Macaca SHALL preserve a checkpoint anchor, retention metadata, state
  hash, and replay references
- **AND** audit replay SHALL still explain the compaction decision without
  requiring raw state payloads

### Requirement: Session-state implementation SHALL preserve Macaca boundaries

The session-state implementation SHALL remain owned by the session-state system
service and replaceable providers. The microkernel, SDK, shells, and generic
application framework SHALL remain provider-neutral and free of
application-specific recovery routing.

#### Scenario: Boundary gates scan session-state implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path
  gates scan the implementation
- **THEN** they SHALL find no concrete session-state provider imports in the
  microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned
  service registrations and typed service commands

#### Scenario: WASM app uses session-state host imports
- **WHEN** a WASM application invokes session-state host imports
- **THEN** the host imports SHALL route through the same `session_state.*` service
  command path used by SDK and YAML applications
- **AND** WASM code SHALL NOT receive raw provider handles or bypass policy

### Requirement: Session-state pack completion SHALL include developer documentation

The `pack.foundation.session.state.v1` proposal SHALL NOT be marked complete
until the detailed developer guide exists and is linked from SDK discovery
metadata.

#### Scenario: Developer reads session-state documentation
- **WHEN** a developer opens `docs/developer-packs/foundation/session-state.md`
- **THEN** the guide SHALL document manifest declaration, session scope, state
  versus workflow boundary, key/value model, revisions, checkpoints, restore,
  compaction, clear, retention, redaction, permission scopes, policy defaults,
  command DTOs, result DTOs, error DTOs, unavailable diagnostics, provider
  replacement, trace/audit fields, and examples
- **AND** examples SHALL use generic data and SHALL NOT hardcode application
  business logic, provider names, credentials, raw state payloads, or
  workflow-specific recovery rules
