# Design: 可插拔上下文工程基础设施

## Context

Macaca 当前已经具备 runtime loop、framework ReActAgent、skill snapshot、memory facade、LLM provider router、session/trace event 等基础能力，但上下文工程控制面仍分散在多个 crate：

- `macaca-web` 的 agent factory 构建 system prompt，注入 persona、application prompt semantics、capabilities、workspace paths 和 skill snapshot。
- `macaca-framework` 的 `ReActAgent` 从 system prompt、working memory、tool definitions 构建模型请求。
- `macaca-runtime` 的 `AgenticLoop` 在请求前使用 `ContextWindowManager` 做窗口裁剪。
- `macaca-skill` 已有 progressive disclosure 形态，但 prompt 注入仍由上层拼接。
- `macaca-memory` 已有 memory facade，但长期记忆不应成为上下文工程本身。
- `macaca-llm` 面向 provider，不应吸收 Macaca-specific 上下文策略。

研究报告 `docs/context-engineering-openclaw-hermes-research.md` 结论表明：OpenClaw 的上下文生命周期接口、prompt cache 边界、context report、pruning/compaction 分离、memory/wiki 分层值得借鉴；Hermes 的压缩摘要模板、工具结果剪枝、session lineage、memory provider hooks、user-message 级动态注入值得局部借鉴。但 Macaca 不能照搬 OpenClaw 的 TS 插件复杂度，也不能复制 Hermes 的单体 agent loop。

本设计采用渐进路线：先定义契约、观察现状、保留 legacy 行为，再逐步迁移上下文来源和策略。

## Goals / Non-Goals

Goals:

- 为 Macaca 增加可插拔 `ContextEngine` 契约。
- 默认 `LegacyContextEngine` 包装现有行为，避免第一阶段改变模型输入语义。
- 为每次模型请求生成 `ContextReport`，回答“哪些上下文进入了请求、为什么、占多少预算”。
- 将 prompt 构建抽象为 stable/dynamic sections，避免动态数据破坏稳定前缀和 prompt cache。
- 让用户未来可替换整套上下文管理系统，或只替换 source provider、policy、adapter。
- 确保上下文策略对所有 application 通用，不硬编码 app/workflow/driver/业务名称。

Non-Goals:

- 不在本提案中实现 LLM-based compaction。
- 不在本提案中实现 active memory recall sub-agent。
- 不在本提案中设计或冻结外部 RPC/WASM context manager 协议。
- 不把 memory recall 作为默认上下文注入。
- 不删除 legacy prompt/context 接口。
- 不把上下文策略放入 `macaca-llm`。
- 不做大型 Web UI 信息架构重构。

## Decisions

### Decision 1: 新增 focused `macaca-context` crate

采用新的 `macaca-context` crate 承载上下文工程的契约、值对象、默认 composer、默认 legacy engine 和通用报告模型。

理由：

- Context engineering 是跨 `framework`、`runtime`、`web`、`memory`、`skill`、`llm` 的 Agent OS 能力，不适合继续放在 Web 或 framework 局部。
- 独立 crate 可以明确依赖方向，避免 `macaca-web` 或 agent loop 变成上下文策略中心。
- 有利于后续发布用户可实现的 trait 和 conformance tests。

约束：

- 首轮 crate 内容必须窄：types、traits、`LegacyContextEngine`、`PromptComposer`、`ContextReport`。
- 不引入新的重型依赖。
- 每个文件保持小而清晰，遵守项目 500 行上限。

Alternatives considered:

- 放在 `macaca-framework`：贴近 `ReActAgent`，但无法自然覆盖 `macaca-runtime` 直接 loop，且容易让 framework 承担过多基础设施职责。
- 放在 `macaca-runtime`：贴近 7x24 执行，但会让 framework prompt semantics 和 skill source 反向耦合 runtime。
- 放在 `macaca-llm`：不采纳。LLM crate 应保持 provider-facing，不应知道 Macaca application/session/skill/memory 语义。

### Decision 2: 使用 Strategy + Facade + Factory/Registry 的插件边界

核心抽象：

