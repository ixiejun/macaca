# Design: 上下文工程后续策略阶段

## Context

当前 `macaca-context` 已提供第一阶段基础：

- `ContextEngine` / `LegacyContextEngine` 作为可替换策略入口。
- `ContextManagerFacade` 作为上层门面。
- `PromptComposer` 用 typed sections 表达 stable/dynamic 和 trusted/untrusted。
- `ContextReport` 记录 request 级上下文来源、预算、hash 和诊断。

研究报告显示，剩余关键能力不应一次性做成“大一统智能上下文引擎”，而应作为可插拔策略阶段逐步接入：

- Pruning：先低风险地处理工具输出、trace event 和大文件输出膨胀。
- Compaction：只在长会话逼近预算时生成摘要，并保留 lineage。
- Memory/wiki：作为 context source provider，而不是 context engine。
- Preflight recall：opt-in、只读、bounded、request-only。
- External adapter：等 in-process trait 稳定后再开放给用户上下文系统。

## Superpowers Brainstorm

### Option A: 先做 non-destructive pruning

设计模式：

- Chain of Responsibility：不同 source renderer 依次处理 tool result、trace event、file read、command output、search result。
- Policy/Strategy：`PruningPolicy` 和 `BudgetPolicy` 可替换。
- Memento/Event Sourcing：原始 event/transcript 不改写，只派生模型可见 snippet。

收益：

- 直接降低 token 膨胀。
- 不改 canonical 数据，回归风险低。
- 所有裁剪决策能进入 `ContextReport`。

风险：

- 过度裁剪会让模型丢失任务细节。
- source renderer 如果混入业务语义，容易破坏 OS 通用性。

控制：

- 默认只裁剪超过阈值的大输出。
- 只生成摘要、bounded excerpt 和 artifact/event reference。
- 禁止按 app/workflow/agent 名称选择特殊裁剪逻辑。

### Option B: 先做 compaction 和 session lineage

设计模式：

- Strategy：不同 compaction engine 可替换。
- Template Method：压缩流程固定为 preflush、summarize、persist successor、report。
- Memento/Event Sourcing：保留原始 transcript segment，创建 successor segment/session。

收益：

- 支撑 7x24 长会话。
- 压缩后仍可审计和恢复。

风险：

- 摘要错误会导致任务丢失或重复执行旧请求。
- session lineage 影响 UI、resume、event store 和 delegate/fork 语义。

控制：

- 摘要使用严格 envelope，明确 reference-only、not instruction。
- 摘要必须保留 task IDs、file paths、decisions、open questions、active task。
- 不原地删除或覆盖原始历史。

### Option C: memory/wiki source provider 优先

设计模式：

- Strategy：不同 memory recall provider 可替换。
- Repository：durable memory 与 wiki/digest 各自维护存储。
- Adapter：外部 memory 系统只通过 source provider 接入。

收益：

- 长期知识进入上下文前可带 provenance、confidence、privacy tier。
- recall 和 curated wiki 不混成一个表或默认大 prompt。

风险：

- recall 被模型误认为系统指令。
- memory provider 生命周期与 session branch/compaction 同步错误。

控制：

- memory recall 默认关闭或显式触发。
- recall 内容只进入 dynamic/untrusted/request-only section。
- `before_compaction`、`after_turn`、`session_switched` hooks 只能通过接口访问上下文摘要。

### Option D: 外部 context manager adapter 优先

设计模式：

- Ports and Adapters：Macaca 定义端口，用户系统实现 adapter。
- Bridge：runtime/framework 与 context backend 解耦。
- Anti-Corruption Layer：外部输出必须转换为 Macaca 内部安全模型。
- Abstract Factory：按配置构造 in-process、process、RPC 或 WASM adapter。

收益：

- 最强的用户替换能力。
- 企业或专业场景可接入自有知识库和上下文管理系统。

风险：

- 远程协议过早冻结会锁死错误抽象。
- 安全、超时、预算和 prompt injection 边界扩大。

控制：

- 先发布 in-process conformance tests。
- 外部 adapter 后置，必须有 schema validation、timeout、payload limit、circuit breaker 和 fallback。

### Recommendation

采用组合路线：

