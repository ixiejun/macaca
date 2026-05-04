# Debug Session Goal Message Flicker

Date: 2026-05-04

## 1. Current Code Facts

This plan follows:

- `AGENTS.md`
- `macaca/docs/design_patterns.md`
- Existing frontend chat/session replay code in `frontend/app/chat/[appId]/page.tsx`
- Backend session/SSE code in `macaca/crates/macaca-web/src/session.rs` and `macaca/crates/macaca-web/src/sse.rs`

Observed symptom:

- In the main thread, after creating a goal, the session chat window receives a pushed message.
- The message disappears after a short time.
- It reappears later.

Relevant runtime evidence:

- Backend logs show the goal was created and `plan_decision` was sent to the session.
- A sampled session had EventLog `plan_decision` entries, but session detail returned a very large app-scoped `plan_decisions` list.
- Session detail turns contained coordinator trace steps such as `thinking`, `tool_call`, and `tool_result`, but did not surface the live `plan_decision` as a `trace_steps` item.

Frontend state flow:

- `handleSend()` optimistically appends a user turn and pending assistant turn.
- `/api/chat/v2` live stream updates `turns` through `applyCoordinatorStreamEvent()`.
- When a `session_id` event arrives, the frontend calls `fetchSession(sid)` and then replaces state with `setTurns(buildTurnsFromSession(latest))`.
- The session stream also calls `refreshFromStore()` on many events; this fetches session detail and replaces state with `buildTurnsFromSession(latest)`.
- The current refresh merge only preserves `driver_trace_steps` in a narrow case. It does not preserve live `trace_steps`, assistant `content`, `status`, or `delegated_traces` when the persisted snapshot is older.
- `applyCoordinatorStreamEvent()` handles `thinking`, `tool_call`, `tool_result`, `assistant`, `content`, `done`, `error`, and `stopped`, but not `plan_decision`.
- Session stream `plan_decision` falls through the default branch and schedules a persisted refresh instead of appending a live trace step.

Backend state flow:

- `broadcast_to_app_sessions()` appends `plan_decision` to each matching session EventLog and sends SSE to the session.
- `save_plan_decision()` and `load_plan_decisions()` use an app-level key, so session detail can include app-scoped historical plan decisions rather than only the current session's decisions.
- `get_session_by_id()` rebuilds traces from EventLog, but the visible frontend turn state is still vulnerable to stale snapshot replacement.

Design-pattern fit before planning:

- Memento: session detail and EventLog snapshots are historical state; restoration must not overwrite newer live state with an older memento.
- Mediator: the chat page needs one state mediator for live SSE events, EventLog replay, and persisted session hydration instead of letting each source mutate `turns` independently.
- Adapter: backend EventLog events and frontend `TraceStep` objects need a small adapter for `plan_decision`.
- Facade: a small frontend session-turn merge facade can hide the source-specific differences without changing backend contracts.

## 2. Superpowers Brainstorm

### Option A: Frontend live-state preserving merge

Scope:

- Introduce a focused helper for merging persisted session turns into current live turns.
- Preserve live assistant `trace_steps`, `content`, `status`, `driver_trace_steps`, and `delegated_traces` when the persisted turn is older or less complete.
- Use the helper in `refreshFromStore()` and the `session_id` hydration path in `handleSend()`.
- Keep endpoint contracts unchanged.

Benefits:

- Directly targets the disappearance/reappearance symptom.
- Small, reversible frontend change.
- Avoids Rust API/storage migration in the first fix.
- Aligns with the Memento pattern by treating persisted session data as a snapshot that must be reconciled with newer in-memory state.

Risks:

- Duplicate trace steps if persisted and live steps are merged without stable identity.
- Matching turns by array index can be fragile if backend inserts or normalizes turns differently.
- Preserving live content could mask a valid backend correction if the server intentionally rewrites a turn.

Controls:

- Prefer latest non-empty fields while avoiding blind concatenation.
- Keep merge scoped to same role/index and especially the last assistant turn.
- Add manual validation for no flicker and no duplicate traces after EventLog catches up.

### Option B: Explicit `plan_decision` SSE adapter

Scope:

- Add `plan_decision` to the frontend stream event handling path.
- Append it as `{ type: 'plan_decision', decision_data: event.data }` to the latest assistant turn's `trace_steps`.
- Avoid scheduling an immediate refresh only because a plan decision arrived.

Benefits:

- Uses the existing `ConversationTurn` rendering support for `plan_decision`.
- Removes one trigger for stale snapshot overwrite.
- Keeps the plan decision visible immediately from live SSE.

Risks:

- If backend emits repeated plan decisions during reconnect/replay, the frontend can show duplicates.
- The event payload shape is not strongly normalized in the frontend.
- If no assistant turn exists, the event needs a safe fallback rather than creating app-specific UI behavior.

Controls:

- Convert through a narrow adapter that only depends on generic event fields.
- Append to the latest assistant turn only when present.
- Add dedupe later only if duplicated payloads are observed.

### Option C: Backend session-scoped plan decision cleanup

Scope:

- Change plan decision persistence/query behavior so session detail does not expose app-wide historical plan decisions as session data.
- Prefer EventLog as the session-scoped source of truth because `broadcast_to_app_sessions()` already appends `plan_decision` per matching session.
- Keep old storage keys available for migration/search if compatibility is needed.

Benefits:

- Fixes the underlying source-of-truth mismatch.
- Reduces huge, noisy `plan_decisions` payloads in session detail.
- Makes replay semantics cleaner for future frontend work.

Risks:

- Rust storage contract change has larger blast radius and requires OpenSpec plus GitNexus impact before edits.
- Existing callers may rely on app-scoped plan decision history.
- Backward compatibility/migration for existing persisted data needs a decision.

