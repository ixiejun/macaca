## Context

当前 `macaca-task` 的上层真实消费方主要是：

- `macaca-tools`
- `macaca-web`
- `macaca-integration-tests`

这些调用面已经从旧接口迁移到 canonical API。剩余的 deprecated 入口主要只存在于 `macaca-task` 自身，用作兼容 wrapper。问题已经从“如何迁移 API”转变成“如何防止未来回退”。

## Goals / Non-Goals

### Goals

- 精确确认上层 crate 不再调用 deprecated task API
- 用最小成本增加回归守卫
- 不影响现有运行时行为

### Non-Goals

- 不引入新的运行时抽象
- 不对整个 workspace 打开粗粒度 `deny(deprecated)`
- 不更改 task/session/review/resume 语义

## Decision

选择使用“源码级 audit test”而不是 crate-level `#![deny(deprecated)]`。

原因：

1. 目标很窄：只需要约束 `macaca-task` 的旧入口，不需要把整个 crate 的所有 deprecated 都一并治理
2. 影响面小：不会误伤同模块里来自其他 crate 的历史 deprecated
3. 容易审查：禁止模式列表清晰，和 canonical replacement 一一对应

## Guard Scope

守卫检查以下上层文件：

- `crates/macaca-tools/src/todo.rs`
- `crates/macaca-web/src/framework_toolkit.rs`
- `crates/macaca-web/src/loop_manager.rs`
- `crates/macaca-web/src/routes.rs`
- `crates/macaca-integration-tests/src/pipeline_dry_run.rs`

禁止重新引入以下旧调用模式：

- `TaskBoard::new(`
- `TaskSpace::new(`
- `.claim_next(`
- `.start_task(`
- `.submit_for_review(`
- `.review_task(`
- `.skip_task(`
- `.create_and_assign(`
- `PlanLoop::new(`
- `WorkerLoop::new(`
- `.run(shutdown`
- `.run(shutdown_clone`

## Trade-offs

### Pros

- 精确、低风险
- 不改变生产代码行为
- 失败信息可直接指向 canonical API

### Cons

- 这是源码约束，不是类型系统约束
- 如果未来文件拆分，需要同步更新审计文件列表

## Migration Notes

如果未来新增新的上层 `macaca-task` 消费文件，应该把文件路径补进这个 audit test，而不是重新允许旧 API 扩散。
