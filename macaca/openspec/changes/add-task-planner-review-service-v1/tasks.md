## 1. Preparation

- [x] 1.1 Read `docs/superpowers/plans/2026-05-08-s4-task-planner-review-serviceization-plan.md`.
- [x] 1.2 Read Route C governance docs and regression matrix.
- [x] 1.3 Inspect current `macaca-task`, `macaca-web`, and `macaca-sdk` task orchestration code.
- [x] 1.4 Run GitNexus impact before editing any existing symbol.

## 2. OpenSpec

- [x] 2.1 Create `add-task-planner-review-service-v1` proposal, design, tasks, and delta spec.
- [x] 2.2 Validate with `openspec validate add-task-planner-review-service-v1 --strict`.
- [x] 2.3 Confirm scope stays on task/planner/review/resume serviceization and does not absorb LLM/Memory/Context provider migration.

## 3. Task Service Contract

- [x] 3.1 Add typed task service commands for goal creation, board query, claim, review, snapshot, and resume signal.
- [x] 3.2 Add task lifecycle events for goal ready, task claimed, review needed, review completed, goal completed, and coordinator resume.
- [x] 3.3 Add deterministic task service snapshot types.
- [x] 3.4 Export the new task service contract from `macaca-task/src/lib.rs`.

## 4. Task Service Runtime Skeleton

- [x] 4.1 Add a task service runtime skeleton in `macaca-task`.
- [x] 4.2 Add injectable strategy seams for planner/reviewer/worker/resume execution.
- [x] 4.3 Add runtime logging and structured event emission at key lifecycle nodes.

## 5. Web Adapter Extraction

- [ ] 5.1 Refactor `macaca-web::loop_manager` toward a command adapter and event bridge.
- [ ] 5.2 Keep current task board, review, and resume behavior compatible during migration.
- [ ] 5.3 Preserve current SSE/EventLog trace delivery while routing through the new Task Service seam.

## 6. SDK Integration

- [x] 6.1 Extend `macaca-sdk` task client with the new task service commands and snapshot methods.
- [x] 6.2 Preserve existing session-scoped task board behavior.
- [ ] 6.3 Keep compatibility wrappers searchable and deprecated where replaced.

## 7. Documentation and Governance

- [ ] 7.1 Update Route C architecture governance with Task Service ownership rules.
- [ ] 7.2 Document that Web is an adapter, not a task orchestration owner.
- [x] 7.3 Confirm no new allowlist rows are needed for the first slice.

## 8. Verification

- [x] 8.1 Run `cargo fmt --all --check`.
- [x] 8.2 Run `cargo test -p macaca-task`.
- [x] 8.3 Run `cargo test -p macaca-web loop_manager`.
- [x] 8.4 Run `cargo test -p macaca-sdk task_client`.
- [x] 8.5 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 8.6 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 8.7 Run `cargo check --workspace`.
- [x] 8.8 Run `npx gitnexus detect-changes -r agent`.