1. 先做 non-destructive pruning，因为它收益高且不改写 canonical history。
2. 再做 compaction/session lineage，因为它涉及持久化、UI、resume 和审计，需要建立在 report/pruning 可解释性之后。
3. 再做 memory/wiki source 和 opt-in preflight recall，保持 memory 是 source，不是 engine。
4. 最后开放外部 adapter，避免提前冻结远程协议。

## Goals / Non-Goals

Goals:

- 让每个上下文阶段都是可替换策略或 source provider。
- 保证 pruning 和 compaction 都可通过 `ContextReport` 解释。
- 保证 canonical transcript/event store 不因裁剪而丢失原文。
- 保证 compaction 保留 lineage，UI 可展示逻辑会话，debug 可追溯原始段。
- 保证 memory/wiki/外部 context 都是 untrusted dynamic source，默认不污染 stable prompt。
- 提供用户替换上下文系统的 adapter 路径和 conformance tests。

Non-Goals:

- 不复制 OpenClaw 的 TS 插件系统复杂度。
- 不复制 Hermes 的单体 agent loop。
- 不默认启用 preflight recall 或 dreaming。
- 不在本提案中定义稳定的远程 RPC/WASM wire protocol 字段全集。
- 不把业务语义、agent 名称、workflow 名称写入 OS crate。

## Decisions

### Decision 1: Pruning 使用 renderable source + policy

新增或扩展概念：

- `ContextRenderable`：把 source 渲染为模型可见 snippet。
- `ContextSnippet`：包含 text、estimated tokens、source id、trust level、artifact reference。
- `PruningPolicy`：决定何时保留全文、摘要、excerpt 或 drop。
- `BudgetPolicy`：按 token budget 分配 source 类别预算。

设计模式：

- Source renderer 使用 Chain of Responsibility 或 Strategy。
- Pruning/Budget 使用 Policy/Strategy。
- 原始数据保留使用 Memento/Event Sourcing 思路。

关键规则：

- Pruning 不得修改 canonical transcript、event store、artifact store。
- 每个裁剪动作必须写入 `ContextReport.decisions`。
- snippet 必须能引用原文 source id 或 artifact id。
- 默认策略只基于 source kind、大小、预算和配置，不基于应用名。

### Decision 2: Compaction 使用 strict envelope + successor lineage

新增或扩展概念：

- `CompactionPolicy`
- `CompactionSummaryEnvelope`
- `SessionLineage`
- `TranscriptSegment`
- `before_compaction` / `after_compaction` lifecycle hooks

摘要 envelope 必须表达：

- `source="compaction"`
- `trusted="false"`
- `instruction_priority="reference_only"`
- `root_session_id`
- `source_segment_id`
- `successor_segment_id`
- fixed sections: `Resolved`、`Decisions`、`Current State`、`Open Questions`、`Active Task`、`Important IDs/Paths`

设计模式：

- Compaction engine 使用 Strategy。
- Compaction flow 使用 Template Method。
- 原始段与 successor 段使用 Memento/Event Sourcing。

关键规则：

- 压缩摘要不得作为新用户指令处理。
- 压缩前必须允许 memory/source provider 执行 bounded flush hook。
- 压缩后创建 successor segment/session，不覆盖原始段。
- logical session 查询默认投影到 tip，debug 查询可展开 lineage。

### Decision 3: Memory/wiki 是 source provider，不是 context engine

新增或扩展概念：

- `MemorySourceProvider`
- `WikiDigestSourceProvider`
- `MemoryRecallPolicy`
- `ContextSourceProvenance`
- `PrivacyTier`
- `ConfidenceScore`

设计模式：

- Recall provider 使用 Strategy。
- Durable memory/wiki store 使用 Repository。
- 外部 memory 系统使用 Adapter。

关键规则：

- memory/wiki 不得默认全量进入 prompt。
- recall 必须带 provenance、confidence、privacy tier。
- recall 进入 dynamic/untrusted/request-only section。
- Memory provider lifecycle hooks 只能接收 bounded context view，不能直接读写 Macaca 内部表。

### Decision 4: Preflight recall 默认关闭

新增或扩展概念：

- `ContextPreflightRecall`
- `RecallToolAllowlist`
- `RecallTimeoutPolicy`

设计模式：

