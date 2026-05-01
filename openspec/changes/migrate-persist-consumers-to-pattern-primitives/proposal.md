# Change: migrate upper persist consumers to pattern primitives

## Why

`macaca-persist` 已完成第一轮设计模式重构，但上层 crate 仍然大量停留在旧消费方式：

- `macaca-web` 继续用 `EventLog::append/query`
- `macaca-web`、`macaca-task`、`macaca-kernel` 仍直接把共享状态和服务类型绑定到 `RedbStore`
- 新增的 `AppendEventCommand`、`EventReplayIterator`、`PersistBackend` 还没有成为上层默认入口

这会导致底层抽象已经存在，但上层仍绕过抽象层，无法真正形成“additive-first、上层逐步迁移”的闭环。

## What Changes

- 将 `macaca-web` 的 durable event 写入迁移到 `AppendEventCommand`
- 将 `macaca-web` 的历史事件顺序恢复迁移到 `EventReplayIterator`
- 将 `macaca-web` 共享 session store、`macaca-task` 的持久化组件、`macaca-kernel` 的 audit logger 迁移到 `PersistBackend` / `PersistStore` 抽象
- 保持旧的 `append/query`、`RedbStore` 兼容入口不删除
- 只在存在真实上层消费点时迁移 `CheckpointBuilder` / `SessionSnapshot`，不为“使用新 API”而制造伪需求

## Impact

- Affected specs:
  - `persist-consumer-migration`
- Affected code:
  - `macaca/crates/macaca-web/src/*`
  - `macaca/crates/macaca-task/src/*`
  - `macaca/crates/macaca-kernel/src/audit.rs`
  - `macaca/crates/macaca-persist/src/*` 仅作兼容边界调整（如需要）
