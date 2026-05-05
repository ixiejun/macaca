# OpenClaw 与 Hermes Agent 上下文工程研究报告

日期：2026-05-05

## 研究目标

本报告研究 `/Users/quantum/code/dev/agent/openclaw` 与 `/Users/quantum/code/dev/agent/hermes-agent` 两个项目的上下文工程设计，提炼可被 Macaca 后续实现借鉴的优点，并明确不建议借鉴的短板。

本报告不修改任何运行时代码，只作为后续设计、OpenSpec 提案和实现拆分的参考。

## 范围与方法

阅读范围：

- OpenClaw 文档：`docs/concepts/context.md`、`docs/concepts/context-engine.md`、`docs/concepts/compaction.md`、`docs/concepts/system-prompt.md`、`docs/concepts/session.md`、`docs/concepts/session-pruning.md`、`docs/concepts/memory.md`、`docs/concepts/active-memory.md`、`docs/plugins/memory-wiki.md`、`docs/cli/memory.md`。
- OpenClaw 源码：`src/context-engine/*`、`src/agents/system-prompt.ts`、`src/agents/system-prompt-cache-boundary.ts`、`src/agents/prompt-cache-stability.ts`、`src/agents/harness/context-engine-lifecycle.ts`、`extensions/memory-core/*`、`extensions/memory-wiki/*`、`extensions/active-memory/index.ts`、`extensions/codex/src/app-server/context-engine-projection.ts`。
- Hermes Agent 文档与源码：`AGENTS.md`、`run_agent.py`、`agent/context_engine.py`、`agent/context_compressor.py`、`agent/prompt_builder.py`、`agent/memory_provider.py`、`agent/memory_manager.py`、`agent/prompt_caching.py`、`hermes_state.py`、`plugins/context_engine/__init__.py`、`plugins/memory/*/README.md`。

判断标准：

- 是否适合作为 7x24 自动运行的 Agent OS 基础设施。
- 是否能支持多 application、多 agent、多 session、多 runtime。
- 是否能约束上下文增长、降低 token 成本、保护 prompt cache。
- 是否能避免把业务语义、应用名、workflow 名硬编码进核心。
- 是否可渐进落地，而不是一次性替换现有 Macaca 执行链路。

## 总体结论

OpenClaw 的上下文工程更适合作为 Macaca 的主参考：它把上下文工程抽象成可插拔生命周期，边界清晰，具备插件槽、compaction、session pruning、memory/wiki 分层、prompt cache 边界和诊断命令。

Hermes Agent 的实现更像一个成熟但高耦合的单体代理：它在压缩策略、SQLite session lineage、memory provider lifecycle、user-message 注入、技能按需加载方面有很多值得借鉴的局部设计，但不适合照搬其 `run_agent.py` 式的大循环和隐式状态耦合。

Macaca 后续应采用的方向：

- 参考 OpenClaw：建立 `ContextEngine` 生命周期接口，并作为 Macaca runtime/framework 的显式策略层。
- 参考 Hermes：借鉴压缩摘要模板、工具结果剪枝、session lineage、memory provider hooks、用户消息级临时上下文注入。
- 避免照搬：OpenClaw 的 TS 插件复杂度和 Hermes 的 Python 单体大文件/隐式耦合。
- 首期不做“大一统智能上下文引擎”，先做可观测、可替换、可回退的基础设施。

## 两个项目的上下文工程模型

### OpenClaw 模型

OpenClaw 对“上下文”的定义非常清晰：上下文是每次模型运行时发送给模型的一切，包括 system prompt、conversation history、tool calls/results、attachments、compaction summaries、pruning artifacts、provider wrapper 等。

它的核心设计是：

