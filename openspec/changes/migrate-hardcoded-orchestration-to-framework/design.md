## Context

当前系统处于 framework 与 legacy runtime 并存状态：

- framework 已支撑部分 coordinator / planner / worker 执行
- `AgenticLoop` 与 direct `llm.chat` 仍存在
- `macaca-web` 仍承载大量 orchestration 逻辑
- `macaca-task` 与 `macaca-tools` 中仍包含 fullstack-autodev 风格的角色名与依赖纪律

这使得系统无法真正做到 config-driven、app-agnostic、framework-native。

## Goals / Non-Goals

**Goals**
- 让 `macaca-framework` 成为默认且唯一的 agent execution substrate
- 将 tool policy 迁为 capability/config-driven
- 将 application-specific orchestration discipline 从 OS substrate 中剥离
- 收敛 session/trace/resume 语义

**Non-Goals**
- 不一次性移除 TodoBoard / PlanLoop / WorkerLoop
- 不在一轮内重写所有持久化模型
- 不改变用户可见 API 作为首要目标

## Decisions

### Decision 1: Execution first, policy second

先统一执行内核，再抽离 app-specific policy。否则会出现“策略抽了，但底层还是两套执行语义”的问题。

### Decision 2: OS provides primitives, app provides workflow discipline

OS 仅保留：

- task dependency
- task state
- execution session
- event/trace substrate

具体的 `architect -> backend -> frontend` 等约束迁到 app workflow/policy。

### Decision 3: Web becomes transport-oriented

`macaca-web` 继续承载 API/SSE/session projection，但不再长期保留 orchestration-heavy glue code。

### Decision 4: `PlanNotebook` and `TodoBoard` have distinct, fixed responsibilities

这条边界在本 change 中被正式固定：

- `PlanNotebook`
  - 是 agent-local planning primitive
  - 是 Macaca OS agent 的“脑内计划本”
  - 负责 planner / agent 的内部 planning state：
    - decomposition 草稿
    - 当前思路与下一步
    - review / follow-up 的临时推理上下文
    - 尚未进入系统执行面的子步骤

- `TodoBoard`
  - 是系统的“正式任务账本”
  - 是 Macaca OS 所有 application 共用的基础设施
  - 是 task/todo 管理基座，也是 agent 自主运行的必备工具
  - 负责所有正式 execution state：
    - 可被 worker claim 的任务
    - 可被 planner review 的任务
    - dependency / retry / completion / persistence
    - user-visible 与 resumable 的任务状态

硬约束：

- planning MAY originate in `PlanNotebook`
- but any task that becomes executable, reviewable, resumable, or user-visible MUST be materialized into `TodoBoard`

因此，迁移目标不是用 `PlanNotebook` 替代 `TodoBoard`，而是：

- 用 `PlanNotebook` 承接 agent 内部 planning
- 用 `TodoBoard` 继续承接 OS-wide durable task substrate

## Risks / Trade-offs

- 迁移 planner/review 到 framework planning primitive 时，容易与 TodoBoard 状态机重叠
  - Mitigation: 保留 TodoBoard 为 durable substrate，planner state 先双写再收敛
- 移除基于 agent name 的工具分支时，可能影响现有 fullstack-autodev
  - Mitigation: 先引入 declarative tool policy，再迁移默认 app 配置
- session/resume framework 化会波及 trace/event 恢复
  - Mitigation: 保持 EventLog 为 source of truth，逐步替换上层 glue

## Migration Plan

1. 盘点并封住 legacy execution path
2. 让 `agent_runner` 等 legacy 路径转为 framework wrapper
3. 抽 capability-driven tool policy
4. 把 app-specific gating 从 OS 底层移到 app workflow/policy
5. 演进 execution session / resume / trace primitive

## Open Questions

- framework 是否需要独立的 orchestration module，而不仅是 agent/pipeline primitives
