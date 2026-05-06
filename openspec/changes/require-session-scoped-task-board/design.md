# Design: Session-scoped Task Board loading

## Context

Todo storage is already keyed by `todo/{app_id}/{session_id}/{agent}/{task_id}`. The unsafe behavior is the web route fallback: when `session_id` is absent, `list_todos` calls the application-wide `list_all_todos`.

## Decision

Use a small guard at the API boundary. `GET /api/apps/{app_id}/todos` remains the Web UI route, but it now requires a non-empty `session_id`. This is a Guard Clause plus Repository-boundary fix: the route validates query scope before delegating to `TodoStore::list_all_todos_for_session`.

## Alternatives

- Front-end-only guard: simpler, but any accidental caller can still trigger an app-wide scan.
- New `/sessions/{session_id}/todos` endpoint: cleaner long term, but unnecessary for this small non-breaking UI fix.

## Risks

- Non-UI clients that relied on app-wide todo listing through this route now receive `400`.
- Internal planner/worker paths are unaffected because they call `TodoStore` / `TaskSpace` directly, not the Web UI route.
