# Foundation Time Pack Design

## Context

`pack.foundation.time.v1` provides a generic time capability for Macaca
applications. It must support wall-clock reads, monotonic reads, timezone and
calendar conversion, formatting/parsing, timers, deadline checks, and deterministic
mock clocks without becoming the workflow scheduler itself.

The pack is a low-level foundation service. Higher-level scheduling, task
retries, workflow alarms, approvals, and recovery semantics belong to workflow
and autonomy services. This pack supplies time primitives and bounded timer
capabilities that those services may use through service boundaries.

## Supplier API Comparison

| Source API family | Relevant concepts | Macaca abstraction |
| --- | --- | --- |
| Apple Foundation | `Date`, `Calendar`, `DateComponents`, `TimeZone`, `DateFormatter`, timers | instant DTOs, calendar/timezone conversion, formatting/parsing commands, timer commands |
| Java `java.time` | `Clock`, `Instant`, `Duration`, `ZonedDateTime`, `ZoneId`, immutable values | injected clock provider, instant/duration/zoned DTOs, deterministic mock clock, timezone ids |
| Android AlarmManager / WorkManager | exact/inexact alarms, background scheduling limits, retry/backoff | timer/deadline commands with exactness hints, host capability diagnostics, handoff to workflow scheduler |
| JavaScript Date / Intl / Temporal | locale formatting, explicit instant/plain/zoned concepts, duration arithmetic | locale-aware format/parse commands, explicit value categories, timezone/calendar metadata |
| POSIX/system clocks | wall-clock versus monotonic clocks, resolution, skew, timers | wall clock command, monotonic command, resolution/skew health, monotonic deadlines |

Design conclusion: Macaca should expose explicit time DTOs and timer commands,
while hiding host-specific clock APIs and host-specific timer facilities behind
replaceable providers.

## Goals

- Provide wall-clock instant, monotonic instant, duration math, timezone lookup,
  timezone conversion, calendar conversion, format, parse, timer, deadline, and
  clock health operations.
- Clearly distinguish wall-clock time from monotonic time and replay/test clocks.
- Support deterministic mock clocks and frozen/advanced clocks for tests.
- Support provider health reporting for resolution, drift, skew, timezone data
  version, calendar support, timer support, and exactness limitations.
- Return structured diagnostics when exact timers, timezone data, locale data, or
  calendar systems are unavailable.

## Non-Goals

- No workflow scheduling state machine.
- No retry policy ownership beyond time primitive support.
- No calendar event management; that belongs to communication calendar pack.
- No location-based timezone inference; that belongs to location timezone pack.
- No provider-specific host timer, OS alarm, or browser timer handles in SDK.
- No raw user content, prompts, manifests, credentials, or provider payloads in
  logs/traces.

## Ownership And Boundaries

- Pack id: `pack.foundation.time.v1`.
- Family: `foundation`.
- Service owner: time system service.
- Provider examples: host clock provider, monotonic provider, deterministic mock
  provider, frozen test clock provider, unavailable provider.
- SDK surface: `sdk.packs.foundation.time`.
- Command namespace: `time.*`.
- Microkernel ownership: identity, policy facade, service-call evidence,
  trace/audit primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, effective capability projection, WASM ABI import exposure.
- Runtime-host ownership: host clock/timer adapter registration, decorators,
  clock health snapshots, and unavailable provider composition.

## Command Surface

| Command | Supplier analogs | DTO notes | Side effects |
| --- | --- | --- | --- |
| `time.now` | Apple `Date`, Java `Instant.now`, JS `Date.now` | clock source, precision request, timezone optional | No |
| `time.monotonic_now` | POSIX monotonic clock, Java injected `Clock` | monotonic tick, resolution, process/host scope | No |
| `time.clock_health` | system clock diagnostics | resolution, skew, drift, timezone data version, provider class | No |
| `time.duration_between` | Java `Duration`, Temporal durations | start/end instants, monotonic/wall-clock mode | No |
| `time.add_duration` | date/time arithmetic | instant, duration, calendar/timezone rules | No |
| `time.convert_timezone` | Foundation `TimeZone`, Java `ZonedDateTime` | instant/local date-time, source/target zone ids | No |
| `time.resolve_timezone` | zone database lookup | zone id, aliases, data version | No |
| `time.calendar_convert` | Foundation `Calendar`, Java calendars | calendar id, components, timezone, locale | No |
| `time.format` | `DateFormatter`, `Intl.DateTimeFormat` | instant/zoned value, locale, style, calendar | No |
| `time.parse` | date parsers / Temporal parsing | text, expected format, timezone/calendar defaults | No |
| `time.create_timer` | timers, AlarmManager, WorkManager hints | deadline, monotonic/wall-clock mode, exactness, max delay | Yes |
| `time.cancel_timer` | timer cancellation | timer id, reason | Yes |
| `time.inspect_timer` | timer diagnostics | timer id, state, due time, exactness, provider diagnostics | No |
| `time.evaluate_deadline` | deadline comparison | deadline, current clock mode, grace period | No |

## DTO Model

Core DTOs:

- `TimeInstant`: UTC instant with precision metadata and source clock id.
- `MonotonicInstant`: provider-local monotonic tick with resolution and boot/session scope.
- `TimeDuration`: signed duration with unit, precision, and overflow behavior.
- `TimeZoneRef`: IANA zone id, fixed offset, alias, data version, unavailable reason.
- `CalendarRef`: ISO-8601 default plus explicitly supported calendar ids.
- `LocaleRef`: locale id and formatting capability metadata.
- `TimeFormatSpec`: style, pattern class, locale, timezone, calendar, strictness.
- `TimerRef`: opaque timer id, deadline, clock mode, exactness hint, state, trace binding.
- `DeadlineSpec`: instant or monotonic target, grace period, timeout behavior.
- `TimeError`: denied, invalid_time, invalid_timezone, invalid_calendar,
  invalid_locale, parse_failed, overflow, unsupported, timer_not_found,
  quota_exceeded, unavailable, provider_failure.

## Permission And Policy Model

Permission scopes:

- `time.read`
- `time.monotonic`
- `time.timezone`
- `time.calendar`
- `time.format`
- `time.parse`
- `time.timer`
- `time.deadline`

Policy rules:

- Read-only time commands are low-risk but still require trace context.
- Timer creation requires resource reservation, timeout bounds, maximum active
  timer count, cancellation behavior, and provider exactness diagnostics.
- Wall-clock deadlines must record timezone/calendar assumptions.
- Monotonic deadlines are preferred for internal timeouts and retries.
- Exact timers require explicit provider capability and may return `unsupported`
  or `degraded` when the host only supports inexact scheduling.
- Mock/frozen clocks must be restricted to test or replay contexts by policy.

## SDK And Developer Documentation

SDK discovery returns command schemas, supported clock modes, timezone data
version, locale/calendar support, timer exactness support, permission scopes,
policy templates, provider availability, health, examples, docs link, and
unavailable diagnostics.

Required developer guide:

- Path: `docs/developer-packs/foundation/time.md`.
- Content: wall-clock versus monotonic time, instant/duration/timezone/calendar
  DTOs, formatting/parsing, timers, deadlines, mock clocks, permissions, policy,
  unavailable diagnostics, provider replacement, trace/audit fields, and examples.
- Examples: read current UTC time, compute a monotonic timeout, convert timezone,
  format localized time, parse strict timestamp, create/cancel timer, inspect
  inexact timer diagnostics, and use a mock clock in tests.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `time_pack_declared`
- `time_pack_admission_validated`
- `time_pack_policy_decision`
- `time_pack_service_call_requested`
- `time_pack_service_call_succeeded`
- `time_pack_service_call_failed`
- `time_pack_timer_created`
- `time_pack_timer_fired`
- `time_pack_timer_cancelled`
- `time_pack_clock_health_recorded`
- `time_pack_unavailable`

Events include pack id, service id, command name, trace id, app/session/task
identifiers, clock mode, timezone id hash when sensitive, provider class,
resolution, exactness, timer state, latency, bounded resource counters, and
bounded error code.

Health checks include wall-clock availability, monotonic availability,
resolution, drift/skew indicators, timezone database version, calendar support,
locale formatting support, max active timers, exact timer support, mock clock
availability, and unavailable reasons.

Snapshots include descriptor version, provider class, clock health, active timer
count, timer ids/hashes, policy template hash, timezone data version, and replay
references. Snapshots do not embed raw application payloads.

## Implementation Slices

1. Contract slice: descriptor, command schemas, instant/duration/timezone/timer
   DTOs, result/error DTOs, health/snapshot DTOs, stable hashes.
2. Admission slice: permission validation, timer policy validation, mock-clock
   context validation, service mapping validation.
3. Service slice: time service trait/provider interface, unavailable provider,
   deterministic mock/frozen provider, host clock/timer provider.
4. SDK slice: discovery, typed command builders, timer helpers, deadline helpers,
   mock clock helpers for tests, unavailable diagnostics, docs link.
5. WASM/app-runtime slice: expose only declared callable time imports through
   service runtime; no raw host timer handles.
6. Observability slice: trace/audit events, timer lifecycle events, health
   snapshots, replay tests.
7. Developer-docs slice: complete `docs/developer-packs/foundation/time.md` and
   link it from catalog metadata.

## Design Patterns

- **Facade**: SDK exposes time helpers and command builders only.
- **Command**: every operation is a typed command/result.
- **Adapter/Bridge**: host clock, monotonic clock, mock clock, timer, and
  unavailable providers adapt to one contract.
- **Strategy**: clock source, timezone data provider, timer exactness, and mock
  behavior are replaceable.
- **Decorator**: trace, policy, resource, timer quota, and redaction wrap calls.
- **Specification**: clock mode, timezone, calendar, timer, and mock-clock rules
  are executable validators.
- **Observer**: timer lifecycle, health, audit, and service-call events are
  subscribable.
- **Memento**: snapshots and effective capability reports preserve replay state.

## Risks And Mitigations

- Risk: wall-clock changes break deadlines.
  Mitigation: distinguish monotonic deadlines from wall-clock instants and record
  clock mode in DTOs and audit.
- Risk: exact timer promises exceed host capability.
  Mitigation: provider capability reports, exactness hints, and structured
  unsupported/degraded diagnostics.
- Risk: time pack absorbs workflow scheduling semantics.
  Mitigation: keep workflow state machines in workflow/autonomy services; time
  pack only owns primitives.
- Risk: timezone/calendar data drift changes results.
  Mitigation: record timezone data version, calendar id, locale, and formatting
  spec in results and snapshots.
- Risk: tests become nondeterministic.
  Mitigation: deterministic mock/frozen clocks restricted by policy and exposed
  through the same service command path.
