## 1. Preparation

- [x] 1.1 Read `docs/superpowers/plans/2026-05-08-s3-sdk-system-facade-convergence-plan.md`.
- [x] 1.2 Read Route C governance docs and confirm S3 does not add new dependency exceptions.
- [x] 1.3 Inspect existing SDK/Web/CLI facade and shell adapter code.
- [x] 1.4 Run GitNexus impact before modifying existing symbols.

## 2. OpenSpec

- [x] 2.1 Create `update-sdk-system-facade-convergence` proposal, design, tasks, and delta spec.
- [x] 2.2 Validate with `openspec validate update-sdk-system-facade-convergence --strict`.
- [x] 2.3 Confirm scope does not absorb S4-S12 provider migrations.

## 3. SDK Client Modules

- [x] 3.1 Add `service_client.rs` with typed service inspection/call commands and client trait.
- [x] 3.2 Add `task_client.rs` with task-board commands/results and current `TodoStore` adapter.
- [x] 3.3 Add `trace_client.rs` with trace tail/replay commands and local adapter boundaries.
- [x] 3.4 Add `package_client.rs` with package inspection commands and local adapter boundaries.
- [x] 3.5 Add `status_client.rs` with status snapshot command/result and current static/kernel adapter.
- [x] 3.6 Export focused clients and command types from `macaca-sdk/src/lib.rs`.

## 4. SystemFacade Composition

- [x] 4.1 Refactor `system_facade.rs` to compose focused clients.
- [x] 4.2 Preserve `query_task_board` behavior and response shape.
- [x] 4.3 Preserve `status_snapshot` behavior and response shape.
- [x] 4.4 Add facade methods for service inspection, trace tail/replay, package inspection, and approval-ready command handling where feasible.
- [x] 4.5 Return structured unavailable/unsupported errors for commands whose real services are not migrated yet.

## 5. Consumers

- [x] 5.1 Keep Web shell task-board adapter on SDK/SystemFacade and update it to the new client composition.
- [x] 5.2 Migrate safe CLI status/list/inspect surfaces to SDK/SystemFacade where behavior-preserving.
- [x] 5.3 Do not migrate PlanLoop/WorkerLoop/review, LLM/Memory/Context, Driver/Skill/MCP, Application lifecycle, or Gateway provider behavior in S3.
- [x] 5.4 Keep deprecated compatibility helpers searchable.

## 6. Documentation and Governance

- [x] 6.1 Update Route C architecture governance with SDK/SystemFacade S3 ownership rules.
- [x] 6.2 Document that SDK clients are command-driven adapters, not provider factories.
- [x] 6.3 Confirm no new allowlist rows are needed; update allowlist only if a dependency is genuinely eliminated or changed.

## 7. Verification

- [x] 7.1 Run `cargo fmt --all --check`.
- [x] 7.2 Run `cargo test -p macaca-sdk`.
- [x] 7.3 Run `cargo test -p macaca-web`.
- [x] 7.4 Run `cargo check -p macaca-cli`.
- [x] 7.5 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 7.6 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 7.7 Run `cargo check --workspace`.
- [x] 7.8 Run `npx gitnexus detect-changes -r agent`.
