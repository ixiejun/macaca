# macaca-task 设计模式渐进式重构计划

## 当前职责

`macaca-task` 是 Agent OS 正式任务账本和任务调度核心，包含 TodoStore、TaskBoard、TaskSpace、PlanLoop、WorkerLoop、decomposer、scheduler、review 相关逻辑。它和 `PlanNotebook` 的边界必须明确：TodoBoard 是系统正式任务账本，PlanNotebook 是 agent 脑内计划本。

重点对象：

- `TodoStore` / `TaskBoard` / `TaskSpace`。
- `PlanLoop`。
- `WorkerLoop`。
- `LlmDecomposer` / review/evaluation。
- task dependency / claim / retry。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| task lifecycle | pending/in_progress/review/done/canceled 转移复杂 | State | `TodoLifecyclePolicy` 明确合法状态迁移 |
| PlanLoop | goal decomposition、dependency、review、resume 混合 | Template Method | 固定 plan loop 阶段，可替换 decomposer/reviewer |
| WorkerLoop | claim、execute、result、retry、event bridge 混合 | Template Method + Command | worker 执行流程标准化 |
| task dependency | claim 前依赖检查不可硬编码 agent/application | Strategy | dependency resolver strategy |
| task board 协调 | 多 agent、planner、coordinator 之间事件协调 | Mediator | TaskSpace 作为 task mediator |
| PlanEvent/WorkerEvent | event 本质是命令/事实 | Command + Observer | 标准事件构造与发布 |

## 小步重构计划

1. 第一切片：为 Todo 状态转移增加 table-driven tests，覆盖 claim、review、retry、cancel、done。
2. 第二切片：抽出 `TodoLifecyclePolicy`，旧方法调用 policy，不改外部 API。
3. 第三切片：给 `PlanLoop` 增加 template step 方法：load_goals、decompose、publish_tasks、review_tasks、resume_coordinator。
4. 第四切片：给 `WorkerLoop` 增加 `WorkerExecutionTemplate`，统一 TaskStarted/AgentEvent/TaskCompleted/TaskFailed。
5. 第五切片：将 dependency gating 抽为 `TaskDependencyResolver`，planner 只写依赖事实，claim 阶段统一判断。

## 示例代码片段

### State policy

```rust
pub trait TodoLifecyclePolicy: Send + Sync {
    fn can_transition(&self, from: TodoStatus, to: TodoStatus, reason: TodoTransitionReason) -> bool;
}

pub struct DefaultTodoLifecyclePolicy;

impl TodoLifecyclePolicy for DefaultTodoLifecyclePolicy {
    fn can_transition(&self, from: TodoStatus, to: TodoStatus, reason: TodoTransitionReason) -> bool {
        matches!(
            (from, to, reason),
            (TodoStatus::Pending, TodoStatus::InProgress, TodoTransitionReason::Claim)
                | (TodoStatus::InProgress, TodoStatus::InReview, TodoTransitionReason::SubmitForReview)
                | (TodoStatus::InReview, TodoStatus::Done, TodoTransitionReason::ReviewAccepted)
        )
    }
}
```

### PlanLoop Template Method

```rust
impl PlanLoop {
    pub async fn tick(&self) -> Result<(), PlanLoopError> {
        let goals = self.load_pending_goals().await?;
        for goal in goals {
            let plan = self.decompose_goal(&goal).await?;
            self.publish_tasks(goal.id, plan).await?;
        }

        let reviews = self.load_review_items().await?;
        self.review_completed_tasks(reviews).await?;
        self.resume_ready_coordinators().await?;
        Ok(())
    }
}
```

## 验证策略

- session scope 不能再被误改为 app scope，必须加 session-isolation regression test。
- task dependency fixture：architect/design task 未完成前 frontend/backend 不可 claim。
- planner 不应覆盖历史 trace，review event 必须 append。

