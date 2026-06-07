# Change: 增加 Knowledge Digest Context Provider

**Status: Approved for implementation (2026-05-06).**

## Why

Macaca 的长期记忆治理层已经能够将候选、原始记忆和审计事件编译为 claim、evidence、freshness、conflict 和 artifacts。上下文工程不能只依赖原始向量召回：当存在经过治理的知识摘要时，模型应该优先看到更稳定、更高质量、可追溯的 digest/claim context，同时保留 freshness、confidence、tombstone、redaction 和 evidence 边界。

本提案补齐 Phase 6：新增 `KnowledgeDigestContextProvider`，把 memory governance 编译出的 digest/claim/artifact 适配为 context candidates，并定义 digest-vs-raw recall 的选择策略。

## What Changes

- 新增 `KnowledgeDigestContextProvider`，消费 memory governance/knowledge layer 暴露的 compiled digest、claims 或 artifacts。
- 定义 `KnowledgeDigestCandidate` 到通用 `ContextCandidate` 的转换。
- 默认优先选择有 evidence、freshness 和 confidence 支撑的 digest/claim，而不是重复注入同源 raw recall。
- 定义 digest-vs-raw selection strategy，支持 freshness、confidence、evidence coverage、scope 和 budget。
- evidence 只报告 source ids、hashes、artifact ids 或 redacted excerpts，默认不泄漏完整敏感原文。
- tombstone、delete propagation、redaction、audit policy 必须继续生效。
- stale digest 不得无条件压过 fresh recall；必须通过 freshness/confidence 策略裁决。
- 所有 selected/skipped/stale/redacted/tombstoned decisions 进入 `ContextReport`。

## Impact

- Affected specs: `knowledge-digest-context`
- Affected code:
  - `macaca/crates/macaca-context`
  - `macaca/crates/macaca-memory` governance/knowledge context adapter
  - framework/runtime 通过 context facade 间接受影响
- Dependencies:
  - 依赖 `add-context-composer-foundation`。
  - 依赖既有 `add-memory-governance-knowledge-layer`。
  - 与 `add-active-vector-memory-context` 协同，用于 digest-vs-raw 选择。
- Compatibility:
  - 未启用 knowledge digest provider 时不改变现有 active recall 行为。
  - 不删除 raw recall provider；digest 只是优先级更高的治理后上下文来源。
