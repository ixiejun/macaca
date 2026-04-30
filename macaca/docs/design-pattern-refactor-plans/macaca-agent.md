# macaca-agent 设计模式渐进式重构计划

## 当前职责

`macaca-agent` 定义 Agent 抽象、基础 Agent 实现、Agent 服务集合和 Agent 状态机。它是上层 runtime、kernel、web loop 调用 Agent 能力时的基础接口层。

重点对象：

- `Agent` trait：执行、状态、能力描述的统一接口。
- `AgentServices`：LLM、memory、tool、event 等可选服务聚合。
- `BasicAgent`：最基础的 Agent 结构体。
- `AgentStateMachine`：当前状态流转逻辑主要由枚举和 match 表达。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| `AgentServices` | 多个 `Option<Box<...>>` 导致调用侧到处判断服务是否存在 | Null Object + Facade | 对外暴露稳定服务门面，缺省服务返回空实现 |
| `BasicAgent` 构造 | 构造参数和服务装配会随能力增长持续膨胀 | Builder | 引入 `AgentBuilder` 风格构建入口，保留旧构造兼容 |
| `AgentStateMachine` | 状态转移规则散在 match 中，后续暂停、恢复、失败原因会继续变复杂 | State | 将不同状态的合法转移封装为策略对象或 transition table |
| Agent capability | 能力可能来自 persona、manifest、skill、driver，来源混合后会难以追踪 | Composite | 把能力合成为 capability graph，而不是字符串列表拼接 |

## 小步重构计划

1. 第一切片：为 `AgentServices` 增加只读门面方法，例如 `llm()`、`memory()`，内部仍读取原字段，不改结构。
2. 第二切片：补齐 Null Object 服务实现，例如 `NoopMemoryService`、`NoopEventSink`，让调用侧不再关心 `Option`。
3. 第三切片：新增 `BasicAgentBuilder`，旧 `BasicAgent::new` 保留并委托给 builder。
4. 第四切片：给 `AgentStateMachine` 增加 transition table 测试，先锁定现有语义。
5. 第五切片：把状态迁移逻辑从大 match 抽成 `AgentLifecyclePolicy` Strategy。

## 示例代码片段

### Null Object + Facade

```rust
pub trait AgentEventSink: Send + Sync {
    async fn emit(&self, event: AgentEvent);
}

pub struct NoopEventSink;

#[async_trait]
impl AgentEventSink for NoopEventSink {
    async fn emit(&self, _event: AgentEvent) {}
}

pub struct AgentServices {
    event_sink: Arc<dyn AgentEventSink>,
}

impl AgentServices {
    pub fn event_sink(&self) -> Arc<dyn AgentEventSink> {
        Arc::clone(&self.event_sink)
    }
}
```

### Builder

```rust
pub struct BasicAgentBuilder {
    id: AgentId,
    services: AgentServices,
    capabilities: Vec<AgentCapability>,
}

impl BasicAgentBuilder {
    pub fn with_services(mut self, services: AgentServices) -> Self {
        self.services = services;
        self
    }

    pub fn build(self) -> BasicAgent {
        BasicAgent::from_parts(self.id, self.services, self.capabilities)
    }
}
```

## 验证策略

- 添加状态转移黄金测试，确保旧状态流转和新 policy 完全一致。
- 对 Null Object 引入前后运行 agent execution 单测，确认缺省服务不会改变输出。
- 使用 GitNexus impact 锁定 `AgentServices` 与 `AgentStateMachine` 调用者后再进入代码阶段。

