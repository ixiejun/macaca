# Decouple Heartbeat From Scheduler Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decouple Heartbeat native cadence from Scheduler job execution while preserving serviceized autonomy boundaries, traceability, auditability, and provider replacement.

**Architecture:** Runtime-host `AutonomySupervisor` will own two sibling autonomy lanes: `SchedulerLane` for Scheduler due-run materialization and `HeartbeatLane` for Heartbeat native cadence. Scheduler remains responsible for scheduled jobs; Heartbeat becomes responsible for agent/system heartbeat profiles, cadence, gates, and heartbeat mementos without using application-facing Scheduler jobs.

**Tech Stack:** Rust workspace under `macaca/`, OpenSpec, Next.js frontend shell under `frontend/`, existing Scheduler/Heartbeat service crates, runtime-host autonomy bootstrap, integration tests.

---

## File Structure

- Modify: `openspec/changes/decouple-heartbeat-from-scheduler-runtime/proposal.md` to describe why the architecture changes.
- Modify: `openspec/changes/decouple-heartbeat-from-scheduler-runtime/design.md` to lock the dual-lane supervisor decisions.
- Modify: `openspec/changes/decouple-heartbeat-from-scheduler-runtime/tasks.md` to track implementation honestly.
- Modify: `openspec/changes/decouple-heartbeat-from-scheduler-runtime/specs/heartbeat-service/spec.md` to add native cadence/profile semantics and remove Scheduler dependency as the primary heartbeat trigger.
- Modify: `openspec/changes/decouple-heartbeat-from-scheduler-runtime/specs/autonomous-runtime/spec.md` to require sibling Scheduler and Heartbeat supervisor lanes.
- Modify: `openspec/changes/decouple-heartbeat-from-scheduler-runtime/specs/scheduler-service/spec.md` to clarify Scheduler does not own heartbeat cadence.
- Modify: `openspec/changes/decouple-heartbeat-from-scheduler-runtime/specs/web-cli-thin-shell-v0/spec.md` to keep heartbeat out of application schedule-management UI.
- Modify: `openspec/changes/decouple-heartbeat-from-scheduler-runtime/specs/serviceization-escape-hatches/spec.md` to gate scheduler-owned heartbeat cadence regressions.
- Later implementation will likely modify runtime-host autonomy supervisor modules, heartbeat local provider modules, scheduler target DTO handling, Web routes, frontend schedule editor, and integration tests.

## Task 1: OpenSpec Design Package

**Files:**
- Modify: `openspec/changes/decouple-heartbeat-from-scheduler-runtime/proposal.md`
- Modify: `openspec/changes/decouple-heartbeat-from-scheduler-runtime/design.md`
- Modify: `openspec/changes/decouple-heartbeat-from-scheduler-runtime/tasks.md`

- [ ] **Step 1: Confirm active OpenSpec context**

Run:

```bash
openspec list
openspec list --specs
```

Expected: active changes are listed, and baseline specs include service/runtime facade specs.

- [ ] **Step 2: Review constitutional documents**

Run:

```bash
sed -n '1,260p' macaca/docs/macaca-os-architecture-governance.md
sed -n '1,260p' macaca/docs/macaca-os-microkernel-boundaries.md
sed -n '1,260p' macaca/docs/macaca-os-serviceization-allowlist.md
```

Expected: confirm runtime-host owns provider composition, Web/CLI/frontend are adapters, and non-kernel capabilities stay serviceized.

- [ ] **Step 3: Validate the proposal package**

Run:

```bash
openspec validate decouple-heartbeat-from-scheduler-runtime --strict
```

Expected: validation passes before implementation starts.

## Task 2: Heartbeat Native Cadence Contract

**Files:**
- Modify: `macaca/crates/foundation/macaca-proto/src/heartbeat_service.rs`
- Modify: `macaca/crates/services/macaca-heartbeat/src/local_provider.rs`
- Test: `macaca/crates/services/macaca-heartbeat/src/local_provider.rs`

- [ ] **Step 1: Add failing heartbeat cadence/profile tests**

Add tests that prove Heartbeat can decide a due native tick without a Scheduler job. Test names should include:

```rust
native_heartbeat_profile_ticks_without_scheduler_job
native_heartbeat_tick_records_trace_and_audit_memento
native_heartbeat_tick_respects_cooldown_gate
```

Expected: tests fail because native profile/cadence APIs do not exist yet.

- [ ] **Step 2: Add provider-neutral DTOs**

Add DTOs for heartbeat profile identity, cadence, scope identity, next tick, last tick, safe action summary, and bounded memento metadata. All comments must be in English and explain the runtime role of each DTO.

- [ ] **Step 3: Implement local provider cadence state**

Implement local provider state for heartbeat profiles, cadence calculation, coalescing, gate evaluation, and memento recording. Add `tracing::info!` or `tracing::warn!` logs at profile registration, tick acceptance, gate skip, action dispatch, and completion.

- [ ] **Step 4: Verify heartbeat service**

Run:

```bash
cd macaca
cargo test -p macaca-heartbeat native_heartbeat -- --nocapture
```

Expected: native heartbeat tests pass with sanitized trace/audit evidence.

## Task 3: Runtime-Host Dual Lane Supervisor

