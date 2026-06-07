# Lazy Session Event Loading

Date: 2026-05-04

## 1. Current Code Facts

This plan follows:

- `AGENTS.md`
- `openspec/AGENTS.md`
- `macaca/docs/design_patterns.md`
- Existing plan: `docs/superpowers/plans/2026-05-04-debug-session-goal-message-flicker.md`

Current behavior:

- `GET /api/apps/{id}/sessions` returns all sessions for an application.
- `GET /api/sessions/detail/{session_id}` loads the stored session, then replays up to 10000 EventLog rows to rebuild coordinator and delegated traces.
- The same session detail endpoint also loads `plan_decisions/{app_id}`, which is app-scoped and can return thousands of historical decisions unrelated to the selected session.
- The frontend calls `fetchAppSessions(appId)` on application load and after sending messages.
- The frontend calls `fetchSession(sessionId)` when selecting a session and also during session stream refresh.
- The frontend calls `fetchSessionEvents(sessionId, 0, 2000)` during refresh to reconstruct driver/delegated traces.
- Delegated tabs currently render from `agentTraces` already loaded into the page state instead of fetching tab-specific trace data only when the tab is opened.

Target intent:

- Events are session-scoped.
- Opening an application loads only the first 20 lightweight session summaries.
- The session log sidebar exposes a bottom "load more" action to fetch the next page.
- Opening a session loads only necessary main-thread messages and metadata.
- Main-thread trace events are fetched only for the selected session.
- Delegated tab trace events are fetched only when that tab is selected.
- The frontend must not fetch application-level historical event/decision data for session rendering.

Design-pattern fit:

- Facade: keep frontend API calls behind `frontend/lib/api.ts` and add query parameters without leaking endpoint construction into the page.
- Adapter: adapt session-scoped EventLog rows into coordinator/delegated trace view models.
- Memento: session detail is a lightweight restored conversation snapshot, not a full trace replay.
- Proxy/Lazy Loading: session traces and delegated traces should load on demand by session and tab.
- Iterator/Pagination: application session lists should be page-limited, starting with a default page size of 20.

GitNexus impact summary before Rust edits:

- `list_app_sessions`: LOW risk, no indexed upstream callers.
- `get_session_by_id`: LOW risk, no indexed upstream callers.
- `get_session_events`: LOW risk, no indexed upstream callers.
- `load_plan_decisions`: LOW risk, one direct caller (`get_session_by_id`).

## 2. Superpowers Brainstorm

### Option A: Minimal backend response diet

Scope:

- Remove app-scoped `plan_decisions` from session detail output.
- Stop replaying EventLog in `get_session_by_id`; return stored messages/turns plus metadata only.
- Keep existing `/api/sessions/{id}/events` endpoint unchanged.

Benefits:

- Biggest immediate latency win for clicking sessions.
- Small and easy to review.
- Removes the incorrect app-level data load from session detail.

Risks:

- Existing frontend currently depends on session detail trace reconstruction after refresh.
- Without frontend lazy event loading, historical trace display may become incomplete.

Controls:

- Combine with frontend EventLog loading for selected coordinator tab.
- Keep stored turns for compatibility, but do not treat detail as full trace source.

### Option B: Paginated session list

Scope:

- Add `limit` and optional `offset` query parameters to session listing endpoints.
- Default `GET /api/apps/{id}/sessions` to `limit=20`.
- Keep response shape as `Vec<SessionListItem>` for frontend compatibility in the first slice.
- Add a frontend "load more" button that appends the next page.

Benefits:

- Directly fixes application-open latency from large session lists.
- Keeps current UI simple.
- Allows later "load more" without changing the first response shape.

Risks:

- Existing clients expecting all sessions by default will now see only 20.
- Sorting still requires reading all session summaries unless a sorted index is introduced later.
- Without a total count in the first response shape, the UI determines whether more pages exist by whether the previous page was full.

Controls:

- Document the default page behavior in OpenSpec.
- Keep `limit` configurable for clients that need more.
- Fetch `limit + 1` internally or request one page at a time and hide the button when a page returns fewer than `limit` items.
- Defer a persistent sorted index until needed.

### Option C: Filtered EventLog endpoint

Scope:

- Extend `/api/sessions/{id}/events` with optional `source`, `agent`, and `event_type` filters.
- Keep `since` and `limit`.
- Frontend fetches coordinator/main-thread events with `source=coordinator`.
- Frontend fetches delegated tab events with `agent={agentName}` or delegated event filters when a tab is selected.

