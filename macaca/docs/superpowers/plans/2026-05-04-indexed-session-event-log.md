# Indexed Session Event Log

Date: 2026-05-04

## 1. Current Code Facts

This plan follows:

- `AGENTS.md`
- `openspec/AGENTS.md`
- `macaca/docs/design_patterns.md`
- Existing change: `openspec/changes/lazy-session-event-loading/`

Current storage:

- Durable EventLog rows are stored as `events/{session_id}/{seq:08d}`.
- `EventEntry` contains `seq`, `timestamp`, `session_id`, `event_type`, `source`, and `payload`.
- `timestamp` is assigned at append time with `chrono::Utc::now()`.
- `seq` is monotonic per session.
- Application session discovery uses separate keys: `app_sessions/{app_id}/{session_id}` and `session/{session_id}`.
- Agent name is not part of the EventLog key. It is usually stored in `payload.agent` or `payload.agent_tab`.
- Source is stored in `EventEntry.source`, e.g. `coordinator`, `executor`, `plan_loop`, or an agent name.

Current read behavior:

- `EventLog::replay(session_id, since, limit)` calls `store.list_keys("events/{session_id}/")`.
- `RedbStore::list_keys(prefix)` iterates the entire `macaca_data` table and filters with `starts_with(prefix)`.
- `GET /api/sessions/{id}/events?agent=backend` currently replays events and then filters by payload agent.
- Even `limit=20` can be slow because the prefix key lookup is a full-table scan over the 144MB `sessions.db`.

Current write consumers:

- `event_persistence.rs::spawn_session_event_collector` writes delegated executor events with `source = "executor"` and `payload.agent`.
- `framework_runner.rs` writes coordinator events, worker events, driver trace events, and skill snapshot events.
- `framework_toolkit.rs` writes MCP runtime events.
- `skill_mcp.rs` writes skill MCP events.
- `run_trace.rs` writes `event_type = "run_trace"`.
- `sse.rs::broadcast_to_app_sessions` writes `plan_decision`.
- `loop_manager.rs` writes loop resume and task/goal flow events.

Current read consumers:

- `routes.rs::get_session_events` reads EventLog for frontend session/coordinator/delegated tabs.
- `routes.rs::get_session_run_trace` reads EventLog and filters `event_type == "run_trace"`.
- `session.rs::get_session_by_id` uses `event_log.count(session_id)` for metadata.
- `session.rs::stream_session_events` subscribes to EventLog notifications and sends update events.
- `frontend/lib/api.ts::fetchSessionEvents` consumes `/api/sessions/{id}/events`.
- `frontend/app/chat/[appId]/page.tsx` calls `fetchSessionEvents` for coordinator and delegated tabs.

GitNexus impact:

- `EventLog::append_command`: CRITICAL risk, 14 direct callers, 46 impacted symbols, 9 affected processes.
- `EventLog::replay`: HIGH risk, direct callers include `get_session_events`, `get_session_run_trace`, `EventLog::query`, and tests.
- Because this is core infrastructure, implementation must be additive and compatibility-preserving.

Design-pattern fit:

- Adapter: keep existing append/replay API stable while adding indexed query behavior.
- Strategy: introduce query modes/index selection without making web routes know storage internals.
- Repository/Index: EventLog owns primary and secondary index writes/reads.
- Facade: web routes call an EventLog query facade rather than manually filtering large vectors.
- Memento: existing `events/{session_id}/{seq}` remains the canonical row and migration source.

## 2. Superpowers Brainstorm

### Option A: Redb prefix range scan only

Scope:

- Add a `scan_prefix(prefix, limit)` or improve `list_keys(prefix)` to use Redb range iteration instead of full-table iteration.
- Keep EventLog key schema unchanged.
- Keep filtering by `source`, `agent`, and `event_type` after session replay.

Benefits:

- Low schema risk.
- Fixes the immediate full-table scan for `events/{session_id}/`.
- Existing data works without migration.

Risks:

- `agent=backend` still scans all events in the session, then filters.
- Large sessions with many events still make delegated tab reads slower than necessary.
- Does not satisfy the target model of application/agent as secondary storage keys.

### Option B: Add secondary indexes on append

Scope:

- Keep canonical row: `events/{session_id}/{seq:08d}`.
- Add secondary index rows during append:
  - `events_by_source/{session_id}/{source}/{seq:08d}`.
  - `events_by_type/{session_id}/{event_type}/{seq:08d}`.
  - `events_by_agent/{session_id}/{agent_name}/{seq:08d}` when an agent can be derived.
  - Optionally `events_by_app/{app_id}/{session_id}/{seq:08d}` if `app_id` is available.
- Index value can be either the full EventEntry JSON or a pointer to canonical key.
- Add `EventLogQuery` with `session_id`, `since`, `limit`, `source`, `agent`, and `event_type`.
- Make `/api/sessions/{id}/events` call the query facade.

Benefits:

- Directly solves delegated tab loading by `session_id + agent_name`.
- Keeps session_id as canonical primary key.
- Does not require web route filtering over large vectors.
- Existing canonical data remains valid.

Risks:

- `AppendEventCommand` currently has no app_id field, so true `events_by_app` cannot be populated for all writes without changing many callers.
- Existing historical events have no secondary index until migrated or backfilled.
- Agent extraction from payload must be generic and conservative.

Controls:

- Add optional fields to `AppendEventCommand`: `app_id`, `agent_name`.
- Auto-derive agent from `payload.agent` / `payload.agent_tab` when explicit `agent_name` is absent.
- Keep route-level app filtering out of EventLog until write callers can pass app_id consistently.
- Implement read fallback to canonical replay for historical rows when an index is absent.

