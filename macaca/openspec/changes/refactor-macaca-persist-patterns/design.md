# Design: macaca-persist 渐进式重构

## Context

`macaca-persist` 位于重构顺序中的阶段 1，属于“先稳底层 contract，再迁移主要消费方”的典型 crate。它不应继续把：

- backend 实现细节
- 追加事件的参数协议
- replay 的顺序语义
- checkpoint 创建参数

散落为零散函数和直接结构体调用。

本次设计目标是：在不改变对外语义的前提下，把这些能力收敛成稳定、可迁移、可测试的持久化原语。

## Goals

- 提供稳定的 event replay iterator 原语
- 提供显式的 append command object
- 提供 checkpoint builder
- 为 backend 切换预留 strategy / proxy 边界
- 为 session/event/checkpoint 明确 memento 风格快照语义

## Non-Goals

- 不引入新 backend
- 不移除现有公开 API
- 不同时迁移所有上层消费方
- 不改变事件排序、会话恢复、checkpoint 恢复的对外行为

## Decisions

### 1. Event replay 先以 Iterator 原语引入

当前最需要稳定的是“按顺序恢复历史事件”的 contract。先增加 iterator 风格的 replay 原语，而不是立刻替换全部读取接口。

理由：

- 刷新恢复、trace 恢复、resume 都依赖顺序回放
- iterator 是 additive-first，最容易做兼容包装

### 2. Event append 参数收敛为 Command Object

为 event append 增加 `AppendEventCommand`，把：

- `session_id`
- `event_type`
- `source`
- `payload`

作为一个完整命令传入。

理由：

- 避免上层继续长参数散传
- 便于未来审计、重放、测试比对

### 3. Checkpoint 创建走 Builder

checkpoint 在语义上天然适合 builder，因为它包含：

- scope / key
- payload / state
- metadata
- timestamp / cursor

本轮新增 builder，不移除旧构造入口。

### 4. Backend 先抽接口，再保留 Redb 默认实现

`PersistBackend` 的目标不是本轮就替换 Redb，而是把：

- 存储 contract
- backend 生命周期
- 错误转换

从 `RedbStore` 的具体实现中拆出来。

因此这轮应优先做：

- trait 抽象
- `RedbStore` 适配实现
- 契约测试

而不是一次性引入新 backend。

### 5. Session / Event / Checkpoint 以 Memento 语义收口

这里的 memento 不要求立刻改外部 API 名称，而是要求内部语义明确：

- session snapshot
- event cursor
- coordinator resume 关键引用
- checkpoint state

这样上层在刷新恢复和 resume 逻辑里，才能依赖稳定快照，而不是依赖存储细节。

## Risks / Trade-offs

- 风险：底层 trait 变多，短期内会增加兼容包装层
  - 缓解：只做 additive-first，新旧路径并存一个阶段

- 风险：iterator / command / builder 如果同时落太多，会影响验证范围
  - 缓解：严格按切片逐步实现，每次只替换一小层内部实现

- 风险：上层已有顺序恢复逻辑可能隐式依赖旧接口行为
  - 缓解：在本 crate 内补 replay 顺序和幂等性测试，不先动上层

## Migration Plan

1. 增加 `EventReplayIterator` 和 contract tests
2. 增加 `AppendEventCommand`
3. 增加 `CheckpointBuilder`
4. 抽出 `PersistBackend`
5. 增加 `SessionSnapshot` / memento 边界
6. 保持旧 API 通过新原语实现

## Verification

- `cargo check -p macaca-persist`
- `cargo test -p macaca-persist -- --nocapture`
- workspace `cargo check`
- 必要时用上层最小联调验证 replay 顺序不变
