## 1. OpenSpec

- [x] 1.1 Create proposal, design, tasks, and spec delta.
- [x] 1.2 Validate `lazy-session-event-loading` with `--strict`.

## 2. Backend

- [x] 2.1 Add `limit` and `offset` query support to session listing.
- [x] 2.2 Default app session lists to 20 lightweight summaries.
- [x] 2.3 Remove app-scoped `plan_decisions` from session detail hot path.
- [x] 2.4 Stop full EventLog trace reconstruction in session detail.
- [x] 2.5 Add `source`, `agent`, and `event_type` filters to session events.
- [x] 2.6 Mark app-scoped plan decision load helper deprecated without deleting it.

## 3. Frontend

- [x] 3.1 Request only 20 session summaries by default.
- [x] 3.2 Add a sidebar "load more" action that appends the next session page.
- [x] 3.3 Add filtered session-event API options.
- [x] 3.4 Load coordinator/main-thread events for the selected session.
- [x] 3.5 Load delegated trace events only when an agent tab is selected.
- [x] 3.6 Show delegated tab loading state before empty trace state.

## 4. Verification

- [x] 4.1 Run `openspec validate lazy-session-event-loading --strict`.
- [x] 4.2 Run `cargo check -p macaca-web`.
- [x] 4.3 Run `cd frontend && npm run lint`.
- [ ] 4.4 Smoke test session list/detail/events endpoints.