- system prompt 由运行时构建，不依赖底层 agent harness 的默认 prompt。
- Project Context 注入固定的 workspace 文件，如 `AGENTS.md`、`SOUL.md`、`TOOLS.md`、`IDENTITY.md`、`USER.md`、`HEARTBEAT.md`、`BOOTSTRAP.md`、`MEMORY.md`。
- 大文件按单文件和总量双重限制截断，并在 `/context` 诊断中显示 raw/injected size。
- skill 只注入 compact list，不默认注入完整 `SKILL.md`。
- tool schema 成本被显式展示，避免只优化文本 prompt 而忽略 JSON schema。
- context engine 作为插件槽，参与 ingest、assemble、compact、afterTurn、subagent lifecycle。
- memory-core 负责 recall/search/promotion/dreaming，memory-wiki 负责结构化知识 vault 和 provenance。
- active-memory 是一个阻塞式 recall sub-agent，限定条件下在主回复前执行一次 bounded recall。
- prompt cache 有明确稳定前缀和动态后缀边界，动态信息尽量放边界后。

OpenClaw 的优雅点是“控制面清晰”：上下文构建、压缩、记忆、技能、工具 schema、session、cache 都有可诊断的边界。

### Hermes Agent 模型

Hermes 的上下文工程集中在 `AIAgent` 会话循环里，外部由若干抽象辅助：

- `ContextEngine` ABC 定义 token usage、should_compress、compress、session lifecycle、context-engine tools。
- 默认 `ContextCompressor` 做 lossy summarization。
- `MemoryProvider` ABC 定义 initialize、system_prompt_block、prefetch、queue_prefetch、sync_turn、on_pre_compress、on_session_switch、on_session_end、tools。
- `MemoryManager` 组合 builtin memory 和最多一个 external provider。
- `prompt_builder.py` 负责系统提示拼装、上下文文件扫描、skill prompt 生成。
- `prompt_caching.py` 负责 Anthropic prompt cache breakpoints。
- `hermes_state.py` 用 SQLite 保存 sessions/messages/token/cost/system_prompt，并通过 parent_session_id 表达 compression continuation。

Hermes 的优秀局部设计：

- context compression summary 有强隔离措辞，明确“REFERENCE ONLY，不要执行摘要里的旧请求”。
- 压缩前先剪枝旧 tool result，保留工具调用摘要，降低无谓 token。
- 压缩后新建 child session，保留 parent lineage，而不是原地改写历史。
- memory prefetch 注入到当前 user message，而不是 system prompt，用于保护 prompt cache。
- memory context 用 `<memory-context>` fence 和 system note 明确不可信背景数据。
- streaming scrubber 能防止 `<memory-context>` 块泄漏到 UI。
- memory provider 有 `on_pre_compress`，在压缩丢弃上下文前给 memory provider 提取 durable insights 的机会。
- skill 支持 template vars、inline shell、skill config 注入和支持文件提示。

Hermes 的主要问题是“控制面混在大循环里”：大量行为由 `run_agent.py` 里的状态变量、分支、缓存和隐式约定连接，迁移和审查成本高。

## OpenClaw 可借鉴设计

### 1. ContextEngine 生命周期接口

OpenClaw 的 `ContextEngine` 接口包含：

- `bootstrap`：第一次看到 session 时初始化或导入历史。
- `ingest` / `ingestBatch`：接收新消息或完整 turn。
- `assemble`：在模型调用前按 token budget 返回有序消息和可选 `systemPromptAddition`。
- `compact`：执行上下文压缩，可返回新 session id/file。
- `afterTurn`：turn 完成后的持久化、索引、后台压缩。
- `maintain`：运行时允许 context engine 请求安全 transcript rewrite。
- `prepareSubagentSpawn` / `onSubagentEnded`：处理子 agent 的 fork/isolated context 生命周期。
- `dispose`：释放资源。

可借鉴点：

- 生命周期足够完整，覆盖正常 turn、压缩、子代理、维护和关闭。
- `assemble` 返回 `estimatedTokens` 和 `promptAuthority`，让引擎明确自己的 token 估计是否可信。
- `systemPromptAddition` 允许动态注入，但不是无边界改写 system prompt。
- 默认 `legacy` 引擎包装旧行为，支持渐进迁移。
- 插件引擎失败时 fallback 到 default engine，但 default engine 失败必须抛错。

Macaca 建议：

