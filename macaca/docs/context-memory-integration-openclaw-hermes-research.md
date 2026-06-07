# OpenClaw / Hermes 上下文工程与记忆系统整合研究

## 研究目标

本文研究 `/Users/quantum/Code/dev/agent/openclaw` 与
`/Users/quantum/Code/dev/agent/hermes-agent` 如何把“工作区引导文件”、
“上下文工程”和“记忆系统”整合进 agent runtime，并提炼 Macaca 后续实现时
可以直接借鉴的架构原则。

Macaca 的目标不是复制某一个项目，而是把这些能力抽象成可插拔基础设施：

- 每个 agent 拥有独立的 `AGENTS.md`、`SOUL.md`、`TOOLS.md`、`IDENTITY.md`、
  `USER.md`、`HEARTBEAT.md`、`MEMORY.md` 等引导文件。
- 每个 agent 拥有 AgentPrivate 独家记忆。
- 同一 session / project 下存在 SessionShared 共享记忆。
- Macaca agent OS 需要完整兼容 Agent Skills 生态，并把“可用 skill 索引、
  skill 选择规则、skill 加载纪律”作为上下文工程的一等输入。
- Macaca agent OS 需要完整支持 MCP 协议，并把 MCP server、tool/resource/prompt
  capability、调用边界和安全约束注入模型可见上下文。
- 长期向量记忆必须支持主动召回。也就是说，长期记忆不是等用户显式关键词触发
  `memory_search` 才出现，而是在每轮模型调用前由 active recall policy 基于当前
  task/session/agent scope 主动从向量数据库检索相关记忆。
- 上下文工程和记忆系统都必须可替换、可组合，不与某个 provider、数据库或
  application 绑定。

## 结论摘要

OpenClaw 的强项是“工作区即 agent 操作系统状态”的设计。它把固定文件作为
agent 的人格、行为、工具、用户画像、心跳任务和长期记忆入口，并通过安全读取、
排序、预算截断、cache boundary 与动态/静态分层，把这些文件稳定注入系统提示。

Hermes 的强项是“provider 生命周期抽象”。它把上下文压缩做成 `ContextEngine`
可替换槽位，把记忆做成 `MemoryProvider` 生命周期接口，并明确区分稳定系统提示
和每轮动态召回上下文。动态召回上下文被 fence 后注入 user message，而不是改写
system prompt，从而保护 prompt cache 和会话持久化边界。

对 Macaca 最合适的路线是组合二者优点：

- 借鉴 OpenClaw 的 agent workspace bootstrap 文件体系和 Project Context 注入模型。
- 借鉴 Hermes 的 ContextEngine / MemoryProvider 生命周期接口、provider 注册与工具路由。
- 在 Macaca 中增加统一的 `AgentProfileContext` 层，把文件引导、active recall、
  knowledge digest、session transcript、tool affordance、Agent Skills、MCP capability
  合并成可审计的 context plan。
- 系统提示只放稳定、高优先级、可缓存内容；每轮召回、session 共享记忆、delegate
  结果等动态内容进入 dynamic source 或 user-message-side fenced context。
- 长期向量记忆的主动召回应进入 context preflight，而不是等待模型在工具调用阶段
  才“想起来”使用 memory tool。

## OpenClaw：文件引导如何成为 agent 可见上下文

OpenClaw 的固定工作区文件定义在 `openclaw/src/agents/workspace.ts`：

- `AGENTS.md`
- `SOUL.md`
- `TOOLS.md`
- `IDENTITY.md`
- `USER.md`
- `HEARTBEAT.md`
- `BOOTSTRAP.md`
- `MEMORY.md`

`loadWorkspaceBootstrapFiles()` 会按固定列表读取这些文件。关键细节：

- 使用 `openBoundaryFile()` 做边界安全读取，防止越权路径、符号链接和超大文件。
- 单文件最大 bootstrap 读取限制为 2 MiB。
- `MEMORY.md` 使用精确大小写入口，避免旧版 `memory.md` 混入。
- 文件内容通过 inode/dev/size/mtime identity 缓存，长生命周期进程能感知修改。
- 对 subagent / cron session 使用 `filterBootstrapFilesForSession()` 降级，只保留最小上下文。

OpenClaw 的系统提示组装在 `openclaw/src/agents/system-prompt.ts`。它定义了文件
进入 prompt 的排序：

```text
agents.md    -> 10
soul.md      -> 20
identity.md  -> 30
user.md      -> 40
tools.md     -> 50
bootstrap.md -> 60
memory.md    -> 70
```

这和用户给出的优先级非常接近，但 OpenClaw 的实际顺序是：

