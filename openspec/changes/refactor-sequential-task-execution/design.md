## Context

Macaca 的 task 系统当前使用优先级调度（BinaryHeap），适合"哪个紧急做哪个"的场景，但不适合"按计划顺序执行"的场景。在实际应用（如 fullstack-autodev）中，任务天然有先后顺序：architect 先规划架构 → frontend/backend 再编码 → 最后集成测试。当前系统虽然有 `depends_on` 字段，但依赖关系必须在任务创建时手动指定 TaskId，且缺少跨 agent 的阶段编排能力。

**利益相关方**：所有使用 Macaca task 系统的应用开发者

**约束**：
- 必须保持 OS 底座的通用性，不能硬编码 agent 名称或应用逻辑
- 必须向后兼容已有的持久化数据（redb）
- 必须保持 crash recovery 能力

## Goals / Non-Goals

**Goals:**
- 同一 agent+session 内的任务按 sequence_number 严格顺序执行
- 支持跨 agent 的依赖关系声明和自动阻塞/解除
- LLM 分解 goal 时能自然表达执行顺序和依赖
- 保持 pull-based 架构（agent 主动 claim，不被 push）

**Non-Goals:**
- 不实现 DAG 可视化或图形化依赖编辑
- 不实现任务抢占或动态重排序
- 不改变 PlanLoop/WorkerLoop 的事件驱动架构
- 不实现跨 application 的依赖（仅 app 内）

## Decisions

### Decision 1: 序号取代优先级作为主排序

**选择**: `sequence_number: u32` 作为 agent+session 内的执行顺序，`priority` 降级为辅助字段（用于跨 agent 资源竞争时的参考，但不影响同 agent 内的执行顺序）。

**理由**: 优先级调度适合"抢占式"场景，但 Macaca 的任务来自 LLM 分解，天然有逻辑顺序。序号更直观，LLM 更容易生成正确的执行计划。

**替代方案**:
- 用 `depends_on` 链式依赖模拟顺序（task2 depends_on task1）→ 过于繁琐，N 个任务需要 N-1 个依赖声明
- 用 `created_at` 时间戳排序 → 不够显式，无法表达"先做第 5 个再做第 3 个"的调整

### Decision 2: 自动序号分配

**选择**: `TaskSpace.create_and_assign()` 在同一 agent+session 内自动分配递增序号。LLM 分解时可以指定 `sequence`（相对顺序），系统映射为绝对序号。

**理由**: 避免 LLM 需要知道全局序号状态，减少冲突。

**规则**:
- 同一 agent+session 内，序号从 1 开始递增
- 批量创建时按 LLM 输出的 `sequence` 字段排序
- 后续追加的任务序号接续最大值
- 可通过 API 手动调整序号（预留）

### Decision 3: 跨 Agent 依赖通过 `depends_on` + 标题解析

**选择**: 保持现有 `depends_on: Vec<TaskId>` 机制，但增强 LLM decomposer 的依赖声明能力——支持 `depends_on_agent_tasks: Vec<AgentTaskRef>` 格式，表达"依赖某个 agent 的所有任务"或"依赖某个 agent 的特定任务"。

**AgentTaskRef 格式**:
```rust
pub enum AgentTaskRef {
    AllTasks { agent: String },           // 等待该 agent 所有任务完成
    SpecificTask { agent: String, title: String },  // 等待特定任务
}
```

**理由**: 
- `AllTasks` 覆盖"architect 完成后 frontend 才开始"的常见模式
- `SpecificTask` 覆盖细粒度依赖
- 在 goal 分解时解析为具体 TaskId，无需运行时额外查询

**替代方案**:
- Phase/Stage 概念 → 增加新抽象层，复杂度高，且本质上是依赖关系的语法糖
- 时序约束（before/after）→ 与 depends_on 语义重叠

### Decision 4: claim_next() 严格顺序

**选择**: `TaskBoard.claim_next()` 返回 `sequence_number` 最小的 Pending 任务。如果最小序号的任务是 Blocked 状态，则不返回任何任务（等待依赖完成）。

