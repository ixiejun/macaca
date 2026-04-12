# Macaca Framework Migration Audit and Plan

## Purpose

这份文档用于回答三个问题：

1. 当前系统里还有哪些执行链、状态机、工具注入和编排逻辑没有迁移到 `macaca-framework`
2. 哪些设计仍然写死、分支过多、带有 application 专有假设
3. 后续应按什么顺序迁移，才能降低风险并逐步收敛到底层 framework

本文档是执行前的迁移审查，不是假设“全部已完成”的架构宣言。

## Latest Implementation Status (2026-04-10)

本轮 `migrate-hardcoded-orchestration-to-framework` 已完成并落地到代码：

- `/api/chat` 路由已下线，仅保留 `/api/chat/v2`。
- `FrameworkRunner::build_agent / build_agent_with_goal` 已禁用（返回错误并标注废弃），统一走 traced builders。
- tool policy 改为 capability/config 驱动；`CreateTodoTool` 不再依赖固定角色名分支，且 `agent` 字段改为必填（不再默认 `"backend"`）。
- `TodoBoard` 中 app-specific `architect` gating 已移除，调度纪律迁到 app workflow/policy。
- planner/review 状态桥接到 framework `PlanNotebook`，session/resume 状态桥接到 framework `ExecutionContext` + `SessionStore`。

验证基线：

- `cargo check -p macaca-web -p macaca-tools -p macaca-task -p macaca-framework -p macaca-integration-tests` 通过。
- `cargo test -p macaca-tools create_todo_` 通过。
- `cargo test -p macaca-web loop_manager::tests::` 通过（验证 capability 驱动选择，不依赖固定角色名）。
- `cargo test -p macaca-integration-tests -- --nocapture` 通过。
- 本地 `chat_v2` 会话（session `579d777d-2bf7-48fc-9c8d-6c9be4a581f1`）EventLog 已持久化
  `delegated_task_start / delegated_thinking / delegated_tool_call / delegated_tool_result / delegated_assistant / delegated_task_complete`。

## Current State Summary

当前系统已经完成了一部分 framework 迁移，但仍然处于“双轨”状态：

- `macaca-web/src/framework_runner.rs`
  - coordinator / planner / worker 的一部分执行已切到 `ReActAgent + Toolkit + Hook`
- `macaca-web/src/chat_orchestrator.rs`
  - `chat_v2` 的 framework coordinator 已可运行
- `macaca-web/src/loop_manager.rs`
  - PlanLoop / WorkerLoop 的 planner 与 worker 执行已大量桥接到 framework

但同时，系统仍保留另一条核心旧路径：

- `macaca-runtime/src/agentic_loop.rs`
- `macaca-web/src/agent_runner.rs`
- `macaca-web/src/chat_orchestrator.rs` 中仍有直接 `state.llm.chat(...)` 和 legacy orchestration 逻辑

结论：

- **framework 已经是主要执行路径之一，但还不是唯一执行底座**
- **系统中仍存在大量非 framework 的 orchestration / policy / state glue**

## Framework Capability Map

`macaca-framework` 当前已经具备可承接迁移的能力面：

- `agent.rs`
  - `Agent` trait
  - `Hook`
  - `HookRegistry`
  - `HookedAgent`
- `react_agent.rs`
  - ReAct 执行循环
- `tool.rs`
  - `Toolkit`
  - `ToolMiddleware`
  - 工具分组与统一调用入口
- `memory.rs`
  - working memory / summary / tagged memory
- `pipeline.rs`
  - `SequentialPipeline`
  - `FanoutPipeline`
  - `MsgHub`
- `plan.rs`
  - `Plan`
  - `PlanNotebook`
- `session.rs`
  - framework session persistence 抽象
- `adapter.rs`
  - 与 `macaca-llm` / `macaca-tools` 的兼容桥接

这意味着后续很多“系统层 if/else + 手工 glue code”理论上都能被 framework 吞掉，而不是继续堆在 `macaca-web`。