- `AGENTS.md` 最高，约束行为和运行规则。
- `SOUL.md` 紧随其后，约束 persona、voice、tone。
- `IDENTITY.md`、`USER.md`、`TOOLS.md` 中间层。
- `BOOTSTRAP.md`、`MEMORY.md` 更靠后。
- `HEARTBEAT.md` 被标记为 dynamic context file，不放入稳定 prefix。

OpenClaw 还把 context 文件拆为两类：

- stable context files：进入 stable prefix，可复用 prompt cache。
- dynamic context files：例如 `HEARTBEAT.md`，放在 cache boundary 后，避免频繁变化破坏缓存。

它对 `SOUL.md` 有特殊提示：

```text
If SOUL.md is present, embody its persona and tone.
```

这说明 `SOUL.md` 不是普通附件，而是 runtime 明确提升权重的 persona layer。

## OpenClaw：记忆系统如何参与上下文工程

OpenClaw 的记忆文档在 `openclaw/docs/concepts/memory.md`。核心模型是文件记忆
加可插拔 active memory：

- `MEMORY.md`：长期 durable facts、preferences、decisions。
- `memory/YYYY-MM-DD.md`：日记式短期运行上下文。
- `DREAMS.md`：后台 dreaming / promotion 审核结果。

OpenClaw 的关键设计不是“文件很简单”，而是“文件是人类可审计的 canonical
surface，provider 负责索引、搜索、召回、治理”。默认 memory-core 支持搜索工具，
而 memory-wiki 提供知识层：

- structured claims
- evidence
- freshness
- contradictions
- dashboards
- compiled digests
- wiki-native tools

系统提示里 memory 的接入点是 `buildMemoryPromptSection()`。此外
`openclaw/src/context-engine/delegate.ts` 提供
`buildMemorySystemPromptAddition()`，使第三方 context engine 可以复用 active memory
的提示说明，而不重复实现 memory prompt formatting。

这是一条重要边界：上下文引擎不拥有记忆系统，但可以引用记忆系统输出的 prompt
addition 或 dynamic source。

## OpenClaw：Heartbeat 是主动行为配置，不只是提示文本

`HEARTBEAT.md` 的文档在 `openclaw/docs/gateway/heartbeat.md`。它的使用方式说明
OpenClaw 已经把“上下文文件”延伸到主动行为系统：

- 默认 heartbeat prompt 会要求读取 `HEARTBEAT.md`。
- 当 heartbeat cadence 为 `0m` 或关闭系统提示段时，普通 run 会省略 `HEARTBEAT.md`。
- `lightContext: true` 时，heartbeat run 只注入 `HEARTBEAT.md`。
- `HEARTBEAT.md` 支持 `tasks:` 结构化块，用于按间隔触发检查任务。

这说明 Macaca 不应把 `HEARTBEAT.md` 当成普通 persona 文件。它应该属于
`AgentProfileFileKind::Heartbeat`，由 heartbeat scheduler 和 context composer
共同解释。

## OpenClaw / Hermes：Skills 生态如何进入上下文

两个项目都把 skills 视为模型可见能力目录，而不是单纯的本地插件。

OpenClaw 在系统提示中构造 `## Skills (mandatory)` 段，要求模型先扫描
`<available_skills>` 的 description，再按规则读取最匹配的 `SKILL.md`。这个设计有
三个关键点：

- skill index 是稳定提示的一部分，但完整 `SKILL.md` 不应默认全量注入。
- 模型必须先选择 skill，再读取 skill 细节，避免每轮 prompt 注入所有 skill 内容。
- skill 使用纪律也属于系统提示，例如只读一个最相关 skill、避免无关读取、遵守
  rate limit 和外部 API 写入约束。

Hermes 的 `prompt_builder.py` 也有类似逻辑：只有当 skills 工具可用时，才注入
skills guidance。它还鼓励复杂任务完成后把可复用流程保存成 skill，并在发现 skill
过期时主动 patch。

这对 Macaca 的意义是：Agent Skills 不是“工具列表的附属品”，而是上下文工程的
独立 source。Macaca 需要把它建模为 `SkillContextProvider`：

- 输出可用 skills 的 compact index，而不是默认输出所有 skill 正文。
- 根据 application、agent、workspace、user scope 解析 skill source。
- 支持 bundled / workspace / app / user / remote skill source。
- 在 context report 中记录本轮注入了哪些 skill index，模型后续读取了哪些 skill。
- skill 的工具、MCP server、配置项和安全约束需要作为 capability metadata 进入
  context plan。

Macaca 必须保持对 Agent Skills 生态的完整兼容，但不能把所有 skill 全量拼进
system prompt。正确形态是“索引常驻、正文按需、使用可审计”。

## OpenClaw / Hermes：MCP 能力如何进入提示词

