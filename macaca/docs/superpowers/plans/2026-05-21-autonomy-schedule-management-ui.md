# Autonomy Schedule Management UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an application-scoped Web UI for browsing, creating, editing, deleting, pausing/resuming, and monitoring serviceized autonomy Scheduler jobs.

**Architecture:** The frontend remains a generic shell adapter and calls serviceized `/api/apps/{app_id}/autonomy/*` routes only. Web routes adapt HTTP DTOs into focused Scheduler SDK client commands, while Scheduler service providers own job lifecycle, run mementos, due calculation, leasing, and execution outcomes.

**Tech Stack:** Rust, Axum, `macaca-proto`, `macaca-sdk`, `macaca-runtime-host`, `macaca-scheduler`, Next.js App Router, TypeScript, React, Tailwind/global CSS, OpenSpec.

---

## Files and Responsibilities

- Create `macaca/crates/foundation/macaca-proto/src/scheduler_service/job_management.rs`: provider-neutral Scheduler job management DTOs, command constructors, and validation helpers.
- Modify `macaca/crates/foundation/macaca-proto/src/scheduler_service.rs`: re-export job management DTOs from the Scheduler service contract.
- Modify `macaca/crates/facade/macaca-sdk/src/scheduler_client.rs`: add focused Scheduler client methods for list/get/update/delete/lifecycle job operations.
- Modify `macaca/crates/services/macaca-scheduler/src/service_contract.rs`: route new job management commands into the provider-neutral Scheduler service trait.
- Modify `macaca/crates/services/macaca-scheduler/src/local_provider.rs`: implement CRUD/lifecycle behavior against local provider state without application-specific logic.
- Modify `macaca/crates/shells/macaca-web/src/routes.rs`: add serviceized autonomy job CRUD route handlers that create trace contexts and call Scheduler client methods.
- Modify `macaca/crates/shells/macaca-web/src/bootstrap.rs`: register the new `/api/apps/{app_id}/autonomy/schedules` GET/PATCH/DELETE/lifecycle routes.
- Create `frontend/lib/autonomy-types.ts`: TypeScript DTOs matching sanitized serviceized Scheduler responses.
- Create `frontend/lib/autonomy.ts`: frontend API Facade for serviceized autonomy routes.
- Create `frontend/components/autonomy/ScheduleManagerPanel.tsx`: top-level generic schedule management panel.
- Create `frontend/components/autonomy/ScheduleSummaryStrip.tsx`: summary metrics presentation.
- Create `frontend/components/autonomy/ScheduleList.tsx`: job list and action controls.
- Create `frontend/components/autonomy/ScheduleEditorDrawer.tsx`: create/edit form with provider-neutral schedule and target fields.
- Create `frontend/components/autonomy/SchedulerRunTimeline.tsx`: bounded run memento timeline.
- Create `frontend/components/autonomy/ScheduleStateBadge.tsx`: lifecycle/run state presentation helper.
- Modify `frontend/app/chat/[appId]/page.tsx`: compose the `AUTONOMY` tab/panel without embedding Scheduler semantics inline.
- Modify `frontend/app/globals.css`: add schedule-management classes matching the existing black/green terminal workspace style.
- Modify `macaca/crates/tests/macaca-integration-tests/tests/autonomy_scheduler_heartbeat_services.rs`: add serviceized Scheduler CRUD tests.
- Modify `macaca/crates/tests/macaca-integration-tests/tests/serviceization_escape_hatches.rs`: block new frontend/Web callers from legacy direct schedule paths.
- Modify `macaca/docs/autonomy-scheduler-heartbeat-services.md`: document UI/API ownership and safe operator workflow.
- Create or update OpenSpec change `openspec/changes/add-autonomy-schedule-management-ui`.

## Task 1: OpenSpec and Boundary Proposal

**Files:**
- Create: `openspec/changes/add-autonomy-schedule-management-ui/proposal.md`
- Create: `openspec/changes/add-autonomy-schedule-management-ui/design.md`
- Create: `openspec/changes/add-autonomy-schedule-management-ui/tasks.md`
- Create: `openspec/changes/add-autonomy-schedule-management-ui/specs/scheduler-service/spec.md`
- Create: `openspec/changes/add-autonomy-schedule-management-ui/specs/sdk-system-facade/spec.md`
- Create: `openspec/changes/add-autonomy-schedule-management-ui/specs/web-cli-thin-shell-v0/spec.md`
- Create: `openspec/changes/add-autonomy-schedule-management-ui/specs/serviceization-escape-hatches/spec.md`