Controls:

- Do this as a second slice after frontend flicker is stabilized.
- Make session-scoped EventLog the canonical path while leaving deprecated app-scoped helpers discoverable.
- Add API smoke tests against session detail and session events.

### Option D: EventLog-first frontend replay

Scope:

- Treat `/api/sessions/:id/events` as the source of truth for coordinator, driver, delegated, and plan traces.
- Use session detail mainly for metadata and historical messages.
- Normalize all relevant EventLog entries through a replay adapter.

Benefits:

- Strongest architectural direction for a 7*24 agent OS because replay is deterministic and source-scoped.
- Reduces hidden races between live streams and persisted snapshots.
- Aligns frontend replay with backend `SessionReplayState` direction.

Risks:

- Larger migration in a stateful UI file.
- Higher chance of regressions in delegated traces, driver traces, stopped/error states, and refresh recovery.
- Needs integration/browser tests that may not currently exist.

Controls:

- Defer until the narrow flicker fix is validated.
- Split replay normalization out of the chat page before changing behavior broadly.

### Option E: Delay/debounce refresh only

Scope:

- Increase `scheduleRefresh()` delay or skip refresh for selected events.

Benefits:

- Very small change.
- Can reduce the visible flicker if persistence usually catches up within the delay.

Risks:

- Does not fix the race; it only hides timing.
- Longer refresh delays make real stale state last longer.
- Still fails under slow persistence or reconnect.

Controls:

- Use only as a temporary mitigation, not the primary fix.

## 3. Recommendation

Use Option A plus Option B as the first implementation slice.

Rationale:

- The immediate bug is caused by frontend state replacement: live SSE state is newer than the fetched persisted snapshot, but the frontend treats the snapshot as authoritative.
- A small merge mediator plus `plan_decision` adapter fixes the user-visible flicker without changing backend storage contracts.
- This keeps the change generic: no workflow, app name, driver name, or business-specific logic is needed.
- Backend session-scoped plan decision cleanup is valid, but it should be a second slice because it changes Rust persistence/API behavior and needs a separate OpenSpec decision.

## 4. Risk Register

- Risk: duplicate coordinator trace steps after merging live and persisted turns.
  Control: preserve the longer/more complete trace list instead of blindly concatenating in the first slice.

- Risk: persisted session has corrected assistant content but live content wins.
  Control: only preserve live content when persisted content is empty or clearly less complete.

- Risk: turn index matching breaks if session turns are rebuilt differently.
  Control: keep the first slice narrow to same-index same-role merges and the active last assistant turn.

- Risk: `plan_decision` payload shape changes.
  Control: store it as opaque `decision_data` and let existing renderer handle generic object fields.

- Risk: backend app-scoped `plan_decisions` payload remains noisy.
  Control: record it as a follow-up backend cleanup, not a blocker for the flicker fix.

## 5. Write-Plan

### Phase 1: Specify the frontend behavior change

1. Open `openspec/AGENTS.md`.
2. Create an OpenSpec change for stable chat session live-state reconciliation.
3. In `proposal.md`, state that live SSE updates must not disappear when a stale persisted session snapshot arrives.
4. In `design.md`, document the Memento/Mediator/Adapter decision:
   - persisted session detail is a snapshot,
   - live SSE is newer until replay catches up,
   - all refresh paths must reconcile instead of replacing active turns,
   - `plan_decision` is adapted into `TraceStep`.
5. In `tasks.md`, list implementation and validation steps.
6. Add an incremental spec requiring:
   - live assistant turn content/trace remains visible during session hydration,
   - `plan_decision` SSE is displayed without forcing stale replacement,
   - persisted replay can catch up without duplicating visible traces.

### Phase 2: Implement the narrow frontend fix

1. Add a small turn reconciliation helper in `frontend/app/chat/[appId]/page.tsx` or a local frontend helper if file size pressure requires splitting.
2. Replace `setTurns(buildTurnsFromSession(latest))` in `refreshFromStore()` with the reconciliation helper.
3. Replace `setTurns(buildTurnsFromSession(latest))` in the `session_id` hydration path with the same helper.
4. Add explicit `plan_decision` handling in `applyCoordinatorStreamEvent()` or the session stream switch.
5. Avoid app-specific, workflow-specific, or driver-specific matching logic.

### Phase 3: Validate

1. Run frontend lint/typecheck commands available in the project.
2. Smoke test backend availability with `/api/apps` and the target app session endpoints.
3. Reproduce the create-goal flow in the main thread:
   - message appears immediately,
   - message does not disappear after hydration/refresh,
   - message does not duplicate after EventLog catches up,
   - plan decision remains visible after browser refresh.
4. Inspect `/api/sessions/detail/:session_id` and `/api/sessions/:session_id/events` if any discrepancy remains.

### Phase 4: Follow-up backend cleanup if needed

1. If session detail remains noisy or slow because of app-scoped `plan_decisions`, create a separate OpenSpec change.
2. Run GitNexus impact before touching Rust symbols such as `save_plan_decision`, `load_plan_decisions`, `broadcast_to_app_sessions`, or `get_session_by_id`.
3. Migrate toward session-scoped EventLog replay as the canonical source while keeping deprecated app-scoped helpers discoverable.

## 6. Initial Acceptance Criteria

- Creating a goal in the main thread no longer causes the session chat message to disappear and reappear.
- `plan_decision` events are visible in the coordinator trace when received live.
- Session hydration and stream refresh preserve newer live state until persisted replay is at least as complete.
- No hardcoded workflow, app name, driver name, or business-specific behavior is introduced.
- The first slice does not change backend HTTP or Rust persistence contracts.