OpenClaw 和 Hermes 都把外部工具系统视为 agent runtime 的扩展能力，但 Hermes 的
plugin / hook / tool registry 和 OpenClaw 的 MCP / tool prompt guidance 给 Macaca
一个明确方向：MCP 不能只是 transport 层，必须进入上下文工程。

Macaca 支持完整 MCP 协议后，模型至少需要看到这些信息：

- 当前 agent 可访问哪些 MCP server。
- 每个 server 暴露哪些 tools、resources、prompts。
- 哪些能力是只读，哪些能力会产生副作用。
- 哪些能力需要 approval、credential、sandbox 或 network 权限。
- resource 是否适合直接读取，还是需要通过 query/search 工具按需加载。
- MCP capability 与内置工具、Agent Skills、memory provider tool 的命名冲突如何处理。

因此 MCP 应该由 `McpContextProvider` 进入 context plan，而不是散落在工具 schema
里。建议分层：

- stable layer：MCP server 简短清单、能力类别、使用纪律、安全约束。
- dynamic layer：server 健康状态、当前 session 授权状态、临时 resource hint。
- tool schema layer：真正传给模型的 MCP tools。
- resource recall layer：必要时把 MCP resources 作为 context source 召回。

MCP 与 Skills 的关系也需要明确：

- skill 可以声明需要某个 MCP server 或 MCP tool。
- MCP server 可以作为 skill runtime dependency。
- skill 选择后，context composer 可以提升相关 MCP capability 的可见度。
- MCP resource 不应默认全量注入，应通过 budget、trust 和 relevance 策略裁剪。

这能避免一个常见问题：模型看到了大量工具 schema，但不知道什么时候应该用哪个
MCP server，也不知道某个 skill 与 MCP capability 的配套关系。

## Hermes：上下文文件如何进入系统提示

Hermes 的 prompt 组装逻辑在 `hermes-agent/agent/prompt_builder.py` 和
`hermes-agent/run_agent.py`。

Hermes 会扫描：

- `$HERMES_HOME/SOUL.md`
- `.hermes.md` / `HERMES.md`
- `AGENTS.md` / `agents.md`
- `CLAUDE.md`
- `.cursorrules`
- `.cursor/rules/*.mdc`

它的优先级是“first match wins”：

```text
1. .hermes.md / HERMES.md
2. AGENTS.md / agents.md
3. CLAUDE.md / claude.md
4. .cursorrules / .cursor/rules/*.mdc
```

`SOUL.md` 是独立路径。`load_soul_md()` 会把 `$HERMES_HOME/SOUL.md` 作为 agent
identity slot 注入，如果已经作为 identity 注入，则 context files 阶段跳过，避免
重复注入。

Hermes 的一个强点是 prompt-injection 扫描。`_scan_context_content()` 会检测：

- ignore previous instructions
- system prompt override
- hidden div / invisible unicode
- curl exfil
- cat secrets

命中后不注入原文，而是注入 blocked notice。这点值得 Macaca 借鉴：agent profile
文件可由用户编辑，但系统必须有安全扫描和诊断。

## Hermes：系统提示稳定层与动态上下文分层

`run_agent.py::_build_system_prompt()` 明确说明 system prompt 只在 session 级缓存，
压缩后才重建。它把层次分为：

1. agent identity，优先使用 `SOUL.md`
2. 用户/网关传入 system prompt
3. persistent memory frozen snapshot
4. skills guidance
5. context files
6. conversation started timestamp
7. platform hint

Hermes 特别强调：

- `ephemeral_system_prompt` 不进入 cached/stored system prompt。
- plugin `pre_llm_call` 的 context 不进入 system prompt。
- 动态上下文注入 user message，以保护 prompt cache prefix。

在 LLM 调用前，Hermes 会：

- 调用 `pre_llm_call` plugin hook 收集 `{context: "..."}`
- 调用 memory manager 的 `prefetch_all()`
- 使用 `build_memory_context_block()` 把 memory recall 包成 `<memory-context>` fence
- 只修改 API call 的临时 `api_msg["content"]`
- 不修改持久化的 `messages`

这是 Macaca 需要直接借鉴的关键点：动态召回上下文必须是 ephemeral injection，
不能污染 canonical transcript。

## 长期向量记忆：主动召回而不是被动工具搜索

前一份记忆系统研究报告已经强调：OpenClaw `active-memory` 对 Macaca 很重要，
因为它对应“运行时主动召回”。这里需要进一步明确：Macaca 的长期向量记忆必须是
active recall 的主要输入源之一，而不是等模型显式调用 memory tool 后才工作。

被动记忆模式的问题：