### Option C: Development reset, no backfill

Scope:

- Stop the web backend and delete the current development `sessions.db`.
- Require all newly appended events to write canonical rows plus secondary indexes.
- Keep legacy `replay()` available for unfiltered compatibility paths and tests, but route filtered UI reads through the indexed query facade.

Benefits:

- Avoids complex and slow migration logic while the product is still under active development.
- Ensures performance fixes are measured on the intended new storage contract.
- Removes fallback ambiguity for the frontend hot path.

Risks:

- Existing local sessions disappear.
- Any developer relying on old local traces must recreate test sessions.

Controls:

- This is explicitly accepted by the user for the development environment.
- Do not delete code paths needed to find deprecated/legacy storage later.

### Option D: Change canonical key to include app/agent

Scope:

- Replace primary storage with a compound key such as `events/{session_id}/{agent}/{seq}`.

Benefits:

- Simple conceptual model for agent tab reads.

Risks:

- Breaks existing replay semantics and data layout.
- Main-thread events and non-agent events need special casing.
- High migration risk and not necessary because secondary indexes solve the problem.

## 3. Recommendation

Use Option B with Option C development reset, and include Option A by optimizing Redb prefix scans if straightforward.

Recommended storage contract:

- Canonical primary key remains:
  - `events/{session_id}/{seq:08d}`.
- Secondary indexes:
  - `events_by_source/{session_id}/{source}/{seq:08d}`.
  - `events_by_agent/{session_id}/{agent_name}/{seq:08d}`.
  - `events_by_type/{session_id}/{event_type}/{seq:08d}`.
- Application remains a secondary session discovery key:
  - `app_sessions/{app_id}/{session_id}` is already the application-to-session index.
  - Event queries remain session-scoped from the web UI.

Rationale:

- The UI requirement is session-first and tab-scoped. `session_id + agent_name` solves the backend tab issue.
- Keeping canonical rows preserves the primary session log contract.
- Additive indexes avoid a breaking rewrite of every EventLog caller.
- The current `application_id` requirement is served by app-to-session discovery first; session event reads use `session_id` primary plus agent/source/type indexes.

## 4. Risk Register

- Risk: CRITICAL blast radius on `append_command`.
  Control: do not change existing caller behavior; add optional command metadata and internal index writes.

- Risk: secondary indexes duplicate storage.
  Control: index rows can store canonical key pointers instead of full EventEntry JSON if storage size becomes an issue.

- Risk: historical data lacks indexes.
  Control: clear the development `sessions.db`; do not implement backfill in this slice.

- Risk: agent extraction misses some events.
  Control: derive from explicit `agent_name`, then `payload.agent`, then `payload.agent_tab`; leave non-agent events unindexed by agent.

- Risk: query with multiple filters needs intersections.
  Control: choose the most selective index first: agent, then source, then event_type, then canonical session; apply remaining filters after indexed read.

- Risk: app_id as secondary key is incomplete.
  Control: preserve existing `app_sessions/{app_id}/{session_id}` as the application secondary key and keep event reads session-scoped.

## 5. Write-Plan

### Phase 1: OpenSpec

1. Create `openspec/changes/index-session-event-log/`.
2. Add proposal/design/tasks/spec for:
   - canonical session EventLog rows,
   - secondary indexes by agent/source/type,
   - compatibility-preserving append behavior,
   - indexed session event query,
   - development reset/no-backfill behavior.
3. Validate with `openspec validate index-session-event-log --strict`.

### Phase 2: Persist layer

1. Add `EventLogQuery` and `EventLog::query_indexed`.
2. Extend `AppendEventCommand` with optional metadata builder methods:
   - `with_app_id`
   - `with_agent_name`
3. On `append_command`, write canonical row first, then index rows.
4. Derive agent name generically from command metadata or payload.
5. Add tests for:
   - canonical replay remains unchanged,
   - indexed agent query returns only that agent,
   - source/type indexed query works,
   - since cursor and limit are preserved.

### Phase 3: Web routes and writers

1. Update `get_session_events` to call `query_indexed` instead of replay-then-filter.
2. Update `get_session_run_trace` to use the event_type index.
3. Update high-value write sites to pass explicit agent/app metadata where already available:
   - executor delegated events,
   - driver traces,
   - coordinator events,
   - plan_loop/worker_loop events.
4. Keep existing callers compiling without metadata.

### Phase 4: Development data reset

1. Stop backend processes that have `sessions.db` open.
2. Delete the development `sessions.db` after code changes are in place.
3. Recreate sessions through the UI so all persisted events use the new indexes.

### Phase 5: Frontend

1. Keep existing `fetchSessionEvents` contract.
2. No UI contract changes should be required after backend indexes are used.
3. Validate that backend tab loads quickly and returns only backend events.

### Phase 6: Validation

1. `cargo test -p macaca-persist event_log`
2. `cargo check -p macaca-web`
3. `cd frontend && npm run lint`
4. Smoke:
   - latest fullstack session detail is fast,
   - `events?agent=backend&limit=20` returns quickly,
   - `events?source=coordinator` returns quickly,
   - `run-trace` returns quickly,
   - browser delegated tab shows loading briefly then trace/empty state.

## 6. Acceptance Criteria

- Canonical event storage is still keyed by `session_id`.
- Agent/source/type event reads do not scan the whole database.
- Backend delegated tabs query by `session_id + agent_name`.
- Existing EventLog append/replay callers keep working.
- Historical development sessions are cleared; new events are indexed at append time.
- Every event still includes `timestamp`.
- No app-, workflow-, agent-, or driver-specific hardcoding is introduced.
