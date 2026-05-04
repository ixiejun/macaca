## 1. OpenSpec

- [x] 1.1 Create proposal, design, tasks, and spec delta.
- [x] 1.2 Validate `fix-chat-session-live-reconciliation` with `--strict`.

## 2. Implementation

- [x] 2.1 Add a frontend session-turn reconciliation helper.
- [x] 2.2 Use reconciliation in stream refresh hydration.
- [x] 2.3 Use reconciliation in `session_id` hydration.
- [x] 2.4 Add `plan_decision` to frontend stream typing and live handling.

## 3. Verification

- [x] 3.1 Run `openspec validate fix-chat-session-live-reconciliation --strict`.
- [x] 3.2 Run `cd frontend && npm run lint`.
- [x] 3.3 Smoke test current services where possible.