**Files:**
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_supervisor.rs`
- Create or Modify: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_supervisor/scheduler_lane.rs`
- Create or Modify: `macaca/crates/runtime/macaca-runtime-host/src/autonomy_supervisor/heartbeat_lane.rs`
- Test: `macaca/crates/tests/macaca-integration-tests/tests/autonomy_scheduler_heartbeat_services.rs`

- [ ] **Step 1: Add failing integration tests**

Add tests proving:

```rust
supervisor_runs_scheduler_lane_without_heartbeat_cadence
supervisor_runs_heartbeat_lane_without_scheduler_due_runs
supervisor_reports_structured_lane_degradation
```

Expected: tests fail because lanes are not separately modeled yet.

- [ ] **Step 2: Split lane interfaces**

Introduce small lane structs or traits so Scheduler and Heartbeat loops can be started, stopped, ticked, and snapshotted independently. English comments must explain why the lanes are siblings and why Heartbeat does not depend on Scheduler jobs.

- [ ] **Step 3: Wire lifecycle and logs**

Update supervisor startup and shutdown so each lane emits sanitized logs for start, tick, skip, failure, shutdown, and snapshot. Ensure lane failure returns structured diagnostics and does not silently fake success.

- [ ] **Step 4: Verify runtime-host and integration behavior**

Run:

```bash
cd macaca
cargo test -p macaca-integration-tests autonomy_scheduler_heartbeat_services -- --nocapture
```

Expected: dual-lane tests pass.

## Task 4: Scheduler Contract Cleanup

**Files:**
- Modify: `macaca/crates/foundation/macaca-proto/src/scheduler_service.rs`
- Modify: `macaca/crates/services/macaca-scheduler/src/local_provider.rs`
- Test: `macaca/crates/services/macaca-scheduler/src/local_provider.rs`

- [ ] **Step 1: Add failing scheduler contract tests**

Add tests proving application-facing Scheduler job targets do not require or expose Heartbeat cadence semantics. Test names should include:

```rust
scheduler_does_not_materialize_native_heartbeat_cadence
scheduler_preserves_generic_service_target_dispatch
```

Expected: tests fail until target handling and docs are clarified.

- [ ] **Step 2: Mark HeartbeatWake target compatibility-only if retained**

If the DTO remains for migration, document it as internal/runtime compatibility and not as an application-facing schedule-management target. Do not remove public DTOs without an explicit migration path.

- [ ] **Step 3: Verify scheduler tests**

Run:

```bash
cd macaca
cargo test -p macaca-scheduler scheduler_ -- --nocapture
```

Expected: Scheduler tests pass and heartbeat cadence is not owned by Scheduler.

## Task 5: Web and Frontend Schedule UI Boundary

**Files:**
- Modify: `frontend/components/autonomy/ScheduleEditorDrawer.tsx`
- Modify: `frontend/lib/autonomy-types.ts`
- Modify: `frontend/lib/autonomy.ts`
- Modify: `macaca/crates/shells/macaca-web/src/routes.rs`

- [ ] **Step 1: Add focused UI/API tests or static boundary checks**

Add a gate or test that fails if application-facing schedule UI exposes `Heartbeat wake` as a normal target option. The check should look only at production UI/API adapter files and avoid test fixtures.

- [ ] **Step 2: Remove heartbeat target from schedule creation UI**

Update the schedule editor to create generic service-command scheduled jobs only, or to show heartbeat target as unavailable/internal if compatibility requires visibility. The UI must not define heartbeat semantics.

- [ ] **Step 3: Keep routes as adapters**

Ensure Web routes adapt requests into typed Scheduler commands and do not construct Heartbeat providers, timers, or heartbeat profiles.

- [ ] **Step 4: Verify frontend shell**

Run:

```bash
cd frontend
npm run lint
npx tsc --noEmit
```

Expected: lint and TypeScript checks pass.

## Task 6: Boundary Gates and Documentation

**Files:**
- Modify: `macaca/crates/tests/macaca-integration-tests/tests/serviceization_escape_hatches.rs`
- Modify: `macaca/docs/autonomy-scheduler-heartbeat-services.md`
- Modify: `openspec/changes/decouple-heartbeat-from-scheduler-runtime/tasks.md`

- [ ] **Step 1: Add escape-hatch gate coverage**

Add checks that reject new production code where Scheduler owns heartbeat native cadence or frontend/Web creates heartbeat timers. Keep the gate generic and avoid app-specific strings.

- [ ] **Step 2: Update architecture docs**

Update autonomy documentation to state that Scheduler and Heartbeat are sibling services coordinated by runtime-host lanes. Include English comments in code-facing examples and sanitized logging requirements.

- [ ] **Step 3: Run architecture verification**

Run:

```bash
cd macaca
cargo test -p macaca-integration-tests --test serviceization_escape_hatches --test route_c_dependency_boundaries -- --nocapture
```

Expected: boundary tests pass.

- [ ] **Step 4: Validate OpenSpec and checklist truthfulness**

Run:

```bash
openspec validate decouple-heartbeat-from-scheduler-runtime --strict
```

Expected: strict validation passes. Only mark tasks complete after the corresponding code and verification evidence exist.

## Self-Review

- Spec coverage: the plan covers Heartbeat native cadence, runtime dual lanes, Scheduler cleanup, UI boundary, escape-hatch gates, tests, and docs.
- Placeholder scan: no placeholders are present; implementation tasks name concrete paths, expected tests, and commands.
- Boundary review: all behavior remains in runtime-host/services; Web/frontend remain adapters; kernel does not gain concrete timers or providers.