## Hardcoded and Non-Extensible Hotspots

### 1. Agent-role hardcoding in runtime orchestration

高频硬编码仍集中在这些位置：

- `macaca-web/src/framework_runner.rs`
- `macaca-web/src/agent_runner.rs`
- `macaca-web/src/loop_manager.rs`
- `macaca-web/src/lib.rs`
- `macaca-tools/src/todo.rs`
- `macaca-task/src/todo_board.rs`

主要模式：

- `match agent_name`
- `if agent_name == "coordinator" || agent_name == "planner"`
- `if agent_name == "architect"`
- `planner / architect / frontend / backend` 作为调度语义本身，而不是配置或 capability

影响：

- 系统仍然隐含“fullstack-autodev 风格的 supervisor + worker 角色模型”
- 新 application 想采用不同角色体系时，仍会被底层逻辑卡住

### 2. Application-specific policy leaking into system crates

最明显的是 `architect -> backend -> frontend` 这类依赖纪律已经深入到底层：

- `macaca-task/src/todo_board.rs`
  - `architect_gate_allows_claim`
- `macaca-web/src/loop_manager.rs`
  - decomposition prompt 中直接强调 architect 优先

这类逻辑不是 OS 级通用能力，而是 `fullstack-autodev` 的应用级策略。

影响：

- 任务系统与具体应用方法论耦合
- TodoBoard / LoopManager 不再是通用 substrate

### 3. Legacy AgenticLoop still duplicates framework responsibilities

`macaca-runtime/src/agentic_loop.rs` 仍承担：

- LLM 调用
- tool call / tool result 注入
- loop detection
- context trimming
- event emission

而 `macaca-framework::react_agent` 已经承担了其中大部分角色。

影响：

- 两套 agent execution substrate 并存
- 同类 bug / observability / policy 要修两遍
- 新能力接入时无法保证两个执行面一致

### 4. `macaca-web` still owns too much orchestration

`macaca-web` 里还承载了过多本应下沉的逻辑：

- `chat_orchestrator.rs`
  - 会话、SSE、legacy LLM call、framework coordinator、cleanup、session snapshot
- `loop_manager.rs`
  - PlanLoop / WorkerLoop 启停
  - planner prompt 生成
  - worker 执行桥接
  - goal completion resume
- `framework_runner.rs`
  - persona prompt 拼装
  - toolset policy
  - workspace/cwd 绑定
  - trace hook 注入

影响：

- framework 还是“被 web 层调用的库”，而不是系统执行内核
- 迁移新入口（gateway/daemon/api）时仍会复制编排代码

### 5. Tool policy is role-switched, not capability-driven

目前工具暴露是靠角色分支：

- `coordinator` 给 goal tools
- `planner` 给 planning/review tools
- worker 给 claim/start/submit/progress tools

这在 `framework_runner.rs` 和 `agent_runner.rs` 中都存在。

影响：

- tool policy 基于名称而不是声明式 capability
- 角色体系扩展时要继续加 if/else

### 6. Resume / lifecycle semantics are system glue, not framework primitive

目前 pause/resume、goal completion resume、SSE event bridging 仍是大量 web/kernel glue：

- `PauseOnGoalMiddleware`
- `goal_to_session`
- `fork_to_session`
- `loop_resumed`
- `event_persistence`

这些能力本质上可以演进成 framework 的 execution/session primitive，而不是长久停留在 `macaca-web`。

## Designs That Should Be Refactored Through macaca-framework

以下设计最适合通过 framework 重构：

### A. Unified agent execution substrate

目标：

- 让 `ReActAgent` 成为唯一默认执行内核
- `AgenticLoop` 退化为兼容层或被移除

迁移对象：

- `macaca-web/src/agent_runner.rs`
- `macaca-runtime/src/agentic_loop.rs`
- `macaca-web/src/chat_orchestrator.rs` 里直接 `llm.chat` 的残留路径

### B. Capability-driven tool binding

目标：

- 工具由 agent capability / manifest policy / app workflow 决定
- 不再由 `agent_name == planner` 这种分支决定

