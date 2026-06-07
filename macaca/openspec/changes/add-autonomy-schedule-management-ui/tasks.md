# Tasks: Add Autonomy Schedule Management UI

## 1. OpenSpec and Governance

- [x] 1.1 Validate this proposal against `macaca-os-architecture-governance.md`,
  `macaca-os-microkernel-boundaries.md`, and
  `macaca-os-serviceization-allowlist.md`.
- [x] 1.2 Confirm the new UI does not use legacy
  `/api/apps/{app_id}/schedules` routes for serviceized autonomy management.
- [x] 1.3 Run `openspec validate add-autonomy-schedule-management-ui --strict`.

## 2. Scheduler Service Job Management Contract

- [x] 2.1 Add provider-neutral Scheduler job management commands/results in
  `macaca-proto`.
- [x] 2.2 Add validation for trace context, app scope, job id, lifecycle
  operations, metadata bounds, and query limits.
- [x] 2.3 Add detailed English comments explaining Scheduler ownership,
  provider neutrality, and shell non-goals.
- [x] 2.4 Add structured error states for unavailable, unsupported, denied,
  conflict, validation failure, and provider failure.

## 3. SDK Scheduler Client

- [x] 3.1 Add focused Scheduler client methods for list/get/update/delete and
  lifecycle transitions.
- [x] 3.2 Preserve structured unavailable behavior when no active Scheduler
  provider is installed.
- [x] 3.3 Add structured logs at command validation, client delegation,
  success, rejection, and failure.
- [x] 3.4 Ensure the SDK client does not construct concrete providers.

## 4. Local Scheduler Provider

- [x] 4.1 Implement app-scoped list/get/update/delete job behavior.
- [x] 4.2 Implement pause/resume lifecycle transitions that affect due-run
  eligibility without deleting run history.
- [x] 4.3 Keep job summaries and run summaries sanitized and bounded.
- [x] 4.4 Add structured logs for job list, get, update, lifecycle, delete,
  mutation success, and mutation rejection.
- [x] 4.5 Add tests proving app scope isolation, lifecycle behavior, and delete
  behavior.

## 5. Web Shell Route Adapters

- [x] 5.1 Add serviceized autonomy CRUD routes under
  `/api/apps/{app_id}/autonomy/schedules`.
- [x] 5.2 Ensure every route creates trace context and calls the Scheduler SDK
  client.
- [x] 5.3 Map service errors to safe HTTP responses without exposing raw
  provider payloads.
- [x] 5.4 Add structured logs for request receipt, command construction,
  service delegation, success, rejection, and failure.
- [x] 5.5 Register routes in `macaca-web` bootstrap without changing legacy
  compatibility routes.

## 6. Frontend API Facade

- [x] 6.1 Add `frontend/lib/autonomy-types.ts` with provider-neutral DTOs.
- [x] 6.2 Add `frontend/lib/autonomy.ts` as the only frontend API facade for
  serviceized autonomy schedule management.
- [x] 6.3 Ensure frontend fetch functions call `/api/apps/{app_id}/autonomy/*`
  only.
- [x] 6.4 Add safe error normalization for unavailable, unsupported, denied,
  conflict, validation, and provider failure states.

## 7. Frontend Schedule Management UI

- [x] 7.1 Add focused schedule-management components under
  `frontend/components/autonomy/`.
- [x] 7.2 Add an application-scoped `AUTONOMY` workspace tab or equivalent panel.
- [x] 7.3 Implement browse, create, edit, delete, pause/resume, refresh, and
  run monitoring interactions.
- [x] 7.4 Match the existing black/green Macaca terminal workspace style.
- [x] 7.5 Keep components generic with no application-specific templates,
  workflow names, provider names, driver names, model names, chain names,
  payment names, gateway names, or business-domain branches.

## 8. Boundary Gates

- [x] 8.1 Extend escape-hatch tests to reject new schedule-management callers
  using legacy direct schedule paths.
- [x] 8.2 Extend dependency-boundary tests to reject provider construction in
  Web/frontend/SDK schedule management paths.
- [x] 8.3 Run targeted boundary tests.

## 9. Documentation and Validation

- [x] 9.1 Update `macaca/docs/autonomy-scheduler-heartbeat-services.md` with
  the schedule-management UI and API ownership model.
- [x] 9.2 Document operator verification for create, list, edit, pause/resume,
  delete, and run monitoring.
- [x] 9.3 Run targeted Rust compile checks and tests.
- [x] 9.4 Run frontend lint/type checks.
- [x] 9.5 Run OpenSpec strict validation.
- [x] 9.6 Mark tasks complete only after implementation and validation evidence
  exists.
