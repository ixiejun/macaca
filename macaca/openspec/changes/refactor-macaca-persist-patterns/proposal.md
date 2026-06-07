# Change: 渐进式重构 macaca-persist 核心持久化原语

## Why

根据 [README.md](/Users/quantum/Code/dev/agent/macaca/docs/design-pattern-refactor-plans/README.md) 的总体要求，以及 [refactor-order.md](/Users/quantum/Code/dev/agent/macaca/docs/design-pattern-refactor-plans/refactor-order.md) 中“阶段 1：最底层稳定 contract”的顺序，`macaca-persist` 应作为 `macaca-proto` 之后的优先重构对象。

`macaca-persist` 当前承载：

- session 持久化
- event log 追加与读取
- checkpoint 保存与恢复
- redb backend 封装

这些能力直接决定：

- 浏览器刷新后的历史恢复
- trace / session / resume 的完整性
- task board / review / coordinator resume 的状态持久化

如果这一层继续维持“backend 细节 + 参数散传 + replay 语义分散”的状态，后续 `macaca-task`、`macaca-framework`、`macaca-web` 的渐进重构会反复被持久化细节牵制。

## What Changes

- 为 `EventLog` 引入稳定的 replay iterator 原语，保留现有查询接口作为兼容层
- 为 event append 引入显式 command object，避免 `session_id/event_type/source/payload` 长参数散传
- 为 checkpoint 引入 builder 风格的构造入口，统一创建参数
- 抽出 `PersistBackend` / backend strategy 边界，让 `RedbStore` 成为默认实现而不是唯一耦合点
- 为 session / event / checkpoint 明确 memento 风格的 snapshot 语义

## Scope

本 change 仅覆盖 `macaca/crates/macaca-persist` 及其对应 OpenSpec 规格，不直接修改上层 crate 的消费逻辑。

本轮允许：

- 在 `macaca-persist` 内部增加 additive-first 原语
- 保留旧接口，由新原语在内部兼容实现

本轮不允许：

- 修改 session/event/checkpoint 的外部 wire schema
- 改变现有刷新恢复、trace 恢复、resume 的业务语义
- 直接在 `macaca-web` / `macaca-task` 中同步做大规模消费侧迁移

## Non-Goals

- 不在本 change 中替换掉 `RedbStore`
- 不引入新的远程持久化 backend
- 不在本轮里重写 event log 存储格式
- 不借 persist 重构修改 SSE、EventLog API、session API 返回结构
- 不在本轮里处理上层“事件是否重复显示”等 UI 问题

## Impact

- Affected specs:
  - `macaca-persist-core`
- Affected code:
  - `macaca/crates/macaca-persist/**`
- Expected risk: Medium
- Risk reason:
  - 这是底层 contract 重构，影响面向上广
  - 但本轮坚持 additive-first，只新增原语，不破坏既有外部行为

## Rollout Strategy

按 `macaca-persist.md` 的切片顺序执行：

1. 先加 `EventReplayIterator`
2. 再加 append command object
3. 再加 checkpoint builder
4. 再抽 `PersistBackend`
5. 最后补 session snapshot / memento 语义与契约测试

每一切片都必须满足：

- `cargo check` 通过
- 对应 crate 测试通过
- 上层行为保持 1:1