- `ContextEngine`：上下文组装策略。
- `ContextEngineProvider`：按配置创建 engine 的工厂。
- `ContextEngineRegistry`：注册和选择 engine provider。
- `ContextManagerFacade`：上层调用的稳定门面。

设计模式：

- `ContextEngine` 使用 Strategy。
- `ContextEngineProvider` 使用 Factory Method。
- 多种 provider family 使用 Abstract Factory。
- engine lookup 使用 Registry。
- `ContextManagerFacade` 使用 Facade。

理由：

- 上层只依赖抽象，用户可以替换具体实现。
- engine 选择发生在 composition root 或 manifest/config 解析层，不发生在业务分支中。
- 保留默认 `legacy` 作为安全回退。

### Decision 3: 第一阶段 trait 保持窄接口

第一阶段只定义最小必要生命周期：

```rust
#[async_trait::async_trait]
pub trait ContextEngine: Send + Sync {
    fn info(&self) -> ContextEngineInfo;

    async fn assemble(
        &self,
        input: ContextAssembleInput,
    ) -> MacacaResult<ContextAssembleResult>;

    async fn after_turn(
        &self,
        input: ContextAfterTurnInput,
    ) -> MacacaResult<()>;
}
```

暂不加入：

- `compact`
- `prepare_child`
- `child_finished`
- `maintain`
- 外部协议生命周期

理由：

- 避免把 OpenClaw 的完整生命周期一次性复制进 Rust crate。
- 首轮需要证明 facade、report、legacy wrapping 和 prompt composer 是否足够。
- 后续 compaction/delegate lineage 可以通过新的 OpenSpec proposal 增量加入。

### Decision 4: PromptComposer 使用 Builder + Composite

`PromptComposer` 不再把 prompt 当成单个字符串拼接，而是构造有类型的 sections：

- `id`
- `source_kind`
- `stability`
- `trust_level`
- `content`
- `metadata`

设计模式：

- Builder：分步骤构建 `CompiledPrompt`。
- Composite：workspace、skill、capability、memory、trace、history 等 source group 可组合为 section tree。
- Value Object：`PromptSection`、`PromptStability`、`TrustLevel`、`ContextSourceId` 表达不可变语义。

关键规则：

- stable sections 渲染在 stable prefix。
- dynamic sections 渲染在 explicit boundary 之后。
- unknown 或 request-specific 数据默认归类为 dynamic。
- dynamic injection 不写回 session transcript。
- tool、skill、capability、workspace 等列表必须确定性排序。

### Decision 5: ContextReport 采用 Observer/Audit Log 思路

`ContextReport` 是 request-scoped 的可观测摘要，默认存储：

- request/session/app/agent/model/engine id。
- token budget 和估算 token。
- stable/dynamic prompt tokens。
- history/tool schema/skill/memory/trace/source tokens。
- source breakdown。
- prompt hash 和 stable hash。
- pruning/fallback/warning decisions。

默认不存储完整 prompt。完整 prompt 仅允许在显式 debug 配置下采集，并且必须标记敏感数据风险。

理由：

- 诊断应回答上下文预算和来源问题，不应默认泄漏完整 prompt。
- 未来 pruning/compaction 需要可解释性，否则很难定位行为回归。

### Decision 6: memory、skill、trace、tool schema 都只是 ContextSource

本设计不允许 memory 或 skill 拥有上下文工程控制权：

- `macaca-memory` 提供 memory source provider 或 recall provider。
- `macaca-skill` 提供 skill index/source provider。
- trace event 和 tool output 通过 renderable source 进入 context view。
- tool schema 作为 provider-facing source 单独统计。

理由：

- memory 是上下文来源之一，不是上下文引擎本身。
- skill progressive disclosure 应继续保持按需读取，不默认全量注入。
- tool schema 成本必须可观测。

### Decision 7: 外部上下文系统通过 Adapter/Bridge 后续接入

本提案只定义本地 in-process trait 作为 source of truth。未来如果用户需要外部上下文管理系统，可通过 adapter 实现同一契约。

边界要求：

- Macaca core 只信任 schema validated 的 `CompiledPrompt`/`ContextReport`。
- 外部输出默认 untrusted。
- 必须有 timeout、payload size limit、budget validation、fallback policy。
- 不能绕过 stable/dynamic/trust boundary。

