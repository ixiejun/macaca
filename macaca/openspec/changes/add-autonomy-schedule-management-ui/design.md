# Design: Autonomy Schedule Management UI

## Context

The local autonomy runtime has been activated through explicit configuration,
and applications can register serviceized Scheduler jobs through
`POST /api/apps/{app_id}/autonomy/schedules`. Operators can also inspect recent
run mementos through
`GET /api/apps/{app_id}/autonomy/scheduler/runs`.

However, the Web UI still lacks a production management surface for browsing,
editing, deleting, pausing, resuming, and monitoring these jobs. A legacy
`/api/apps/{app_id}/schedules` CRUD route exists, but it is a compatibility path
that directly uses older task-scheduler machinery. The new UI must not depend on
that path.

## Goals

- Provide a generic application-scoped schedule management surface.
- Keep Web/frontend as thin shell adapters.
- Route all job management through Scheduler service contracts and focused SDK
  clients.
- Show safe trace and audit identifiers for every mutation and run memento.
- Match the current Web UI terminal style.
- Add tests that prevent new direct schedule-management escape hatches.

## Non-Goals

- Do not build a global autonomy control center in this change.
- Do not add application-specific templates or workflow presets.
- Do not migrate unrelated task-board, session, chat, driver, skill, MCP, or
  GenUI behavior.
- Do not remove the legacy schedule routes yet; only prevent the new UI from
  using them.

## Ownership

- `service.scheduler` owns job lifecycle, schedule specs, due calculation,
  run leases, run state, and sanitized job/run mementos.
- `service.heartbeat` owns wake coalescing, gates, wake lifecycle, and heartbeat
  wake evidence.
- `macaca-sdk` owns focused Scheduler client commands and structured unavailable
  behavior.
- `macaca-web` owns HTTP parsing, scope validation, trace creation, command
  adaptation, response adaptation, and safe logs.
- `frontend` owns rendering, operator input collection, loading/error state,
  and generic command submission.

## API Design

The serviceized route namespace SHALL be:

```text
GET    /api/apps/{app_id}/autonomy/schedules
POST   /api/apps/{app_id}/autonomy/schedules
GET    /api/apps/{app_id}/autonomy/schedules/{job_id}
PATCH  /api/apps/{app_id}/autonomy/schedules/{job_id}
DELETE /api/apps/{app_id}/autonomy/schedules/{job_id}
PUT    /api/apps/{app_id}/autonomy/schedules/{job_id}/lifecycle
GET    /api/apps/{app_id}/autonomy/scheduler/runs
```

Every route SHALL:

- Parse and validate application scope.
- Create a `TraceContext`.
- Construct provider-neutral Scheduler SDK commands.
- Call the Scheduler client.
- Return structured unavailable, unsupported, denied, conflict, validation, and
  provider-failure responses.
- Emit logs at request received, command constructed, service call completed,
  rejection, and failure.
- Return sanitized DTOs only.

## Frontend Design

The UI SHALL add an `AUTONOMY` workspace tab or equivalent application-scoped
panel. The first implementation should prefer an `AUTONOMY` tab because the
current workspace already uses tabs for operational streams.

Frontend files:

- `frontend/lib/autonomy-types.ts`
- `frontend/lib/autonomy.ts`
- `frontend/components/autonomy/ScheduleManagerPanel.tsx`
- `frontend/components/autonomy/ScheduleSummaryStrip.tsx`
- `frontend/components/autonomy/ScheduleList.tsx`
- `frontend/components/autonomy/ScheduleEditorDrawer.tsx`
- `frontend/components/autonomy/SchedulerRunTimeline.tsx`
- `frontend/components/autonomy/ScheduleStateBadge.tsx`

The existing chat page SHALL only compose the new panel and switch tabs. It
SHALL NOT inline Scheduler CRUD semantics.

## Design Patterns

- **Facade**: frontend API wrapper and SDK Scheduler client hide transport
  details while exposing stable provider-neutral commands.
- **Command**: list, get, create, update, delete, lifecycle, and run query
  operations are typed command/result DTOs.
- **Adapter / Bridge**: Web routes adapt HTTP input to Scheduler service
  commands and service results to JSON responses.
- **State**: job lifecycle and run state are explicit DTO fields and UI badge
  states.
- **Observer / Memento**: run history uses bounded sanitized run summaries as
  replayable mementos.
- **Specification**: validation rules constrain scope, job id, interval,
  metadata size, target kind, and lifecycle operations.

These patterns preserve extension points without over-designing a new framework.

## Trace, Audit, and Logging

Every query and mutation SHALL carry trace context. Mutations SHALL return
audit-compatible identifiers when the Scheduler service provides them. Logs
SHALL include route operation, app id, job id when available, trace id,
structured error code, and safe lifecycle operation.

Logs and responses SHALL NOT include raw prompts, manifests, WASM bytes, package
bytes, provider payloads, private keys, credentials, raw signatures, or
unbounded output.

## Error Handling

The UI SHALL render these structured states:

- `unavailable`
- `unsupported`
- `denied`
- `conflict`
- `validation_error`
- `provider_failure`

Each state SHALL show a safe message and trace id when available. Raw error
payloads SHALL NOT be rendered.

## Test Strategy

- Unit-test Scheduler DTO validation.
- Test local Scheduler provider CRUD and lifecycle state transitions.
- Integration-test serviceized Web route adapters.
- Add boundary tests preventing new direct Scheduler provider construction or
  legacy route usage for the new UI.
- Run frontend lint/type checks.
- Validate OpenSpec strictly.

## Risks and Mitigations

- **Risk:** Reusing legacy `/api/apps/{app_id}/schedules` looks faster.
  **Mitigation:** Add escape-hatch tests and keep frontend API facade pointed
  only at `/autonomy`.
- **Risk:** The existing chat page grows larger.
  **Mitigation:** Put UI into focused `frontend/components/autonomy/*` modules.
- **Risk:** CRUD semantics drift into Web shell.
  **Mitigation:** Web routes only adapt commands; Scheduler service owns state.
- **Risk:** UI exposes sensitive payloads.
  **Mitigation:** Render only sanitized job/run DTO fields.
