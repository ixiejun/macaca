## 1. Preparation

- [x] 1.1 Run GitNexus impact for `ExecutorEvent`, `TaskResult`, `TaskExecutor::execute_task`, `ApplicationExecutor`, `SimpleScheduler`, `AgentStatusTracker`, and `Kernel`.
- [x] 1.2 Run or record baseline kernel tests before implementation when practical.
- [x] 1.3 Validate this OpenSpec change with `openspec validate refactor-macaca-kernel-patterns --strict`.

## 2. Executor lifecycle helper

- [x] 2.1 Add `executor/event_factory.rs`.
- [x] 2.2 Add factory unit tests for started/completed/failed/result payloads.
- [x] 2.3 Export factory from `executor/mod.rs`.
- [x] 2.4 Migrate web-local helper usage to kernel helper.
- [x] 2.5 Migrate legacy `TaskExecutor::execute_task` direct lifecycle construction.

## 3. Scheduler factory

- [x] 3.1 Add `SchedulerKind` and `SchedulerFactory`.
- [x] 3.2 Keep default factory output equivalent to `SimpleScheduler`.
- [x] 3.3 Use factory from `Kernel`.
- [x] 3.4 Mark direct `SimpleScheduler` construction deprecated but keep callable.
- [x] 3.5 Add scheduler factory tests.

## 4. Agent status transition policy

- [x] 4.1 Add `AgentStatusTransitionPolicy`.
- [x] 4.2 Route status tracker state/activity/idle helpers through the policy.
- [x] 4.3 Add transition policy tests.

## 5. Executor payload boundary

- [x] 5.1 Replace selected `ApplicationExecutor` event/result construction with factory calls.
- [x] 5.2 Keep broadcast/event_tx behavior unchanged.
- [x] 5.3 Add or keep regression tests for emitted event fields.

## 6. Kernel builder/facade

- [x] 6.1 Add `KernelBuilder`.
- [x] 6.2 Make `KernelBuilder::build` match `Kernel::new` defaults.
- [x] 6.3 Keep `Kernel::new` compatible but mark it deprecated.
- [x] 6.4 Add builder tests.

## 7. Verification

- [x] 7.1 Run `cargo fmt`.
- [x] 7.2 Run `cargo test -p macaca-kernel -- --nocapture`.
- [x] 7.3 Run `cargo test -p macaca-integration-tests kernel -- --nocapture`.
- [x] 7.4 Run `cargo check -p macaca-kernel -p macaca-web -p macaca-app -p macaca-sdk -p macaca-cli`.
- [x] 7.5 Run `openspec validate refactor-macaca-kernel-patterns --strict`.
- [x] 7.6 Run `gitnexus_detect_changes(scope: "all")`.
