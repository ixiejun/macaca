# Foundation Time Pack

`pack.foundation.time.v1` provides provider-neutral temporal primitives for
Macaca applications. It covers wall-clock reads, monotonic reads, clock health,
duration arithmetic, timezone conversion, calendar conversion, formatting,
parsing, timers, deadline evaluation, frozen test clocks, and unavailable
diagnostics without exposing host clock or timer handles.

## Manifest Declaration

Declare the pack in an application service contract:

```yaml
service_contract:
  optional_packs:
    - pack.foundation.time.v1
```

Use `required_packs` only when the application cannot run without a registered
time provider. When no provider is installed, admission returns an explicit
`time_provider_not_installed` diagnostic instead of faking clock or timer
support.

## Permissions

The pack defines these provider-neutral scopes:

- `time.read`: wall-clock reads and time inspection.
- `time.monotonic`: monotonic elapsed-time reads.
- `time.timezone`: timezone resolution and conversion.
- `time.calendar`: calendar conversion.
- `time.format`: localized or canonical formatting.
- `time.parse`: strict timestamp parsing.
- `time.timer`: timer create, cancel, and inspect operations.
- `time.deadline`: deadline evaluation.

## Commands

- `time.now`: read the current wall-clock instant with optional timezone and
  calendar context.
- `time.monotonic_now`: read a monotonic instant for elapsed-time decisions.
- `time.clock_health`: inspect wall-clock, monotonic, timezone, locale, and
  timer availability.
- `time.duration_between`: compute a signed duration between two instants.
- `time.add_duration`: add a duration with an explicit overflow policy.
- `time.convert_timezone`: convert an instant into a target timezone.
- `time.resolve_timezone`: resolve a timezone query with an optional region
  hint.
- `time.calendar_convert`: convert an instant into a target calendar.
- `time.format`: format an instant with a format spec, locale, and timezone.
- `time.parse`: parse a timestamp artifact through a strict format spec.
- `time.create_timer`: create a session-bound timer with exactness metadata.
- `time.cancel_timer`: cancel a previously created timer.
- `time.inspect_timer`: inspect sanitized timer state.
- `time.evaluate_deadline`: evaluate a deadline against a bounded clock context.

## DTO Guidance

Use `TimeClockSource::WallClock` only for user-facing or protocol timestamps.
Use `TimeClockSource::Monotonic` for elapsed-time, timeout, and retry decisions.
Use `TimeClockSource::FrozenTest` only in policy-approved test or replay
contexts.

Timer IDs, timezone database versions, locale IDs, and descriptor hashes are
safe observability metadata. Raw provider payloads, raw user content, prompts,
credentials, private keys, package bytes, manifests, and unbounded timer state
must never enter logs, traces, snapshots, SDK diagnostics, or examples.

## Result And Error DTOs

All commands use a bounded result envelope with status, optional data, optional
error, trace id, and descriptor hash. Standard statuses are `success`, `denied`,
`invalid_time`, `invalid_timezone`, `invalid_calendar`, `invalid_locale`,
`parse_failed`, `overflow`, `unsupported`, `timer_not_found`,
`quota_exceeded`, `unavailable`, and `provider_failure`.

## Examples

Current UTC time:

```json
{
  "timezone": {
    "zone_id": "UTC",
    "data_version": "tzdb-2026a"
  },
  "calendar": {
    "calendar_id": "iso8601"
  }
}
```

Monotonic timeout reference:

```json
{
  "source": "monotonic"
}
```

Timezone conversion:

```json
{
  "instant": {
    "epoch_millis": 1800000000000,
    "timezone_id": "UTC",
    "calendar_id": "iso8601"
  },
  "target_timezone": {
    "zone_id": "America/New_York",
    "data_version": "tzdb-2026a"
  }
}
```

Localized formatting:

```json
{
  "instant": {
    "epoch_millis": 1800000000000,
    "timezone_id": "UTC",
    "calendar_id": "iso8601"
  },
  "format": {
    "pattern_ref": "rfc3339",
    "locale": {
      "locale_id": "en-US"
    },
    "timezone": {
      "zone_id": "UTC",
      "data_version": "tzdb-2026a"
    }
  }
}
```

Strict timestamp parsing:

```json
{
  "input_ref": "artifact:timestamp",
  "format": {
    "pattern_ref": "rfc3339",
    "locale": {
      "locale_id": "en-US"
    },
    "timezone": {
      "zone_id": "UTC",
      "data_version": "tzdb-2026a"
    }
  },
  "strict": true
}
```

Timer create and cancel:

```json
{
  "duration": {
    "millis": 5000,
    "nanos_adjustment": 0
  },
  "exactness": "inexact_allowed",
  "session_binding": "session-ref"
}
```

Inexact timer diagnostic:

```json
{
  "status": "unsupported",
  "error": {
    "code": "unsupported",
    "message": "exact timers are not available for this provider",
    "retryable": false
  }
}
```

Mock clock test usage:

```json
{
  "source": "frozen_test"
}
```

Production policy must deny frozen test clocks unless the execution context is
explicitly a test or replay context.

Unavailable provider diagnostic:

```json
{
  "status": "unavailable",
  "error": {
    "code": "unavailable",
    "message": "time provider is not installed",
    "retryable": false
  }
}
```

## Provider Replacement

Providers are replaceable service implementations. Expected provider classes
include `host-clock`, `frozen-test-clock`, `mock`, and `unavailable`. Provider
adapters must expose descriptor metadata, health, snapshots, command support,
timer cleanup behavior, unavailable states, and sanitized diagnostics through
the service runtime. SDKs, shells, kernel code, and applications must not
instantiate provider clock, timer, timezone, or locale objects directly.
