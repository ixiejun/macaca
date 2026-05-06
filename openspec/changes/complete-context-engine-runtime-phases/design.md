# Design: 上下文工程 Phase 0-5 运行时闭环

## Context

当前实现分为两层：

- Contract layer：`macaca-context` 已有 `ContextEngine`、`ContextReport`、`PromptComposer`、`PruningContextEngine`、compaction envelope、memory/wiki source provider、preflight recall config、external adapter safety contracts。
- Runtime layer：framework/runtime/web 仍主要通过 `ContextManagerFacade::legacy()` 运行；真实 prompt 构造仍在 `framework_runner.rs` 字符串拼接；runtime direct loop 只打 debug context report；compaction、memory recall、engine selection 尚未形成运行时行为。

本设计的核心原则是：**只有进入真实模型调用路径、可配置、可观测、可回退、可测试，才算 Phase 已实现。**

## Goals / Non-Goals

Goals:

- 将报告 Phase 0-5 全部落到运行时验收。
- 把 contract 完成状态和 runtime 完成状态拆成不同任务、不同测试。
- 让 framework/runtime 共享同一 context engine selection/fallback 机制。
- 将 persona/application/workspace/skill/tool schema/history/trace/memory 作为 typed context sources 进入 `PromptComposer`。
- 保持 Macaca OS crate 通用，不硬编码 app、workflow、agent 或业务名称。

Non-Goals:

- 不在本变更中设计远程 RPC/WASM adapter wire protocol。
- 不默认启用 memory recall 或 compaction 的激进行为。
- 不删除 legacy prompt/context 入口；迁移完成后只标记 deprecated 并保留兼容层。
- 不把 context policy 放进 `macaca-llm` provider crate。

## Runtime Acceptance Model

每个 Phase 同时需要四类证据：

- Contract：trait/value object/policy 已存在。
- Runtime：framework/runtime/web 的真实执行路径使用它。
- Diagnostics：EventLog/API/UI 能解释行为。
- Verification：单元、集成或 E2E 测试覆盖。

只有四类证据都满足，Phase 才能在任务清单里标为完成。

## Decisions

### Decision 1: 用 ContextRuntimeFacade 统一 framework/runtime 调用

新增一个 runtime-facing facade，封装：

- engine selection
- source collection
- prompt composition
- pruning/recall/compaction policies
- fallback
- report persistence hooks

设计模式：

- Facade：上层只调用一个稳定入口。
- Strategy：engine、policy、source provider 可替换。
- Factory/Registry：从 config/manifest/profile 解析 engine 和 policy。

理由：

- 直接在 `framework_runner.rs` 和 `agentic_loop.rs` 分别拼逻辑会再次分叉。
- 统一 facade 可以确保每次模型调用都有 report、fallback 和安全边界。

### Decision 2: Engine selection 必须配置驱动

选择优先级：

1. agent profile / manifest override
2. application manifest context config
3. system config default
4. built-in `legacy`

内置 engine：

- `legacy`：保持当前 messages/options 兼容。
- `windowed`：复用/迁移现有 token window 裁剪能力。
- `pruning`：对 tool result/trace/file/read/search 大输出做 bounded rendering。
- `summary`：在 window/pruning 基础上支持 compaction summary 和 lineage。

失败策略：

- selected engine 失败时根据 fallback policy 回退到 `legacy` 或 empty optional contribution。
- fallback 必须记录 `context_report` decision 和 `context_engine_fallback` EventLog 事件。

### Decision 3: PromptComposer 迁移真实 prompt source

真实 source 映射：

- Stable trusted：core OS rules、persona high-priority sections、application stable policy、tool/schema stable metadata。
- Stable or dynamic by source metadata：workspace guide files、skill index、capability list。
- Dynamic untrusted：trace snippets、tool results、memory recall、external adapter output、session metadata、time/runtime state。

OpenClaw-style workspace guide files 应作为 source provider，而不是 hardcoded prompt string：

- `AGENTS.md`
- `SOUL.md`
- `TOOLS.md`
- `IDENTITY.md`
- `USER.md`
- `HEARTBEAT.md`

这些文件的加载顺序、priority、token budget 和 truncation 必须进入 `ContextReport`，并且可由 application/profile 配置替换或关闭。

### Decision 4: Phase 2 pruning 先接入 selected engine，不改 canonical 数据

Pruning 运行时行为：

- `legacy` 默认不裁剪，只 report。
- `pruning`/`summary` engine 负责模型上下文裁剪。
- 原始 tool result、trace event、file read、stdout 保留在 EventLog/artifact/session store。
- 模型上下文只带 summary/excerpt/source ref。