- 用户必须说出足够精确的关键词，模型才可能调用 memory search。
- 模型可能不知道存在相关记忆，因此不会调用工具。
- 长期自治场景里，agent 需要主动延续项目上下文、用户偏好、历史决策，而不是让
  用户重复提醒。
- 多 agent session 中，delegate agent 需要自动获得与当前 task 相关的 AgentPrivate
  和 SessionShared 记忆，否则会失去协作连续性。

Macaca 当前长期向量记忆默认采用 Milvus，并已有重要拓扑约定：

- `application_id` 映射为默认 vector database。
- `agent_id` / `agent_name` 映射为默认 collection。
- collection 是 agent private long-term vector memory 的默认隔离单元。

这不应被写死为 Milvus 细节，而应上升为 `VectorMemoryBackend` contract。Milvus 是
默认实现，其他向量数据库只要能表达等价拓扑即可替换。

主动召回的推荐流程：

```text
Context Preflight
  -> derive recall query from latest user turn + goal + session state
  -> query AgentPrivate vector collection
  -> query SessionShared / project collection when enabled
  -> query knowledge digest / promoted claims when available
  -> rerank by relevance, freshness, confidence, scope, privacy, budget
  -> emit ContextCandidate
  -> compose into dynamic / ephemeral context
```

关键约束：

- active recall 查询必须默认覆盖 `AgentPrivate`，并按策略覆盖 `SessionShared`。
- vector recall 必须支持 metadata filter，例如 application、session、agent、source、
  visibility、created_at、updated_at、tombstone。
- recall 结果不写回 canonical transcript。
- recall 结果默认进入 fenced context 或 dynamic source。
- context report 记录命中数量、scope、provider、token/char budget、redaction 状态。
- tombstone 和 governance policy 必须在 recall 阶段生效，防止被删除记忆复活。

工具搜索仍然需要保留，但它是第二层能力：

- active recall：runtime 在模型调用前主动给出高置信候选。
- memory tools：模型在需要更深、更精确、更广泛查询时主动调用。
- knowledge artifacts：模型或 UI 可以查看治理后的 digest、wiki、decision log。

这三者不是替代关系，而是不同粒度的上下文入口。

## Hermes：记忆系统的 provider 生命周期

Hermes 的 `agent/memory_provider.py` 定义了 `MemoryProvider` 抽象。核心生命周期：

- `is_available()`
- `initialize(session_id, **kwargs)`
- `system_prompt_block()`
- `prefetch(query, session_id=...)`
- `queue_prefetch(query, session_id=...)`
- `sync_turn(user_content, assistant_content, session_id=...)`
- `get_tool_schemas()`
- `handle_tool_call(tool_name, args, **kwargs)`
- `shutdown()`

可选生命周期：

- `on_turn_start()`
- `on_session_end()`
- `on_session_switch()`
- `on_pre_compress()`
- `on_memory_write()`
- `on_delegation()`

`MemoryManager` 负责：

- 注册 built-in provider。
- 最多激活一个 external provider，避免工具 schema 膨胀和语义冲突。
- 聚合 system prompt block。
- 聚合 prefetch context。
- turn 完成后 sync。
- queue 下一轮 prefetch。
- memory tool name 到 provider 的路由。
- built-in memory write 后通知 external provider mirror。

Hermes 的限制是“一次只能一个 external provider”。Macaca 不一定要照抄，因为
Macaca 的目标是基础设施级可装配系统。更适合 Macaca 的策略是：允许多个 provider，
但通过 capability negotiation、scope ownership、tool namespace 和 budget policy
约束组合，而不是简单拒绝第二个 provider。

## Hermes：Honcho 记忆插件的启发

Honcho provider 展示了复杂 memory provider 如何与 runtime 结合：

- 支持 recall mode：`context`、`tools`、`hybrid`。
- 支持 cron / flush context guard，避免后台任务污染用户建模。
- 支持 session key resolution。
- 支持 context prewarm，第一轮尽量可用。
- 支持 context cadence / dialectic cadence，控制成本。
- 支持 lazy init，tools-only 模式下首次工具调用才创建 session。
- 支持 `on_memory_write()`，把 built-in memory 写入 mirror 到外部系统。
- 支持 `on_delegation()`，父 agent 接收 subagent 完成结果作为记忆观察。

值得注意的是 Honcho 删除了从 `SOUL.md` 自动同步 aiPeer 的逻辑，注释理由是：
`SOUL.md` 是 persona，不是 identity config。这个边界对 Macaca 很重要：

- `SOUL.md` 是表达风格和人格气质。
- `IDENTITY.md` 才是身份定义和自我认知。
- `MEMORY.md` 是长期事实和偏好。
- 不能把 persona、identity、memory 混成一个 provider 状态。

## Hermes：ContextEngine 可替换槽位

