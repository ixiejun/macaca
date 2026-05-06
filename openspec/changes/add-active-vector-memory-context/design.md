# Design: Active Vector Memory Context Provider

## Context

Macaca 已实现记忆系统的 scope、facade、provider runtime、向量拓扑和治理层。系统要求一个 application 对应一个向量数据库语义，每个 agent 在该 application 数据库下拥有自己的 collection 语义，同时 session shared memory 可被 session 内 agent 共享。具体默认实现可以是 Milvus，但 context 层不能出现 vendor hardcode。

## Goals / Non-Goals

Goals:

- 在模型调用前主动召回相关长期记忆。
- 默认使用 session id 作为上下文召回主键语义，application id 和 agent name 作为路由/副键。
- 让 active recall 以 context provider 形式接入 composer。
- 保证 scope 隔离、治理过滤、预算、超时和诊断。
- 允许用户替换 memory system 或 recall strategy。

Non-Goals:

- 不实现新的 vector database backend。
- 不在 context provider 中直接调用 Milvus SDK。
- 不把所有记忆塞入 prompt。
- 不把 recall 结果写入 transcript。
- 不实现复杂 LLM sub-agent recall。

## Decisions

### Decision 1: Context provider 只依赖 ActiveRecallCapability

`MemoryActiveRecallContextProvider` 依赖窄接口，例如 `ActiveRecallCapability` 或 memory facade adapter。它不依赖具体 vector provider、collection 名称或 vendor client。

### Decision 2: Scope routing 使用 session primary key 语义

召回请求必须包含：

- `session_id`：当前会话主键语义。
- `application_id`：选择 application 级 memory namespace/database 语义。
- `agent_name`：选择 agent private collection 语义。
- `memory_scope`：`AgentPrivate`、`SessionShared` 等。

这不是关系数据库主键设计，而是记忆路由契约。

### Decision 3: Recall output 是 dynamic fenced context

召回内容默认：

- dynamic cache class。
- request-only。
- fenced memory context。
- 不作为 system instruction。
- 不写回 canonical transcript。

### Decision 4: Governance 通过 Decorator 包裹 recall

tombstone、redaction、promotion state、audit、visibility policy 应作为 recall capability 的 decorator 或 policy pipeline，context provider 不绕过治理层。

## Risks / Trade-offs

- Risk: 每次模型调用增加向量搜索延迟。Mitigation: latency budget、timeout、max hits、并行查询、fail-open。
- Risk: 召回噪声影响模型。Mitigation: conservative default、rerank/score threshold、small budget、report 可诊断。
- Risk: private memory 泄漏给其他 agent。Mitigation: scope routing 和 policy enforcement。
- Risk: context 层耦合 Milvus。Mitigation: provider-neutral topology contract，只依赖 facade/capability。

## Migration Plan

1. 在 context composer 之后新增 memory recall provider adapter。
2. 将 active recall diagnostics 映射到 context report。
3. 默认配置保守启用或按应用/agent policy 启用。
4. 保留显式 memory tool；将被替代的直接 prompt memory injection 标记 deprecated。
