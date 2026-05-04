# Trace Event Timestamp Display

Date: 2026-05-04

## Brainstorm

### Option A: Frontend-only display from existing timestamps

- Use `EventEntry.timestamp` for events fetched from EventLog.
- Add optional `timestamp` to frontend trace step DTOs.
- Stamp live SSE events with `new Date().toISOString()` when the browser receives them.
- Render a compact local time badge in coordinator, delegated, and driver trace blocks.

Benefits:
- No backend API change; `EventEntry.timestamp` already exists.
- Works for newly persisted historical traces and live traces.
- Small, reversible UI-only change.

Risks:
- Live SSE time is client receive time, not exact backend append time.
- Some legacy persisted turn traces may lack timestamp and render without a badge.

### Option B: Add timestamps to every SSE payload in backend

Benefits:
- Live and persisted events can share backend event time.

Risks:
- Broader backend protocol change across many event variants.
- Requires updating all consumers and contracts.
- Overkill for UI layout request.

### Option C: Backend trace DTO migration

Benefits:
- Fully canonical timestamp in stored `TraceStep` and delegated trace schemas.

Risks:
- Broad persistence/schema migration surface.
- Not needed because EventLog already has timestamps.

## Recommendation

Use Option A. It matches current architecture: EventLog is the source of persisted event time, and the frontend can display timestamps generically without hardcoding application or agent names.

## Write-Plan

1. Add OpenSpec change `show-trace-event-timestamps`.
2. Extend frontend trace DTOs with optional timestamp fields.
3. Add a reusable timestamp formatter/badge component.
4. Preserve `EventEntry.timestamp` when converting EventLog events into coordinator/delegated/driver trace steps.
5. Add live receive timestamps for SSE-only trace steps.
6. Render timestamps in coordinator trace blocks, delegated trace blocks, and driver trace renderer headers.
7. Validate with `openspec validate`, frontend lint, and a smoke reload if needed.
