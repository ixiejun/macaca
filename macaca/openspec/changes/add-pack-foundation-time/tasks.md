## 1. Supplier API Research And Scope

- [x] 1.1 Read and summarize Apple Foundation Date, Calendar, DateComponents,
  TimeZone, DateFormatter, and timer concepts.
- [x] 1.2 Read and summarize Java `java.time` Clock, Instant, Duration,
  ZonedDateTime, ZoneId, and immutable value principles.
- [x] 1.3 Read and summarize Android AlarmManager and WorkManager behavior for
  exact/inexact alarms, background scheduling limits, retry/backoff, and host
  restrictions.
- [x] 1.4 Read and summarize JavaScript Date, Intl.DateTimeFormat, and Temporal
  concepts for formatting, parsing, explicit instant/plain/zoned values, and
  duration arithmetic.
- [x] 1.5 Read and summarize POSIX/system clock behavior for wall-clock versus
  monotonic clocks, resolution, drift, and timer deadlines.
- [x] 1.6 Convert the supplier comparison into Macaca-owned abstractions and
  explicitly reject provider-native clock/timer handles.
- [x] 1.7 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.foundation.time.v1` descriptor metadata: lifecycle,
  stability, service ids, command namespace, command schemas, permission scopes,
  policy template, resource template, SDK metadata, docs link, health, snapshot,
  and unavailable diagnostics.
- [x] 2.2 Define command DTOs for `time.now`, `time.monotonic_now`,
  `time.clock_health`, `time.duration_between`, `time.add_duration`,
  `time.convert_timezone`, `time.resolve_timezone`, `time.calendar_convert`,
  `time.format`, `time.parse`, `time.create_timer`, `time.cancel_timer`,
  `time.inspect_timer`, and `time.evaluate_deadline`.
- [x] 2.3 Define shared DTOs for instant, monotonic instant, duration, timezone
  ref, calendar ref, locale ref, format spec, timer ref, deadline spec, clock
  source, exactness hint, and stable descriptor hashes.
- [x] 2.4 Define result/error DTOs for success, denied, invalid_time,
  invalid_timezone, invalid_calendar, invalid_locale, parse_failed, overflow,
  unsupported, timer_not_found, quota_exceeded, unavailable, and
  provider_failure.
- [x] 2.5 Add schema compatibility tests and stable hash tests for command,
  result, health, snapshot, provider capability, and unavailable DTOs.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement manifest declaration validation for required/optional
  `pack.foundation.time.v1`.
- [x] 3.2 Validate scopes: `time.read`, `time.monotonic`, `time.timezone`,
  `time.calendar`, `time.format`, `time.parse`, `time.timer`, and
  `time.deadline`.
- [x] 3.3 Add policy checks for clock mode, timer count, maximum timer duration,
  exactness request, mock-clock context, locale/calendar availability, timezone
  data availability, and provider capability.
- [x] 3.4 Add resource reservations before timer creation and release resources
  on fire, cancel, timeout, provider failure, and session shutdown.
- [ ] 3.5 Add tests proving denied, unavailable, quota, unsupported, and invalid
  timezone/calendar paths do not invoke a concrete provider where they should be
  rejected before side effects.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Define the time service trait/provider interface behind the service
  runtime.
- [x] 4.2 Implement unavailable provider behavior for absent time service,
  unsupported monotonic clock, unsupported exact timers, missing timezone data,
  missing locale data, and disabled mock clock.
- [x] 4.3 Implement deterministic mock/frozen clock provider for contract,
  replay, and SDK examples.
- [x] 4.4 Implement or bind host wall-clock, monotonic clock, formatting/parsing,
  timezone, and timer providers with bounded health diagnostics.
- [x] 4.5 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, timer
  cleanup, and provider capability reports.

## 5. SDK, WASM ABI, And Application Framework

- [x] 5.1 Extend SDK discovery with pack metadata, command schemas, clock modes,
  timezone data version, locale/calendar support, permissions, policy templates,
  timer exactness, provider availability, health, diagnostics, and docs link.
- [x] 5.2 Add SDK command builders for every `time.*` command; builders must only
  produce canonical traced service calls.
- [x] 5.3 Add SDK helpers for monotonic timeout, timezone conversion, localized
  formatting, strict parsing, timer create/cancel, deadline evaluation, and mock
  clock setup in test contexts.
- [x] 5.4 Extend effective capability projection so applications can inspect
  callable commands, denied commands, unavailable timezone/calendar/timer
  features, provider capability flags, and replay references.
- [x] 5.5 Expose WASM host imports only for declared callable time commands and
  route every import through the service runtime path.
- [x] 5.6 Add app-framework tests proving YAML, WASM, GenUI, and headless apps all
  use the same time execution path.

## 6. Trace, Audit, Replay, And Gates

- [ ] 6.1 Emit sanitized events for declaration, admission, policy, resource,
  service calls, timer lifecycle, clock health, success, failure, denied, and
  unavailable states.
- [ ] 6.2 Add audit redaction tests proving raw user content, prompts, manifests,
  package bytes, credentials, private keys, provider payloads, and unbounded
  output do not enter observability surfaces.
- [ ] 6.3 Add replay tests proving every time command is trace-addressable and can
  reconstruct wall-clock/monotonic decisions with clock source and timezone data
  version.
- [x] 6.4 Add dependency-boundary tests proving kernel, SDK, shells, and
  application framework do not import concrete time providers.
- [x] 6.5 Add no-direct-provider-call gates proving SDK helpers and WASM host
  imports cannot bypass service runtime.
- [ ] 6.6 Run `openspec validate add-pack-foundation-time --strict`, targeted
  cargo tests, dependency-boundary gates, file-size gates, and audit replay
  checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/foundation/time.md`.
- [x] 7.2 Document purpose, manifest declaration, wall-clock versus monotonic
  time, instant/duration/timezone/calendar DTOs, permissions, policy defaults,
  timer quotas, exactness diagnostics, command DTOs, result DTOs, error DTOs,
  formatting/parsing, timers, deadlines, mock clocks, unavailable diagnostics,
  and provider replacement.
- [x] 7.3 Add minimal examples for current UTC time, monotonic timeout, timezone
  conversion, localized formatting, strict timestamp parsing, timer create/cancel,
  inexact timer diagnostics, and mock clock test usage.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack
  catalog index before marking this proposal complete.
