## ADDED Requirements

### Requirement: Macaca SHALL provide a supplier-grade Foundation Key-Value State Pack

Macaca SHALL provide `pack.foundation.key.value.state.v1` as a
provider-neutral, serviceized key-value state pack for namespaced get, put,
delete, exists, batch operations, prefix list, compare-and-set, increment, TTL,
watch, snapshot, restore, migration, and compaction.

#### Scenario: Application declares key-value state access
- **WHEN** an application declares `pack.foundation.key.value.state.v1` with
  required namespaces and permission scopes
- **THEN** admission SHALL validate pack id, lifecycle, namespace declarations,
  permission scopes, policy bounds, service mappings, command schemas, and
  provider capability requirements
- **AND** admission SHALL produce an effective capability report with callable,
  denied, unsupported, and unavailable command states

#### Scenario: Required key-value provider is unavailable
- **WHEN** `pack.foundation.key.value.state.v1` is required but no admitted
  provider can satisfy declared namespaces and commands
- **THEN** application readiness SHALL be blocked with structured unavailable
  diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to ad hoc files, or
  fake success

#### Scenario: Optional key-value provider is unavailable
- **WHEN** `pack.foundation.key.value.state.v1` is optional and unavailable
- **THEN** admission SHALL mark the effective capability set as degraded
- **AND** SDK helpers and WASM host imports SHALL refuse to build callable
  service calls for unavailable commands

### Requirement: Key-value commands SHALL use typed canonical service calls

Every `kv.*` operation SHALL be represented as a typed command/result DTO and
SHALL traverse the canonical service runtime path with trace, policy, resource,
entitlement, approval, health, snapshot, and structured error behavior.

#### Scenario: Read command succeeds
- **WHEN** a declared and policy-allowed `kv.get`, `kv.batch_get`, or
  `kv.list_keys` command is invoked with a valid namespace and key or prefix
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the
  key-value state service provider
- **AND** it SHALL emit sanitized policy, service-call, result, and replay events
  with stable trace identifiers and bounded counters

#### Scenario: Compare-and-set conflict is detected
- **WHEN** `kv.compare_and_set` is invoked with a stale revision or mismatched
  expected value
- **THEN** Macaca SHALL return a typed conflict result with current revision
  metadata when policy allows it
- **AND** the provider SHALL NOT apply the new value

#### Scenario: Mutation command is denied before side effects
- **WHEN** `kv.put`, `kv.delete`, `kv.batch_put`, `kv.batch_delete`,
  `kv.restore_namespace`, `kv.migrate_namespace`, or `kv.compact_namespace` is
  rejected by permission, policy, approval, entitlement, or resource checks
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the
  concrete provider
- **AND** audit evidence SHALL include only bounded reason codes, namespace/key
  hashes, and policy decision metadata

#### Scenario: Provider lacks a requested state feature
- **WHEN** a provider supports basic get/put/delete but does not support TTL,
  watch, snapshot, restore, migration, compaction, or strong consistency
- **THEN** Macaca SHALL return a typed unsupported result for that command or
  consistency level
- **AND** SDK discovery SHALL report the command or option as non-callable for
  the current effective capability set

### Requirement: Key-value state SHALL be namespace scoped and revision aware

`pack.foundation.key.value.state.v1` SHALL expose app-scoped namespaces,
normalized key references, typed values, revisions, TTL policies, and conflict
semantics. It SHALL NOT expose raw provider database handles to applications.

#### Scenario: Application writes a key with expected revision
- **WHEN** an application invokes `kv.put` with an expected revision
- **THEN** Macaca SHALL apply compare-revision behavior when supported by the
  provider or return `unsupported` if the provider cannot guarantee it
- **AND** the result SHALL include the new revision or a typed conflict result

#### Scenario: Application sends an invalid key
- **WHEN** an application sends an empty key, oversized key, reserved prefix,
  provider-native key syntax, or key outside the declared namespace
- **THEN** Macaca SHALL return a typed `invalid_key`, `invalid_namespace`, or
  `denied` result before provider execution
- **AND** audit evidence SHALL use key hashes or bounded safe key summaries

#### Scenario: Application attempts to store a raw secret
- **WHEN** an application attempts to store raw secret material as a KV value
- **THEN** Macaca SHALL reject the command unless the value is a secret reference
  permitted by policy
- **AND** traces, audit records, snapshots, and diagnostics SHALL NOT include the
  raw secret value

### Requirement: Key-value list, watch, snapshot, and replay SHALL be bounded and sanitized

Macaca SHALL bound and sanitize prefix scans, watch events, snapshots, restore
diagnostics, traces, and audit records for `pack.foundation.key.value.state.v1`.

#### Scenario: Prefix listing is paged
- **WHEN** `kv.list_keys` is invoked on a large namespace or prefix
- **THEN** Macaca SHALL return bounded pages with continuation tokens
- **AND** trace/audit evidence SHALL include key counters and latency without
  storing unbounded key listings

#### Scenario: Watch stream is started and cancelled
- **WHEN** `kv.watch_namespace` starts a watch stream
- **THEN** Macaca SHALL reserve stream resources, emit a watch-start event, and
  return bounded watch events from the requested revision or provider-supported
  start point
- **AND** cancellation, timeout, compaction, provider failure, and session
  shutdown SHALL release resources and emit terminal watch events

#### Scenario: Snapshot is recorded
- **WHEN** `kv.snapshot_namespace` records a snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class,
  namespace metadata, retained revision range, policy template hash, bounded
  counters, and sanitized replay pointers
- **AND** it SHALL exclude raw values, raw secrets, credentials, prompts,
  manifests, package bytes, private keys, raw provider payloads, and unbounded
  key listings

### Requirement: Key-value implementation SHALL preserve Macaca boundaries

The key-value implementation SHALL remain owned by the key-value state system
service and replaceable providers. The microkernel, SDK, shells, and generic
application framework SHALL remain provider-neutral and free of
application-specific state routing.

#### Scenario: Boundary gates scan key-value implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path
  gates scan the implementation
- **THEN** they SHALL find no concrete KV provider imports in the microkernel,
  SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned
  service registrations and typed service commands

#### Scenario: WASM app uses key-value host imports
- **WHEN** a WASM application invokes key-value host imports
- **THEN** the host imports SHALL route through the same `kv.*` service command
  path used by SDK and YAML applications
- **AND** WASM code SHALL NOT receive direct database handles or bypass policy

### Requirement: Key-value pack completion SHALL include developer documentation

The `pack.foundation.key.value.state.v1` proposal SHALL NOT be marked complete
until the detailed developer guide exists and is linked from SDK discovery
metadata.

#### Scenario: Developer reads key-value state documentation
- **WHEN** a developer opens
  `docs/developer-packs/foundation/key-value-state.md`
- **THEN** the guide SHALL document manifest declaration, namespace and key
  model, value model, permission scopes, policy defaults, resource limits,
  approval cases, command DTOs, result DTOs, error DTOs, CAS, TTL, watch streams,
  snapshots, restore, migration, compaction, unavailable diagnostics, provider
  replacement, trace/audit fields, and examples
- **AND** examples SHALL use generic data and SHALL NOT hardcode application
  business logic, provider names, credentials, or workflow-specific keys
