# Design: 基于 macaca-agent 抽象的 traced agent construction primitive

## Context

当前 `macaca-agent` 已经具备一组新的基础抽象：

- `AgentServices` facade/no-op fallback
- `BasicAgentBuilder`
- `AgentCapabilitySet`
- `AgentLifecyclePolicy`

但当前 `macaca-web::FrameworkRunner` 仍然承担了事实上的 agent factory 角色，并没有真正建立在这些抽象之上。它位于 web crate 内，强耦合：

- `AppState`
- session active sender / SSE
- `EventLog`
- OS-level skill/MCP visibility
- workspace/cwd 路径
- executor broadcast

这使得两个问题同时存在：

- “agent 如何被构建” 仍然不是 `macaca-framework` 自身的原语，而是 web 对 framework 的一层重 glue。
- `macaca-agent` 新引入的 builder/services/capability/lifecycle 抽象还没有被上层真正消费，底层重构价值没有向上释放。

随着 gateway、daemon、future runtime host、SDK registration 继续推进，这个边界会越来越贵。

## Goals

- 为 traced agent 构建提供 framework-level primitive，并让它建立在 `macaca-agent` 新抽象之上。
- 让 web 只负责 OS adapter，而不是拥有完整构建逻辑。
- 让 task 系统表达稳定的执行意图，而不是依赖 web 内部 helper 命名。
- 保持现有 trace/tool/session 行为 1:1 兼容。

## Non-Goals

- 不改变 ReActAgent 主循环语义。
- 不把 PlanLoop/WorkerLoop 调度直接搬进 framework。
- 不在本提案里重做 driver/skill/MCP policy。
- 不强行消除所有 web glue；只迁移“agent 构建职责”，不是迁走所有运行时职责。
- 不重新设计 `macaca-agent` 已落地的抽象；这里只消费它们。

## Proposed Architecture

### 1. Framework construction contract is based on macaca-agent

在 `macaca-framework` 引入稳定构建接口：

```rust
pub struct AgentBuildRequest {
    pub role: AgentRoleKind,
    pub identity: AgentIdentity,
    pub prompt: AgentPromptParts,
    pub model: ModelSelection,
    pub session: Option<AgentSessionContext>,
    pub trace: AgentTraceContext,
    pub tools: AgentToolPolicy,
    pub services: AgentServices,
    pub lifecycle: AgentLifecycleConfig,
    pub capabilities: AgentCapabilitySet,
}

#[async_trait]
pub trait TracedAgentFactory: Send + Sync {
    type Output: Agent;

    async fn build(
        &self,
        request: AgentBuildRequest,
    ) -> Result<Self::Output, AgentBuildError>;
}
```

这里的重点不是把 web 的所有状态搬进 framework，而是把“构建一个 traced agent 所需的通用信息”做成 request object，并显式复用 `macaca-agent` 的新抽象，而不是在 framework/web 再各建一套平行结构。

建议的映射关系：

- `AgentServices`：由 web/runtime adapter 组装，framework 只消费 facade
- `BasicAgentBuilder`：作为统一基础 builder 或 builder contract 的参考实现
- `AgentCapabilitySet`：作为 intent/tool policy 计算后的内部 capability 表达
- `AgentLifecyclePolicy`：作为 runtime state/lifecycle 决策的基础 contract

### 2. Web provides OS adapters and macaca-agent inputs

`macaca-web` 保留对 OS 资源的适配，但不再拥有构建主流程。

建议拆分：

```rust
pub struct WebAgentBuildAdapters {
    pub llm_router: Arc<dyn RoutedModelProvider>,
    pub toolkit_contributors: Vec<Arc<dyn ToolkitContributor>>,
    pub trace_sink: Arc<dyn AgentTraceSink>,
    pub session_store: Arc<dyn SessionContextStore>,
    pub workspace_resolver: Arc<dyn WorkspaceResolver>,
}
```

`FrameworkRunner` 迁移为薄兼容层：

