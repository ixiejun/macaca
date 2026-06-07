## Context

The UI already loads application sessions lazily and fetches trace data per selected session/tab. The remaining bottleneck is the storage layer: filtered reads such as `agent=backend` still begin with `EventLog::replay()`, and Redb `list_keys(prefix)` currently iterates the full table before filtering by prefix.

Historical data is not a product requirement right now. The user explicitly accepted clearing all historical development sessions, so this change does not introduce a migration/backfill system.

## Goals

- Preserve `session_id` as the primary EventLog identity.
- Treat `application_id` as the existing application-to-session discovery key, not as an application-wide event scan.
- Make selected trace reads use `session_id` plus optional source/agent/type secondary indexes.
- Keep the implementation generic across applications and agents.
- Preserve timestamps on every persisted event.

## Non-Goals

- Do not hardcode application names, workflow names, driver names, or agent names.
- Do not delete legacy append/replay interfaces.
- Do not add startup migrations or backfill jobs.
- Do not change frontend route contracts in this slice.

## Decisions

### Repository/Index boundary

`EventLog` owns both canonical event rows and index rows. Web routes call an `EventLogQuery` facade and do not assemble storage keys or manually filter large replays.

### Key schema

Canonical row:

```text
events/{session_id}/{seq:08d} -> EventEntry JSON
```

Secondary indexes:

```text
events_by_source/{session_id}/{source}/{seq:08d} -> canonical event key
events_by_agent/{session_id}/{agent_name}/{seq:08d} -> canonical event key
events_by_type/{session_id}/{event_type}/{seq:08d} -> canonical event key
```

Index values store canonical pointers instead of duplicating full JSON. Reads resolve pointers back to canonical rows so event payload shape remains unchanged.

### Query strategy

When a query has filters, `EventLog` chooses one index in this order: agent, source, event type. It resolves matching canonical rows, applies any remaining filters in memory, and honors `since` plus `limit`.

Unfiltered queries use canonical session replay. This preserves existing behavior while benefiting from Redb range-prefix listing.

### Agent extraction

Agent indexing is generic:

- Use explicit `AppendEventCommand.agent_name` when available.
- Otherwise derive from `payload.agent`.
- Otherwise derive from `payload.agent_tab`.
- If no agent exists, omit the agent index row.

### Development reset

The current local `sessions.db` is deleted after backend processes are stopped. New sessions recreate canonical and indexed EventLog data. No compatibility fallback to old unindexed rows is required for UI filtered reads.

## Risks / Trade-offs

- `EventLog::append_command` has CRITICAL blast radius. Mitigation: keep the method signature compiling and add optional metadata plus internal index writes.
- Index writes add extra storage operations per event. Mitigation: store small canonical key pointers rather than duplicated EventEntry JSON.
- Multiple filters may still require post-filtering. Mitigation: choose the most selective index first and keep filters session-scoped.
- Clearing history removes local traces. Mitigation: accepted development reset; no production migration is implied.

## Validation

- `openspec validate index-session-event-log --strict`
- `cargo test -p macaca-persist event_log`
- `cargo check -p macaca-web`
- Smoke filtered endpoints after recreating a session:
  - `/api/sessions/{id}/events?agent={agent}&limit=20`
  - `/api/sessions/{id}/events?source=coordinator&limit=20`
  - `/api/sessions/{id}/run-trace?limit=20`