- 在 `macaca-framework` 或 `macaca-runtime` 引入 `ContextEngine` trait。
- 第一期提供 `LegacyContextEngine`，只包装现有行为，不改变执行结果。
- 新引擎必须通过 config/application manifest 选择，不能根据 app name 硬编码。
- `assemble` 必须产出诊断信息：included messages、dropped/pruned items、estimated tokens、budget、source breakdown。

### 2. Prompt cache 边界

OpenClaw 用 `<!-- OPENCLAW_CACHE_BOUNDARY -->` 把稳定 system prompt prefix 和动态 suffix 分开。

稳定区包括：

- 核心执行规则。
- 工具/技能基础说明。
- 大部分 project context。

动态区包括：

- 当前 channel/session 相关内容。
- runtime metadata。
- heartbeat/group/reaction/voice 等易变内容。

可借鉴点：

- cache 边界是一个显式、可测试的文本分界点。
- 动态 `systemPromptAddition` 被插入到 cache boundary 后，避免破坏稳定前缀。
- 对能力 id、structured prompt section 做 normalize/sort，避免 map/set 顺序破坏 cache。

Macaca 建议：

- 在 `PromptComposer` 内引入稳定段和动态段的结构，而不是先拼字符串再靠约定维护。
- 对 application registry、agent list、tool list、skill list 做确定性排序。
- 动态 trace/session/event/time 等信息不得进入 stable prefix。
- 提供 prompt hash 和 cache-break diff 诊断，显示本次请求为何破坏缓存。

### 3. `/context` 诊断思想

OpenClaw 的 `/context list` / `/context detail` 不只是 dump prompt，而是展示：

- system prompt size。
- Project Context 每个文件 raw/injected size 和 truncation 状态。
- skills list size。
- tool list text size。
- tool schema JSON size。
- session token usage。
- top tool schema / top skill entries。

可借鉴点：

- 诊断以预算和来源拆分为中心，不暴露完整敏感 prompt。
- 明确 tool schema 也占上下文。
- raw vs injected 让用户知道是源文件大还是注入策略不合理。

Macaca 建议：

- 提供 `GET /api/apps/{app_id}/sessions/{session_id}/context/report`。
- Trace UI 中按 turn 展示 context budget report。
- 后端保存每次 LLM 调用的 context report summary，不保存完整 system prompt，除非 debug flag 开启。
- 报告字段建议包含：`stable_prompt_tokens`、`dynamic_prompt_tokens`、`history_tokens`、`tool_schema_tokens`、`memory_tokens`、`skill_index_tokens`、`pruned_tokens`、`compaction_count`。

### 4. Session pruning 与 compaction 分离

OpenClaw 明确区分：

- Pruning：只在内存中 trim 旧 tool results，不改写 transcript。
- Compaction：把旧对话总结成摘要并写入 transcript。

可借鉴点：

- 工具结果膨胀是上下文爆炸的常见原因，不需要每次都动用 LLM 总结。
- pruning 不改历史，风险低。
- compaction 才是语义摘要和 transcript 变更。

Macaca 建议：

- 第一期先做 tool/event result pruning，不做复杂语义压缩。
- 对 trace event、tool result、file read、大 stdout 分级处理：保留短摘要、可点击拉取原文、模型上下文只带必要片段。
- compaction 仅在 session history 逼近阈值或用户主动触发时执行。

### 5. Memory-core 与 Memory-wiki 分层

OpenClaw 把记忆拆成两个层：

- Active memory / memory-core：recall、semantic search、short-term promotion、dreaming、memory tools。
- Memory-wiki：结构化 knowledge vault、claims/evidence/provenance、dashboards、compiled digests、wiki tools。

可借鉴点：

- recall 和 curated knowledge 是两类需求，不应该混成一个表。
- wiki 的 structured claims/evidence 让长期知识可审查、可纠错。
- compiled digest 避免 runtime 直接 scrape Markdown。

Macaca 建议：