```rust
impl FrameworkRunner {
    pub async fn build_worker_agent(...) -> Result<HookedAgent<ReActAgent>, String> {
        self.factory
            .build(self.web_request_mapper.worker_request(...))
            .await
    }
}
```

在这个兼容层中，web 的职责是：

- 将 `AppState`、session、workspace、SSE/EventLog、executor、skill/MCP visibility 转换成 `AgentBuildRequest`
- 组装 `AgentServices`
- 计算初始 `AgentCapabilitySet`
- 选择对应的 traced intent

而不是自己完成全部 agent 装配。

### 3. Separate build intent from crate-local helper naming

当前 builder 名称已经带有 web 时代的历史痕迹：

- `build_traced_agent`
- `build_worker_agent`
- `build_traced_agent_with_goal`
- `build_planner_decomposition_agent`
- `build_coordinator`

这些 API 在过渡期可以保留，但内部应首先映射成统一 intent：

```rust
pub enum AgentBuildIntent {
    CoordinatorChat,
    PlannerDecomposition { goal_id: Option<TaskId> },
    PlannerReview { task_id: TaskId },
    PlannerFollowUp { goal_id: Option<TaskId> },
    WorkerTask { task_id: TaskId },
}
```

Framework primitive 应接收 intent，再由 policy/template 决定：

- prompt part
- tool visibility
- trace sink
- lifecycle tool suppression
- allowed tool narrowing
- capability composition
- agent service binding

### 4. Task crate depends on intent contract and macaca-agent-compatible launcher

`macaca-task` 不应该知道 `build_worker_agent` 还是 `build_traced_agent_with_goal` 这种 web helper 名字。它应该只表达：

- 需要 planner decomposition
- 需要 planner review
- 需要 worker execution

建议引入执行 contract：

```rust
#[async_trait]
pub trait AgentExecutionLauncher: Send + Sync {
    async fn launch(
        &self,
        intent: AgentBuildIntent,
        input: AgentExecutionInput,
    ) -> Result<AgentExecutionOutput, AgentExecutionError>;
}
```

过渡期内，这个 trait 可以放在 `macaca-framework` 或一个更中性的边界 crate，由 `macaca-web` 提供 runtime adapter 实现。

关键点是：launcher contract 输出的不是 web helper 行为，而是“已按 `macaca-agent` 基础抽象构建好的 traced agent 执行能力”。

### 5. AgentServices facade remains the only service binding surface

迁移后，上层不应直接操作“也许有 memory / 也许有 ipc / 也许有 persist”的裸 `Option<Box<...>>`。所有构建路径都应通过 `AgentServices` facade 暴露服务绑定面。

```rust
pub struct AgentBuildRequest {
    // ...
    pub services: AgentServices,
}
```

这意味着：

- web adapter 负责把 OS 资源变成 `AgentServices`
- framework factory 只通过 facade 消费服务
- task 侧执行 contract 不感知具体服务装配细节

### 6. Trace sink stays explicit

当前系统已经证明：如果构建入口没有 trace sink，用户可见性就会断。

因此新 primitive 必须强制带 trace context：

```rust
pub struct AgentTraceContext {
    pub session_id: Option<String>,
    pub task_id: Option<TaskId>,
    pub source_agent: String,
    pub sink: Arc<dyn AgentTraceSink>,
}
```

这条规则必须是硬约束：

- 无 trace context 不允许构建 traced runtime agent
- trace sink 缺省只能是显式 no-op，不能静默绕过

### 7. BasicAgentBuilder provides the migration baseline

即使最终 runtime agent 仍然是 `HookedAgent<ReActAgent>`，它的构建过程也应对齐 `BasicAgentBuilder` 的思想：

- request object 驱动
- 默认值集中
- capability/service/lifecycle 显式输入
- 旧入口委托新入口

也就是说，这次迁移不是要求上层真的去实例化 `BasicAgent`，而是要求它们的构建逻辑遵守同一 builder contract，而不是继续写大而散的参数装配函数。

### 8. Toolkit assembly becomes contributor-based