Hermes 的 `agent/context_engine.py` 定义了 `ContextEngine` 抽象：

- `update_from_response()`
- `should_compress()`
- `compress()`
- `should_compress_preflight()`
- `has_content_to_compress()`
- `on_session_start()`
- `on_session_end()`
- `on_session_reset()`
- `get_tool_schemas()`
- `handle_tool_call()`
- `get_status()`
- `update_model()`

`run_agent.py` 的选择流程：

1. 读取 config `context.engine`
2. 查 `plugins/context_engine/<name>`
3. 查通用 plugin system 的 `register_context_engine()`
4. fallback 到 built-in `ContextCompressor`

这给 Macaca 的启发是：context engine 应该是独立 slot，不应把压缩、召回、文件注入、
知识层都塞进一个巨型模块。Macaca 可以使用：

- `ContextEngine` 负责预算、压缩、上下文计划。
- `ProfileContextProvider` 负责 agent files。
- `MemoryRecallProvider` 负责记忆召回。
- `KnowledgeDigestProvider` 负责治理后的知识摘要。
- `ContextComposer` 负责按优先级和预算合成最终上下文。

## 两个项目的共同关键原则

### 1. 文件引导是高优先级上下文，不是普通文档

OpenClaw 和 Hermes 都把 `AGENTS.md` / `SOUL.md` 等文件提升到系统提示或
Project Context，而不是让模型“有需要再读”。这保证 agent 的基础行为稳定。

Macaca 应该把这些文件建模为 `AgentProfileFile`，包含：

- kind
- path
- priority
- trust level
- injection target
- cache stability
- max chars / token budget
- last loaded identity
- diagnostics

### 2. 动态召回不要污染 canonical transcript

Hermes 的 user-message-side ephemeral injection 是关键经验。Macaca 已经在
active recall 中强调不写回 canonical transcript，后续整合时必须继续坚持。

建议 Macaca 明确区分：

- canonical messages：用户和 agent 真实对话。
- context sources：每轮动态拼装，不持久化为对话。
- context report：可审计地记录本轮注入了哪些 source、scope、token、摘要、诊断。

### 3. 稳定上下文与动态上下文必须分层

OpenClaw 用 cache boundary 区分 stable 和 dynamic。Hermes 通过 system prompt
缓存和 user-message injection 区分稳定/动态。

Macaca 应该为每个 context source 声明：

- `CacheClass::StablePrefix`
- `CacheClass::DynamicSuffix`
- `CacheClass::EphemeralUserContext`
- `CacheClass::ToolOnly`

### 4. Provider 只暴露 capability，不泄漏实现

Hermes 的 `MemoryProvider` 和 `ContextEngine` 是可替换接口。OpenClaw 的 memory
plugin / wiki / context engine delegate 也体现了类似思想。

Macaca 应该避免在上层 hardcode Milvus、某个 memory provider、某个 app name。
provider 只声明：

- 支持哪些 `MemoryScope`
- 支持哪些索引拓扑
- 支持哪些 recall mode
- 支持哪些 governance capability
- 支持哪些 tool schemas
- 支持哪些 consistency / deletion guarantees

## Macaca 推荐架构

### Agent Profile 层

新增或完善 `macaca-context` 下的 profile context 抽象：

```text
AgentProfileContext
  files:
    AGENTS.md      priority high, stable
    SOUL.md        priority high, stable
    TOOLS.md       priority medium, stable or dynamic when toolset changes
    IDENTITY.md    priority medium, stable
    USER.md        priority low, stable
    HEARTBEAT.md   priority low, dynamic / heartbeat only
    MEMORY.md      priority memory bridge, not full raw injection by default
```

建议优先级按用户给出的语义，而不是照搬 OpenClaw 数字顺序：

- high：`AGENTS.md`、`SOUL.md`
- medium：`TOOLS.md`、`IDENTITY.md`
- low：`USER.md`、`HEARTBEAT.md`
- memory bridge：`MEMORY.md`

`MEMORY.md` 不建议每轮完整注入。它应作为：

- 小规模本地记忆的 fallback。
- memory provider 的 seed / audit surface。
- 人类可编辑的长期事实入口。
- active recall 的一个 source。

### Context Composition 层

建议使用 Chain of Responsibility + Strategy：

```text
ContextComposer
  -> ProfileFileProvider
  -> SkillContextProvider
  -> McpContextProvider
  -> ActiveRecallProvider
  -> SessionTranscriptProvider
  -> KnowledgeDigestProvider
  -> ToolAffordanceProvider
  -> HeartbeatProvider
  -> ChannelRuntimeProvider
```

每个 provider 输出 `ContextCandidate`：

