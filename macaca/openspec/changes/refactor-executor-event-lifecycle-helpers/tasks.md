## 1. Implementation

- [x] 1.1 在 `loop_manager.rs` 中新增私有 helper，用于构造 `ExecutorEvent::TaskStarted`
- [x] 1.2 在 `loop_manager.rs` 中新增私有 helper，用于构造成功 `TaskResult` 和 `ExecutorEvent::TaskCompleted`
- [x] 1.3 在 `loop_manager.rs` 中新增私有 helper，用于构造 `ExecutorEvent::TaskFailed`
- [x] 1.4 将 planner decomposition/review/follow-up 中重复的生命周期事件构造切换为 helper
- [x] 1.5 将 worker task claim/retry 中重复的生命周期事件构造切换为 helper

## 2. Tests

- [x] 2.1 添加或更新 unit test，覆盖 completed event helper 的 `TaskResult` 字段保持不变
- [x] 2.2 添加或更新 unit test，覆盖 failed event helper 的 `task_id`、`agent`、`error` 字段保持不变

## 3. Verification

- [x] 3.1 运行 `openspec validate refactor-executor-event-lifecycle-helpers --strict`
- [x] 3.2 运行 helper 相关单元测试
- [x] 3.3 运行 `cargo check -p macaca-web`
- [x] 3.4 运行 GitNexus `detect_changes(scope=staged)` 并确认影响范围符合本次局部重构
