# Change: 基于 macaca-agent 新抽象迁移 agent 构建逻辑

## Why

当前系统已经完成了 `macaca-agent` 的一轮设计模式重构，引入了以下基础抽象：

- `AgentServices` facade + no-op fallback
- `BasicAgentBuilder`
- `AgentCapabilitySet` / `AgentCapabilityNode`
- `AgentLifecyclePolicy`

但这些新抽象目前仍主要停留在 `macaca-agent` crate 自身，`macaca-framework`、`macaca-web`、`macaca-task` 的相关 agent 构建逻辑还没有迁移到这套基础之上。与此同时，系统已经禁止直接使用 `FrameworkRunner::build_agent / build_agent_with_goal`，也已经把 coordinator / planner / worker 的主要执行链路切到 traced builders。但“真正的 agent 构建逻辑”仍然主要停留在 `macaca-web/src/framework_runner.rs`：

- system prompt 组装
- model selection / routed adapter
- toolkit build 与 tool middleware 注入
- skill / MCP / workspace 上下文注入
- SSE / EventLog / executor trace hook 桥接
- coordinator / worker / planner 不同 builder 入口的差异化逻辑

这带来四个结构性问题：

1. `macaca-agent` 新增的 builder / services / capability / lifecycle 抽象还没有成为上层构建的统一基础。
2. `macaca-framework` 还是“执行库”，不是“消费 `macaca-agent` 核心抽象的构建 primitive”。
3. `macaca-web` 继续拥有过多跨层 agent 装配知识，gateway/daemon/其他入口无法复用同一构建路径。
4. `macaca-task` 的 planner/worker 执行意图仍然要通过 web 层 helper 落地，任务系统无法面向稳定的 framework-level factory contract 编排。

根据：

- `macaca/docs/design-pattern-refactor-plans/README.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-agent.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-framework.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-web.md`
- `macaca/docs/macaca-framework-migration-audit.md`
- `openspec/changes/refactor-macaca-agent-patterns/proposal.md`

下一步应把 agent 构建逻辑迁移到“以 `macaca-agent` 新抽象为基础的 framework-level primitive”上，并让 web/task 消费这套基础，而不是继续各自维护一层平行装配逻辑。

## What Changes

- 在 `macaca-framework` 中引入以 `macaca-agent` 新抽象为基础的 traced agent construction primitive，例如 `TracedAgentFactory` / `AgentBuildRequest` / `AgentTraceContext` / `ToolkitContributor`，并明确这些 primitive 如何消费 `AgentServices` facade、`BasicAgentBuilder`、`AgentCapabilitySet`、`AgentLifecyclePolicy`。
- 将当前 `macaca-web/src/framework_runner.rs` 中的通用构建流程迁移为“构造 `macaca-agent` build input + 调用 framework primitive”，web 只保留 `AppState`、session、SSE/EventLog、workspace 等 OS 侧 adapter。
- 明确 coordinator / planner decomposition / planner review / worker 四类构建意图的标准 request/config，不再把差异写成 web 层散落的 builder 函数。
- 让 `macaca-task` 表达“需要哪类 agent 执行哪类任务”的框架级意图，并通过稳定 contract 依赖 `macaca-agent`/framework 的构建能力，而不是依赖 web 内部构建 helper 的具体命名和路径。
- 保留现有 traced builder 行为、trace 协议、tool policy、EventLog/SSE 行为不变，先迁移构建职责，再考虑进一步收缩 web glue。

## Non-Goals

- 不改变 coordinator / planner / worker 当前端到端行为语义。
- 不改变 EventLog、SSE、frontend trace 恢复协议。
- 不改变 planner prompt、review prompt、goal evaluation prompt 内容，除非为保持兼容必须做机械性参数传递调整。
- 不改变 TodoBoard / PlanLoop / WorkerLoop 的调度语义。
- 不在本 change 中重写 tool policy 或 skill/MCP gating 规则。
- 不立即删除 `macaca-web` 中所有构建 helper；先引入 framework primitive，再逐步让旧入口委托到新入口。
- 不重新设计 `macaca-agent` 的基础抽象；本 change 的目标是消费和迁移到这些抽象，不是再重做一轮底层设计。

## Impact

- Affected specs: `framework-agent-construction`
- Affected code:
  - `macaca/crates/macaca-agent/src/**` 只在需要补充兼容接口时小范围调整
  - `macaca/crates/macaca-framework/src/**`
  - `macaca/crates/macaca-web/src/framework_runner.rs`
  - `macaca/crates/macaca-web/src/chat_orchestrator.rs`
  - `macaca/crates/macaca-web/src/loop_manager.rs`
  - `macaca/crates/macaca-task/src/**` 中与 planner/worker 执行意图建模有关的部分
- Expected risk: Medium to High
- Risk reason:
  - 这是跨 crate 的职责下沉，不是局部 helper 抽取。
  - 构建链路直接影响 trace、tool visibility、driver/MCP/skill 注入和 session 恢复。
- Risk mitigation:
  - additive-first：先让 framework primitive 消费新的 `macaca-agent` 抽象，再让 web 入口委托。
  - 所有现有 builder 名称与行为在过渡期保留。
  - 每个迁移阶段都要锁定 live SSE、EventLog、刷新恢复、tool policy 的兼容测试。

## Rollout Strategy

按阶段推进，不允许一轮内同时改抽象、改调度、改行为：

1. 在 framework 中定义“基于 `macaca-agent` 新抽象”的构建 primitive 和 request/config 模型。
2. 在 web 中实现 adapter，把现有 `FrameworkRunner` 内部委托到新 primitive，而不是自行装配 agent。
3. 将 coordinator 构建迁移到新 primitive，保持现有 `build_coordinator` API 兼容。
4. 将 planner/worker traced builders 迁移到新 primitive，保持现有 task 流和 trace 行为。
5. 将 `macaca-task` 中与 agent 执行意图相关的接口改为面向 framework contract，而不是 web helper 命名。
6. 只有在兼容验证完成后，才允许废弃旧 web-only 构建实现细节。