```text
source_id
scope
priority
trust_level
cache_class
target
budget_hint
content
redaction_policy
diagnostics
```

`ContextComposer` 负责排序、预算裁剪、去重、降级和 report。

Skills 与 MCP 在这条链路中的角色不同：

- `SkillContextProvider` 输出 skill index、skill 使用纪律和按需读取 hint。
- `McpContextProvider` 输出 MCP capability map、server health、resource/tool/prompt 边界。
- `ToolAffordanceProvider` 输出最终可调用工具摘要，避免模型只看到 schema 而缺少策略。
- `ActiveRecallProvider` 可以把与当前 task 相关的 skill usage memory、MCP resource
  history、provider quirks 一并作为长期向量记忆召回。

### Memory Integration 层

Macaca 已经设计了：

- `MemoryScope`
- `AgentPrivate`
- `SessionShared`
- `MemoryFacade`
- `MemoryRouter`
- provider capability
- vector topology abstraction
- governance / knowledge layer

后续整合时应把 agent profile 文件和 memory scope 连接起来：

- `SOUL.md` -> agent persona，不进入 memory provider 自动学习。
- `IDENTITY.md` -> agent identity，可作为 AgentPrivate identity seed。
- `USER.md` -> user profile，可作为 SessionShared/UserScoped seed，但要可审计。
- `MEMORY.md` -> AgentPrivate durable facts seed。
- session transcript -> SessionShared candidate source。
- delegate result -> parent agent AgentPrivate 或 SessionShared candidate，由 policy 决定。
- skill 使用经验、失败修复、provider quirks -> AgentPrivate 或 ApplicationShared
  memory candidate，由 promotion policy 决定是否进入长期向量记忆。
- MCP resource/tool 的成功用法、权限限制、常见失败 -> AgentPrivate 或
  ApplicationShared memory candidate，但不得记录 secret。

### Agent Skills / MCP Context 层

Macaca 需要把 Skills 和 MCP 纳入同一个上下文工程体系，而不是分别在工具注册阶段
临时拼字符串。

推荐抽象：

```text
CapabilityContextProvider
  -> SkillContextProvider
  -> McpContextProvider
  -> ToolContextProvider
```

输出统一的 capability candidate：

```text
capability_id
capability_kind: skill | mcp_tool | mcp_resource | mcp_prompt | builtin_tool
provider_id
scope
description
usage_policy
safety_level
requires_approval
cache_class
budget_hint
```

这样做的好处：

- 模型能理解 skill、MCP、工具之间的关系。
- context composer 能按 task relevance 裁剪 capability index。
- 上层 application 能替换 skill resolver 或 MCP registry，不影响 runtime 主循环。
- trace report 能解释为什么某个 skill 或 MCP capability 在本轮可见。

### Runtime Hook 层

建议 Macaca 在 runtime 主循环提供以下 hook：

- `on_session_start`
- `before_context_compose`
- `after_context_compose`
- `before_model_call`
- `after_model_call`
- `after_turn_commit`
- `before_compaction`
- `after_compaction`
- `on_memory_write`
- `on_delegation_complete`
- `on_heartbeat_tick`

这些 hook 不应该直接允许任意插件改 system prompt。应要求插件返回结构化
`ContextCandidate` 或 `MemoryCandidate`，再由 composer/router 统一裁决。

## 需要避免的短板

### 不要照搬 OpenClaw 的“文件全文注入”

OpenClaw 的 markdown 文件模式易懂、可审计，但如果 Macaca 每个 agent / session
都无脑注入所有文件，会造成：

- token 膨胀
- prompt cache 失效
- 低优先级内容覆盖高优先级内容
- `MEMORY.md` 长大后污染 prompt

Macaca 应该默认只注入高优先级短文件，低优先级文件走预算控制或召回。

### 不要照搬 Hermes 的“只允许一个 external memory provider”

Hermes 为了简单性限制一个 external provider。Macaca 是 agent OS，应允许多 provider
协作，但必须有 capability negotiation 和 scope ownership，否则会出现重复写入、
工具冲突和召回噪声。

### 不要把 persona、identity、memory 混为一体

Honcho 注释已经明确：`SOUL.md` 不是 aiPeer identity config。Macaca 应该在类型层
强制区分：

- persona
- identity
- user profile
- durable memory
- operational policy
- heartbeat task

### 不要让 plugin 直接拼 raw prompt

Hermes 的 `pre_llm_call` 可以返回字符串 context，简单但容易让插件绕过上下文治理。
Macaca 更适合要求插件返回结构化 candidate：

```text
kind
scope
trust
source
content
expires_at
max_tokens
redaction
```

再由 composer 统一 fence、排序、预算和审计。

## 对 Macaca 的落地建议