验收重点：

- 大 stdout/file read 不完整进入 LLM payload。
- ContextReport 解释 render mode、pruned tokens、source ref。
- UI 可通过 authorized debug path 查看原文引用。

### Decision 5: Phase 3 compaction 是运行时流程，不只是 envelope

Compaction flow：

1. engine/policy 检测 budget threshold 或 manual compact request。
2. 触发 `before_compaction` hooks，让 memory/source provider 做 bounded flush。
3. 生成 strict reference-only summary。
4. 创建 successor transcript segment 或 child session。
5. 更新 lineage tip。
6. 后续 resume/logical session 默认进入 tip。
7. EventLog 写入 `context_compaction` 和 `context_lineage_updated`。

UI/API：

- report panel 显示 compaction count、summary source、successor id。
- debug view 可展开 root-to-tip lineage。

### Decision 6: Phase 4 recall 必须工具化、预算化、request-only

Recall runtime 行为：

- 提供只读 `memory_search` / `memory_get` / `wiki_digest_get` 类工具或 source provider entry。
- preflight recall 默认关闭，只有 config/profile opt-in。
- recall 输出带 provenance、confidence、privacy tier。
- recall 只进入 dynamic/untrusted/request-only section。
- recall 不写回 canonical transcript。
- ContextReport 显示 memory/wiki tokens、source ids、privacy tier 摘要和 warning。

### Decision 7: Phase 0 report 全覆盖必须以持久化为准

所有模型调用入口必须在请求前或请求组装时产生持久化 `context_report`：

- framework ReAct path
- runtime direct agentic loop
- simple/declarative agent calls
- future SDK/agent calls that invoke LLM provider through Macaca runtime

仅 debug log 不算 Phase 0 完成。

## Migration Plan

1. 修正 `add-context-engine-policy-phases/tasks.md` 语义，标明 contract 完成但 runtime 未完成的项。
2. 引入 runtime facade 和 config schema，默认使用 `legacy`。
3. 迁移 framework report wrapper，使它调用 selected engine 而不是 hardcoded `legacy`。
4. 迁移 runtime direct loop，使 context report 持久化到 EventLog。
5. 迁移 prompt source 到 `PromptComposer` typed sections，并添加 stable/dynamic hash tests。
6. 接入 `pruning` engine 到真实模型请求路径。
7. 增加 compaction trigger/manual API/lineage tip/resume/UI。
8. 增加 memory/wiki source provider、只读 recall tools、preflight recall opt-in。
9. 增加 `windowed`/`summary` engines、fallback event 和 config/manifest/profile selection。
10. 完成 E2E/集成测试后更新 tasks，只有 runtime 验收通过的 Phase 才标为完成。

## Closure Addendum: Phase 6-10

`complete-context-engine-runtime-phases` now also records the closure evidence that was implemented
under `complete-context-engine-all-phases` so the two OpenSpec changes do not contradict each other.
The closure keeps the original design-pattern boundaries:

- Phase 6 uses a Repository/Adapter boundary (`ContextSourceArtifactRepository`) for pruned source
  retrieval. Context engines and UI code do not construct raw storage keys.
- Phase 7 uses Memento-style compaction snapshots plus a lineage facade so normal UI keeps one
  logical session while diagnostics can expand root-to-tip state.
- Phase 8 uses Chain of Responsibility source providers for memory/wiki/digest recall. Recall rows
  are dynamic, untrusted, request-only, bounded by policy, and diagnostics persist metadata rather
  than canonical transcript messages.
- Phase 9 uses Ports and Adapters / Bridge for custom in-process engines and external context
  adapters. External output is schema-validated, bounded, trust-fenced, and fail-open/fallback.
- Phase 10 keeps legacy prompt/context entry points searchable as deprecated compatibility shims,
  while production context assembly enters through facade/composer/runtime selection.

## Risks / Trade-offs

- Risk: prompt migration 改变模型行为。
  Mitigation: legacy 默认保持 byte/semantic equivalent；PromptComposer 先 snapshot 测试，再分阶段替换。

- Risk: pruning 丢失关键信息。
  Mitigation: 默认阈值保守；source ref 可回查；ContextReport 记录所有裁剪。

- Risk: compaction 摘要被当成新指令。
  Mitigation: strict reference-only envelope；summary 进入 untrusted dynamic section。

- Risk: engine selection 配置复杂化。
  Mitigation: 单一解析入口，优先级清晰，默认 legacy，fallback 可观测。

- Risk: memory recall 引入 prompt injection 或隐私泄漏。
  Mitigation: read-only allowlist、privacy tier、untrusted fence、request-only injection、debug redaction。
