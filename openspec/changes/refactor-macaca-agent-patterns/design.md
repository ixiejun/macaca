# Design: macaca-agent 渐进式设计模式重构

## Context

`macaca-agent` 位于 Agent OS 后端基础层，上层 runtime、kernel、task loop、web runner、SDK 都会间接依赖这里的 Agent 抽象。它不适合做激进改造。本设计采用“先加抽象、再委托、再替换”的路径，让每一步都保持旧行为。

参考设计模式：

- Null Object：缺省服务使用 no-op 实现，避免调用侧散落空判断。
- Facade：`AgentServices` 对外暴露稳定服务访问门面。
- Builder：`BasicAgentBuilder` 收敛构造参数和默认值。
- State / Strategy：`AgentLifecyclePolicy` 显式表达状态转移规则。
- Composite：Agent capability 支持多来源组合，但对外输出保持兼容。

## Goals

- 降低 `AgentServices` 的可选服务泄漏。
- 让 `BasicAgent` 构造可扩展、可读、可测试。
- 让 Agent 状态转移规则可单测、可审计。
- 为未来 persona/manifest/skill/driver/MCP capability 合并建立基础结构。
- 保持行为 1:1 还原，不影响当前应用执行链路。

## Non-Goals

- 不在本 change 中重构 `macaca-framework::ReActAgent`。
- 不在本 change 中修改 task 分解、claim、review、resume 逻辑。
- 不在本 change 中修改 LLM、tool、driver、MCP 的运行时行为。
- 不把 `macaca-agent` 改成 application-specific 实现。

## Proposed Design

### 1. AgentServices Facade

第一步只增加只读访问方法，不改现有字段、不强制修改所有调用点。

```rust
impl AgentServices {
    pub fn event_sink(&self) -> &dyn AgentEventSink {
        self.event_sink
            .as_deref()
            .unwrap_or(NoopEventSink::global())
    }

    pub fn memory(&self) -> &dyn AgentMemory {
        self.memory
            .as_deref()
            .unwrap_or(NoopAgentMemory::global())
    }
}
```

该切片的关键是行为保持：如果旧逻辑在没有服务时不产生任何动作，新 no-op 服务也必须不产生任何动作。

### 2. Null Object 缺省服务

缺省服务必须满足三个条件：

- 不写持久化。
- 不发 trace/event。
- 不改变 Agent 输出。

```rust
pub struct NoopEventSink;

#[async_trait]
impl AgentEventSink for NoopEventSink {
    async fn emit(&self, _event: AgentEvent) {}
}

pub struct NoopAgentMemory;

#[async_trait]
impl AgentMemory for NoopAgentMemory {
    async fn remember(&self, _item: MemoryItem) -> Result<(), AgentError> {
        Ok(())
    }
}
```

### 3. BasicAgentBuilder

Builder 是 additive API。旧构造函数保留，并委托给 builder。

```rust
pub struct BasicAgentBuilder {
    id: AgentId,
    name: Option<String>,
    services: AgentServices,
    capabilities: AgentCapabilitySet,
}

impl BasicAgentBuilder {
    pub fn new(id: AgentId) -> Self {
        Self {
            id,
            name: None,
            services: AgentServices::default(),
            capabilities: AgentCapabilitySet::default(),
        }
    }

    pub fn services(mut self, services: AgentServices) -> Self {
        self.services = services;
        self
    }

    pub fn build(self) -> BasicAgent {
        BasicAgent::from_parts(
            self.id,
            self.name,
            self.services,
            self.capabilities,
        )
    }
}
```

旧 API 的兼容方式：

```rust
impl BasicAgent {
    pub fn new(id: AgentId, services: AgentServices) -> Self {
        BasicAgentBuilder::new(id)
            .services(services)
            .build()
    }
}
```

### 4. AgentLifecyclePolicy

状态转移应先由测试锁定，再由 policy 接管。

```rust
pub trait AgentLifecyclePolicy: Send + Sync {
    fn can_transition(
        &self,
        from: AgentState,
        to: AgentState,
        reason: AgentTransitionReason,
    ) -> bool;
}

pub struct DefaultAgentLifecyclePolicy;

impl AgentLifecyclePolicy for DefaultAgentLifecyclePolicy {
    fn can_transition(
        &self,
        from: AgentState,
        to: AgentState,
        reason: AgentTransitionReason,
    ) -> bool {
        matches!(
            (from, to, reason),
            (AgentState::Idle, AgentState::Running, AgentTransitionReason::Start)
                | (AgentState::Running, AgentState::Idle, AgentTransitionReason::Complete)
                | (AgentState::Running, AgentState::Failed, AgentTransitionReason::Fail)
        )
    }
}
```

`AgentStateMachine` 继续保留为对外对象，只把内部判断委托给 policy。

### 5. Capability Composite

Capability 来源未来会增多。第一阶段不改变外部输出，只增加内部组合结构。

```rust
pub enum AgentCapabilityNode {
    Leaf(AgentCapability),
    Group {
        source: CapabilitySource,
        children: Vec<AgentCapabilityNode>,
    },
}

pub struct AgentCapabilitySet {
    root: Vec<AgentCapabilityNode>,
}

impl AgentCapabilitySet {
    pub fn flatten_for_legacy_api(&self) -> Vec<AgentCapability> {
        self.root
            .iter()
            .flat_map(AgentCapabilityNode::flatten)
            .collect()
    }
}
```

这样可以保留旧 API 的 `Vec<AgentCapability>` 行为，同时为后续解释“这个能力来自 skill 还是 driver”保留结构化来源。

## Compatibility Rules

- 旧 public API 不删除，最多标记为 deprecated，且必须委托新实现。
- `Default` 行为必须与旧构造一致。
- 缺省 no-op 服务不能向 EventLog、SSE、run_trace 写任何内容。
- 所有状态转移测试必须先于状态机重构落地。
- capability flatten 后的 legacy 输出必须和旧输出一致。

## Migration Order

1. 添加行为锁定测试。
2. 添加 Null Object 服务和 facade 方法。
3. 添加 BasicAgentBuilder，旧构造委托。
4. 添加 lifecycle policy，状态机内部委托。
5. 添加 capability composite，旧输出由 flatten 生成。
6. 替换 crate 内部调用点。
7. 仅在后续独立 change 中逐步迁移其他 crate 调用点。

## Verification

- `cargo test -p macaca-agent`
- `cargo check -p macaca-agent`
- 如调用侧有编译影响，再运行 workspace `cargo check`
- GitNexus:
  - 实施前对 `AgentServices`、`BasicAgent`、`AgentStateMachine` 运行 upstream impact。
  - 提交前运行 `gitnexus_detect_changes(scope: "all")`。

