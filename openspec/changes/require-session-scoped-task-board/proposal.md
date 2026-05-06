# Change: Require session-scoped Task Board loading

## Why

The Web UI Task Board must never fall back to scanning every todo in an application. Operators open the board for the current application session, so the API boundary should require both `app_id` and `session_id`.

## What Changes

- Require `session_id` for `GET /api/apps/{app_id}/todos`.
- Return `400` when `session_id` is missing or blank.
- Keep the front-end Task Board lazy-loaded on click, but prevent it from calling the API without a current session id.

## Impact

- Affected specs: `session-scoped-task-board`
- Affected code:
  - `macaca/crates/macaca-web/src/routes.rs`
  - `frontend/lib/api.ts`
  - `frontend/components/TaskBoardModal.tsx`