**理由**: 严格顺序意味着"前面的任务没完成/被阻塞，后面的也不能跳过执行"。这保证了执行计划的确定性。

**边界情况**:
- Blocked 任务阻塞后续所有任务 → 符合预期（依赖未满足就不该继续）
- Failed 任务：标记 Failed 后，后续任务保持 Pending，由 PlanLoop 决策是否取消后续任务或重试
- 手动跳过：预留 `skip_task()` API，将任务标记为 Cancelled 并推进到下一个

### Decision 5: 数据迁移策略

**选择**: 增量迁移，不破坏已有数据。

**规则**:
- 新字段 `sequence_number` 默认值为 0
- 已有 TodoItem 反序列化时，若无 `sequence_number` 字段，按 `created_at` 排序自动分配序号
- 迁移在 `TodoStore` 初始化时自动完成（一次性）
- `priority` 字段保留但不再用于 claim_next() 排序

## Risks / Trade-offs

| Risk | Impact | Mitigation |
|------|--------|------------|
| Blocked 任务导致 agent 完全停滞 | 如果依赖永远不完成，agent 空转 | PlanLoop 检测超时 Blocked 任务，emit AnomalyDetected |
| LLM 生成错误的依赖关系（循环依赖） | 死锁 | 在 create_and_assign() 时做简单环检测（DFS） |
| 已有持久化数据不含 sequence_number | 升级后行为变化 | 按 created_at 自动分配序号，行为接近原 FIFO |
| 同 agent 内无法并行执行多个任务 | 降低吞吐 | 这是设计目标：顺序执行保证确定性；需要并行时用多 agent |

## Migration Plan

1. **Phase 1**: 添加 `sequence_number` 字段到 `TodoItem`，默认 0，`claim_next()` 改为序号排序（兼容旧数据）
2. **Phase 2**: 增强 `LlmDecomposer` 输出 sequence + 跨 agent 依赖
3. **Phase 3**: `TaskSpace` 自动分配序号 + 环检测 + 依赖解析增强
4. **Phase 4**: 迁移工具 + 旧数据自动升级 + 移除 priority 排序逻辑

**Rollback**: 每个 Phase 独立可回滚。Phase 1 回滚：恢复 priority 排序。Phase 2-4 回滚：保留 sequence_number 但忽略。

### Decision 6: 前端 Task 面板按序号展示

**选择**: `TaskBoardModal` 中每个 `TaskCard` 前方显示执行序号徽标（如 `#1`、`#2`），agent 分组内的任务按 `sequence_number` 升序排列。`PriorityBadge` 替换为 `SequenceBadge`。

**理由**: 序号是核心排序维度，用户需要直观看到任务的执行顺序。优先级不再是主排序字段，在 UI 中去掉它减少认知负担。

**展示规则**:
- 序号徽标显示在任务标题左侧，格式 `#N`，使用 monospace 字体
- Blocked 状态的任务显示依赖提示（如锁图标 + "等待 architect"）
- agent 分组内任务严格按 sequence_number 升序
- 进度条可保留（`N/M DONE`）

### Decision 7: 后端 API 排序保证

**选择**: `list_todos` 和 `list_agent_todos` API 在返回前按 `sequence_number` 升序排序，前端不需要自行排序。

**理由**: 服务端排序是单一数据源原则，避免前后端排序不一致。前端仅做展示，不做排序逻辑。

## Open Questions

- Q1: 是否需要支持"部分顺序"——即同一 agent 内允许某些任务并行（相同 sequence_number）？
  - 当前决策：不支持，严格顺序。如需并行，拆分为多个 agent。
- Q2: Failed 任务后续任务的处理策略——自动 cancel 还是等待人工/PlanLoop 决策？
  - 当前决策：保持 Pending，由 PlanLoop 通过 AnomalyDetected 事件决策。
- Q3: 是否在前端展示任务间的依赖关系线（连线/箭头）？
  - 当前决策：不做。仅在 Blocked 任务上显示文字提示"等待 XXX"，不做图形化依赖线。