Benefits:

- Aligns with the desired session/tab lazy loading contract.
- Avoids downloading all session events when only one tab is visible.
- Generic across applications and agents.

Risks:

- Filtering after replay still reads more than the final returned count if the store lacks indexed filters.
- Query semantics must be precise enough to avoid missing events with absent `agent` fields.

Controls:

- Clamp limits.
- Implement simple in-memory filtering first, with a fetch cap.
- Use generic event fields only: `source`, payload `agent`, and `event_type`.

### Option D: Full event storage/index redesign

Scope:

- Add persistent indexes for app sessions sorted by update time and EventLog by session/source/agent.
- Replace list/replay scans with indexed reads.

Benefits:

- Best long-term performance for large installations.
- Cleanest data model.

Risks:

- Larger migration, storage compatibility, and testing burden.
- Too much for the immediate UI regression.

Controls:

- Defer until the lazy API contract is proven.

## 3. Recommendation

Implement Options A + B + C in one incremental change.

Rationale:

- Option A removes the known incorrect app-scoped payload from the hot path.
- Option B enforces the intended application-open contract: lightweight first 20 sessions.
- Option C gives the frontend a generic way to lazy load only the selected session/tab trace events.
- Option D is not needed yet; the first slice can use existing storage and preserve endpoint paths.

## 4. Risk Register

- Risk: session detail no longer contains reconstructed traces.
  Control: frontend explicitly loads coordinator events for the selected session and delegated events for selected tabs.

- Risk: default session list limit hides older sessions.
  Control: add `limit`/`offset` parameters and keep the first 20 visible by default.

- Risk: delegated event filtering misses events where `agent` is nested or absent.
  Control: filter by payload `agent`, `agent_tab`, and delegated event naming; keep live SSE path unchanged.

- Risk: events endpoint still scans more rows internally.
  Control: clamp limits and use a bounded fetch cap; defer indexed storage if real data still shows latency.

- Risk: removing app-scoped `plan_decisions` breaks hidden consumers.
  Control: mark `load_plan_decisions` deprecated and stop calling it from session detail, but do not delete the storage/helper.

## 5. Write-Plan

### Phase 1: OpenSpec

1. Create `openspec/changes/lazy-session-event-loading/`.
2. Add proposal/design/tasks/spec for:
   - paginated lightweight app session loading,
   - lightweight session detail,
   - session-scoped trace event filtering,
   - deprecated app-scoped plan decision reads.
3. Validate with `openspec validate lazy-session-event-loading --strict`.

### Phase 2: Backend

1. Add `limit`/`offset` to session list query parameters.
2. Default `list_app_sessions` to first 20 session summaries.
3. Stop loading app-scoped `plan_decisions` in `get_session_by_id`.
4. Stop rebuilding all coordinator/delegated traces in `get_session_by_id`; return stored messages/turns and metadata only.
5. Extend `get_session_events` with optional `source`, `agent`, and `event_type` filters.
6. Keep old `load_plan_decisions` helper but mark it deprecated and unused by session detail.

### Phase 3: Frontend

1. Update `fetchAppSessions` to request `limit=20` by default.
2. Add sidebar pagination state and a bottom "load more" button.
3. Extend `fetchSessionEvents` to accept source/agent/event_type filters.
4. On session select/refresh, load lightweight session detail plus coordinator events only.
5. On delegated tab select, fetch only that agent's events for the active session.
6. Keep live SSE updates as the immediate path while lazy fetches fill historical state.

### Phase 4: Validation

1. Run OpenSpec validation.
2. Run Rust checks for `macaca-web`/CLI as needed.
3. Run frontend lint.
4. Smoke:
   - app open loads only 20 sessions,
   - session detail response does not include app-wide `plan_decisions`,
   - coordinator tab fetches session-scoped events,
   - delegated tab fetches only selected agent traces.

## 6. Acceptance Criteria

- Opening an application does not fetch all application sessions by default.
- The session log sidebar can fetch older sessions through "load more".
- Opening a session does not return application-scoped `plan_decisions`.
- Session detail no longer performs full trace reconstruction as the default hot path.
- Trace events are fetched by session and visible tab.
- No hardcoded application, workflow, agent, or driver names are introduced.