- Macaca 后续可以保留简单 `MEMORY.md`/notes 作为人类可读层，同时建立 machine-facing digest/index。
- 对跨 session、跨 app 的长期知识必须带 provenance、confidence、privacy tier。
- 运行时 recall 不应该直接读取所有长期知识，而应通过检索工具和预算裁剪进入上下文。

### 6. Active Memory 的前置 bounded recall

OpenClaw active-memory 是一个子代理，在主回复前运行一次，限定：

- 只在配置 opt-in。
- 只针对指定 agent。
- 只在 direct/group/channel 等允许的 session 类型。
- 有 timeout、max summary chars、工具 allowlist。
- 输出作为隐藏 untrusted prompt prefix 注入，不直接给用户。

可借鉴点：

- 它解决“模型没主动搜 memory 导致上下文缺失”的问题。
- 通过严格 gating 和 timeout 控制成本。
- 只允许 memory tools，避免 recall sub-agent 变成任意执行器。

Macaca 建议：

- 后续可做 `ContextPreflightAgent`，但只允许 read-only recall tools。
- 默认关闭，只在 application/agent profile opt-in。
- 结果必须短摘要，标记 untrusted context。
- 超时或失败不得阻塞主流程太久，应降级为空 recall。

### 7. Subagent context lifecycle

OpenClaw 的 `prepareSubagentSpawn` 支持 parent/child session、`isolated`/`fork`、TTL、rollback；`onSubagentEnded` 支持结束清理。

可借鉴点：

- 子 agent 上下文模式是基础设施级概念，不是工具参数字符串。
- spawn 失败有 rollback，避免泄漏临时状态。
- child session 结束需要清理或合并上下文。

Macaca 建议：

- Macaca 的 delegate/fork/main thread 应统一建模为 context lineage。
- 支持 `isolated`、`fork_summary`、`fork_full_recent`、`shared_artifact_only` 等模式。
- 不允许上层应用手写“把 parent 所有 event 复制给 child”的逻辑。

## Hermes Agent 可借鉴设计

### 1. 压缩摘要模板

Hermes 的 `SUMMARY_PREFIX` 非常关键，它明确：

- 这是 earlier turns 的 compacted summary。
- 仅作为 reference。
- 不要回答或执行 summary 里提到的旧问题。
- 当前任务由 summary 中的 Active Task 指示。
- 只回应 summary 之后最新的 user message。
- 当前文件/配置状态可能已反映 summary 中的工作，避免重复执行。

可借鉴点：

- 摘要如果写得像普通用户消息，会诱导模型重新执行旧任务。
- 需要强分隔、强语义标签、明确“不是新指令”。

Macaca 建议：

- 所有 compaction summary 必须带固定 envelope，例如 `<context_summary source="compaction" trusted="false">`。
- 摘要内容结构固定：`Resolved`、`Decisions`、`Current State`、`Open Questions`、`Active Task`、`Important IDs/Paths`。
- 摘要生成 prompt 要求保留 opaque IDs、file paths、task IDs、session IDs。

### 2. 工具结果剪枝先于 LLM summarization

Hermes 的 compressor 先执行 cheap pre-pass：

- 旧 tool result 用一行摘要替代。
- 对 terminal/read_file/search/web/tool 等做类型化摘要。
- dedupe 重复读取结果。
- 对 tool_call args 进行 JSON-preserving truncation，避免 provider 400。

可借鉴点：

- 大多数上下文膨胀来自工具结果，不一定需要 LLM 摘要。
- 保留“做过什么”的元信息比保留完整 stdout 更重要。
- 裁剪 tool call args 时必须保持 JSON 合法。

Macaca 建议：

- 为每类 trace/tool event 建立 `ContextRenderable` 或 `ContextSnippet` 策略。
- tool output 原文保存在 event store，LLM context 只拿 bounded excerpt。
- 对文件读、命令输出、搜索结果、浏览器快照分别定义摘要器。

### 3. Session compression lineage

Hermes 用 SQLite `parent_session_id` 表示压缩后的 child session，并保留 root-to-tip 映射：

