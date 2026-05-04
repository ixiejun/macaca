# Change: Show trace event timestamps

## Why

The web trace UI shows event content but not when each trace event occurred. EventLog entries already include timestamps, so historical trace views can expose that context without changing backend storage.

## What Changes

- Preserve event timestamps when EventLog events are converted into coordinator, delegated, and driver trace steps.
- Add a compact timestamp badge to trace event headers.
- Stamp live SSE-only trace steps with the browser receive time until backend SSE payloads carry event time.
- Keep the HTTP/SSE contracts unchanged.

## Impact

- Affected specs: `trace-event-ui`
- Affected code: frontend trace types, chat event conversion, trace renderers
- Compatibility impact: none; timestamp fields are optional in frontend DTOs
