# Heartbeat Agent Operations UI Design

## Context

Macaca Web already exposes application-scoped scheduled work through the
Application Operations dialog. Manifest-declared heartbeat agents are now
executed through native `service.heartbeat` cadence and `service.agent_execution`,
but operators cannot inspect the heartbeat agent declarations, native profile
state, or recent heartbeat run mementos from the same application-level surface.

The feature must keep Web and frontend as thin shells. They may render and adapt
requests, but they must not own heartbeat cadence semantics, agent execution, or
application manifest interpretation.

## Recommended Approach

Use a Facade + Command + Memento + Observer design:

- Application Service remains the owner of manifest-declared heartbeat agent
  projections.
- Heartbeat Service owns native profile state, profile edits, run mementos, and
  sanitized snapshots.
- SDK exposes focused Heartbeat client methods so Web routes call a typed facade
  instead of constructing providers.
- Web adds application-scoped command-adapter routes under
  `/api/apps/{app_id}/autonomy/heartbeat`.
- Frontend adds a Heartbeat Operations panel beside the existing Scheduler
  panel inside `ApplicationOperationsDialog`.

## Alternatives Considered

1. Frontend-only rendering from existing app metadata.
   This would not provide profile editing or run mementos and would leave the
   user without evidence of actual heartbeat execution.

2. Direct manifest editing from Web.
   This would make the shell mutate application-owned configuration semantics
   and risks leaking app-specific behavior into Web.

3. Scheduler-backed heartbeat jobs.
   This would regress the heartbeat/scheduler separation and make heartbeat look
   like a special scheduled task again.

## Data Flow

1. Frontend opens the existing application operations dialog.
2. Heartbeat tab calls Web heartbeat operations routes.
3. Web builds typed trace-bearing commands.
4. Web queries Application Service for heartbeat agent declarations.
5. Web queries Heartbeat SDK client for snapshot and run mementos.
6. Web maps responses to sanitized DTOs without raw prompts, manifest text, or
   provider payloads.
7. Profile edits flow through a typed Heartbeat profile command and return audit
   ids.

## Non-Goals

- No raw `HEARTBEAT.md` content in UI responses.
- No per-application business logic or application-name branches.
- No frontend-owned cadence execution or hidden Scheduler job creation.
- No direct provider construction in SDK or Web.

## Testing Strategy

- Add Web route tests for declaration + snapshot aggregation and profile update
  payload mapping.
- Add Heartbeat service tests for profile update mementos and audit ids.
- Add frontend type/lint checks for the new operations panel.
- Run existing scheduler/heartbeat/autonomy boundary tests to guard ownership.
