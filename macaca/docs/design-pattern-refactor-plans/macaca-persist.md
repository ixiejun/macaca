# macaca-persist 设计模式渐进式重构计划

## 当前职责

`macaca-persist` 管理持久化存储、checkpoint、event log 和 redb backend。它直接影响刷新恢复、历史 trace 加载、resume coordinator、task board 状态恢复。

重点对象：

- `PersistStore` trait。
- `RedbStore`。
- `CheckpointManager`。
- `EventLog`。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| session/event 保存 | 写入、查询、replay 语义容易分散 | Memento | 明确 session/event/checkpoint snapshot |
| storage backend | redb 与未来 sqlite/remote store 切换成本高 | Proxy + Strategy | backend 策略化，错误统一转换 |
| EventLog replay | 前端刷新需要按顺序恢复历史事件 | Iterator | 提供稳定 event iterator |
| checkpoint | checkpoint 创建和恢复参数易散 | Builder | `CheckpointBuilder` |
| event append | trace event payload 构造重复 | Command | `PersistCommand::AppendEvent` 便于审计和 replay |

## 小步重构计划

1. 第一切片：给 EventLog 增加 `EventReplayIterator`，现有查询方法保留。
2. 第二切片：抽出 `PersistBackend`，RedbStore 作为默认实现。
3. 第三切片：新增 `SessionSnapshot` memento，统一 session metadata、event cursor、todo state 引用。
4. 第四切片：把 event append 参数封装为 `AppendEventCommand`，避免 event_type/source/payload 乱传。
5. 第五切片：给 event replay 增加幂等测试，覆盖“实时推送 + 刷新恢复不重复”。

## 示例代码片段

```rust
pub struct AppendEventCommand {
    pub session_id: String,
    pub event_type: String,
    pub source: String,
    pub payload: serde_json::Value,
}

pub trait EventLogStore: Send + Sync {
    async fn append(&self, cmd: AppendEventCommand) -> Result<EventId, PersistError>;
    async fn replay(&self, session_id: &str, from: EventCursor) -> EventReplayIterator;
}
```

```rust
pub struct SessionSnapshot {
    pub session_id: String,
    pub last_event_cursor: EventCursor,
    pub coordinator_resume_key: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

## 验证策略

- 对同一 session 的 event 写入顺序做 snapshot。
- 模拟 SSE 断开、刷新、重新订阅，确认历史 event 不重复、增量 event 不丢失。
- 所有 persist backend 共享 contract tests。