agent 构建过程中最容易继续堆 if/else 的部分是 toolkit 组装。它应该在 framework primitive 里变成 contributor 模式，而不是留在 `framework_runner.rs` 大函数里。

```rust
#[async_trait]
pub trait ToolkitContributor: Send + Sync {
    async fn contribute(
        &self,
        request: &AgentBuildRequest,
        toolkit: &mut Toolkit,
    ) -> Result<(), AgentBuildError>;
}
```

典型 contributor：

- base tools contributor
- todo/task contributor
- skill catalog contributor
- OS MCP contributor
- skill-backed MCP contributor
- trace middleware contributor

这样可以把“构建职责”从 web 中移出去，同时不改变具体工具是否可见的规则。

## Migration Phases

### Phase 1: Define framework contracts on top of macaca-agent

- 在 `macaca-framework` 中新增 build request / intent / trace context / toolkit contributor / traced factory trait。
- request/contract 必须直接消费 `AgentServices`、`AgentCapabilitySet`、lifecycle config。
- 不改现有 web 调用点。

### Phase 2: Introduce web adapters

- 在 `macaca-web` 中把 `AppState`、SSE/EventLog、workspace、skill/MCP 注入改为 adapter/contributor。
- web adapter 负责组装 `AgentServices` 和 capability input，而不是构建完整 agent。
- `FrameworkRunner` 仍对外暴露现有方法，但内部委托 framework factory。

### Phase 3: Migrate coordinator construction

- `build_coordinator` 内部改走新 factory。
- 保持 pause/resume middleware、SSE 事件、session snapshot 行为不变。

### Phase 4: Migrate planner/worker construction

- `build_worker_agent`
- `build_traced_agent_with_goal`
- `build_planner_decomposition_agent`

都改为 request + intent 映射。

### Phase 5: Decouple task execution intent

- 让 task 侧面向 execution launcher / build intent contract。
- 不要求 task crate 直接 import web runner helper 名称。
- task 侧通过 contract 获得的是基于 `macaca-agent` 抽象构建的执行能力。

### Phase 6: Compatibility cleanup

- 在验证完成后，标记 web-only legacy construction internals 为 deprecated。
- 后续独立 change 再决定是否删除兼容层。

## Risks

### Risk 1: Trace compatibility regression

如果 factory 下沉时丢失 trace hook 或 tool middleware，前端实时/历史 trace 会立即退化。

Mitigation:

- 兼容测试必须覆盖 live SSE、EventLog、刷新恢复。
- 所有 traced builder 都必须通过统一 trace sink contract 构建。

### Risk 2: Tool visibility drift

如果 contributor 顺序或 narrowing 规则变化，planner/worker/coordinator 看到的工具集会发生偏移。

Mitigation:

- 增加 toolkit definition snapshot tests。
- 每类 intent 固定验证 allowed tools 集合。

### Risk 3: web/task dependency inversion 处理不当

如果直接让 `macaca-task` 依赖 `macaca-web` trait，会继续固化错误方向。

Mitigation:

- execution launcher contract 必须定义在 framework 或中性边界，不得定义在 web crate。

### Risk 4: macaca-agent 抽象被旁路

如果 framework/web/task 只是再加一层新的 request/factory，但内部仍然直接拼裸 capability/service/state 结构，那么这次迁移只是换名，不是完成真正的上收敛。

Mitigation:

- design review 时明确检查是否直接消费 `AgentServices`、`AgentCapabilitySet`、builder-style request。
- 测试中增加“legacy facade 仍可用，但新路径必须走统一构建 contract”的断言。

## Verification

- `openspec validate migrate-agent-construction-to-framework-primitives --strict`
- `cargo check -p macaca-agent -p macaca-framework -p macaca-web -p macaca-task`
- 回归测试：
  - coordinator live trace
  - planner decomposition/review trace
  - worker task trace
  - browser refresh history recovery
  - tool visibility snapshot per intent
  - `AgentServices` facade/no-op compatibility across new build path
  - capability flatten compatibility across new build path

