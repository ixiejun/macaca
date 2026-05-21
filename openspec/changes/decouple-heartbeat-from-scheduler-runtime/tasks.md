## 1. OpenSpec and Planning

- [x] 1.1 Write Superpowers design document for decoupling Heartbeat from Scheduler.
- [x] 1.2 Write Superpowers implementation plan with concrete files, tests, and commands.
- [x] 1.3 Create OpenSpec proposal, design, tasks, and deltas.
- [x] 1.4 Run `openspec validate decouple-heartbeat-from-scheduler-runtime --strict`.

## 2. Heartbeat Native Cadence Contract

- [x] 2.1 Add provider-neutral heartbeat profile, scope, cadence, action summary, and memento DTOs.
- [x] 2.2 Add local Heartbeat provider tests for native cadence without Scheduler jobs.
- [x] 2.3 Implement native cadence evaluation, gate handling, and bounded heartbeat mementos.
- [x] 2.4 Add sanitized logs and trace/audit evidence for profile ticks, gate decisions, and action dispatch.

## 3. Runtime-Host Dual Lane Supervisor

- [x] 3.1 Add SchedulerLane and HeartbeatLane abstractions or modules under runtime-host.
- [x] 3.2 Ensure SchedulerLane can run due materialization without Heartbeat cadence.
- [x] 3.3 Ensure HeartbeatLane can run native heartbeat ticks without Scheduler due-run materialization.
- [x] 3.4 Add structured diagnostics for independent lane degradation and shutdown.

## 4. Scheduler Contract Cleanup

- [x] 4.1 Clarify Scheduler target contract so Scheduler does not own Heartbeat native cadence.
- [x] 4.2 Retain any Scheduler `HeartbeatWake` target as internal/runtime compatibility only if migration requires it.
- [x] 4.3 Add tests proving Scheduler preserves generic scheduled target dispatch without materializing Heartbeat cadence.

## 5. Web and Frontend Boundary

- [x] 5.1 Remove or hide Heartbeat wake target from application-facing schedule-management UI.
- [x] 5.2 Ensure Web schedule routes remain typed Scheduler command adapters only.
- [x] 5.3 Add a boundary gate or focused UI test preventing heartbeat-as-schedule regressions.

## 6. Verification and Documentation

- [x] 6.1 Update autonomy Scheduler/Heartbeat documentation with dual-lane ownership.
- [x] 6.2 Run focused Heartbeat, Scheduler, runtime-host, and integration tests.
- [x] 6.3 Run frontend lint and TypeScript checks if frontend UI changes are implemented.
- [x] 6.4 Run serviceization escape-hatch and dependency-boundary tests.
- [x] 6.5 Run `openspec validate decouple-heartbeat-from-scheduler-runtime --strict` after implementation.