### Phase A：Agent Profile Bootstrap

目标：把每个 agent 的引导文件纳入统一 profile context，不接入复杂记忆写回。

实现项：

- 定义 `AgentProfileFileKind`。
- 定义 `AgentProfileFilePriority`。
- 实现安全读取、frontmatter strip、大小限制、路径边界检查。
- 实现 `ProfileFileContextProvider`。
- 在 context report 中记录每个文件是否注入、截断、跳过、诊断。
- `HEARTBEAT.md` 默认只在 heartbeat run 或显式配置下进入 dynamic context。

### Phase B：Memory as Context Source

目标：把 AgentPrivate 和 SessionShared 记忆以 active recall 方式进入 context。

实现项：

- `MemoryRecallProvider` 输出 `ContextCandidate`。
- `MEMORY.md` 作为 provider-neutral seed source，不默认全文注入。
- active recall 查询同时覆盖 AgentPrivate 和 SessionShared。
- active recall 默认从长期向量记忆主动召回，优先查询当前 application database 下的
  当前 agent collection，再按 policy 查询 session/project shared collection。
- active recall query 由当前 user turn、goal、session metadata、agent identity、
  tool/skill/MCP usage context 共同派生，而不是只使用用户原文关键词。
- vector recall 使用 metadata filter 和 governance filter，确保 tombstone、privacy、
  visibility、session scope 生效。
- 每轮 recall 结果进入 ephemeral/dynamic context，不写 canonical transcript。
- report 记录 memory scope、provider、hit count、redaction，不记录完整敏感内容。

### Phase C：Skills / MCP Capability Context

目标：把 Macaca 完整兼容的 Agent Skills 生态和 MCP 协议能力纳入上下文工程。

实现项：

- `SkillContextProvider` 输出 compact skill index 和 skill 使用纪律。
- skill 正文默认不全量注入，模型选择后再通过 skill reader / tool 加载。
- `McpContextProvider` 输出 MCP server/tool/resource/prompt capability map。
- MCP resource 默认不全量注入，按 relevance 和 budget 作为 context source 召回。
- skill 可以声明 MCP dependency，context composer 根据 skill relevance 提升相关 MCP
  capability 的可见度。
- context report 记录本轮可见的 skill/MCP capability、来源、预算和安全策略。

### Phase D：Knowledge Digest 与治理层

目标：把长期治理后的 claim/digest/artifact 作为更高质量上下文。

实现项：

- `KnowledgeDigestProvider` 读取 compiled digest。
- digest 优先于 raw recall，但必须保留 evidence 引用。
- promotion policy 控制 session candidate 到 long-term memory。
- tombstone 阻止被删除记忆复活。

### Phase E：Provider 插件化

目标：用户可以替换 context engine 和 memory provider。

实现项：

- `ContextEngineRegistry`
- `MemoryProviderRegistry`
- capability negotiation
- provider namespace / tool namespace
- config-driven selection
- fallback chain
- provider health / diagnostics

### Phase F：Runtime Hooks

目标：让上层 application 和插件能参与上下文和记忆，但不能绕过治理。

实现项：

- 所有 hook 返回结构化 candidate。
- 禁止插件直接追加 raw system prompt，除非声明高权限且可审计。
- 支持 hook failure fail-open / fail-closed 策略。
- context report 和 memory audit 关联 trace event。

## 推荐的设计模式

### Strategy

用于：

- context engine selection
- memory recall policy
- promotion policy
- redaction policy
- budget allocation policy

### Chain of Responsibility

用于：

- 多个 context provider 逐层产出 candidate
- 多个 memory source 逐层召回
- 多个 safety scanner 逐层诊断

### Facade

用于：

- `MemoryFacade`
- `ContextFacade`
- `AgentProfileFacade`

上层 application 只面向 facade，不接触具体 provider。

### Decorator

用于：

- governance wrapper
- audit wrapper
- redaction wrapper
- metrics wrapper
- timeout/fallback wrapper

### Adapter

用于：

- OpenClaw-style markdown memory adapter
- Milvus-like topology vector adapter
- third-party memory provider adapter
- context engine plugin adapter

### Builder

用于：

- `ContextPlanBuilder`
- `AgentProfileContextBuilder`
- `MemoryRecallRequestBuilder`

Builder 适合把大量可选输入变成可验证的 context plan，避免主循环面条化。

## 风险清单

### Prompt 优先级冲突

风险：`SOUL.md` 和 `AGENTS.md` 给出冲突行为。

建议：在类型层声明优先级，composer 输出冲突诊断。`AGENTS.md` 的行为规则高于
`SOUL.md` 的风格规则。

### 记忆污染

风险：把临时任务进展写成长记忆，导致后续 agent 重复旧任务。

