## Context

`macaca-persist` 现已提供以下 additive-first 原语：

- `AppendEventCommand`
- `EventReplayIterator`
- `PersistBackend`
- `CheckpointBuilder`
- `SessionSnapshot`

但上层真实消费面并未系统迁移，当前表现为：

- `macaca-web` 仍以 convenience API 为主，导致 durable append / ordered replay 的 contract 没有显式进入上层
- `macaca-web` / `macaca-task` / `macaca-kernel` 的部分组件仍将共享持久化状态直接声明为 `RedbStore`
- checkpoint / session snapshot 暂无真实上层消费点，不能为了迁移而硬造调用

## Goals

- 让上层真实消费点优先走 `AppendEventCommand`
- 让上层真实历史恢复路径优先走 `EventReplayIterator`
- 让共享持久化依赖优先声明为 `PersistBackend` / `PersistStore`
- 维持行为 1:1，不改变刷新恢复、增量推送、todo/scheduler/audit 语义

## Non-Goals

- 不删除 `RedbStore`
- 不删除 `EventLog::append/query`
- 不引入新 persist backend
- 不为 `CheckpointBuilder` / `SessionSnapshot` 伪造没有业务价值的上层调用

## Brainstorm

### 方案 A：全量机械替换所有旧入口

做法：

- 所有 `append` 都替换成 `append_command`
- 所有 `query` 都替换成 `replay`
- 所有 `Arc<RedbStore>` 一律切成 trait object
- 强行在上层插入 `CheckpointBuilder` / `SessionSnapshot`

优点：

- 迁移覆盖面最大

风险：

- 会制造大量“只是为了新 API 而改”的代码
- 容易把无收益的调用点一起搅动，放大 blast radius
- 对 `SessionSnapshot` / `CheckpointBuilder` 来说，当前没有真实消费点，强行迁移是伪需求

### 方案 B：只迁移真实热点，按模式语义收口

做法：

- `macaca-web` 的 durable append 改走 command object
- `macaca-web` 的历史恢复改走 replay iterator
- `macaca-web` / `macaca-task` / `macaca-kernel` 中共享持久化依赖改声明为 `PersistBackend` / `PersistStore`
- 对暂无真实消费点的 builder / memento，只保留 additive-first，不强造调用

优点：

- 与设计模式目标一致
- 代码变更小，收益集中
- 行为风险最低

风险：

- 不是“所有新原语都在本轮上层出现一次”
- 需要在 proposal 中明确“真实消费优先”的边界，避免误解为迁移不彻底

### 方案 C：先在 `macaca-persist` 内再包一层 facade，再迁移上层

做法：

- 先新增 `SessionEventStore` / `SessionSnapshotStore` 等 facade
- 然后让上层全部改依赖 facade

优点：

- 抽象更完整

风险：

- 当前会过度设计
- 会引入一轮额外 API 设计和更大 blast radius
- 与“优先小步、可逆、1:1 行为还原”的要求冲突

## Decision

选择方案 B。

原因：

- 它正好对齐 `Iterator`、`Command`、`Strategy` 这三类已经落地的 persist 模式
- 它避免为 `Builder` / `Memento` 伪造上层调用
- 它能让上层开始真正消费新 contract，同时把风险控制在热点路径

## Migration Plan

1. 迁移 `macaca-web` 的 event append 调用到 `AppendEventCommand`
2. 迁移 `macaca-web` 的 session 历史恢复到 `EventReplayIterator`
3. 将 `macaca-web` 的共享 session store 从 `RedbStore` 收敛到 `PersistBackend`
4. 将 `macaca-task::TodoStore` / `TaskScheduler` 收敛到 `PersistBackend`
5. 将 `macaca-kernel::AuditLogger` 收敛到 `PersistBackend`
6. 补回归验证，确保刷新恢复、实时增量、todo/scheduler/audit 语义不变

## Risks / Trade-offs

- `macaca-web::session` 是高价值恢复路径，若 replay 迁移处理不当会影响刷新恢复
  - 缓解：仅替换读取原语，不改 trace 重建逻辑

- `PersistBackend` 向上收敛会触碰共享状态与构造函数签名
  - 缓解：沿用 `EventLog::new<T>(Arc<T>)` 的 additive-first 泛型构造方式

- GitNexus 对底层存储改动会天然给出较大过程影响范围
  - 缓解：每个符号编辑前单独跑 impact，避免一次跨太多高风险符号
