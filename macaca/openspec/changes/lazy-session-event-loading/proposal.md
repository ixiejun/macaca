# Change: Lazy session event loading

## Why

Opening an application or selecting a session currently loads too much historical data. Session detail can include app-scoped `plan_decisions` and reconstruct large trace histories, which makes the web UI slow when applications have many sessions or events.

## What Changes

- Default application session lists to the first 20 lightweight session summaries.
- Keep session detail lightweight: messages, stored turns, metadata, event URL/count, but no app-scoped plan decision history and no full EventLog trace reconstruction.
- Extend session EventLog reads with generic filters for source, agent, and event type.
- Update the frontend to lazy load coordinator traces for the selected session and delegated traces for the selected tab.
- Keep deprecated app-scoped plan decision helpers available for migration discovery, but stop using them for session detail.

## Impact

- Affected specs: `lazy-session-event-loading`
- Affected code: `macaca/crates/macaca-web/src/session.rs`, `macaca/crates/macaca-web/src/routes.rs`, `macaca/crates/macaca-web/src/sse.rs`, `frontend/lib/api.ts`, `frontend/app/chat/[appId]/page.tsx`
- Compatibility impact: `GET /api/apps/{id}/sessions` defaults to 20 rows unless `limit` is provided; `GET /api/sessions/detail/{id}` no longer returns app-scoped `plan_decisions`.
