## Context

OpenClaw 的 active-memory 表明运行时主动召回是上下文工程的重要组成部分。Macaca 已有 `macaca-context` 基础设施和 context report，需要 memory active recall 作为 context source 接入，而不是让记忆系统直接控制 prompt。

## Goals

- 在模型请求前主动召回相关记忆。
- 默认合并 agent private 和 session shared recall。
- 保证 token budget、latency budget 和 trust boundary。
- 生成可诊断 recall report。
- 允许替换 active recall provider/strategy。

## Non-Goals

- 不把所有记忆自动塞进 prompt。
- 不默认写回 canonical transcript。
- 不在本变更中实现复杂 LLM subagent recall。
- 不把 context engine 逻辑放进 `macaca-memory`。

## Decisions

### Decision 1: Active recall is a capability, not the context engine

`ActiveRecallCapability` 只返回 bounded recall candidates 和 diagnostics。`macaca-context` 决定如何将其作为 dynamic source 组装。

### Decision 2: Default recall reads private then shared

默认策略：

1. 查询当前 agent `AgentPrivate`。
2. 查询当前 session/project `SessionShared`。
3. 可选查询 `ApplicationShared` 和 `UserScoped`。
4. 可选查询 knowledge/supplement。
5. 按 relevance、freshness、visibility、budget 合并。

### Decision 3: Recall output is dynamic and fenced

Active recall output 默认：

- request-only。
- dynamic section。
- untrusted 或 memory-trusted，而非 system instruction。
- 不写入 session transcript。

### Decision 4: Report first, content bounded

Active recall 必须记录：

- source provider
- visibility/scope
- score
- snippet length/tokens
- selected/skipped decision
- latency
- fallback/error

默认不持久化完整 memory content，除非 explicit debug。

## Risks / Trade-offs

- Risk: active recall 增加 latency。
  - Mitigation: latency budget、parallel search、timeout/fallback。
- Risk: recall 内容污染 system prompt。
  - Mitigation: dynamic/untrusted fencing，由 context engine 统一渲染。
- Risk: private memory 泄漏到 shared context。
  - Mitigation: report visibility，route policy，no automatic promotion。

## Migration Plan

1. 定义 active recall DTO/trait/policy。
2. 实现 default policy。
3. 接入 MemoryFacade prefetch。
4. 接入 `macaca-context` source/report。
5. 增加 tests。
