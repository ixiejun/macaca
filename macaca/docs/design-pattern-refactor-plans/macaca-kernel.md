# macaca-kernel 设计模式渐进式重构计划

## 当前职责

`macaca-kernel` 是 Agent OS 内核层，管理 kernel、orchestrator、scheduler、executor、registry 和 agent 状态。它承担“系统如何调度和执行 agent”的核心语义。

重点对象：

- `Kernel`：系统门面。
- `AgentOrchestrator`：agent 注册和运行协调。
- `SimpleScheduler` / scheduler trait。
- `ApplicationExecutor` / executor event broadcast。
- registry/status 模块。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| Kernel 对外 API | 下游需要知道 orchestrator、scheduler、executor 多个细节 | Facade | `Kernel` 保持唯一内核门面 |
| Scheduler | 调度策略会从 simple 扩展到 priority、dependency、resource aware | Strategy | `Scheduler` 策略化 |
| Orchestrator | agent、task、executor、event 之间协调复杂 | Mediator | orchestrator 只做协调，不承载业务策略 |
| ExecutorEvent | lifecycle event 构造重复，字段一致性靠人工 | Factory Method / Builder | `ExecutorEventFactory` 创建 start/progress/complete/fail |
| Agent status | Idle/Working/Paused/Error 状态变多 | State | `AgentRuntimeState` 显式状态机 |

## 小步重构计划

1. 第一切片：保留现有 `ExecutorEvent` enum，新增 lifecycle helper，减少散落构造。
2. 第二切片：把 scheduler selection 从配置 if/else 抽成 `SchedulerFactory`。
3. 第三切片：为 agent status 写 transition test，再抽状态机 policy。
4. 第四切片：ApplicationExecutor 只负责执行和事件发布，不承担 event payload 补字段。
5. 第五切片：Kernel facade 对 web/runtime 暴露更少内部 Arc。

## 示例代码片段

### ExecutorEvent lifecycle helper

```rust
pub struct ExecutorEventFactory {
    task_id: TaskId,
    agent: String,
}

impl ExecutorEventFactory {
    pub fn started(&self) -> ExecutorEvent {
        ExecutorEvent::TaskStarted {
            task_id: self.task_id,
            agent: self.agent.clone(),
        }
    }

    pub fn completed(&self, output: String) -> ExecutorEvent {
        ExecutorEvent::TaskCompleted {
            task_id: self.task_id,
            agent: self.agent.clone(),
            result: TaskResult::success(self.task_id, output),
        }
    }
}
```

### Scheduler Strategy

```rust
pub trait SchedulerStrategy: Send + Sync {
    async fn next(&self, queue: &TaskQueue, agents: &AgentRegistry) -> Option<ScheduledTask>;
}

pub struct DependencyAwareScheduler;
pub struct PriorityScheduler;
```

## 验证策略

- `ExecutorEventFactory` 引入前后对 SSE/EventLog payload 做 snapshot。
- scheduler 策略迁移时用相同 pending task 集合比较 claim 顺序。
- 修改 Kernel facade 前必须跑 GitNexus impact，因为 kernel 是上游依赖最多的 crate 之一。

