# Change: Add Foundation Time Pack

## Why

Developers need `pack.foundation.time.v1` as a trustworthy, provider-neutral
time capability. Applications need current time, monotonic time, duration math,
timezone conversion, calendar calculation, formatting/parsing, timers, deadlines,
and schedule hints without hardcoding host clocks or bypassing policy.

Time is foundational for workflows, retries, TTLs, task deadlines, audit
timestamps, billing windows, calendar operations, and recovery. If each
application reads host time or schedules timers differently, Macaca cannot
replay, audit, bound, or recover autonomous execution reliably.

## Supplier And Platform API Research

The proposal is derived from a capability-by-capability comparison of mature
time APIs:

- Apple Foundation `Date`, `Calendar`, `DateComponents`, `TimeZone`,
  `DateFormatter`, and timers: wall-clock instants, calendar math, timezone
  rules, formatting/parsing, and run-loop scheduled timers.
- Java `java.time`: `Clock`, `Instant`, `Duration`, `ZonedDateTime`, `ZoneId`,
  immutable time values, injected clocks for tests, and timezone-aware
  conversion.
- Android `AlarmManager` and WorkManager: exact versus inexact alarms,
  background execution constraints, retry/backoff policy, and host scheduling
  limits.
- JavaScript `Date`, `Intl.DateTimeFormat`, and Temporal concepts: locale-aware
  formatting, explicit instant/plain/zoned time objects, duration arithmetic,
  and time-zone/calendar representation.
- POSIX/system clocks: wall-clock time versus monotonic clocks, clock drift,
  resolution, and timer deadlines.

Macaca borrows the stable concepts, not provider APIs:

- distinguish wall-clock time from monotonic time;
- model instants, durations, timezones, calendars, and deadlines explicitly;
- make timers and schedule hints service commands, not hidden app loops;
- expose clock resolution, skew, and provider health;
- support deterministic mock clocks for tests and replay;
- normalize scheduling limitations into structured diagnostics.

## What Changes

- Define `pack.foundation.time.v1` as the canonical app-facing time pack.
- Add an industrial command surface covering current instant, monotonic instant,
  duration math, timezone lookup/conversion, calendar conversion, formatting,
  parsing, timer create/cancel, deadline evaluation, and clock health.
- Define provider-neutral DTO requirements for instant, monotonic timestamp,
  duration, timezone id, calendar id, locale, format style, timer id, deadline,
  clock source, resolution, skew, and unavailable diagnostics.
- Define permission scopes for read time, timezone/calendar conversion, locale
  formatting, timers, and deadline scheduling.
- Require a detailed developer guide under `docs/developer-packs/foundation/time.md`
  before this proposal can be marked complete.
- Keep implementation ownership in a time system service; kernel, SDK, shells,
  and application framework remain provider-neutral.

## Impact

- Affected specs: `pack-foundation-time`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs, descriptor validators, application
  admission, SDK discovery, SDK command helpers, time service provider,
  mock/unavailable providers, trace/audit event schema, replay tests, and
  dependency-boundary gates.
- Non-goals: provider-specific clock APIs in SDK, direct shell timers as OS
  semantics, app-specific schedule workflows, or task scheduling/retry ownership
  inside the time pack.