- Preflight recall 是 optional pipeline step。
- Recall tool selection 使用 Policy。

关键规则：

- 只允许 read-only recall/search/get 类工具。
- 默认关闭，只能由 config/application manifest/agent profile opt-in。
- 超时或失败必须降级为空 recall，并记录 warning。
- 结果必须短摘要、untrusted、request-only，不写回 canonical transcript。

### Decision 5: 外部上下文系统通过 adapter 后置接入

新增或扩展概念：

- `ExternalContextAdapter`
- `ContextEngineConformanceSuite`
- `ContextAdapterSafetyPolicy`
- `ContextFallbackPolicy`

设计模式：

- Adapter/Bridge 解耦 Macaca runtime 和用户 context backend。
- Anti-Corruption Layer 验证外部输出。
- Abstract Factory 按配置创建 adapter family。

关键规则：

- in-process trait 和 conformance tests 是 source of truth。
- 外部 adapter 输出必须转换成 `CompiledPrompt` / `ContextReport` / `ContextSnippet` 等内部模型。
- 外部输出必须通过 schema validation、budget validation、trust boundary validation。
- 外部系统失败不得绕过 fallback policy。

## Write Plan

### Phase 1: Policy 和 renderer 契约

- 扩展 `macaca-context`，加入 source rendering、snippet、pruning policy、budget policy 的窄接口。
- 为 tool result、trace event、file read、command output、search result 定义 source kind 和 renderer contract。
- 添加单元测试验证默认 policy 不基于 app/workflow/agent 名称。

### Phase 2: Non-destructive pruning

- 在 context engine/facade 下接入 renderer chain。
- 大输出渲染为 summary、bounded excerpt 和 source reference。
- 原文保留在 event/artifact store。
- `ContextReport` 记录 included、summarized、dropped、pruned tokens。

### Phase 3: Compaction 和 session lineage

- 定义 compaction summary envelope。
- 增加 `CompactionPolicy` 和 lifecycle hooks。
- 增加 successor transcript segment 或 child session lineage。
- 更新 resume/logical session 查询语义。

### Phase 4: Memory/wiki source

- 将 durable memory 和 wiki/digest 作为 source provider。
- recall 返回 provenance、confidence、privacy tier。
- recall 仅进入 dynamic/untrusted/request-only section。
- compaction 前触发 bounded memory flush hook。

### Phase 5: Opt-in preflight recall

- 增加只读 recall pipeline step。
- 支持 tool allowlist、timeout、max chars/tokens。
- 失败时降级为空 recall，并进入 report warning。

### Phase 6: User adapter path

- 发布 context engine/source/policy conformance tests。
- 支持本地 in-process custom provider 注册。
- 后续再引入 process/RPC/WASM adapter，并强制 safety policy。

## Risks / Trade-offs

- Risk: pruning 导致模型丢失关键细节。
  Mitigation: 默认阈值保守；保留 bounded excerpt 和原文 reference；每个裁剪决策进入 report。

- Risk: compaction 摘要诱导模型执行旧请求。
  Mitigation: fixed envelope 标明 reference-only/untrusted；摘要结构强制包含 active task 和 open questions。

- Risk: lineage 改动影响 resume 和 UI。
  Mitigation: logical session 默认投影到 tip；原始 segment 只在 debug/审计路径展开。

- Risk: memory recall 引入 prompt injection。
  Mitigation: recall untrusted fenced；不进入 stable prefix；不作为 system instruction。

- Risk: 外部 context manager 破坏预算或稳定性。
  Mitigation: adapter safety policy、timeout、payload limit、schema validation、fallback 和 circuit breaker。

- Risk: 接口一次性膨胀。
  Mitigation: 每个 Phase 单独落地；trait 只在需要时扩展；外部 adapter 放在最后。

## Open Questions

- Compaction lineage 首轮应建模为 successor transcript segment，还是复用 session parent/child 字段？实现前需要审计现有 session/event store。
- 原文 artifact reference 是复用现有 event id，还是新增 artifact id？实现前需要看 event payload 大小和 UI 拉取路径。
- 外部 adapter 首轮是否只支持 in-process Rust provider？本设计倾向是，远程协议等至少两个真实 provider 需求出现后再冻结。
