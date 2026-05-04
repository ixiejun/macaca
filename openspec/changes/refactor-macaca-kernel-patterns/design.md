## Context

`macaca-kernel` owns the system coordination surface: kernel facade, registry, scheduler, status tracker, executor, event bus, and orchestration. It is consumed by web, app, sdk, cli, and integration tests. `ApplicationExecutor` and `ForkManager` are large files, but this change prioritizes low-risk primitive extraction before file splitting.

The current code already contains a few abstractions:

- `Kernel` is the public facade.
- `Scheduler` is a strategy trait.
- `SimpleScheduler` is the only scheduler strategy.
- `AgentStatusTracker` is the status write boundary.
- `ExecutorEvent` is the observer contract consumed by SSE/EventLog/session restore.

However, lifecycle event construction still appears in multiple places, including `macaca-web`, and `Kernel::new` fixes the scheduler implementation directly.

## Goals

- Keep behavior 1:1 compatible.
- Preserve `ExecutorEvent` and `TaskResult` payload shape.
- Move lifecycle event construction into kernel-owned helpers.
- Make scheduler construction extensible without changing the selected scheduler.
- Make status transitions explicit without introducing new lifecycle semantics.
- Add kernel builder/facade primitives without removing `Kernel::new`.
- Mark old direct construction surfaces deprecated after additive replacements exist.

## Non-Goals

- Do not split `ApplicationExecutor` or `ForkManager` into many files in this change.
- Do not change task scheduling order.
- Do not change worker supervision, fork resume, queue semantics, SSE, EventLog, session restore, or web trace behavior.
- Do not hardcode application, workflow, driver, or agent names.
- Do not introduce third-party dependencies.

## Decisions

### Decision 1: Use factory helpers for executor lifecycle payloads

`ExecutorEventFactory` owns construction of started/completed/failed events and success/failed `TaskResult` values. It does not change `ExecutorEvent` or `TaskResult` shapes.

Alternative considered: deprecate `ExecutorEvent` variants directly. Rejected because enum variants are pattern-matched by SSE/EventLog/session restore and deprecating variants would create noisy warnings on readers, not just constructors.

### Decision 2: Use factory for scheduler construction

`SchedulerFactory` and `SchedulerKind` provide a strategy construction boundary. The first implementation only returns `SimpleScheduler` to preserve behavior.

Alternative considered: add new scheduler algorithms now. Rejected because this refactor must not change task assignment semantics.

### Decision 3: Use state transition policy for status updates

`AgentStatusTransitionPolicy` centralizes existing status/activity mutations. It intentionally does not add new state rules.

Alternative considered: enforce a strict lifecycle graph. Rejected as too risky for this slice because current callers rely on flexible activity updates.

### Decision 4: Keep deprecated interfaces callable

`Kernel::new` and direct `SimpleScheduler` construction remain callable but are marked deprecated. Compatibility code and tests may use local `#[allow(deprecated)]`; new production code should use `KernelBuilder` and `SchedulerFactory`.

Alternative considered: remove or make old constructors private. Rejected because this project uses additive-first migrations and needs grepable deprecated APIs for later consumer migration.

## Risks / Mitigations

- Risk: `ExecutorEvent` regressions can break UI trace and history restore.
  Mitigation: factory tests assert exact payload fields and web helpers delegate to kernel helpers.

- Risk: `ApplicationExecutor` edits can affect live worker execution.
  Mitigation: only replace payload construction; keep send/broadcast order unchanged.

- Risk: `Kernel::new` has many consumers.
  Mitigation: keep `Kernel::new` callable and delegate internally to `KernelBuilder`.

- Risk: scheduler behavior changes.
  Mitigation: default factory returns only `SimpleScheduler` and parity tests cover empty-registry behavior.

## Migration Plan

1. Add OpenSpec contract and validate it.
2. Run GitNexus impact for changed kernel symbols.
3. Add executor lifecycle factory and tests.
4. Migrate web-local executor helper bodies and kernel worker payload construction.
5. Add scheduler factory and use it from `Kernel`.
6. Add status transition policy and route tracker helpers through it.
7. Replace selected `ApplicationExecutor` lifecycle payload construction with factory calls.
8. Add `KernelBuilder`; keep `Kernel::new` as deprecated compatibility entry.
9. Run targeted tests/checks and GitNexus detect changes.

## Open Questions

None for this slice. Splitting oversized `ApplicationExecutor` and `ForkManager` files remains a follow-up proposal after payload construction boundaries are stable.