迁移对象：

- `macaca-web/src/framework_runner.rs`
- `macaca-web/src/agent_runner.rs`
- `macaca-tools` 中 supervisor/worker 假设

建议落点：

- framework `Toolkit` + declarative tool policy layer

### C. Framework-native orchestration pipeline

目标：

- 将 coordinator -> planner -> worker -> reviewer 这类流转表示成 framework pipeline，而不是 web 中手工 glue

迁移对象：

- `macaca-web/src/loop_manager.rs`
- `macaca-web/src/chat_orchestrator.rs`

建议落点：

- `SequentialPipeline`
- `FanoutPipeline`
- `MsgHub`
- 新增 framework orchestration primitives

### D. Plan / review notebook migration

目标：

- 将 planner 的 decomposition / review / follow-up 状态管理逐步下沉到 framework `PlanNotebook`
- TodoBoard 保留 durable task substrate，但 planner reasoning/state 由 framework plan primitive 驱动

迁移对象：

- `macaca-task/src/plan_loop.rs`
- `macaca-web/src/loop_manager.rs`

### Formal Boundary: `PlanNotebook` vs `TodoBoard`

这条边界在迁移中是正式约束，不再是待讨论问题：

- `PlanNotebook`
  - 是 Macaca OS agent 的“脑内计划本”
  - 属于 agent-local planning primitive
  - 负责：
    - 目标拆解草稿
    - 当前思路与下一步
    - review/follow-up 的临时推理状态
    - 尚未进入系统执行面的 planning context
  - 不负责：
    - worker claim
    - durable task state source of truth
    - user-visible execution ledger

- `TodoBoard`
  - 是系统的“正式任务账本”
  - 是 Macaca OS 所有 application 共用的基础设施
  - 是 task/todo 管理基座，也是 agent 自主运行的必备工具
  - 负责：
    - 正式任务创建
    - 状态流转
    - dependency / review / retry / completion
    - session / trace / persistence 对齐
    - worker 可消费、planner 可审查、用户可见的任务状态
  - 是系统执行面的 source of truth

正式约束：

- planning 可以先发生在 `PlanNotebook`
- 但任何任务一旦变成：
  - executable
  - reviewable
  - resumable
  - user-visible
  就必须 materialize 到 `TodoBoard`

一句话定义：

- `PlanNotebook` 管 agent 自己怎么想
- `TodoBoard` 管系统正式怎么执行

### E. Session / trace / resume as framework concern

目标：

- 把 “agent execution session” 与 “goal pause/resume” 抽象为 framework session primitive
- web 只负责 transport，不负责业务 resume 语义

迁移对象：

- `macaca-web/src/session.rs`
- `macaca-web/src/chat_orchestrator.rs`
- `macaca-web/src/hook_consumer.rs`
- `macaca-web/src/event_persistence.rs`

### F. Application-specific dependency discipline moves out of OS substrate

目标：

- `architect` 优先、`frontend/backend` 等待 review 之类规则，迁回 app workflow / app policy
- OS 底层只提供 dependency + gating primitive

迁移对象：

- `macaca-task/src/todo_board.rs`
- `macaca-web/src/loop_manager.rs`

## Migration Principles

后续迁移必须遵守：

1. **先统一执行底座，再抽策略**
   - 先让执行链都走 framework
   - 再把角色/策略从 if/else 中抽出来

2. **OS 提供 primitive，App 提供 policy**
   - OS 不再写死 `architect/backend/frontend`
   - app 通过 workflow / manifest / capability 表达依赖和角色

3. **transport 与 orchestration 解耦**
   - SSE / HTTP / session detail 是 transport
   - goal pause/resume / agent pipeline 是 orchestration

4. **同一能力只保留一套实现**
   - LLM routing 已经统一了一次
   - agent execution / tool binding / session trace 也应同样统一

## Migration Plan Table

