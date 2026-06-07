# Change: Migrate task consumers to canonical task APIs

## Why

`macaca-task` 已完成第一轮基于设计模式的重构，新的 canonical API 已经存在：

- `TaskBoard::for_agent`
- `TaskBoard::claim_next_task`
- `TaskBoard::mark_task_in_progress`
- `TaskBoard::submit_task_for_review`
- `TaskBoard::fail_task`
- `TaskSpace::for_session`
- `TaskSpace::create_task_assignment`
- `TaskSpace::apply_review_result`
- `TaskSpace::cancel_task`
- `PlanLoop::with_components` / `run_with_default_template`
- `WorkerLoop::with_components` / `run_with_default_template`

旧 public 入口仍被保留并标记为 `deprecated`，这是为了后续迁移可检索、可回滚，而不是为了让上层继续依赖它们。当前真实上层消费面已经基本迁移完成，本轮需要把这一事实固定为仓库约束，防止回退。

## What Changes

- 审计 `macaca-tools`、`macaca-web`、`macaca-integration-tests` 的 `macaca-task` 调用面，确认所有 deprecated task API 调用都已迁移
- 增加一个精确的回归守卫，阻止这些上层 crate 重新引入 deprecated task API 调用
- 不删除 `macaca-task` 中的 deprecated wrapper；它们继续仅作为过渡兼容层存在

## Impact

- Affected specs:
  - `macaca-task-consumer-migration` (new)
- Affected code:
  - `macaca/crates/macaca-integration-tests/tests/task_api_migration_audit.rs`
  - `openspec/changes/migrate-task-consumers-to-canonical-apis/*`

## Non-Goals

- 不再继续重构 `macaca-task` 内部实现
- 不把 `macaca-web` 的 orchestration 继续抽成新 facade
- 不删除 deprecated wrapper