设计模式：

- Ports and Adapters：Macaca 定义端口，用户系统实现适配器。
- Bridge：Macaca runtime 与 context backend 解耦。
- Anti-Corruption Layer：把外部输出转换成 Macaca 内部安全模型。

## Data Model Sketch

这些结构是设计草案，最终字段以实现和测试为准。

```rust
pub struct ContextAssembleInput {
    pub app_id: ApplicationId,
    pub session_id: SessionId,
    pub agent_name: String,
    pub model: String,
    pub base_messages: Vec<LlmMessage>,
    pub tools: Vec<ToolDefinition>,
    pub budget: ContextBudget,
    pub request_metadata: ContextRequestMetadata,
}
```

```rust
pub struct ContextAssembleResult {
    pub messages: Vec<LlmMessage>,
    pub options_patch: ContextOptionsPatch,
    pub report: ContextReport,
}
```

```rust
pub struct ContextReport {
    pub request_id: String,
    pub app_id: ApplicationId,
    pub session_id: SessionId,
    pub agent_name: String,
    pub engine_id: String,
    pub model: String,
    pub estimated_total_tokens: u32,
    pub token_budget: u32,
    pub stable_prompt_tokens: u32,
    pub dynamic_prompt_tokens: u32,
    pub history_tokens: u32,
    pub tool_schema_tokens: u32,
    pub skill_tokens: u32,
    pub memory_tokens: u32,
    pub trace_tokens: u32,
    pub pruned_tokens: u32,
    pub stable_prompt_hash: String,
    pub prompt_hash: String,
    pub sources: Vec<ContextSourceReport>,
    pub decisions: Vec<ContextDecisionReport>,
}
```

## Migration Plan

1. 创建 OpenSpec 并获得批准。
2. 使用 GitNexus 对拟修改符号做 upstream impact analysis。
3. 新增 `macaca-context` 最小 crate 和测试。
4. 实现 `LegacyContextEngine`，保证输入 messages/options 与现有行为兼容。
5. 在 framework/runtime 的模型请求路径旁路接入 report 生成，不改变 payload。
6. 引入 `PromptComposer` typed sections，但默认渲染保持兼容。
7. 暴露 context report 的持久化/API/UI 最小入口。
8. 逐步迁移上层 prompt/context 构造调用到 facade。
9. 将被替代的旧接口标记 deprecated，禁止新增调用，但不删除。
10. 完成测试、OpenSpec task 更新和归档前验证。

## Risks / Trade-offs

- Risk: 首轮接口过大导致过度设计。
  Mitigation: 只实现 assemble/after_turn/report，compaction/child lifecycle 后续提案再加。

- Risk: legacy wrapping 仍可能微小改变 prompt 文本或 tool options。
  Mitigation: 增加等价测试或 snapshot 测试，第一阶段以“不改变 payload”为验收。

- Risk: `ContextReport` 泄漏敏感 prompt 内容。
  Mitigation: 默认只保存摘要、hash、source id、大小和决策；完整 prompt 必须 debug opt-in。

- Risk: 上下文引擎被 Web 或 application 具体逻辑耦合。
  Mitigation: 上层只依赖 facade；engine selection 只能通过 config/manifest/profile。

- Risk: memory recall 被误当成 trusted instruction。
  Mitigation: 本提案不默认启用 recall；未来 recall 必须进入 dynamic/untrusted section。

- Risk: 新 crate 增加 workspace 复杂度。
  Mitigation: 保持 crate narrow，避免引入重型依赖和业务逻辑。

## Open Questions

- `ContextReport` 首轮应持久化到现有 event log、session store，还是单独 report store？实现前需要基于现有存储和 UI 查询路径评估。
- `ContextBudget` 的默认 token 估算首轮使用简单字符近似还是接入 provider-specific tokenizer？倾向先近似，避免新增依赖。
- `PromptComposer` 首轮是否直接替换 Web prompt 构造，还是先作为 adapter 旁路生成 report？倾向先旁路，确认等价后迁移。
