# Change: 完成上下文工程后续策略阶段

## Why

`add-pluggable-context-engine-foundation` 已建立 `macaca-context`、`ContextEngine`、`PromptComposer` 和 `ContextReport` 的基础边界，但研究报告中更能支撑 7x24 长期运行的后续 Phase 尚未进入规范：非破坏性 pruning、compaction/session lineage、memory/wiki 分层、bounded recall、以及用户可替换的外部上下文系统适配。

本变更将这些后续阶段纳入一个分阶段提案，目标是在不强耦合 Macaca Agent OS 与某个上下文管理实现的前提下，让默认实现可观测、可回退，并让用户能够通过 engine、source provider、policy 或 adapter 替换自己的上下文管理系统。

## What Changes

- 扩展 `context-engine` 能力规范，覆盖报告中尚未实现的 Phase 2-5。
- 增加 `ContextRenderable` / source renderer 与 `PruningPolicy`，对 tool result、trace event、file read、command output 等大上下文做非破坏性裁剪。
- 增加 `CompactionPolicy`、严格 compaction summary envelope 和 session lineage 要求，确保长会话压缩后仍可审计、可恢复。
- 增加 memory recall 与 wiki/digest 分层要求，把 memory 作为可选、预算受限、带 provenance 的 context source，而不是默认全量注入。
- 增加 opt-in `ContextPreflightRecall` 要求，只允许只读 recall 工具、超时降级、request-only untrusted 注入。
- 增加外部 context manager adapter 路径，要求 schema validation、budget validation、timeout、circuit breaker、fallback 和 conformance tests。
- 明确每个模块的设计模式选择：Strategy/Policy、Chain of Responsibility、Memento/Event Sourcing、Adapter/Bridge、Anti-Corruption Layer、Facade。

## Impact

- Affected specs: `context-engine`
- Affected code:
  - `macaca/crates/macaca-context`：后续承载 renderable、policy、compaction、lineage、adapter contract 和 conformance tests。
  - `macaca/crates/macaca-runtime` / `macaca-framework`：后续在模型请求路径调用 pruning/compaction/recall policy，不直接实现具体策略。
  - `macaca/crates/macaca-web`：后续展示 context report、pruning/compaction/lineage 诊断，不拥有上下文策略。
  - `macaca/crates/macaca-memory`：后续提供 memory/wiki source provider 与 lifecycle hooks，不成为 context engine 本身。
  - `macaca/crates/macaca-persist` / event/session store：后续保存原始事件、compaction lineage 和 report summary。
- Compatibility:
  - 默认 `legacy` 行为必须继续可用。
  - Pruning 不得改写 canonical transcript 或 event store。
  - Compaction 不得删除原始历史；UI 可展示逻辑会话，debug 可展开 lineage。
  - 外部上下文系统失败时必须按策略 fallback，不能使核心 agent loop 崩溃。
- Non-impact:
  - 不引入 application-specific、workflow-specific 或 agent-name-specific 逻辑。
  - 不默认启用 active memory/preflight recall。
  - 不把 context policy 放入 `macaca-llm`。
