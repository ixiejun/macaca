## Context

macaca-framework 提供了完整的 Agent 框架（ReActAgent/Toolkit/WorkingMemory/Formatter/Pipeline），但当前 Goal-Task 链路仍使用 macaca-runtime 的 AgenticLoop + macaca-web 的 ad-hoc 编排。本设计将执行层迁移到 framework，保留 OS 层的调度和持久化。

## Goals / Non-Goals

**Goals:**
- Worker/Planner 执行从 `delegate_task + 轮询` 迁移到 `ReActAgent.reply()`
- Coordinator 对话从 `run_agentic_stream` 迁移到 `ReActAgent.reply()` + SSE 桥接
- 工具注入从 `AgentToolSet` 迁移到 `Toolkit`（含分组和中间件）
- 记忆管理从 `ContextWindowManager` 迁移到 `InMemoryWorkingMemory`（含标签）
- 全链路有统一的错误处理和超时保护

**Non-Goals:**
- 不重写 PlanLoop/WorkerLoop 的调度逻辑（tick/notify/事件分发）
- 不修改 TodoStore/TaskBoard 的持久化
- 不修改 HTTP API 接口

## Decisions

### Decision 1: 执行层 vs 调度层分离

```
┌──────────────────────────────────────────┐
│         调度层（保留不变）                  │
│  PlanLoop → PlanEvent → 消费者            │
│  WorkerLoop → WorkerEvent → 消费者        │
│  TodoStore / TaskBoard / TaskSpace        │
├──────────────────────────────────────────┤
│         执行层（迁移到 framework）          │
│  消费者内部: delegate_task → ReActAgent    │
│  Coordinator: run_agentic_stream → ReAct  │
│  工具: AgentToolSet → Toolkit             │
│  记忆: ContextWindowManager → Working Mem │
└──────────────────────────────────────────┘
```

**原则**: 消费者仍然接收 PlanEvent/WorkerEvent，但执行方式从 `executor.delegate_task()` 改为本地构建 ReActAgent 并直接调用 `agent.reply()`。这消除了 delegate_task 的异步轮询模式。

### Decision 2: LlmProviderAdapter

```rust
// macaca-framework/src/adapter.rs
pub struct LlmProviderAdapter {
    provider: Arc<dyn macaca_llm::LlmProvider>,
    formatter: Box<dyn Formatter>,
}

impl ChatModel for LlmProviderAdapter {
    async fn chat(&self, messages: Vec<Value>, options: &ChatOptions) -> Result<ChatResponse, ModelError> {
        // 1. 将 framework 的 JSON messages 转回 LlmMessage
        // 2. 调用 provider.chat()
        // 3. 将 LlmResponse 转为 framework 的 ChatResponse
    }
}
```

**替代**: 重写 LLM 调用层 → 工作量太大，风险高
**选择**: Adapter 模式，复用现有 macaca-llm

### Decision 3: ToolSetAdapter

```rust
// macaca-framework/src/adapter.rs
pub struct ToolSetBridge;

impl ToolSetBridge {
    /// 将 macaca-tools 的 Tool 包装为 framework 的 ToolHandler
    pub fn from_tool_set(tool_set: &dyn macaca_tools::ToolSet) -> Toolkit {
        let mut toolkit = Toolkit::new();
        for tool_def in tool_set.to_definitions() {
            toolkit.register(Box::new(LegacyToolHandler {
                name: tool_def.name,
                description: tool_def.description,
                schema: tool_def.input_schema,
                tool_ref: tool_set.get_tool(&tool_def.name),
            }), None);
        }
        toolkit
    }
}
```

### Decision 4: SSE 桥接（Coordinator）

ReActAgent 通过 Hook 发射 SSE 事件：

```rust
struct SseEmitterHook {
    tx: mpsc::Sender<Result<Event, Infallible>>,
    event_log: Arc<EventLog>,
    session_id: String,
}

impl Hook for SseEmitterHook {
    async fn pre_reply(&self, msg: Msg) -> AgentResult<Msg> {
        // emit "thinking" event
        Ok(msg)
    }
    async fn post_reply(&self, msg: Msg) -> AgentResult<Msg> {
        // emit "content" / "done" event
        Ok(msg)
    }
}
```

Tool 调用事件通过 ToolMiddleware 发射：

```rust
struct SseToolMiddleware {
    tx: mpsc::Sender<Result<Event, Infallible>>,
}

impl ToolMiddleware for SseToolMiddleware {
    async fn before(&self, name: &str, args: &mut Value) -> Result<(), ToolError> {
        // emit "tool_call" SSE event
        Ok(())
    }
    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<(), ToolError> {
        // emit "tool_result" SSE event
        Ok(())
    }
}
```

### Decision 5: Worker 直接执行（消除轮询）

当前 Worker 流程:
```
WorkerEvent::TaskClaimed → delegate_task → loop { sleep(3s); poll result } → update board
```

迁移后:
```
WorkerEvent::TaskClaimed → build ReActAgent → agent.reply(task_prompt) → update board
```

ReActAgent.reply() 是同步阻塞（async await），返回最终结果。不需要轮询。

### Decision 6: Planner 直接执行

当前 Planner 流程:
```
GoalReady → delegate_task("planner", decompose_prompt) → let _ = (忽略错误)
ReviewNeeded → delegate_task("planner", review_prompt) → let _ = (忽略错误)
```

迁移后:
```
GoalReady → build planner ReActAgent → agent.reply(decompose_prompt) → 处理错误
ReviewNeeded → build planner ReActAgent → agent.reply(review_prompt) → 处理错误
```

### Decision 7: Agent 工厂（FrameworkRunner）

```rust
// macaca-web/src/framework_runner.rs
pub struct FrameworkRunner;

impl FrameworkRunner {
    /// 根据 persona 配置构建 ReActAgent
    pub async fn build_agent(
        state: &AppState,
        app_id: &ApplicationId,
        agent_name: &str,
        session_id: Option<&str>,
    ) -> Result<ReActAgent, AgentError> {
        // 1. 加载 persona (IDENTITY.md → system prompt)
        // 2. 创建 LlmProviderAdapter (state.llm + DashScopeFormatter)
        // 3. 构建 Toolkit (base tools + per-agent todo tools)
        // 4. 创建 InMemoryWorkingMemory
        // 5. 组装 ReActAgent
    }
}
```

## Risks / Trade-offs

| Risk | Impact | Mitigation |
|------|--------|------------|
| ReActAgent 与现有 SSE 流不兼容 | 前端事件中断 | Hook + Middleware 桥接 |
| Adapter 性能开销（多一层转换） | 延迟增加 | 转换是内存操作，<1ms |
| 新旧引擎行为不一致 | 用户困惑 | 先可选，验证后切换 |
| ReActAgent 不支持 pause/resume | create_goal 后 coordinator 无法暂停 | 扩展 ReActAgent 支持外部 pause signal |

## Migration Plan

渐进式，6 个 Phase，每个独立可验证：
1. Adapter 桥接层（独立 crate 内，不影响运行时）
2. Agent 工厂（新文件，不改现有代码）
3. Coordinator 可选引擎（`?engine=framework` 参数）
4. Worker 迁移（替换消费者内部执行方式）
5. Planner 迁移（替换消费者内部执行方式）
6. 验证 + 切换默认
