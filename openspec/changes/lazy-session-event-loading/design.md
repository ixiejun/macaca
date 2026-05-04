## Context

Macaca web currently mixes three layers of data:

- application-level session discovery,
- selected session messages,
- selected session trace/event history.

This creates an expensive hot path. Loading one session can read all app-level `plan_decisions` and replay thousands of session EventLog rows before the user has selected which trace tab they want to inspect.

## Goals

- Application load fetches only lightweight session summaries, defaulting to 20.
- Session detail fetches only session messages, stored turns, metadata, and event pointers.
- Trace loading is session-scoped and tab-scoped.
- All logic remains application-generic and agent-generic.

## Non-Goals

- Do not redesign EventLog storage indexes in this slice.
- Do not delete legacy app-scoped plan decision storage.
- Do not change live SSE delivery.
- Do not hardcode application, workflow, agent, or driver names.

## Decisions

### Session list pagination

Add query parameters to session list endpoints:

- `limit`, clamped to a safe maximum.
- `offset`, defaulting to 0.

`/api/apps/{id}/sessions` defaults to 20 summaries.

The response remains `Vec<SessionListItem>` in the first slice to avoid a broad frontend pagination UI rewrite.

### Lightweight session detail

Treat session detail as a Memento of the conversation state, not as a trace replay endpoint. It returns stored messages/turns and metadata only. The EventLog remains the source for trace details.

`plan_decisions/{app_id}` remains as deprecated legacy storage but is no longer attached to every session detail response.

### Filtered EventLog reads

Extend `/api/sessions/{id}/events` with optional filters:

- `source`, matching `EventEntry.source`.
- `agent`, matching `payload.agent` or `payload.agent_tab`.
- `event_type`, matching `EventEntry.event_type`.

Filtering is applied after bounded replay in this slice. If large installations still need better performance, a future storage-index change can optimize the same contract.

### Frontend lazy loading

Use the API facade in `frontend/lib/api.ts` to construct filtered event queries.

The chat page loads coordinator events for the selected session and loads delegated events only when the selected workspace tab is an agent tab. Live SSE remains the fast path for active execution.

## Risks / Trade-offs

- Defaulting session lists to 20 changes behavior for clients expecting all sessions. Mitigation: clients can pass an explicit `limit`.
- Filtering after replay may still scan more events than returned. Mitigation: clamp fetch caps and keep the contract ready for indexed storage.
- Removing trace reconstruction from session detail can expose frontend assumptions. Mitigation: the frontend now fetches trace events explicitly.

## Validation

- `openspec validate lazy-session-event-loading --strict`
- `cargo check -p macaca-web`
- `cd frontend && npm run lint`
- Smoke endpoints with `/api/apps/{id}/sessions`, `/api/sessions/detail/{id}`, and filtered `/api/sessions/{id}/events`.