建议：promotion policy 默认保守。session progress 进入 SessionShared 或 transcript
search，不直接进入 AgentPrivate long-term。

### Provider 重复写入

风险：多个 provider 同时捕获同一 turn，产生重复或冲突。

建议：引入 scope ownership 和 write intent。一个 source 的 candidate 只经过一个
promotion pipeline，其他 provider 可以 mirror，但必须带 source id。

### Token 膨胀

风险：agent 文件、memory recall、knowledge digest、session summary、skill index、
MCP capability map 同时注入。

建议：所有 source 都必须有 budget hint。低优先级内容默认摘要或按需召回。

### Capability 膨胀

风险：Agent Skills、MCP tools、builtin tools、memory tools 同时暴露后，模型面对过大的
能力空间，不知道应该选什么，甚至出现 duplicate tool name 或错误调用。

建议：把 capability index 和 tool schema 分离。上下文里只注入 compact capability
map 和选择纪律，schema 层做 namespace、dedup、approval 标记。Skill 与 MCP 的关联
通过 capability metadata 表达。

### MCP Prompt / Resource 注入

风险：MCP server 暴露的 prompt/resource 可能包含不可信内容，若直接注入 system
prompt，会形成高优先级 prompt injection。

建议：MCP resource 默认作为 untrusted/dynamic context，必须 fence；MCP prompt 只有
在被系统配置为 trusted provider 且通过 scanner 后，才允许进入较高优先级上下文。

### 向量召回噪声

风险：长期向量记忆主动召回如果过宽，会把历史无关事实带入当前任务，干扰模型判断。

建议：active recall 必须使用 scope filter、metadata filter、freshness、confidence、
task relevance 和 token budget。召回结果要可解释，并在 context report 中记录 skipped
reason。

### 安全与 prompt injection

风险：用户可编辑文件包含“忽略系统提示”等攻击内容。

建议：借鉴 Hermes `_scan_context_content()`，增加 profile file scanner；命中高危内容时
注入 blocked diagnostic，不注入原文。

### Cache 失效

风险：每轮动态 context 修改 system prompt，导致 prefix cache 失效。

建议：稳定 profile files 放 stable prefix；active recall、heartbeat、session events
放 dynamic suffix 或 user-message fenced context。

## Macaca 最小可行整合蓝图

建议第一版实现不要一次性追求完整 wiki / dreaming / provider marketplace，而是先建立
正确边界：

```text
AgentRuntime
  -> AgentProfileResolver
  -> ContextComposer
      -> ProfileFileContextProvider
      -> SkillContextProvider
      -> McpContextProvider
      -> ActiveMemoryContextProvider
      -> KnowledgeDigestContextProvider
      -> RuntimeToolContextProvider
  -> LlmCall
  -> MemoryFacade
      -> capture candidate
      -> promote / defer / tombstone
```

关键不变量：

- `AGENTS.md`、`SOUL.md` 是高优先级稳定 profile context。
- `HEARTBEAT.md` 是 heartbeat-specific dynamic context。
- `MEMORY.md` 是 memory seed/audit surface，不是默认全文 prompt。
- AgentPrivate 和 SessionShared 是 memory scope，不是文件路径。
- Agent Skills 的 compact index 可以进入稳定上下文，但 `SKILL.md` 正文按需读取。
- MCP capability map 可以进入上下文，但 MCP resource/prompt 默认按需、动态、可审计。
- 长期向量记忆默认参与 active recall，且在模型调用前主动召回。
- dynamic recall、MCP resource recall、skill loaded context 不进入 canonical transcript。
- 所有注入都有 context report。
- 所有写入都有 memory audit。

## 参考源码路径

OpenClaw：

- `openclaw/src/agents/workspace.ts`
- `openclaw/src/agents/system-prompt.ts`
- `openclaw/src/agents/bootstrap-files.ts`
- `openclaw/src/context-engine/delegate.ts`
- `openclaw/src/memory/root-memory-files.ts`
- `openclaw/docs/concepts/memory.md`
- `openclaw/docs/concepts/soul.md`
- `openclaw/docs/gateway/heartbeat.md`

Hermes：

- `hermes-agent/agent/prompt_builder.py`
- `hermes-agent/run_agent.py`
- `hermes-agent/agent/context_engine.py`
- `hermes-agent/agent/context_compressor.py`
- `hermes-agent/agent/memory_provider.py`
- `hermes-agent/agent/memory_manager.py`
- `hermes-agent/hermes_cli/plugins.py`
- `hermes-agent/plugins/memory/__init__.py`
- `hermes-agent/plugins/context_engine/__init__.py`
- `hermes-agent/plugins/memory/honcho/__init__.py`
