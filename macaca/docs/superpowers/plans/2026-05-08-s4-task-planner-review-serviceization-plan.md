# S4 Task/Planner/Review 服务化实施计划

## Scope

Implement S4 from `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`: move Task/Planner/Review orchestration out of `macaca-web::loop_manager` into a Task Service boundary that is compatible with `ServiceRuntime` and `SystemFacade`.

S4 covers goal decomposition, task claiming, review, coordinator resume, task lifecycle events, and task service snapshots. It does not serviceize LLM, Memory, Context, Driver, Skill, or MCP provider logic; those belong to later phases.

## Required Governance Inputs

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/route-c-regression-matrix.md`
- `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`
- `docs/superpowers/plans/2026-05-08-s3-sdk-system-facade-convergence-plan.md`
- `docs/superpowers/plans/2026-05-08-s1-service-runtime-v1-plan.md`

## Architecture Decision

Use a Task Service façade with mediator-style orchestration and explicit command/event boundaries.

Design patterns:

- Facade: Task Service exposes a small surface for goal/task/review/snapshot operations.
- Mediator: Task Service coordinates goal decomposition, worker claiming, review, and resume.
- State: Goal, task, review, and resume transitions are explicit and auditable.
- Observer: task lifecycle emits structured events to SSE/EventLog/trace adapters.
- Strategy: planner, reviewer, worker assignment, fallback decomposition, and resume policy are replaceable.
- Command: inputs become typed task service commands.
- Adapter / Bridge: Web `loop_manager` becomes an adapter over Task Service commands and event sinks.
- Specification: command constructors validate session scope, trace presence, limits, and review constraints.

Rejected alternatives:

- Move `loop_manager` wholesale into `macaca-task`: rejected because it would drag Web/framework/provider concerns into the task crate.
- Keep orchestration in Web and only add wrappers: rejected because it preserves Web as the system coordinator.
- Skip Task Service and call `ServiceRuntime` directly from SDK: rejected because task semantics belong in `macaca-task`, not SDK.

## Proposed OpenSpec Change

Expected change id:

- `add-task-planner-review-service-v1`

Expected artifacts:

- `openspec/changes/add-task-planner-review-service-v1/proposal.md`
- `openspec/changes/add-task-planner-review-service-v1/design.md`
- `openspec/changes/add-task-planner-review-service-v1/tasks.md`
- `openspec/changes/add-task-planner-review-service-v1/specs/task-service/spec.md`

The proposal should state:

- S4 is additive-first and preserves current user-visible task flows.
- Web must stop being the long-term coordinator for planner/worker/review behavior.
- Task Service must expose explicit commands, events, and snapshots.
- S4 does not migrate LLM/Memory/Context provider execution.
- Old compatibility entry points remain searchable and may be deprecated, but not removed yet.

## Implementation Slices

### Slice S4.1: Impact and Boundary Audit

Files to inspect before editing:

- `macaca/crates/macaca-task/src/lib.rs`
- `macaca/crates/macaca-task/src/task_service.rs` or new service module locations
- `macaca/crates/macaca-task/src/plan_loop.rs`
- `macaca/crates/macaca-task/src/worker_loop.rs`
- `macaca/crates/macaca-task/src/task_board.rs`
- `macaca/crates/macaca-web/src/loop_manager.rs`
- `macaca/crates/macaca-web/src/framework_runner.rs`
- `macaca/crates/macaca-web/src/sse.rs`
- `macaca/crates/macaca-web/src/event_persistence.rs`
- `macaca/crates/macaca-sdk/src/task_client.rs`
- `macaca/crates/macaca-runtime-host/src/service_runtime.rs`

Required actions:

1. Run GitNexus impact before modifying any existing symbol.
2. Warn if impact returns HIGH or CRITICAL.
3. Confirm the first slice can be additive-first without breaking `/api/chat/v2`, task board, trace, or resume flows.

### Slice S4.2: Task Service Contract

Files:

- New: `macaca/crates/macaca-task/src/service.rs`
- New: `macaca/crates/macaca-task/src/commands.rs`
- New: `macaca/crates/macaca-task/src/events.rs`
- New: `macaca/crates/macaca-task/src/snapshot.rs`
- Modify: `macaca/crates/macaca-task/src/lib.rs`

Behavior:

- Define typed commands for goal creation, task query, task claim, review submission, lifecycle snapshot, and resume signal emission.
- Define task service event types for goal ready, task claimed, review requested, review completed, goal completed, and resume requested.
- Define deterministic snapshots sorted by stable identifiers.
- Keep commands provider-neutral and trace-aware.

Rules:

- No provider construction.
- No app/provider/workflow/model/driver/gateway hardcoding.
- Detailed English comments explaining each command/event/snapshot role.
- Structured logs for start/completion/rejection.
- Keep files below 500 lines.

### Slice S4.3: Task Service Runtime Skeleton

Files:

- New: `macaca/crates/macaca-task/src/runtime.rs`
- New: `macaca/crates/macaca-task/src/provider.rs`
- Modify: `macaca/crates/macaca-task/src/lib.rs`

Behavior:

- Create a Task Service provider interface that can own the current TaskSpace/TaskBoard/PlanLoop/WorkerLoop orchestration.
- Expose a runtime skeleton that can start, stop, snapshot, and publish task lifecycle events.
- Keep the runtime capable of using injected strategies for planner/reviewer/worker execution.
- Keep event emission compatible with existing Web SSE/EventLog adapters.

Rules:

- The runtime must not pull LLM/Memory/Context provider code into `macaca-task`.
- The runtime must not become a new general-purpose workflow engine.
- The runtime must preserve the current trace and resume semantics.

### Slice S4.4: Web Adapter Extraction

Files:

- Modify: `macaca/crates/macaca-web/src/loop_manager.rs`
- Potentially add: `macaca/crates/macaca-web/src/task_service_adapter.rs`
- Potentially add: `macaca/crates/macaca-web/src/task_service_events.rs`

Behavior:

- Convert `loop_manager` into a thin adapter that translates HTTP/session state into Task Service commands.
- Keep task board, decomposition, review, worker delegation, and coordinator resume behavior compatible.
- Route task lifecycle events through the new Task Service event surface.
- Keep the current plan/worker fallback behavior available as compatibility logic until a later phase can replace it with service-native strategies.

Rules:

- Web must stop defining new task semantics.
- Web may still host compatibility execution adapters until the service runtime path is complete.
- Web logs must identify app/session scope, task ids, and event types.

### Slice S4.5: SDK Integration

Files:

- Modify: `macaca/crates/macaca-sdk/src/task_client.rs`
- Modify: `macaca/crates/macaca-sdk/src/system_facade.rs` if needed

Behavior:

- Add task service commands and snapshot client methods to the SDK boundary.
- Preserve current task board query behavior.
- Add typed client methods for task service command submission and task event query/replay where safe.

Rules:

- The SDK remains a facade and client boundary, not a task orchestrator.
- No provider/service hardcoding.

### Slice S4.6: Tests

Files:

- New: `macaca/crates/macaca-task/tests/task_service.rs`
- Modify targeted tests in `macaca/crates/macaca-web`

Test cases:

1. Goal creation emits a task service event and snapshot update.
2. Task claim / review / resume flow preserves current lifecycle semantics.
3. Task service snapshot is deterministic.
4. Missing trace or invalid scope is rejected before dispatch.
5. Web compatibility adapter still preserves current task board response shape.
6. Resume events are emitted when goal completion or review flow requires coordinator wakeup.

Constraints:

- Tests must not require network, frontend, browser, real LLM provider, Web3 node, EVM node, or external services.
- Use local mock services and deterministic traces only.

### Slice S4.7: Governance and Documentation

Files:

- Update: `macaca/docs/route-c-architecture-governance.md`
- Optional update: `macaca/docs/route-c-regression-matrix.md` only if a regression expectation needs to be made more explicit

Documentation must state:

- Task Service owns task lifecycle orchestration.
- Web is an adapter and renderer, not the long-term coordinator.
- Planner/reviewer/worker execution strategies can be replaced later without changing the command surface.
- S4 does not migrate LLM/Memory/Context providers.

## Dependency Boundary Expectations

Likely new direct dependencies:

- `macaca-task -> macaca-runtime-host` only if a runtime-host integration seam is needed.
- `macaca-web -> macaca-task` remains allowed during migration, but semantic ownership must move to the task service.

Expected S0 gate outcome:

- No new kernel/provider edges.
- No new presentation/provider construction hub edges.
- No new service provider -> presentation edges.

## Verification

Run after implementation:

```bash
openspec validate add-task-planner-review-service-v1 --strict
cargo fmt --all --check
cargo test -p macaca-task
cargo test -p macaca-web loop_manager
cargo test -p macaca-sdk task_client
cargo test -p macaca-integration-tests route_c_dependency_boundaries
cargo test -p macaca-integration-tests --test route_c_baseline
cargo check --workspace
npx gitnexus detect-changes -r agent
```