- 原 session 以 `end_reason='compression'` 结束。
- 新 child session 继续写消息。
- session list 默认把 compression root 投影到 tip。
- resume 时解析到有消息的 descendant。

可借鉴点：

- 不原地重写历史，更适合审计。
- UI 上展示“一个逻辑会话”，内部保留 lineage。
- 可避免压缩后旧 session 看起来消失。

Macaca 建议：

- session store 支持 `parent_session_id`、`lineage_kind`、`lineage_root_id`。
- compaction 创建 successor session 或 successor transcript segment，不删除原 segment。
- UI 展示 logical session，debug 模式可展开 lineage。

### 4. MemoryProvider 生命周期

Hermes 的 `MemoryProvider` 设计虽然简单，但 hooks 很实用：

- `initialize(session_id, hermes_home, platform, agent_context, agent_identity, agent_workspace, parent_session_id, user_id)`
- `system_prompt_block()`
- `prefetch(query, session_id)`
- `queue_prefetch(query, session_id)`
- `sync_turn(user, assistant, session_id)`
- `on_turn_start`
- `on_session_switch`
- `on_pre_compress`
- `on_session_end`
- `on_memory_write`
- `on_delegation`

可借鉴点：

- memory provider 不只是 search/store tool，还需要 session lifecycle。
- `on_pre_compress` 特别重要，压缩前有机会保存 durable insights。
- `on_session_switch` 解决压缩、resume、branch 后 provider 内部状态错写的问题。

Macaca 建议：

- context engine 和 memory engine 不要完全混同，但要通过事件联动。
- 提供 `before_compaction`、`after_compaction`、`session_switched`、`delegate_completed` hooks。
- 外部 memory provider 只能通过接口拿上下文，不能直接读 Macaca 内部表。

### 5. User message 级临时上下文注入

Hermes 把 external memory prefetch 和 plugin `pre_llm_call` 结果注入当前 user message，而不是 system prompt：

- 不修改持久化 messages。
- API-call-time only。
- 保护 system prompt cache。
- 用 `<memory-context>` fence 标识不可信背景。

可借鉴点：

- 动态 recall 不应污染稳定 system prompt。
- 不应持久化已注入 context，避免下一 turn 反复叠加。
- UI 需要 scrubber 防止内部 context 泄漏。

Macaca 建议：

- 将每次 LLM request 视为 `CompiledPrompt`，由 session transcript + dynamic injections 派生。
- dynamic injections 不写回 session event store，只写 request diagnostics。
- 所有 recall/context injection 必须带 trust boundary。

### 6. 技能按需加载

Hermes skill 机制包含：

- system prompt 中只注入技能索引/指导。
- slash command 能把 skill 作为 user message 注入。
- skill 内容支持模板变量和配置注入。
- supporting files 以路径清单告诉模型，按需读取。
- 支持平台禁用和 external skill dirs。

可借鉴点：

- 技能全量注入会迅速污染上下文。
- skill 的脚本、模板、reference 应作为资源按需读取。
- 技能配置值可以注入，但敏感值必须避免泄漏。

Macaca 建议：

- `macaca-skill` 后续应提供 `SkillIndexContext`，只含 name/description/location/capability。
- 执行时按需读取 `SKILL.md` 和 supporting files。
- Skill 配置只注入非 secret、bounded、必要字段。

## 不建议借鉴的短板

### 不建议照搬 OpenClaw 的部分

- 不要照搬其 TS 插件系统复杂度。Macaca 是 Rust 基础设施，应该先定义 trait 和 domain service，再考虑插件 ABI。
- 不要把 context engine 与具体 provider/runtime 细节绑定。OpenClaw 需要适配 PI/Codex/ACP 等多 harness，Macaca 应抽象在 runtime/framework 层。
- 不要默认启用过多 memory/active-memory/dreaming 功能。Macaca 需要先保证 session isolation 和 trace correctness。
- 不要让 prompt 规则无限增长。OpenClaw prompt 经验丰富但内容较多，Macaca 应把规则拆为 stable core、application policy、agent profile、dynamic context。

