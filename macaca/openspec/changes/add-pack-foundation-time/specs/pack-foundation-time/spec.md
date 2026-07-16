## ADDED Requirements

### Requirement: Macaca SHALL provide a supplier-grade Foundation Time Pack

Macaca SHALL provide `pack.foundation.time.v1` as a provider-neutral,
serviceized time pack for wall-clock reads, monotonic reads, duration math,
timezone lookup/conversion, calendar conversion, formatting/parsing, timer
creation/cancellation, deadline evaluation, and clock health diagnostics.

#### Scenario: Application declares time access
- **WHEN** an application declares `pack.foundation.time.v1` with required
  permission scopes
- **THEN** admission SHALL validate pack id, lifecycle, permission scopes, policy
  bounds, service mappings, command schemas, and provider capability requirements
- **AND** admission SHALL produce an effective capability report with callable,
  denied, unsupported, and unavailable command states

#### Scenario: Required time provider is unavailable
- **WHEN** `pack.foundation.time.v1` is required but no admitted provider can
  satisfy declared commands
- **THEN** application readiness SHALL be blocked with structured unavailable
  diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to direct host clock
  calls, or fake success

#### Scenario: Optional exact timer support is unavailable
- **WHEN** `pack.foundation.time.v1` is optional or exact timer support is
  optional and the active provider only supports inexact timers
- **THEN** admission and SDK discovery SHALL report degraded or unsupported timer
  exactness
- **AND** SDK helpers SHALL refuse exact timer commands unless policy and provider
  capabilities allow them

### Requirement: Time commands SHALL use typed canonical service calls

Every `time.*` operation SHALL be represented as a typed command/result DTO and
SHALL traverse the canonical service runtime path with trace, policy, resource,
entitlement, health, snapshot, and structured error behavior.

#### Scenario: Monotonic timeout is evaluated
- **WHEN** a declared and policy-allowed `time.monotonic_now` or
  `time.evaluate_deadline` command is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the
  time service provider
- **AND** it SHALL emit sanitized policy, service-call, result, and replay events
  with clock mode, resolution, provider class, and stable trace identifiers

#### Scenario: Timer creation is denied before side effects
- **WHEN** `time.create_timer` is rejected by timer quota, max duration, exactness
  policy, mock-clock policy, entitlement, or resource checks
- **THEN** Macaca SHALL return a typed denied, quota, or unsupported result before
  creating a host timer
- **AND** audit evidence SHALL include bounded reason codes and timer policy
  metadata

#### Scenario: Timezone data is missing
- **WHEN** `time.convert_timezone`, `time.resolve_timezone`, `time.calendar_convert`,
  `time.format`, or `time.parse` requires unavailable timezone, locale, or
  calendar data
- **THEN** Macaca SHALL return a typed unavailable or unsupported result
- **AND** SDK discovery SHALL report the affected option as non-callable for the
  current effective capability set

### Requirement: Time values SHALL distinguish wall-clock, monotonic, and replay clocks

`pack.foundation.time.v1` SHALL expose explicit DTOs for wall-clock instants,
monotonic instants, durations, timezones, calendars, locales, timers, and
deadlines. It SHALL NOT expose raw host timer handles or provider-native clock
objects to applications.

#### Scenario: Application requests current wall-clock time
- **WHEN** an application invokes `time.now`
- **THEN** Macaca SHALL return a UTC instant with precision, source clock id,
  provider class, and trace binding
- **AND** the result SHALL NOT be used as a monotonic timeout unless explicitly
  converted through supported deadline commands

#### Scenario: Application requests a mock clock outside test context
- **WHEN** an application requests mock or frozen clock behavior outside a policy
  approved test/replay context
- **THEN** Macaca SHALL return a typed denied result
- **AND** it SHALL NOT replace the active provider clock

### Requirement: Time timers, health, snapshots, and replay SHALL be bounded and auditable

Macaca SHALL bound and sanitize timer lifecycle, clock health, snapshots,
timezone/calendar diagnostics, traces, and audit records for
`pack.foundation.time.v1`.

#### Scenario: Timer is created and cancelled
- **WHEN** `time.create_timer` creates a timer
- **THEN** Macaca SHALL reserve timer resources, emit a timer-created event, and
  return an opaque timer reference
- **AND** cancellation, firing, timeout, provider failure, and session shutdown
  SHALL release resources and emit terminal timer events

#### Scenario: Clock health is recorded
- **WHEN** `time.clock_health` or service snapshot records clock health
- **THEN** Macaca SHALL include provider class, resolution, monotonic support,
  timezone data version, calendar/locale support, timer exactness support, max
  timer count, and unavailable reasons
- **AND** it SHALL exclude raw application payloads and provider-private data

#### Scenario: Replay reconstructs time decisions
- **WHEN** an audit replay inspects a time-dependent decision
- **THEN** replay evidence SHALL include command name, clock mode, instant or
  monotonic tick metadata, timezone data version when relevant, policy decision,
  and trace identifiers
- **AND** replay SHALL NOT require re-reading the live host clock to understand
  the original decision

### Requirement: Time implementation SHALL preserve Macaca boundaries

The time implementation SHALL remain owned by the time system service and
replaceable providers. The microkernel, SDK, shells, and generic application
framework SHALL remain provider-neutral and free of application-specific time
routing.

#### Scenario: Boundary gates scan time implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path
  gates scan the implementation
- **THEN** they SHALL find no concrete time provider imports in the microkernel,
  SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned
  service registrations and typed service commands

#### Scenario: WASM app uses time host imports
- **WHEN** a WASM application invokes time host imports
- **THEN** the host imports SHALL route through the same `time.*` service command
  path used by SDK and YAML applications
- **AND** WASM code SHALL NOT receive raw host timer handles or bypass policy

### Requirement: Time pack completion SHALL include developer documentation

The `pack.foundation.time.v1` proposal SHALL NOT be marked complete until the
detailed developer guide exists and is linked from SDK discovery metadata.

#### Scenario: Developer reads time pack documentation
- **WHEN** a developer opens `docs/developer-packs/foundation/time.md`
- **THEN** the guide SHALL document manifest declaration, wall-clock versus
  monotonic time, instant/duration/timezone/calendar DTOs, permission scopes,
  policy defaults, timer quotas, exactness diagnostics, command DTOs, result
  DTOs, error DTOs, formatting/parsing, timers, deadlines, mock clocks,
  unavailable diagnostics, provider replacement, trace/audit fields, and examples
- **AND** examples SHALL use generic data and SHALL NOT hardcode application
  business logic, provider names, credentials, or workflow-specific timers
