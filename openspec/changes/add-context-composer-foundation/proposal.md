# Change: 增加 Context Composer 集成基座

## Why

Macaca 已经具备可插拔 context engine、memory facade、skill runtime 和 MCP runtime 的基础能力，但多来源上下文仍需要一个更窄、更稳定的组合边界。若继续让 runtime/framework/web 分别拼接 profile、memory、skills、MCP、tools 和 trace，会让上下文工程重新退化成分散字符串拼接，并阻碍用户替换自己的上下文管理系统。

本提案建立 `ContextCandidate -> ContextPlan -> CompiledContext -> ContextReport` 的组合基座，让 profile、active memory、skills/MCP 和治理策略都以 provider/candidate 形式接入，不直接控制 prompt 或 transcript。

## What Changes

- 定义 `ContextCandidate`、`ContextProvider`、`ContextComposer`、`ContextPlan`、`CompiledContext` 的集成契约。
- 定义 provider stage、scope、priority、trust、cache class、target、budget 和 diagnostics 的最小字段。
- 使用 Chain of Responsibility 组织 provider 收集顺序，使用 Builder 构建 `ContextPlan`，使用 Composite 表达最终上下文 sections。
- 使用 Strategy 抽象预算、排序、去重、截断、渲染和 report policy。
- 提供 `ContextFacade` 作为 runtime/framework 的唯一调用入口。
- 明确 dynamic context 不得写回 canonical transcript，且默认不进入 stable prefix。
- 本提案只做组合基座，不实现具体 profile 文件加载、主动向量记忆、skills/MCP capability 注入或外部 provider runtime。

## Impact

- Affected specs: `context-composer`
- Affected code:
  - `macaca/crates/macaca-context`
  - `macaca/crates/macaca-framework` 的模型请求前上下文入口
  - `macaca/crates/macaca-runtime` 的模型请求前上下文入口
  - 后续 `macaca-memory`、`macaca-skill`、MCP runtime 的 provider adapter
- Dependencies:
  - 依赖既有 `add-pluggable-context-engine-foundation` 的 context engine/report 基础。
  - 是 `add-agent-profile-context-provider`、`add-active-vector-memory-context`、`add-skills-mcp-capability-context`、`add-context-governance-provider-runtime` 的前置提案。
- Compatibility:
  - 默认 composer 必须能包装 legacy 行为，不改变未启用 provider 时的模型请求语义。
  - 已存在的 legacy prompt/context 入口保留并逐步标记 deprecated，不删除。
