## ADDED Requirements

### Requirement: Upper crates SHALL consume EventLog via additive-first persist primitives

当上层 crate 需要向会话事件日志写入 durable event，或按顺序恢复历史 event 时，系统 SHALL 优先消费 `AppendEventCommand` 与 `EventReplayIterator`，而不是继续把旧 convenience API 作为默认入口。

#### Scenario: Web layer appends session events through command object

- **WHEN** `macaca-web` 将 delegated、mcp、resume 等 durable session event 写入 `EventLog`
- **THEN** 写入路径使用 `AppendEventCommand` 作为内部默认入口
- **AND** 外部事件 payload、event_type、source 语义保持不变

#### Scenario: Web layer restores session history through replay iterator

- **WHEN** `macaca-web` 为刷新恢复或历史 trace 重建读取某个 session 的事件历史
- **THEN** 读取路径通过 `EventReplayIterator` 顺序消费事件
- **AND** 恢复出的事件顺序、数量和 trace 聚合结果与迁移前保持一致

### Requirement: Upper crates SHALL depend on persist strategy abstractions for shared storage

当上层 crate 的共享存储组件只依赖通用持久化 contract，而不依赖 redb 专有行为时，系统 SHALL 让这些组件依赖 `PersistBackend` 或 `PersistStore` 抽象，而不是直接将共享状态类型绑定到 `RedbStore`。

#### Scenario: Shared task or audit components stop binding to RedbStore

- **WHEN** `macaca-task` 或 `macaca-kernel` 的共享持久化组件只使用 `get/set/delete/list_keys` 等通用 contract
- **THEN** 这些组件的存储字段和构造入口依赖 `PersistBackend` 或 `PersistStore`
- **AND** `RedbStore` 继续作为默认实现接入

### Requirement: Additive-first builder or memento primitives SHALL not force artificial upper-layer adoption

对于 `CheckpointBuilder`、`SessionSnapshot` 这类 additive-first 原语，若当前不存在真实上层消费需求，系统 SHALL 保持其可用但不强行制造调用点。

#### Scenario: No fake adoption for unused primitives

- **WHEN** 上层代码当前不存在真实 checkpoint builder 或 session snapshot 消费需求
- **THEN** 迁移过程不应为了“覆盖新 API”而新增无业务价值的包装或伪调用
- **AND** proposal / tasks / implementation 明确记录该边界
