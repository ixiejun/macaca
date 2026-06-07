## 1. Specification

- [x] 1.1 Update Superpowers plan to remove backfill and document development reset.
- [x] 1.2 Add OpenSpec proposal, design, tasks, and spec delta.
- [x] 1.3 Validate OpenSpec in strict mode.

## 2. Persist Layer

- [x] 2.1 Add EventLog query/index primitives for source, agent, and event type.
- [x] 2.2 Write secondary index rows during append without breaking existing callers.
- [x] 2.3 Optimize Redb prefix listing to avoid full-table scans.
- [x] 2.4 Add EventLog tests for indexed source, agent, type, since, and limit behavior.

## 3. Web Routes

- [x] 3.1 Update session events endpoint to use indexed EventLog queries.
- [x] 3.2 Update run-trace endpoint to use the event-type index.
- [x] 3.3 Keep HTTP response shape and frontend API contract unchanged.

## 4. Development Data Reset

- [x] 4.1 Stop backend processes before deleting the development sessions database.
- [x] 4.2 Delete the local development `sessions.db`.

## 5. Validation

- [x] 5.1 Run `cargo test -p macaca-persist event_log`.
- [x] 5.2 Run `cargo check -p macaca-web`.
- [x] 5.3 Run `npx gitnexus detect-changes --repo agent`.
