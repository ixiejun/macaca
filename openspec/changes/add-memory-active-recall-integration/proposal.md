# Change: 增加 Memory Active Recall 与上下文工程集成

## Why

记忆不应只在 agent 显式调用 `memory_search` 工具时才可用。Macaca 需要在模型调用前基于当前 user message、recent turns、agent role、session/project 目标和 token/latency budget 主动召回相关记忆，并将召回结果以可解释、可审计、可降级的方式提供给上下文工程。

本变更建立 active recall pipeline，使 `AgentPrivate`、`SessionShared`、`ApplicationShared`、`UserScoped` 和 knowledge/supplement 来源可以按策略合并进入 context report 和动态上下文。

## What Changes

- 在 `macaca-memory` 单 crate 内增加 active recall capability 和 pipeline。
- 定义 `MemoryPrefetchRequest`、`MemoryPrefetchResult`、`ActiveRecallPolicy`、`RecallCandidate`、`RecallDecision`。
- 默认召回策略查询 agent private 和 session shared memory，并按 budget 合并。
- 与 `macaca-context` 集成，作为 dynamic/untrusted context source，不写回 canonical transcript。
- 在 `ContextReport` 或 memory diagnostics 中记录 recall 来源、scope、score、budget、latency、fallback。
- 支持替换 active recall provider/strategy。

## Impact

- Affected specs: `macaca-memory-active-recall`
- Affected code:
  - `macaca/crates/macaca-memory/src/core/`
  - `macaca/crates/macaca-memory/src/index/`
  - `macaca/crates/macaca-memory/src/providers/`
  - `macaca/crates/macaca-context` integration path
- Compatibility:
  - 未启用 active recall 时不改变现有 prompt。
  - active recall injection 默认 dynamic/request-only，不写回 transcript。
  - 不新增额外 crate。