### 不建议照搬 Hermes 的部分

- 不要复制 `run_agent.py` 单体循环。Hermes 大量上下文逻辑、工具循环、记忆、压缩、缓存、UI 回调耦合在一个类里，不适合 Macaca 的 crate 化架构。
- 不要用可变实例字段作为 context engine 与 agent loop 的隐式协议，例如 `last_prompt_tokens`、`threshold_tokens`、`compression_count` 被外部直接读取。
- 不要把 system prompt 只在 session 初始构建后长期缓存。Macaca 需要显式 stable/dynamic 分层，而不是依赖“少重建”。
- 不要使用 regex 风险扫描作为唯一安全边界。Hermes 对 context files 有 prompt injection regex 扫描，但 Macaca 应以 trust boundary、source metadata、tool policy 为主。
- 不要把外部 memory provider 限制为只有一个作为长期架构。Hermes 这么做能防止 schema 膨胀，但 Macaca 可以支持多个 provider，只是每次运行按 policy 选择和预算裁剪。

## Macaca 推荐架构

### 核心概念

建议 Macaca 上下文工程拆成以下对象：

- `ContextEngine`：上下文生命周期策略。
- `PromptComposer`：把 stable/dynamic sections 组合成 provider request。
- `ContextBudget`：模型窗口、保留 token、工具 schema token、history token、memory token 等预算。
- `ContextReport`：每次请求的可观测摘要。
- `ContextSource`：上下文来源，包括 system、workspace_file、skill_index、memory_recall、trace_event、tool_result、compaction_summary。
- `ContextSnippet`：某个 source 渲染给模型的 bounded 片段。
- `CompactionPolicy`：何时压缩、压缩目标、摘要格式、是否创建 successor transcript。
- `PruningPolicy`：何时剪枝、剪枝哪些 event/tool output。
- `MemoryRecallPolicy`：是否前置 recall、工具 allowlist、timeout、max chars。
- `SessionLineage`：main/delegate/fork/compaction 的会话关系。

### 建议 trait 草案

```rust
#[async_trait::async_trait]
pub trait ContextEngine: Send + Sync {
    fn info(&self) -> ContextEngineInfo;

    async fn bootstrap(&self, input: ContextBootstrapInput) -> MacacaResult<ContextBootstrapResult>;

    async fn ingest_turn(&self, input: ContextTurnInput) -> MacacaResult<ContextIngestResult>;

    async fn assemble(&self, input: ContextAssembleInput) -> MacacaResult<ContextAssembleResult>;

    async fn compact(&self, input: ContextCompactInput) -> MacacaResult<ContextCompactResult>;

    async fn after_turn(&self, input: ContextAfterTurnInput) -> MacacaResult<()>;

    async fn prepare_child(&self, input: ContextChildPrepareInput) -> MacacaResult<ContextChildHandle>;

    async fn child_finished(&self, input: ContextChildFinishedInput) -> MacacaResult<()>;
}
```

`ContextAssembleResult` 建议包含：

```rust
pub struct ContextAssembleResult {
    pub messages: Vec<LlmMessage>,
    pub system_prompt: String,
    pub estimated_tokens: u32,
    pub prompt_authority: PromptAuthority,
    pub report: ContextReport,
}
```

`ContextReport` 建议包含：

```rust
pub struct ContextReport {
    pub request_id: String,
    pub app_id: ApplicationId,
    pub session_id: SessionId,
    pub agent_name: String,
    pub model: String,
    pub token_budget: u32,
    pub estimated_total_tokens: u32,
    pub stable_prompt_tokens: u32,
    pub dynamic_prompt_tokens: u32,
    pub history_tokens: u32,
    pub tool_schema_tokens: u32,
    pub memory_tokens: u32,
    pub skill_tokens: u32,
    pub pruned_tokens: u32,
    pub sources: Vec<ContextSourceReport>,
}
```

## 分阶段落地建议

### Phase 0：只做观测，不改行为

目标：

- 统计当前 Macaca 每次 LLM 调用的上下文组成。
- 不改变 prompt 内容，不影响模型行为。