- [ ] **Step 1: Confirm OpenSpec validates before implementation**

Run:

```bash
openspec validate add-autonomy-schedule-management-ui --strict
```

Expected: PASS. If it fails, fix proposal/spec syntax before touching code.

- [ ] **Step 2: Confirm design coverage**

Check that the OpenSpec design explicitly answers:

```text
Which layer owns job lifecycle? service.scheduler
Which layer owns HTTP parsing? macaca-web shell
Which layer owns rendering? frontend shell
Which boundary carries calls? Scheduler SDK client / service runtime
What is prohibited? legacy /api/apps/{app_id}/schedules, provider construction, app-specific templates
```

Expected: all questions have concrete answers in `design.md`.

## Task 2: Scheduler Service DTOs and Client Contract

**Files:**
- Create: `macaca/crates/foundation/macaca-proto/src/scheduler_service/job_management.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/scheduler_service.rs`
- Modify: `macaca/crates/facade/macaca-sdk/src/scheduler_client.rs`

- [ ] **Step 1: Write proto-level unit tests for command validation**

Add tests that construct invalid blank job ids, blank lifecycle operations, excessive limits, and valid app-scoped query commands.

Run:

```bash
cargo test -p macaca-proto scheduler_service -- --nocapture
```

Expected: validation tests fail until DTOs are implemented.

- [ ] **Step 2: Implement DTOs with English comments**

Add provider-neutral command/result DTOs:

```rust
SchedulerListJobsCommand
SchedulerGetJobCommand
SchedulerUpdateJobCommand
SchedulerDeleteJobCommand
SchedulerLifecycleJobCommand
SchedulerJobSummary
SchedulerJobMutationResult
SchedulerJobLifecycleCommand
```

Each constructor must require `TraceContext`, `AutonomyScope`, bounded limits, and non-empty job ids where applicable.

- [ ] **Step 3: Extend Scheduler SDK client**

Add focused client methods:

```rust
list_jobs(command: SchedulerListJobsCommand)
get_job(command: SchedulerGetJobCommand)
update_job(command: SchedulerUpdateJobCommand)
delete_job(command: SchedulerDeleteJobCommand)
transition_job(command: SchedulerLifecycleJobCommand)
```

Expected behavior: unavailable providers return structured unavailable/unsupported errors, never panics.

- [ ] **Step 4: Verify proto and SDK compile**

Run:

```bash
cargo check -p macaca-proto -p macaca-sdk
```

Expected: PASS.

## Task 3: Local Scheduler Provider CRUD

**Files:**
- Modify: `macaca/crates/services/macaca-scheduler/src/service_contract.rs`
- Modify: `macaca/crates/services/macaca-scheduler/src/local_provider.rs`
- Modify: `macaca/crates/services/macaca-scheduler/src/local_provider/run_control.rs`

- [ ] **Step 1: Write provider tests for app-scoped CRUD**

Add tests proving:

```text
register job -> list returns only same app scope
get job -> returns sanitized summary
update job -> updates schedule spec and metadata without changing app scope
pause job -> due runs are not leased
resume job -> due runs become eligible again
delete job -> job no longer appears and future runs are not created
```

Run:

```bash
cargo test -p macaca-scheduler -- --nocapture
```

Expected: tests fail until provider CRUD is implemented.

- [ ] **Step 2: Implement provider-neutral CRUD**

Implement CRUD by mutating local provider state under existing synchronization primitives. Use explicit lifecycle state transitions and structured logs:

```text
scheduler.job.list.requested
scheduler.job.get.requested
scheduler.job.update.requested
scheduler.job.lifecycle.requested
scheduler.job.delete.requested
scheduler.job.mutation.completed
scheduler.job.mutation.rejected
```

Logs must include trace id, scope kind, job id when available, lifecycle operation, and safe error codes only.

- [ ] **Step 3: Verify provider tests**