| Phase | 目标 | 主要改动 | 风险 | 优先级 |
|---|---|---|---|---|
| P0 | 盘点并封住 legacy 执行入口 | 标记所有 `AgenticLoop` / direct `llm.chat` 入口；新增兼容边界文档 | 低 | P0 |
| P1 | 统一 agent execution 到 framework | `agent_runner` 收敛到 framework；移除新的 legacy 调用面 | 中 | P0 |
| P2 | 工具策略 capability 化 | 用 declarative tool policy 替换 `agent_name` 分支 | 中 | P0 |
| P3 | 把 planner/review 流程下沉到 framework planning primitive | 让 planner decomposition/review/follow-up 使用 framework plan abstraction | 中高 | P1 |
| P4 | 抽离 application-specific discipline | 把 architect-first / role ordering 从 OS 底层迁到 app policy/workflow | 中高 | P1 |
| P5 | 会话/恢复/trace framework 化 | 把 pause/resume/session trace 从 web glue 迁为 framework primitive | 高 | P1 |
| P6 | web 层瘦身 | `macaca-web` 仅保留 API/SSE/session transport | 中 | P2 |

## Detailed Execution Checklist

### Phase P0 — Establish migration boundary

- 识别并列出所有 legacy execution path
- 标记哪些路径允许保留作为兼容层
- 明确禁止新增非 framework agent 执行路径

### Phase P1 — Framework-only execution

- 将 `agent_runner` 的 legacy 执行改造成 framework wrapper
- 消除 `chat_orchestrator` 中直接 `state.llm.chat(...)` 的业务执行路径
- 让 framework 成为 coordinator / planner / worker / direct-agent 的统一底座

### Phase P2 — Declarative tool policy

- 定义 agent capability -> tool binding 规则
- 将 `coordinator/planner/worker` 的工具差异转成 declarative policy
- 移除 `framework_runner` / `agent_runner` 中基于名字的主要分支

### Phase P3 — Planning primitive migration

- 将 planner 的 decomposition 和 review 状态逐步映射到 framework `PlanNotebook`
- TodoBoard 只承载 durable task state
- planner 内部推理与进度状态迁入 framework

### Phase P4 — Move app-specific workflow out of OS substrate

- 移除 `architect_gate_allows_claim` 里的具体角色名依赖
- 用 app workflow / dependency metadata 表达顺序纪律
- fullstack-autodev 作为 app policy，而不是 task engine policy

### Phase P5 — Framework-native execution session

- 抽象 goal pause/resume primitive
- 抽象 execution trace/session primitive
- 让 session restore 与 live trace 依赖统一 execution session model

### Phase P6 — Web transport-only layer

- `macaca-web` 只负责：
  - HTTP API
  - SSE transport
  - session data projection
- 业务 orchestration 尽量下沉到 framework/task/kernel

## Recommended Immediate Next Actions

建议下一轮优先做三件事：

1. **建立 legacy execution inventory**
   - 明确所有还在走 `AgenticLoop` 或 direct `llm.chat` 的入口

2. **抽出 declarative tool policy**
   - 这是清除 `coordinator/planner/worker` 大量硬编码的最低成本突破口

3. **为 app-specific gating 建立 manifest/workflow 表达**
   - 为替换 `architect_gate_allows_claim` 做准备

## Success Criteria

迁移完成时，至少应满足：

- 新增 application 不需要修改 OS 源码中的角色名分支
- agent 执行默认全部通过 framework
- `macaca-web` 不再承载主要 orchestration 语义
- TodoBoard / LoopManager 不再包含 fullstack-autodev 专属流程纪律
- 事件、trace、resume 通过统一 execution/session primitive 表达

## Bottom Line

当前系统最大的结构问题，不再是“有没有 framework”，而是：

- **framework 已存在，但还不是唯一底座**
- **系统层仍承载过多 app-specific 和 role-specific 逻辑**

所以后续迁移的主线应该不是继续零散修补，而是：

**把执行、工具、计划、会话、恢复逐步收敛进 `macaca-framework`，同时把 application-specific policy 从 OS 底层剥离出去。**
