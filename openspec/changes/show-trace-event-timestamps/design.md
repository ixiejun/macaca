## Context

`EventEntry` already carries `timestamp` as an ISO string from the backend. The current frontend drops this value when converting EventLog entries into `TraceStep` and `DelegatedTraceStep`, so the UI cannot display event time.

## Goals

- Show event occurrence time for main thread trace events and delegated trace events.
- Use persisted backend timestamps when available.
- Keep rendering generic across applications, agents, drivers, and event types.
- Avoid backend protocol changes in this UI slice.

## Non-Goals

- Do not change EventLog schema.
- Do not add app-, workflow-, driver-, or agent-specific rendering.
- Do not migrate legacy stored turn traces that lack timestamps.

## Decisions

### Timestamp source

For events loaded through `/api/sessions/{id}/events`, use `EventEntry.timestamp`.

For live SSE events that do not carry a timestamp field today, use `new Date().toISOString()` at receive time. This is explicitly a UI fallback, not a canonical event timestamp.

### Rendering

Add one reusable timestamp badge component/formatter and place it in the header row for each trace block. If a step has no timestamp, render no badge instead of fake data.

### DTO shape

Add optional `timestamp?: string | number` to trace DTOs so existing numeric driver timestamps continue to render and EventLog ISO strings can be preserved.

## Risks / Trade-offs

- Live SSE receive time can differ from backend event time. Mitigation: only use it as a temporary UI fallback and keep EventLog timestamp as the preferred source.
- Some old stored turns may not have timestamps. Mitigation: optional field and no badge when absent.

## Validation

- `openspec validate show-trace-event-timestamps --strict`
- `cd frontend && npm run lint`
