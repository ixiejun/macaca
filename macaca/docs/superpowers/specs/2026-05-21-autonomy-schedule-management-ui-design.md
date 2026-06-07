# Autonomy Schedule Management UI Design

Date: 2026-05-21

## Decision

Use **Approach A: application-scoped schedule management inside the existing
Macaca Web workspace**.

The first production UI will add a generic `AUTONOMY / SCHEDULES` management
surface to the current application workspace. It will let operators browse,
create, edit, pause/resume, delete, and monitor application-scoped scheduler
jobs without giving the frontend ownership of scheduler semantics.

## Governance Constraints

This design follows:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`

The Web UI and `macaca-web` shell are presentation adapters only. They may parse
input, call focused clients or service facades, render state, and show trace or
audit diagnostics. They must not define task, scheduler, heartbeat, application,
or provider semantics.

The UI must not call the legacy `/api/apps/{app_id}/schedules` path for this
feature because that route currently uses the older direct `macaca_task`
scheduler path. The new UI must use serviceized `/api/apps/{app_id}/autonomy/*`
routes backed by Scheduler service clients.

## Goals

- Provide an application-scoped Web UI to browse scheduler jobs.
- Provide generic CRUD controls for Scheduler jobs.
- Provide lifecycle controls such as pause, resume, and delete without
  hardcoding workflow names, application names, service providers, drivers,
  models, chains, gateways, payments, or business domains.
- Provide recent run monitoring with state, safe status, attempt, trace id,
  audit id, and timestamps.
- Preserve the current Macaca Web visual language: black terminal surface,
  green OS-console styling, bordered panels, compact uppercase labels, and
  trace-oriented diagnostics.
- Keep all mutation calls traceable and auditable through serviceized backend
  commands.

## Non-Goals

- Do not build a global autonomy control center in the first slice.
- Do not add application-specific schedule templates.
- Do not encode product workflow semantics in frontend code.
- Do not let frontend or Web shell directly construct scheduler providers.
- Do not replace the Scheduler or Heartbeat service contracts.

## Proposed User Experience

The schedule manager should appear as a workspace-level surface rather than a
separate product application. Two acceptable placements are:

- Add an `AUTONOMY` tab next to the existing main thread and agent tabs.
- Add a compact `SCHEDULES` entry in the right-side system panel that opens a
  full workspace panel.

The recommended first placement is an `AUTONOMY` tab because schedules are
application-scoped operational state, similar to traces and context reports.

The panel contains:

- A summary strip with active job count, paused job count, recent failures, and
  latest trace id.
- A schedule list table/card stack with job id, name, lifecycle, schedule spec,
  target kind, last run state, next/due timestamp when available, and actions.
- A create/edit drawer that submits provider-neutral schedule data only.
- A run timeline showing recent sanitized Scheduler run mementos.
- A structured unavailable state when local autonomy is disabled or the
  Scheduler service is unavailable.

## Backend API Surface

The current serviceized Web API supports creation and run monitoring:

- `POST /api/apps/{app_id}/autonomy/schedules`
- `GET /api/apps/{app_id}/autonomy/scheduler/runs`

Full CRUD requires extending the same serviceized namespace:

- `GET /api/apps/{app_id}/autonomy/schedules`
- `GET /api/apps/{app_id}/autonomy/schedules/{job_id}`
- `PATCH /api/apps/{app_id}/autonomy/schedules/{job_id}`
- `DELETE /api/apps/{app_id}/autonomy/schedules/{job_id}`
- `PUT /api/apps/{app_id}/autonomy/schedules/{job_id}/lifecycle`

All routes must:

- Parse HTTP input only.
- Create a `TraceContext`.
- Scope requests to `AutonomyScope::application(app_id)`.
- Call the Scheduler service through the existing focused scheduler client or
  an equivalent facade.
- Return structured unavailable, unsupported, denied, conflict, validation, and
  provider failure states.
- Emit sanitized logs at key execution points.
- Avoid exposing raw payloads, prompts, manifests, package bytes, provider
  payloads, credentials, private keys, signatures, or unbounded output.

## Frontend Component Design

Add focused frontend modules instead of growing the existing chat page into a
god component:

- `frontend/lib/autonomy-types.ts`
- `frontend/lib/autonomy.ts`
- `frontend/components/autonomy/ScheduleManagerPanel.tsx`
- `frontend/components/autonomy/ScheduleSummaryStrip.tsx`
- `frontend/components/autonomy/ScheduleList.tsx`
- `frontend/components/autonomy/ScheduleEditorDrawer.tsx`
- `frontend/components/autonomy/SchedulerRunTimeline.tsx`
- `frontend/components/autonomy/ScheduleStateBadge.tsx`

Each component owns one presentation responsibility. The large application
workspace page should only compose the new panel, load state, and switch tabs.

The frontend API client is a Facade. It hides fetch details from components but
does not own Scheduler semantics.

## Data Flow

```text
Macaca Web UI
  -> frontend autonomy API facade
  -> /api/apps/{app_id}/autonomy/* Web shell route
  -> Scheduler focused client or SystemFacade
  -> ServiceRuntime decorators
  -> service.scheduler provider
  -> sanitized job/run DTOs
  -> Web UI render state
```

The frontend never imports Rust providers and never calls legacy task-scheduler
storage directly. It renders service DTOs and sends typed commands only.

## Design Patterns

- **Facade**: `frontend/lib/autonomy.ts` and backend focused Scheduler client
  provide stable command surfaces.
- **Command**: create, update, delete, lifecycle, and query calls are typed
  command/result DTOs.
- **Adapter / Bridge**: Web routes adapt HTTP DTOs into Scheduler service DTOs.
- **State**: job lifecycle and run state are modeled explicitly in DTOs and UI
  badges.
- **Observer / Memento**: recent runs are bounded sanitized mementos suitable
  for audit-oriented display.
- **Specification**: route and form validation enforce allowed schedule specs,
  target kinds, metadata bounds, and app scope.

These patterns clarify ownership without over-designing a new frontend
framework.

## Error Handling

The UI should show service states as first-class operational information:

- `unavailable`: autonomy runtime or Scheduler provider is not active.
- `unsupported`: provider does not support a requested schedule or target kind.
- `denied`: policy/capability gate rejected the mutation.
- `conflict`: concurrent lifecycle update or stale job version.
- `validation_error`: invalid interval, schedule spec, target, or metadata.
- `provider_failure`: Scheduler service accepted the boundary call but the
  provider failed.

Errors must be displayed with safe messages and optional trace ids. Raw backend
payloads must not be rendered.

## Testing and Verification Expectations

Implementation should add targeted tests for:

- Serviceized Web CRUD routes use Scheduler service clients, not legacy direct
  `macaca_task::TaskScheduler` construction.
- App scope is enforced for list/get/update/delete/lifecycle/run queries.
- Structured unavailable and provider failure responses render correctly.
- Frontend API facade maps DTOs without app-specific branches.
- Schedule manager components render empty, loading, error, active, paused, and
  failed-run states.
- Existing chat/session/agent workspace behavior does not regress.
- Boundary gates continue rejecting presentation-shell ownership leaks.

## Risks

- The existing legacy schedule route may look tempting because it already has
  CRUD behavior. Reusing it would violate the intended serviceized autonomy
  boundary for this feature.
- The current serviceized route surface is not yet full CRUD. Backend work must
  land before the frontend can honestly claim complete schedule management.
- The existing chat page is already large. The new UI must be split into focused
  components rather than adding another large inline block.
- A future global autonomy dashboard is useful, but it should come after the
  app-scoped CRUD path is complete and serviceized.

## Approval Gate

After this design is accepted, the next step is to write an implementation plan
and OpenSpec change for the serviceized schedule management UI and API surface.
