# Change: 增加主动向量记忆上下文 Provider

## Why

Macaca 的长期记忆必须是主动的，而不是只能由用户或 agent 通过关键词工具被动触发。已有 memory facade、scope、provider runtime、vector backend topology 和 active recall 基础，需要进一步接入 context composer：在模型调用 preflight 阶段按 session、agent、application 和 policy 主动召回 `AgentPrivate` 与 `SessionShared` 记忆，并以 dynamic/request-only context 注入。

本提案聚焦“记忆如何进入上下文工程”，而不是重新定义底层记忆系统或向量数据库实现。

## What Changes

- 新增 `MemoryActiveRecallContextProvider` 或等价 adapter，将 memory active recall 结果转换为 `ContextCandidate`。
- 召回主键语义遵循 session id；application id 和 agent name 作为路由/副键参与 scope 和 provider topology。
- 默认召回当前 agent 的 `AgentPrivate` 和当前 session 的 `SessionShared`。
- 保持向量拓扑抽象：`application -> database`、`agent -> collection` 是 provider-neutral 语义，不在 context provider 中硬编码 Milvus。
- 召回结果必须经过 governance、tombstone、redaction、scope policy 和 budget。
- recall context 默认 dynamic、request-only、fenced，不写回 canonical transcript。
- recall diagnostics 进入 `ContextReport`，默认不持久化完整 memory content。

## Impact

- Affected specs: `active-vector-memory-context`
- Affected code:
  - `macaca/crates/macaca-context`
  - `macaca/crates/macaca-memory`
  - framework/runtime context facade integration
- Dependencies:
  - 依赖 `add-context-composer-foundation`。
  - 依赖既有 memory fabric/vector topology/provider runtime/active recall/governance 提案。
- Compatibility:
  - 未启用 active recall provider 时不改变 prompt。
  - 显式 memory tools 继续可用，但不作为主动召回的唯一入口。
