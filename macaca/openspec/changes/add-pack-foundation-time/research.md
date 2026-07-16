# Foundation Time Pack Research

## Purpose

This note records supplier/API research for `pack.foundation.time.v1`. The pack
must provide provider-neutral wall-clock, monotonic-clock, calendar, timezone,
formatting, parsing, timer, deadline, and mock-clock capabilities without
exposing host clock handles or provider-specific timer APIs.

## Source Baseline

- Apple `DateComponents`:
  <https://developer.apple.com/documentation/foundation/datecomponents>
- Apple `Calendar.dateComponents(in:from:)`:
  <https://developer.apple.com/documentation/foundation/calendar/datecomponents%28in%3Afrom%3A%29>
- Apple `DateFormatter`:
  <https://developer.apple.com/documentation/foundation/dateformatter>
- Java `Instant`:
  <https://docs.oracle.com/javase/8/docs/api/java/time/Instant.html>
- Java `ZonedDateTime`:
  <https://docs.oracle.com/javase/8/docs/api/java/time/ZonedDateTime.html>
- Android alarms:
  <https://developer.android.com/develop/background-work/services/alarms>
- Android persistent work / WorkManager:
  <https://developer.android.com/develop/background-work/background-tasks/persistent>
- JavaScript `Date`:
  <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date>
- JavaScript `Intl.DateTimeFormat`:
  <https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/DateTimeFormat>
- TC39 Temporal `ZonedDateTime`:
  <https://tc39.es/proposal-temporal/docs/zoneddatetime.html>
- TC39 Temporal string parsing/serialization:
  <https://tc39.es/proposal-temporal/docs/strings.html>
- POSIX `clock_gettime` / `clock_getres`:
  <https://pubs.opengroup.org/onlinepubs/009695399/functions/clock_getres.html>
- POSIX `<time.h>` clock identifiers:
  <https://pubs.opengroup.org/onlinepubs/009695399/basedefs/time.h.html>

## Apple Foundation Summary

Apple Foundation separates instants, calendar components, time zones,
formatting/parsing, and timers. Macaca should borrow the separation, not the
Foundation object model:

- `Date`-like instants represent points in time and should be distinct from
  localized calendar components.
- `Calendar`, `DateComponents`, and `TimeZone` show that calendar arithmetic
  must carry calendar and timezone assumptions explicitly.
- `DateFormatter` shows formatting/parsing is locale-sensitive and should be a
  service command with locale, calendar, timezone, and strictness metadata.
- Timer concepts belong behind a service state machine with quota and
  cancellation behavior. They must not surface run-loop or platform timer
  handles in SDK/ABI contracts.

## Java `java.time` Summary

Java `java.time` contributes a strong value-object model:

- `Instant`, `Duration`, `ZoneId`, and `ZonedDateTime` are immutable data
  categories that keep instants, durations, and zone-aware representations
  explicit.
- `Clock` supports injected clocks, which maps to Macaca deterministic mock,
  frozen, and replay clock providers.
- Timezone-aware values record the zone rules needed for correct calendar math.
  Macaca results should include timezone-data version or provider capability
  evidence when conversions are not purely fixed-offset.
- Value identity should not matter. Macaca DTOs should be immutable value
  records with stable descriptor hashing and serde-compatible schema evolution.

## Android AlarmManager / WorkManager Summary

Android distinguishes immediate/background work from exact or inexact alarms and
enforces host restrictions:

- Inexact alarms are the normal low-cost scheduling primitive; exact alarms are
  privileged and should require explicit provider support and policy approval.
- WorkManager-style persistent work contributes retry/backoff and background
  constraints, but those are workflow semantics. `pack.foundation.time.v1`
  should expose timer/deadline primitives and leave workflow retries to the
  workflow packs/autonomy services.
- Host restrictions must be reported as structured diagnostics such as
  `unsupported`, `quota_exceeded`, `unavailable`, or `provider_failure`.