Run:

```bash
cargo test -p macaca-scheduler -- --nocapture
```

Expected: PASS.

## Task 4: Serviceized Web Route Adapters

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/routes.rs`
- Modify: `macaca/crates/shells/macaca-web/src/bootstrap.rs`

- [ ] **Step 1: Write route tests or integration tests for serviceized CRUD**

Add tests proving the new routes call `state.scheduler_client` and never instantiate `macaca_task::TaskScheduler`.

Route surface:

```text
GET    /api/apps/{app_id}/autonomy/schedules
GET    /api/apps/{app_id}/autonomy/schedules/{job_id}
PATCH  /api/apps/{app_id}/autonomy/schedules/{job_id}
DELETE /api/apps/{app_id}/autonomy/schedules/{job_id}
PUT    /api/apps/{app_id}/autonomy/schedules/{job_id}/lifecycle
GET    /api/apps/{app_id}/autonomy/scheduler/runs
```

Run:

```bash
cargo test -p macaca-integration-tests autonomy_scheduler_heartbeat_services -- --nocapture
```

Expected: route tests fail until handlers exist.

- [ ] **Step 2: Implement route DTO adapters**

Routes must:

```text
parse app_id and job_id
create TraceContext with route-safe operation label
validate request body fields and metadata bounds
construct Scheduler SDK commands
call scheduler_client
map structured service errors to HTTP status
return sanitized JSON only
emit structured logs at request, command construction, success, rejection, and failure
```

- [ ] **Step 3: Register routes in bootstrap**

Register only `/autonomy` serviceized routes. Do not alter legacy `/api/apps/{app_id}/schedules` behavior in this task.

- [ ] **Step 4: Verify Web compile**

Run:

```bash
cargo check -p macaca-web
```

Expected: PASS.

## Task 5: Frontend Autonomy API Facade

**Files:**
- Create: `frontend/lib/autonomy-types.ts`
- Create: `frontend/lib/autonomy.ts`

- [ ] **Step 1: Add TypeScript DTOs**

Define DTOs for:

```ts
AutonomyScheduleJob
AutonomyScheduleListResponse
AutonomyScheduleMutationResponse
AutonomyScheduleEditorDraft
AutonomySchedulerRun
AutonomySchedulerRunsResponse
AutonomyServiceError
```

DTOs must use provider-neutral names such as `target_kind`, `service_id`, `command_name`, `wake_scope_key`, and `wake_reason_code`.

- [ ] **Step 2: Add API facade functions**

Implement functions:

```ts
fetchAutonomySchedules(appId: string)
fetchAutonomySchedule(appId: string, jobId: string)
createAutonomySchedule(appId: string, draft: AutonomyScheduleEditorDraft)
updateAutonomySchedule(appId: string, jobId: string, draft: AutonomyScheduleEditorDraft)
deleteAutonomySchedule(appId: string, jobId: string)
transitionAutonomySchedule(appId: string, jobId: string, lifecycle: 'pause' | 'resume')
fetchAutonomySchedulerRuns(appId: string, limit?: number)
```

Each function must call `/api/apps/${appId}/autonomy/...`, never legacy `/schedules`.

- [ ] **Step 3: Run TypeScript lint/check**

Run:

```bash
cd frontend && npm run lint
```

Expected: PASS or project-known warnings only.

## Task 6: Frontend Schedule Management Components

**Files:**
- Create: `frontend/components/autonomy/ScheduleManagerPanel.tsx`
- Create: `frontend/components/autonomy/ScheduleSummaryStrip.tsx`
- Create: `frontend/components/autonomy/ScheduleList.tsx`
- Create: `frontend/components/autonomy/ScheduleEditorDrawer.tsx`
- Create: `frontend/components/autonomy/SchedulerRunTimeline.tsx`
- Create: `frontend/components/autonomy/ScheduleStateBadge.tsx`
- Modify: `frontend/app/globals.css`

- [ ] **Step 1: Build component skeletons**

Each file must have one focused responsibility and stay under 200 lines where practical.

Use existing visual language:

```text
black workspace background
green borders and labels
uppercase mono labels
compact operational telemetry
trace/audit ids rendered as safe diagnostic chips
```

- [ ] **Step 2: Implement manager state flow**

`ScheduleManagerPanel` owns load, refresh, create/edit drawer state, mutation pending state, and safe error display. It delegates rendering to child components.

- [ ] **Step 3: Implement generic editor**

The editor must expose provider-neutral fields only:

```text
name
interval seconds
target kind: heartbeat_wake or service
service id
command name
wake scope key
wake reason code
metadata key/value pairs
```

Do not add application-specific templates or workflow presets.

- [ ] **Step 4: Verify frontend checks**

Run:

```bash
cd frontend && npm run lint
```

Expected: PASS or project-known warnings only.

## Task 7: Compose AUTONOMY Tab in Existing Workspace

**Files:**
- Modify: `frontend/app/chat/[appId]/page.tsx`

- [ ] **Step 1: Add generic autonomy tab**

Add an `AUTONOMY` tab alongside the main thread and agent tabs. The page must only compose `ScheduleManagerPanel` and must not inline CRUD logic.

- [ ] **Step 2: Preserve app-owned UI behavior**

If an application owns the full workspace surface, keep existing behavior unchanged unless the shell already exposes universal side panels for that mode.

- [ ] **Step 3: Verify frontend checks**

Run:

```bash
cd frontend && npm run lint
```

Expected: PASS or project-known warnings only.

## Task 8: Boundary Gates and Regression Tests

**Files:**
- Modify: `macaca/crates/tests/macaca-integration-tests/tests/serviceization_escape_hatches.rs`
- Modify: `macaca/crates/tests/macaca-integration-tests/tests/route_c_dependency_boundaries/gate.rs`
- Modify: `macaca/crates/tests/macaca-integration-tests/tests/autonomy_scheduler_heartbeat_services.rs`

- [ ] **Step 1: Add escape-hatch checks**

Block new production callers that use the legacy schedule route or direct `macaca_task::TaskScheduler` construction for autonomy schedule management.

- [ ] **Step 2: Add dependency boundary assertions**

Assert frontend/Web schedule management paths are shell adapters and do not construct Scheduler providers.

- [ ] **Step 3: Run targeted integration gates**

Run:

```bash
cargo test -p macaca-integration-tests --test autonomy_scheduler_heartbeat_services --test serviceization_escape_hatches --test route_c_dependency_boundaries
```

Expected: PASS.

## Task 9: Documentation and Manual Verification

**Files:**
- Modify: `macaca/docs/autonomy-scheduler-heartbeat-services.md`
- Modify: `openspec/changes/add-autonomy-schedule-management-ui/tasks.md`

- [ ] **Step 1: Document operator flow**

Document:

```text
enable local autonomy runtime
open application workspace
open AUTONOMY tab
create heartbeat wake schedule
observe run timeline
pause/resume/delete schedule
inspect trace/audit ids
```

- [ ] **Step 2: Run OpenSpec validation**

Run:

```bash
openspec validate add-autonomy-schedule-management-ui --strict
```

Expected: PASS.

- [ ] **Step 3: Run full targeted validation set**

Run:

```bash
cargo check -p macaca-proto -p macaca-sdk -p macaca-scheduler -p macaca-web
cargo test -p macaca-scheduler -- --nocapture
cargo test -p macaca-integration-tests --test autonomy_scheduler_heartbeat_services --test serviceization_escape_hatches --test route_c_dependency_boundaries
cd frontend && npm run lint
```

Expected: PASS or explicitly documented pre-existing frontend lint warnings.

- [ ] **Step 4: Update OpenSpec task checkboxes truthfully**

Only mark tasks complete after code, docs, tests, and validation have actually run and matched the expected result.

## Self-Review

- Spec coverage: The plan covers service contracts, SDK facade, Web shell adapters, frontend generic shell UI, boundary gates, docs, and validation.
- Placeholder scan: No `TBD`, `TODO`, or vague "handle edge cases" instructions remain.
- Type consistency: Frontend DTO names, route names, and Scheduler command names are consistent across tasks.
- Boundary consistency: The plan explicitly prohibits legacy `/api/apps/{app_id}/schedules` usage for this feature and keeps Scheduler lifecycle semantics in `service.scheduler`.
