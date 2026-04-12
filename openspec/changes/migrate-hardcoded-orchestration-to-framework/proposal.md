# Change: Migrate hardcoded orchestration and app-specific logic to macaca-framework

## Why

虽然 `macaca-framework` 已经接入 coordinator / planner / worker 的部分执行链，但系统整体仍然保留大量：

- 基于 agent name 的 if/else
- application 专有的 orchestration discipline
- legacy `AgenticLoop` 与 direct `llm.chat` 路径
- 停留在 `macaca-web` 的 session / resume / tool binding glue code

这使得系统仍然难以扩展到更多 application、更多角色模型和更多执行入口。

## What Changes

- 统一 agent execution substrate，逐步淘汰 legacy `AgenticLoop` 业务路径
- 将 tool binding 从角色名分支迁移为 capability/config-driven policy
- 将 planner decomposition/review/follow-up 流程更多下沉到 framework planning primitive
- 将 application-specific orchestration discipline 从 OS substrate 中移除，迁到 app workflow/policy
- 将 session/trace/resume 逐步演进为 framework-level execution primitive

## Explicit Non-Goals

- 本 change 不修改前端视觉设计
- 本 change 不改变现有 HTTP API 形状作为首要目标
- 本 change 不在第一阶段移除 TodoBoard / PlanLoop / WorkerLoop 的 durable substrate

## Impact

- Affected specs: `framework-driven-orchestration`
- Affected code:
  - `macaca-web`
  - `macaca-runtime`
  - `macaca-task`
  - `macaca-tools`
  - `macaca-framework`
- **BREAKING (internal)**:
  - agent execution 和 tool policy 的内部接线会显著变化
