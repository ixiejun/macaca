## ADDED Requirements

### Requirement: Macaca SHALL provide a supplier-grade Foundation Config Pack

Macaca SHALL provide `pack.foundation.config.v1` as a provider-neutral,
serviceized configuration pack for schema declaration, get, list, effective
resolution, validation, provenance explanation, watch, reload, snapshot, and
redacted export operations.

#### Scenario: Application declares configuration access
- **WHEN** an application declares `pack.foundation.config.v1` with schema refs,
  config sources, selectors, and permission scopes
- **THEN** admission SHALL validate pack id, lifecycle, schema refs, selectors,
  permission scopes, policy bounds, service mappings, command schemas, and
  provider capability requirements
- **AND** admission SHALL produce an effective capability report with callable,
  denied, unsupported, and unavailable command states

#### Scenario: Required config source is unavailable
- **WHEN** `pack.foundation.config.v1` is required but no admitted provider can
  satisfy required config sources or schemas
- **THEN** application readiness SHALL be blocked with structured unavailable
  diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently read raw shell environment as a
  fallback, or fake success

#### Scenario: Optional remote config is unavailable
- **WHEN** a remote config source is optional and unavailable
- **THEN** admission SHALL mark the effective capability set as degraded
- **AND** SDK discovery SHALL explain which source is unavailable and which
  commands remain callable

### Requirement: Config commands SHALL use typed canonical service calls

Every `config.*` operation SHALL be represented as a typed command/result DTO and
SHALL traverse the canonical service runtime path with trace, policy, resource,
entitlement, health, snapshot, validation, redaction, and structured error
behavior.

#### Scenario: Effective config resolves successfully
- **WHEN** a declared and policy-allowed `config.resolve_effective` command is
  invoked with valid schema and selectors
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the
  config service provider
- **AND** it SHALL emit sanitized policy, service-call, validation, result, and
  replay events with source hashes, layer order, schema ids, redaction summary,
  and stable trace identifiers

#### Scenario: Raw secret value is rejected
- **WHEN** `config.get`, `config.resolve_effective`, `config.reload`, or
  `config.validate` encounters a raw secret value instead of an allowed secret
  reference
- **THEN** Macaca SHALL return a typed `secret_value_forbidden` or denied result
- **AND** traces, audit records, snapshots, diagnostics, and SDK examples SHALL
  NOT include the raw secret value

#### Scenario: Config reload fails validation
- **WHEN** `config.reload` loads new source data that violates the declared schema
- **THEN** Macaca SHALL keep the prior effective config unless policy explicitly
  allows partial degradation
- **AND** it SHALL emit validation_failed diagnostics with bounded field paths and
  redacted values

### Requirement: Config values SHALL be layered, typed, and provenance-aware

`pack.foundation.config.v1` SHALL expose explicit DTOs for keys, values, schemas,
layers, selectors, source refs, validation reports, provenance, redaction, watch
events, and snapshots. It SHALL NOT expose raw provider config handles to
applications.

#### Scenario: Developer explains a value
- **WHEN** an application invokes `config.explain_provenance` for a key
- **THEN** Macaca SHALL return selected layer, overridden layers, source refs,
  validation result, redaction summary, and compatibility metadata
- **AND** it SHALL not reveal raw secret values or provider-private payloads

#### Scenario: Unsupported selector is used
- **WHEN** an application requests a selector or profile that is not declared or
  provider-supported
- **THEN** Macaca SHALL return a typed `unsupported_selector` or denied result
- **AND** OS code SHALL NOT branch on hardcoded environment/profile names

### Requirement: Config watch, snapshot, and replay SHALL be bounded and sanitized

Macaca SHALL bound and sanitize watch events, reload diagnostics, validation
reports, snapshots, traces, and audit records for `pack.foundation.config.v1`.

#### Scenario: Config watch stream is started and cancelled
- **WHEN** `config.watch` starts a watch stream
- **THEN** Macaca SHALL reserve stream resources, emit a watch-start event, and
  return bounded watch events with source/version metadata
- **AND** cancellation, timeout, validation failure, provider failure, and
  session shutdown SHALL release resources and emit terminal watch events

#### Scenario: Effective config snapshot is recorded
- **WHEN** `config.snapshot` records effective config
- **THEN** the snapshot SHALL include descriptor version, provider class, schema
  ids, source hashes, effective config hash, policy template hash, validation
  result, redaction summary, and replay references
- **AND** it SHALL exclude raw secret values, unbounded config values, raw
  environment dumps, credentials, prompts, manifests, package bytes, private
  keys, and raw provider payloads

### Requirement: Config implementation SHALL preserve Macaca boundaries

The config implementation SHALL remain owned by the config system service and
replaceable providers. The microkernel, SDK, shells, and generic application
framework SHALL remain provider-neutral and free of application-specific config
routing.

#### Scenario: Boundary gates scan config implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path
  gates scan the implementation
- **THEN** they SHALL find no concrete config provider imports in the
  microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned
  service registrations and typed service commands

#### Scenario: WASM app uses config host imports
- **WHEN** a WASM application invokes config host imports
- **THEN** the host imports SHALL route through the same `config.*` service
  command path used by SDK and YAML applications
- **AND** WASM code SHALL NOT receive raw environment handles or bypass policy

### Requirement: Config pack completion SHALL include developer documentation

The `pack.foundation.config.v1` proposal SHALL NOT be marked complete until the
detailed developer guide exists and is linked from SDK discovery metadata.

#### Scenario: Developer reads config pack documentation
- **WHEN** a developer opens `docs/developer-packs/foundation/config.md`
- **THEN** the guide SHALL document manifest declaration, config/code separation,
  schema model, key model, value types, layers, selectors/profiles, precedence,
  validation, provenance, watch/reload, snapshots, redaction, secret references,
  permission scopes, policy defaults, command DTOs, result DTOs, error DTOs,
  unavailable diagnostics, provider replacement, trace/audit fields, and examples
- **AND** examples SHALL use generic data and SHALL NOT hardcode application
  business logic, provider names, credentials, or workflow-specific config keys