任务：

- 在 LLM 调用前生成 `ContextReport`。
- 统计 system prompt、history、tools schema、trace/event snippets 的粗略 token。
- Trace UI 展示 context report summary。
- 对超大 tool result / trace event 做告警，不裁剪。

验收：

- 能回答某个 session 的上下文由哪些来源构成。
- 能定位 token 最大的工具 schema、event、memory、skill。

### Phase 1：PromptComposer 分层

目标：

- 把 system prompt 从字符串拼接迁移到 stable/dynamic sections。
- 保证排序确定性和 prompt cache 友好。

任务：

- 定义 `PromptSection { id, stability, trust_level, content }`。
- stable sections 和 dynamic sections 分开渲染。
- tool list、agent list、skill list、workspace files 确定性排序。
- dynamic recall、session metadata、time、runtime state 放 dynamic。

验收：

- 同一 app/session 在无动态变化时 stable prompt hash 不变。
- 动态信息变化不影响 stable prompt hash。

### Phase 2：Context pruning

目标：

- 优先解决工具结果和 trace event 膨胀。
- 不做 LLM 摘要，不改 transcript。

任务：

- 为 tool result/trace event 增加 `ContextRenderable`。
- 大输出渲染为摘要 + bounded excerpt + artifact reference。
- 原文仍存 event store。
- UI 可查看原文，模型只拿上下文摘要。

验收：

- 大 stdout/file read 不再完整进入模型上下文。
- Pruning 结果可在 ContextReport 中解释。

### Phase 3：Compaction 和 session lineage

目标：

- 长 session 接近上下文窗口时自动压缩。
- 保留审计历史。

任务：

- 定义 compaction summary schema。
- 压缩前触发 memory flush hook。
- 压缩后创建 successor transcript segment 或 child session。
- logical session 展示 root-to-tip。
- manual compact 支持 focus topic。

验收：

- 压缩后会话可继续。
- 原始历史仍可查。
- UI 能显示 compaction event 和 successor lineage。

### Phase 4：Memory recall 与 wiki 分层

目标：

- 支持长期记忆和结构化知识，但不污染默认上下文。

任务：

- `memory_search` / `memory_get` 类工具化 recall。
- durable memory 与 compiled wiki digest 分离。
- recall 结果带 provenance、confidence、privacy tier。
- 可选 preflight recall agent，默认关闭。

验收：

- 模型不会默认加载所有 memory。
- 需要历史事实时可通过 search/get 精确召回。
- recall 注入在 ContextReport 中可见。

### Phase 5：插件化 ContextEngine

目标：

- 允许不同 application/agent profile 选择上下文策略。

任务：

- `LegacyContextEngine` 默认启用。
- `WindowedContextEngine` 支持 token window 裁剪。
- `SummaryContextEngine` 支持 compaction。
- config/manifest 选择引擎，不按 app name 硬编码。

验收：

- 引擎切换无需修改上层业务代码。
- 引擎失败可 fallback，fallback 事件可观测。

## 风险与控制

### 风险：上下文裁剪导致任务丢失

控制：

- 第一阶段只观测。
- pruning 不改 transcript。
- compaction summary 保留 task IDs、file paths、decisions、open questions。
- 压缩前执行 memory flush。

### 风险：动态上下文污染 prompt cache

控制：

- stable/dynamic section 强类型建模。
- dynamic injection 不写回 session transcript。
- prompt cache break report。

### 风险：memory recall 引入 prompt injection

控制：

- recall 注入必须标记 untrusted。
- memory/wiki 结果带来源、隐私等级、置信度。
- 不把 recall 当系统指令。

### 风险：插件化过早导致复杂度失控

控制：

- 先 trait + legacy engine。
- 不做 ABI/plugin crate，直到至少两个真实 engine 需要替换。
- OpenSpec 限定最小行为边界。

### 风险：token 估算不准

控制：

- 估算只用于预警和预裁剪。
- 记录 provider 返回的真实 usage。
- ContextReport 同时保存 estimated 和 actual usage。