- Timer creation must reserve resources and release them on fire, cancel,
  timeout, failure, and session shutdown.

## JavaScript Date / Intl / Temporal Summary

JavaScript exposes legacy `Date`, locale-aware `Intl.DateTimeFormat`, and the
Temporal proposal's explicit instant/plain/zoned model:

- `Date` is epoch-millisecond based and timezone-agnostic as an instant
  representation. Macaca should not expose JavaScript's mutable Date object or
  implementation-specific parsing quirks.
- `Intl.DateTimeFormat` maps to `time.format` with locale, calendar, timezone,
  and style metadata.
- Temporal's explicit instant, plain date/time, zoned date/time, calendar, and
  timezone types support Macaca's requirement for explicit value categories.
- Parsing should be strict and typed. Provider-native parser behavior must be
  normalized into `parse_failed`, `invalid_timezone`, `invalid_calendar`,
  `invalid_locale`, or `overflow`.

## POSIX / System Clock Summary

POSIX clock APIs establish the core clock taxonomy:

- `CLOCK_REALTIME` is wall-clock time and can move due to clock setting or
  synchronization. It is appropriate for timestamps but risky for internal
  deadlines.
- `CLOCK_MONOTONIC` is non-settable and does not jump backward after its
  unspecified origin. It is the preferred basis for timeouts and retry
  deadlines.
- Clock resolution is observable via `clock_getres`; Macaca should expose clock
  resolution, skew/drift diagnostics, and provider health.
- POSIX timer IDs and signal/thread notification behavior are provider details.
  Macaca should expose opaque `TimerRef` records and service events instead.

## Macaca-Owned Abstractions

`pack.foundation.time.v1` should define these provider-neutral concepts:

- `TimeInstant`: UTC instant with precision, clock source, timezone-data
  version when relevant, and replay metadata.
- `MonotonicInstant`: monotonic tick with provider-local origin class,
  resolution, session/host scope, and comparison rules.
- `TimeDuration`: signed duration with unit, precision, arithmetic mode, and
  overflow behavior.
- `TimeZoneRef`: IANA zone id, fixed offset, alias, data version, unavailable
  reason, and provider attribution.
- `CalendarRef`: supported calendar id, default ISO-8601 behavior, locale
  binding, and unavailable diagnostics.
- `LocaleRef`: locale id, formatting capability, fallback/degraded state, and
  provider attribution.
- `TimeFormatSpec`: style or bounded pattern class, locale, timezone, calendar,
  strictness, and redaction class.
- `TimerRef`: opaque timer id, deadline, clock mode, exactness hint, state,
  policy binding, resource reservation, and trace binding.
- `DeadlineSpec`: wall-clock or monotonic target, grace period, timeout policy,
  and replay evidence.
- `TimeProviderCapability`: wall-clock support, monotonic support, timezone data
  version, calendar/locale support, maximum active timers, exactness support,
  mock-clock availability, health, and unavailable reasons.

## Rejected API Leakage

Macaca must not expose these provider-native shapes as stable SDK/ABI contracts:

- Apple Foundation `Date`, `Calendar`, `DateComponents`, `DateFormatter`, run
  loop timers, or platform timer handles.
- Java `Clock`, `Instant`, `ZonedDateTime`, `ZoneId`, formatter objects, or Java
  exception types.
- Android `AlarmManager`, `PendingIntent`, WorkManager `WorkRequest`,
  constraints, retry policy objects, or host permission names.
- JavaScript mutable `Date`, `Intl.DateTimeFormat` instances, Temporal objects,
  browser timer ids, or implementation-specific parser behavior.
- POSIX `clockid_t`, `timer_t`, signal delivery modes, `timespec` structs, errno
  values, or host-specific clock identifiers.

All operations must enter through typed Macaca service commands with trace
context, policy checks, resource reservations for timers, structured result
envelopes, sanitized audit events, unavailable provider behavior, and provider
replacement support.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