## 推荐取舍

应借鉴：

- OpenClaw 的 ContextEngine 生命周期和 legacy fallback。
- OpenClaw 的 stable/dynamic prompt cache boundary。
- OpenClaw 的 context diagnostics。
- OpenClaw 的 memory-core / memory-wiki 分层。
- OpenClaw 的 subagent context lifecycle。
- Hermes 的 compaction summary 防误执行模板。
- Hermes 的 tool result pruning。
- Hermes 的 session compression lineage。
- Hermes 的 memory provider lifecycle hooks。
- Hermes 的 user-message 级动态 recall 注入。

应避免：

- Hermes 的巨型 `run_agent.py` 单体执行循环。
- Hermes 的隐式可变字段协议。
- 把动态 recall 写入 system prompt。
- 把所有 skills/memory/workspace files 默认全量注入。
- 基于 app name、workflow name、agent name 写特殊逻辑。
- 在没有 ContextReport 的情况下启用自动裁剪和压缩。

## Macaca 后续 OpenSpec 建议

建议提案名称：

- `add-context-engine-observability`

第一份提案只覆盖 Phase 0 + Phase 1：

- 新增 `ContextReport`。
- 新增 `PromptComposer` stable/dynamic 分层。
- 不启用 pruning/compaction。
- 不改变 LLM 请求内容，除非只是结构化重组且 byte-equivalent。

第二份提案：

- `add-context-pruning-policy`

覆盖 tool result/trace event pruning，要求：

- 原文保留。
- 模型上下文 bounded。
- ContextReport 可解释。

第三份提案：

- `add-context-compaction-lineage`

覆盖 compaction summary、successor transcript、session lineage、UI 展示。

## 最小可执行任务清单

1. 盘点 Macaca 当前 LLM 调用入口，确认所有模型请求都经过一个可插桩点。
2. 设计 `ContextReport` 数据结构和存储位置。
3. 在 trace event 中关联 `request_id` 与 `context_report_id`。
4. 把现有 prompt 构造拆成 stable/dynamic sections，但先保持输出等价。
5. 增加 context report API 和 UI 简表。
6. 对 tool schema、history、workspace files、memory/skills 分别统计 token。
7. 基于报告决定 pruning 的首批对象，避免拍脑袋优化。

## 参考文件索引

OpenClaw：

- `openclaw/docs/concepts/context.md`
- `openclaw/docs/concepts/context-engine.md`
- `openclaw/docs/concepts/compaction.md`
- `openclaw/docs/concepts/system-prompt.md`
- `openclaw/docs/concepts/session-pruning.md`
- `openclaw/src/context-engine/types.ts`
- `openclaw/src/context-engine/registry.ts`
- `openclaw/src/context-engine/legacy.ts`
- `openclaw/src/context-engine/delegate.ts`
- `openclaw/src/agents/system-prompt.ts`
- `openclaw/src/agents/system-prompt-cache-boundary.ts`
- `openclaw/src/agents/harness/context-engine-lifecycle.ts`
- `openclaw/extensions/memory-core/src/tools.ts`
- `openclaw/extensions/memory-core/src/prompt-section.ts`
- `openclaw/extensions/active-memory/index.ts`
- `openclaw/extensions/memory-wiki/src/*`
- `openclaw/extensions/codex/src/app-server/context-engine-projection.ts`

Hermes Agent：

- `hermes-agent/run_agent.py`
- `hermes-agent/agent/context_engine.py`
- `hermes-agent/agent/context_compressor.py`
- `hermes-agent/agent/prompt_builder.py`
- `hermes-agent/agent/prompt_caching.py`
- `hermes-agent/agent/memory_provider.py`
- `hermes-agent/agent/memory_manager.py`
- `hermes-agent/hermes_state.py`
- `hermes-agent/plugins/context_engine/__init__.py`
- `hermes-agent/plugins/memory/honcho/README.md`
- `hermes-agent/plugins/memory/supermemory/README.md`
- `hermes-agent/plugins/memory/holographic/README.md`
