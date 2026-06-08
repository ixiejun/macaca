# AgentScope Java 2.0 到 macaca-framework 的 1:1 升级改写计划

## 0. 目标与边界

本文档给出从当前 `macaca-framework`（基于 AgentScope 1.x 思路）升级到 AgentScope Java 2.0 能力等价 Rust 版本的详细设计和实施计划。

核心目标：

- AgentScope Java 2.0 已有能力，Macaca 必须具备等价能力。
- 对 Macaca 其他系统的对外接口保持稳定，消费代码不直接依赖 AgentScope 具体结构。
- `macaca-framework` 本身可替换：未来即使换成其他 agent framework，只要 Macaca 对外 ABI、service command、event、policy、trace 契约不变，OS 能力、效果和运行方式不变。
- 升级必须遵循 Macaca 宪法文档：
  - `docs/macaca-os-architecture-governance.md`
  - `docs/macaca-os-microkernel-boundaries.md`
  - `docs/macaca-os-serviceization-allowlist.md`
  - `docs/design_patterns.md`
- 基于 Apache-2.0 上游实现改写的 Rust 源文件，必须在文件头部加入 Apache-2.0 来源声明和 SPDX 标识。

非目标：

- 不做 Java FFI，不直接运行 AgentScope Java。
- 不把 AgentScope 的 provider、业务名称、模型名称、workflow 名称硬编码进 Macaca OS 层。
- 不让 framework 替代 kernel、task service、application framework、driver service、skill service、MCP service、memory/context service 的系统边界。

## 1. 已阅读的上游与本地资料

上游 AgentScope Java 2.0：

- 官方文档入口：`https://java.agentscope.io/v2/zh/docs/index.html`
- 完整文档聚合：`https://java.agentscope.io/llms-full.txt`
- 本地源码：`/Users/quantum/Code/dev/agentscope-java`
- 核心模块：
  - `agentscope-core`
  - `agentscope-harness`
  - `agentscope-extensions`
  - `agentscope-distribution`
  - `agentscope-examples`
- 关键文档：
  - `docs/v2/zh/docs/change-log.md`
  - `docs/v2/zh/docs/building-blocks/agent.md`
  - `docs/v2/zh/docs/building-blocks/message-and-event.md`
  - `docs/v2/zh/docs/building-blocks/middleware.md`
  - `docs/v2/zh/docs/building-blocks/model.md`
  - `docs/v2/zh/docs/building-blocks/tool.md`
  - `docs/v2/zh/docs/building-blocks/permission-system.md`
  - `docs/v2/zh/docs/harness/*.md`
  - `docs/v2/zh/integration/**/*.md`
- 上游 license 配置：`.licenserc.yaml` 声明 `SPDX-License-Identifier: Apache-2.0`。

Macaca 本地资料：

- 当前 `macaca-framework`：`crates/runtime/macaca-framework/src/*.rs`
- Framework host/service 入口：`crates/runtime/macaca-runtime-host/src/framework_runtime_agent_service.rs`
- Web 消费入口：`crates/shells/macaca-web/src/framework_runner/**`
- 现有迁移文档：
  - `docs/macaca-framework-migration-audit.md`
  - `docs/macaca-framework-incremental-refactor-candidates.md`
  - `docs/design-pattern-refactor-plans/macaca-framework.md`
- 现有 OpenSpec：
  - `openspec/changes/implement-agent-framework`
  - `openspec/changes/migrate-agent-construction-to-framework-primitives`
  - `openspec/changes/migrate-goal-pipeline-to-framework`
  - `openspec/changes/add-application-framework-service-v1`

## 2. AgentScope Java 2.0 能力清单

### 2.1 Core building blocks

AgentScope 2.0 的 core 由以下能力组成：

- `Agent` 接口族：`CallableAgent`、`StreamableAgent`、`ObservableAgent`。
- `ReActAgent`：统一推理-行动循环，支持 `call`、`streamEvents`、`observe`。
- `RuntimeContext`：per-call 上下文，携带 `sessionId`、`userId`、`SessionKey`、字符串属性、类型化属性。
- `AgentState`：持久化运行时状态，包含上下文、任务状态、工具状态、Plan Mode 状态、权限状态等。
- `Session`：`InMemorySession`、`JsonSession`，扩展支持 MySQL、Redis。
- `Msg` 与 role 子类：`UserMessage`、`AssistantMessage`、`SystemMessage`、`ToolResultMessage`。
- `ContentBlock`：`TextBlock`、`DataBlock`、`ImageBlock`、`AudioBlock`、`VideoBlock`、`ThinkingBlock`、`ToolUseBlock`、`ToolResultBlock`、`HintBlock`。
- `AgentEvent`：生命周期、模型调用、文本/思考/data 块、工具调用、工具结果、HITL、外部执行、停止/中断等事件。
- `MiddlewareBase`：`onAgent`、`onReasoning`、`onActing`、`onModelCall`、`onSystemPrompt`。
- `Toolkit`：工具注册、反射工具、工具组、meta tool、MCP tool、skill tool、上下文注入、tool emitter。
- `PermissionEngine`：工具执行前的 allow / ask / deny 决策，支持 permission mode 与规则。
- `ModelRegistry` 与 provider credential：`dashscope`、`openai`、`anthropic`、`gemini`、`ollama` 等 model id 解析、重试、fallback model。
- `Formatter`：provider-specific message/request/response 转换。
- `PlanNotebook`：Plan、SubTask、PlanState、PlanStorage、PlanHintMiddleware。
- `State` 包重构：v1 `StateModule`、`StatePersistence` 删除，状态由 `AgentState + Session` 承接。
- `GracefulShutdown`：活跃请求跟踪、部分推理策略、shutdown state save。
- `Tracer`：Noop、Otel tracing middleware、TracerRegistry。

### 2.2 Harness engineering

`HarnessAgent` 是 2.0 的工程化重点，它在 ReActAgent 外层叠加长期运行能力：

- Workspace：`AGENTS.md`、`MEMORY.md`、skills、subagents、tools config、session log。
- Session/Context：同 sessionId 跨进程、跨机器恢复完整上下文。
- Memory：对话压缩、事实流水账、`MEMORY.md`、memory tools、后台 flush/maintenance。
- Context compaction：摘要压缩、大工具结果落盘、context overflow 兜底重试、参数预截断。
- Filesystem：本地、共享存储、沙箱、远端 filesystem spec。
- Sandbox：Docker、Kubernetes、E2B、Daytona、AgentRun、本地 snapshot、远端 snapshot、并发 lease。
- Skill：classpath / filesystem / git / nacos / mysql repository，动态加载，四层合成，自学习和审核闸门。
- Subagent：workspace 声明子 agent，同步 `agent_send` 与后台 `agent_spawn`，任务记录和终态提醒。
- Plan Mode：只读规划阶段、计划文件持久化、HITL 退出执行阶段。
- MCP：workspace `tools.json` 声明 server、白名单/黑名单、tool filter。
- Store：in-memory、Redis、JDBC dialect，用于远端 filesystem 和长期状态。

### 2.3 Extensions and integrations

上游 2.0 的扩展面：

- Protocol：A2A client/server、Agent Protocol、AG-UI。
- Ecosystem：Chat Completions Web、Studio、Kotlin、Training。
- Infrastructure：Higress、Nacos、RocketMQ、Scheduler。
- RAG：Simple、Bailian、Dify、RAGFlow、Haystack。
- Memory：Mem0、ReMe、Bailian。
- Session：MySQL、Redis。
- Skill repository：Git、MySQL、Nacos。
- Spring Boot starters：A2A、AG-UI、Nacos、Chat Completions、Admin、Core。

## 3. 当前 macaca-framework 差距

当前 `crates/runtime/macaca-framework/src` 已具备部分基础：

- `agent.rs`：`Agent`、`Hook`、`HookedAgent`。
- `react_agent.rs`：ReAct loop。
- `message.rs`：`Msg`、`ContentBlock`。
- `model.rs`、`formatter.rs`：model / formatter 抽象。
- `tool.rs`：`Toolkit`、tool handler、middleware、tool group。
- `memory.rs`：working memory、long-term memory、compression。
- `plan.rs`：PlanNotebook。
- `session.rs`：SessionStore。
- `mcp.rs`：MCP stdio/http client 与 tool handler。
- `a2a.rs`：A2A 数据结构和格式转换。
- `construction.rs`：traced factory / execution launcher 抽象。

关键缺口：

- 仍保留 v1 风格 `Hook`，缺少 2.0 `MiddlewareBase` 五阶段洋葱模型。
- 缺少 `RuntimeContext` 作为 per-call 一等上下文。
- 缺少 `AgentState + SessionKey + Session` 的 v2 状态模型；当前 `StateModule` 更接近 v1。
- 缺少完整 `AgentEvent` 类型体系和 `streamEvents` 等价接口。
- `Msg` 角色约束、block id、`DataBlock`、`HintBlock`、tool state、usage、generate reason 不完整。
- Tool permission/HITL/external execution 不是 framework 内生机制。
- ModelRegistry、credential、retry/fallback、execution config 不完整。
- Harness 能力基本缺失，尤其 workspace、filesystem/sandbox、subagent、skill repository、Plan Mode、context compaction、大工具结果卸载。
- `Pipeline` 仍存在，但 AgentScope 2.0 已移除 `pipeline.*`，Macaca 应保留兼容层但不作为新编排主路径。
- 多个文件超过 500 行红线，例如 `mcp.rs`、`memory.rs`、`formatter.rs`、`tool.rs`、`react_agent.rs`、`plan.rs`、`model_impls.rs`、`a2a.rs`、`pipeline.rs`、`message.rs`，升级前必须纳入模块拆分计划。

## 4. 可插拔框架设计

### 4.1 两层 ABI：Macaca stable ABI + framework adapter ABI

设计原则：Macaca 消费者只依赖 Macaca stable ABI，不依赖 AgentScope 版本。

建议新增/收敛为三类 crate 边界：

- `macaca-proto`：稳定 DTO、command/result、event、error、capability identifiers。
- `macaca-framework-contract`：agent framework 的稳定 Rust trait 和数据模型，不绑定 AgentScope。
- `macaca-framework-agentscope2`：AgentScope 2.0 等价实现，作为可替换 provider。

当前 `macaca-framework` 可先演进为 `macaca-framework-contract + default implementation` 的过渡形态；待稳定后拆 crate。

对外稳定接口：

- `AgentRuntimeProvider`
- `AgentRuntimeFactory`
- `AgentHandle`
- `AgentCallCommand`
- `AgentCallResult`
- `AgentEventStream`
- `RuntimeContext`
- `AgentStateSnapshot`
- `FrameworkHealth`
- `FrameworkDescriptor`
- `FrameworkCapabilityMatrix`

具体 AgentScope 2.0 映射只在 adapter provider 内部存在。

### 4.2 关键设计模式

- Facade：`SystemFacade` 和 focused SDK client 只暴露 Macaca command/result。
- Command：所有跨服务调用用 typed command/result，不暴露 framework 内部对象。
- Adapter/Bridge：AgentScope2 provider、未来其他 framework provider、Java/Python/remote framework 都作为 adapter。
- Strategy：model routing、tool permission、sandbox backend、session backend、event sink、formatter、skill repository 都可替换。
- Decorator：trace、audit、policy、resource、entitlement、metering 通过 service runtime decorator 包在 framework provider 外层。
- State：agent call、tool call、HITL、external execution、shutdown、sandbox lease、session restore 都建模为状态机。
- Observer：`AgentEvent`、trace、audit、web SSE、AG-UI、A2A、Chat Completions 都由事件观察/转换产生。
- Memento：`AgentStateSnapshot`、session checkpoint、sandbox snapshot、tool pending state、plan state 可恢复。
- Specification：capability admission、permission rule、tool rule、workspace path rule、sandbox policy、dependency gate 可执行化。
- Abstract Factory：framework provider 和 host-side composition 只在 `macaca-runtime-host` 或 approved composition root 构造。

### 4.3 分层所有权

| 能力 | 所属层 | 规则 |
|---|---|---|
| AgentScope2 等价核心 trait/DTO | Runtime/Application framework contract | Provider-neutral，不含具体 provider 名称 |
| ReAct loop 默认实现 | Runtime framework provider | 可替换，不进入 kernel |
| RuntimeContext | Framework contract + proto | 每次调用必带 trace/session/tenant/application/task |
| AgentState/Session | Framework contract + Memory/Context service bridge | 持久化由 service/provider 承担 |
| Permission/HITL | Policy service + framework middleware bridge | tool side effect 前必须执行 |
| Tool/MCP/Skill | Tool/Skill/MCP services + framework adapter | framework 只能编排声明能力 |
| Workspace/Filesystem/Sandbox | Driver/Filesystem/Sandbox service + harness adapter | 不允许直接写宿主路径绕过 policy |
| A2A/AG-UI/Chat Completions | Gateway/protocol service + event adapter | 不成为 framework core 反向依赖 |
| Web/CLI | Shell | 只订阅事件和发 command |

## 5. Apache-2.0 合规规则

AgentScope Java 2.0 使用 Apache-2.0。所有从上游设计、结构、代码语义改写到 Macaca 的 Rust 源文件必须：

- 文件头部加入 SPDX：
  - `// SPDX-License-Identifier: Apache-2.0`
- 文件头部加入来源说明：
  - `// Derived from AgentScope Java 2.0 concepts and APIs.`
  - `// Copyright 2024-2026 the original AgentScope author or authors.`
  - `// Licensed under the Apache License, Version 2.0.`
- 如果文件同时包含 Macaca 原创实现，保留 Macaca 自身版权说明，并明确“implementation adapted for Macaca Agent OS”.
- 不复制大段 Java 源码；用 Rust 惯用结构重写，并保留能力等价测试证明。
- 在仓库根或 `docs/` 增加第三方 notice 汇总，记录 AgentScope Java 2.0、版本、来源 URL、license。
- CI 增加 license header gate，至少覆盖 `crates/runtime/macaca-framework*` 和未来 `macaca-framework-agentscope2`。

## 6. 细粒度实施计划

### Phase 0：冻结基线与创建 OpenSpec

- [ ] 0.1 创建 OpenSpec change：`upgrade-framework-to-agentscope2`.
- [ ] 0.2 阅读 `openspec/project.md`、现有 `implement-agent-framework` 与 active changes，确认冲突。
- [ ] 0.3 在 OpenSpec 中增加或修改 capability：
  - `agent-framework`
  - `framework-runtime-provider`
  - `framework-event-stream`
  - `framework-session-state`
  - `framework-harness`
  - `framework-provider-contract`
- [ ] 0.4 写 `proposal.md`：说明为什么必须从 v1 风格升级到 2.0。
- [ ] 0.5 写 `design.md`：采用 stable ABI + adapter provider。
- [ ] 0.6 写 `tasks.md`：按本文 phase 拆任务。
- [ ] 0.7 写 delta specs，所有新增/修改 requirement 都要包含 scenario。
- [ ] 0.8 运行 `openspec validate upgrade-framework-to-agentscope2 --strict`。

### Phase 1：上游能力审计与映射矩阵

- [ ] 1.1 为 `agentscope-core` 建立 symbol inventory：agent、event、message、middleware、model、permission、session、state、tool、plan、tracing、shutdown。
- [ ] 1.2 为 `agentscope-harness` 建立 symbol inventory：workspace、filesystem、sandbox、memory、middleware、skill、subagent、store、tools。
- [ ] 1.3 为 `agentscope-extensions` 建立 module inventory：A2A、AG-UI、Agent Protocol、Chat Completions、RAG、Memory、Session、Skill、infra、training、studio。
- [ ] 1.4 建立 `AgentScope2 → Macaca crate/module` 映射表。
- [ ] 1.5 建立 `AgentScope2 → Macaca service boundary` 映射表。
- [ ] 1.6 建立 `AgentScope2 deprecated v1 APIs → Macaca compatibility path` 映射表。
- [ ] 1.7 建立能力覆盖矩阵，状态为 `missing / partial / equivalent / delegated-to-service / intentionally-compat-only`。

### Phase 2：模块拆分与文件红线修复

- [ ] 2.1 拆 `message.rs` 为 `message/mod.rs`、`message/block.rs`、`message/role.rs`、`message/builders.rs`、`message/validation.rs`、`message/event_reconstruction.rs`。
- [ ] 2.2 拆 `event` 新模块：`event/mod.rs`、`event/lifecycle.rs`、`event/model.rs`、`event/content.rs`、`event/tool.rs`、`event/hitl.rs`、`event/external.rs`、`event/codec.rs`。
- [ ] 2.3 拆 `react_agent.rs` 为 `react/mod.rs`、`react/loop.rs`、`react/reasoning.rs`、`react/acting.rs`、`react/finish.rs`、`react/pending.rs`。
- [ ] 2.4 拆 `tool.rs` 为 `tool/mod.rs`、`tool/registry.rs`、`tool/group.rs`、`tool/schema.rs`、`tool/execution.rs`、`tool/middleware.rs`、`tool/context.rs`、`tool/meta.rs`。
- [ ] 2.5 拆 `mcp.rs` 为 `mcp/mod.rs`、`mcp/client.rs`、`mcp/stdio.rs`、`mcp/http.rs`、`mcp/tool.rs`、`mcp/resources.rs`、`mcp/config.rs`。
- [ ] 2.6 拆 `memory.rs` 为 `memory/mod.rs`、`memory/working.rs`、`memory/session_view.rs`、`memory/long_term.rs`、`memory/compaction.rs`、`memory/eviction.rs`。
- [ ] 2.7 拆 `formatter.rs` 与 `model_impls.rs` 到 provider-specific 子模块。
- [ ] 2.8 拆 `plan.rs`、`a2a.rs`、`pipeline.rs`，确保每个文件小于 500 行。
- [ ] 2.9 每个拆分步骤只搬代码，不改行为，逐步跑 `cargo check -p macaca-framework`。

### Phase 3：Stable framework contract

- [ ] 3.1 新增 `FrameworkDescriptor`：name、version、upstream compatibility、capability matrix、license notice。
- [ ] 3.2 新增 `AgentRuntimeProvider` trait：build、call、stream_events、observe、interrupt、snapshot、restore、health。
- [ ] 3.3 新增 `AgentRuntimeFactory` trait：从 provider-neutral `AgentSpec` 构建 provider handle。
- [ ] 3.4 新增 `AgentHandle`：封装 provider-owned agent，不暴露具体类型。
- [ ] 3.5 新增 `AgentCallCommand`：messages、runtime_context、stream options、structured output、policy envelope。
- [ ] 3.6 新增 `AgentCallResult`：final message、generate reason、usage、state snapshot ref、event cursor。
- [ ] 3.7 新增 `FrameworkError`：unavailable、unsupported、denied、interrupted、max_iterations、tool_suspended、external_pending、provider_failure。
- [ ] 3.8 在 runtime-host composition root 注册默认 `AgentScope2FrameworkProvider`。
- [ ] 3.9 Web/CLI/SDK 只能通过 service/facade 调用 provider，不直接构造 provider。

### Phase 4：Message 2.0

- [ ] 4.1 将 `Msg` content 统一为有序 `Vec<ContentBlock>`；保留文本便捷构造但内部归一化为 `TextBlock`。
- [ ] 4.2 增加 role 子类型或 builder：User、Assistant、System、ToolResult。
- [ ] 4.3 增加严格 role validation：
  - user 仅允许 text/data/image/audio/video。
  - system 仅允许 text。
  - assistant 允许所有 block。
  - tool result 使用专门构造路径。
- [ ] 4.4 增加 `DataBlock`，兼容旧 `ImageBlock/AudioBlock/VideoBlock`。
- [ ] 4.5 增加 `HintBlock`。
- [ ] 4.6 为所有 block 增加稳定 `id`。
- [ ] 4.7 为 `ToolUseBlock` 增加 `ToolCallState`。
- [ ] 4.8 为 `ToolResultBlock` 增加 `ToolResultState`。
- [ ] 4.9 为 assistant message 增加 `usage` 与 `GenerateReason`。
- [ ] 4.10 增加 message helper：`text_content`、`content_blocks<T>`、`first_content_block<T>`、`has_content_blocks<T>`。
- [ ] 4.11 增加 serde roundtrip tests、role validation tests、v1 compatibility tests。

### Phase 5：AgentEvent 2.0 与 streamEvents

- [ ] 5.1 定义 `AgentEvent` 公共字段：event id、created_at、type、reply_id、trace context。
- [ ] 5.2 增加生命周期事件：agent start/end、max iters、request stop、interrupted。
- [ ] 5.3 增加 model call start/end。
- [ ] 5.4 增加 text/thinking/data block start/delta/end。
- [ ] 5.5 增加 tool call start/delta/end。
- [ ] 5.6 增加 tool result start/text delta/data delta/end。
- [ ] 5.7 增加 HITL 事件：require user confirm、user confirm result。
- [ ] 5.8 增加 external execution 事件：require external execution、external execution result。
- [ ] 5.9 实现 event stream → final assistant message 的 accumulator。
- [ ] 5.10 为 `Agent` 增加 provider-neutral `stream_events`；旧 `reply` 保留为 consume stream 的兼容 facade。
- [ ] 5.11 Web SSE、AG-UI、Chat Completions、trace EventLog 从同一 event stream 转换。
- [ ] 5.12 增加事件序列 golden tests，确保 start/delta/end 可重建消息。

### Phase 6：RuntimeContext 与 AgentState/Session

- [ ] 6.1 新增 `RuntimeContext`：session_id、user_id、tenant_id、application_id、task_id、trace_id、session_key、string extra、typed extensions。
- [ ] 6.2 所有 agent call/tool call/middleware/model call 必须携带 `RuntimeContext`。
- [ ] 6.3 新增 `SessionKey`：可组合 user/agent/session/tenant/application scope。
- [ ] 6.4 新增 `AgentState`：context messages、pending tools、permission state、plan mode state、task context、read cache、custom namespaces。
- [ ] 6.5 新增 `Session` trait：load、save、delete、list、snapshot、health。
- [ ] 6.6 将当前 `SessionStore` 作为 compat adapter，逐步迁到 `Session`。
- [ ] 6.7 实现 in-memory session 与 JSON file session。
- [ ] 6.8 通过 Memory/Context/Persist services 接入 durable session provider。
- [ ] 6.9 移除新代码对 `StateModule` 的依赖；保留 `StateModule` 仅作为 v1 compat。
- [ ] 6.10 增加跨调用恢复、跨进程 JSON restore、session isolation tests。

### Phase 7：Middleware 2.0

- [ ] 7.1 定义 `MiddlewareBase` 五阶段：
  - `on_agent`
  - `on_reasoning`
  - `on_acting`
  - `on_model_call`
  - `on_system_prompt`
- [ ] 7.2 实现洋葱型调用链和 pipeline 型 system prompt 链。
- [ ] 7.3 增加 middleware ordering：用户 middleware 在 provider 内置 middleware 前执行，service decorators 在 provider 外执行。
- [ ] 7.4 用 `LegacyHookDispatcher` 将旧 `Hook` 桥接到 middleware。
- [ ] 7.5 将 tracing、task reminder、structured output reminder、long-term memory、RAG compat 等迁到 middleware。
- [ ] 7.6 更新 ReAct loop，所有 reasoning/acting/model call 都经过 middleware。
- [ ] 7.7 增加 order tests、error propagation tests、legacy hook compatibility tests。

### Phase 8：ReActAgent 2.0

- [ ] 8.1 用 Template Method 拆分主循环：input ingest、pending external event、next action、reasoning、acting、finish、suspend、max iterations。
- [ ] 8.2 支持 `call(messages, RuntimeContext)`。
- [ ] 8.3 支持 `stream_events(messages, RuntimeContext)`。
- [ ] 8.4 支持 `observe(messages, RuntimeContext)`。
- [ ] 8.5 支持 pending tool recovery。
- [ ] 8.6 支持 user confirm / external execution 暂停后恢复。
- [ ] 8.7 支持 serial/concurrent tool execution config。
- [ ] 8.8 支持 stopOnReject。
- [ ] 8.9 支持 structured output reminder 与结构化输出调用。
- [ ] 8.10 支持 graceful shutdown partial reasoning policy。
- [ ] 8.11 增加 deterministic fake model/tool tests，覆盖所有 generate reason。

### Phase 9：Model 2.0

- [ ] 9.1 新增 `ModelRegistry`，按 provider-neutral model id 解析。
- [ ] 9.2 新增 `ModelCredential` 与 `ModelCard` DTO。
- [ ] 9.3 支持内置 provider id 格式，但 provider 名称只留在 provider adapter/service 层。
- [ ] 9.4 支持 `GenerateOptions`：temperature、top_p、max_tokens、stop、response_format、tool_choice、extra。
- [ ] 9.5 支持 `ExecutionConfig`：timeout、retry、backoff、concurrency、fallback model。
- [ ] 9.6 更新 formatter：OpenAI、DashScope、Anthropic、Gemini、Ollama。
- [ ] 9.7 model call 事件必须包含 sanitized request metadata 和 usage，不泄露 raw prompt/provider payload。
- [ ] 9.8 增加 retry/fallback tests、formatter roundtrip tests、sanitization tests。

### Phase 10：Toolkit 与 Permission 2.0

- [ ] 10.1 定义 `AgentTool` / `ToolBase` 等价 Rust trait。
- [ ] 10.2 完善 `Toolkit` 注册、schema、tool group、scope、preset args、meta tool。
- [ ] 10.3 增加 `ToolExecutionContext` 到 `RuntimeContext` 的兼容桥。
- [ ] 10.4 支持 schema-only tool。
- [ ] 10.5 支持 streaming tool result 事件。
- [ ] 10.6 支持 tool suspend。
- [ ] 10.7 新增 `PermissionEngine`：allow、ask、deny。
- [ ] 10.8 新增 `PermissionMode` 与 `PermissionRule`。
- [ ] 10.9 将危险路径、shell、file、MCP、skill 工具执行纳入 permission。
- [ ] 10.10 HITL ask 生成 `RequireUserConfirmEvent`，确认后生成 `UserConfirmResultEvent` 并恢复。
- [ ] 10.11 tool side effect 前必须经过 policy/resource/entitlement/service decorators。
- [ ] 10.12 增加 permission matrix tests、HITL resume tests、tool group activation tests。

### Phase 11：MCP 2.0

- [ ] 11.1 对齐 stdio、SSE、HTTP/streamable HTTP transport config。
- [ ] 11.2 支持 protocol version 配置。
- [ ] 11.3 支持 enable/disable tool filter。
- [ ] 11.4 支持 HTTP headers/query params。
- [ ] 11.5 支持 sync/async client wrapper 等价。
- [ ] 11.6 支持 elicitation。
- [ ] 11.7 支持 list tools、remove client。
- [ ] 11.8 将 Higress semantic tool search 作为 gateway/tool service extension，不进入 framework core。
- [ ] 11.9 增加 MCP mock server integration tests。

### Phase 12：Plan Mode 与 planning tools

- [ ] 12.1 将 `PlanNotebook` 对齐 2.0 Plan/SubTask/State。
- [ ] 12.2 新增 `PlanStorage` trait 与 in-memory/default durable adapter。
- [ ] 12.3 新增 `PlanHintMiddleware`。
- [ ] 12.4 新增 Plan Mode 状态：只读规划、用户确认、退出规划、进入执行。
- [ ] 12.5 新增 planning tools：create/update/list/exit plan mode。
- [ ] 12.6 明确 PlanNotebook 仍是 agent 脑内计划，不替代 Macaca TaskBoard。
- [ ] 12.7 增加 Plan Mode HITL tests、TaskBoard boundary tests。

### Phase 13：Harness workspace

- [ ] 13.1 新增 `HarnessAgent` 薄包装，内部持有 ReActAgent 和 harness shared objects。
- [ ] 13.2 新增 `WorkspaceManager`：workspace root、AGENTS.md、MEMORY.md、tools.json、skills、subagents、plans、sessions。
- [ ] 13.3 新增 `WorkspaceContextMiddleware`，每轮重建 system prompt。
- [ ] 13.4 新增 `AtPathExpansionMiddleware`。
- [ ] 13.5 新增 workspace index 和 path policy。
- [ ] 13.6 workspace 文件读写必须通过 filesystem abstraction，不直接绕过 sandbox/policy。
- [ ] 13.7 增加 workspace injection tests、path policy tests、多租户 path isolation tests。

### Phase 14：Harness context and memory

- [ ] 14.1 新增 `CompactionMiddleware`。
- [ ] 14.2 新增 `ToolResultEvictionMiddleware`，超大工具结果落盘，仅保留占位符。
- [ ] 14.3 新增 context overflow retry。
- [ ] 14.4 新增 pre-compaction arg truncation。
- [ ] 14.5 新增 `MemoryFlushMiddleware` 和后台 maintenance。
- [ ] 14.6 新增 memory tools：get/search/append。
- [ ] 14.7 对接 Macaca Memory/Context services；上游 Mem0/ReMe/Bailian 作为 optional providers。
- [ ] 14.8 增加 compaction golden tests、大工具结果 offload tests、memory service unavailable tests。

### Phase 15：Harness filesystem and sandbox

- [ ] 15.1 定义 `AbstractFilesystem` 等价 trait：read/write/edit/list/glob/grep/upload/download/execute。
- [ ] 15.2 实现 local filesystem provider。
- [ ] 15.3 实现 sandbox-backed filesystem provider adapter。
- [ ] 15.4 定义 filesystem spec：local、remote、docker、kubernetes、e2b、daytona、agentrun。
- [ ] 15.5 定义 `IsolationScope`：session/user/agent/org/tenant/application。
- [ ] 15.6 定义 sandbox client/manager/state/lease/snapshot traits。
- [ ] 15.7 Docker/Kubernetes/E2B/Daytona/AgentRun 作为 optional driver/sandbox service providers。
- [ ] 15.8 snapshot/restore 对接 Memento，支持进程重启恢复。
- [ ] 15.9 增加 absent provider unavailable tests、lease concurrency tests、snapshot restore tests。

### Phase 16：Harness skills

- [ ] 16.1 对齐 `AgentSkill`、`RegisteredSkill`、`SkillRepository`、`SkillRegistry`。
- [ ] 16.2 支持 classpath/filesystem/git/nacos/mysql repository 等价，非本地 provider 走 service/plugin。
- [ ] 16.3 支持 Markdown `SKILL.md` progressive disclosure。
- [ ] 16.4 支持 dynamic skill middleware。
- [ ] 16.5 支持 skill prompt builder、skill load tool、lazy resources。
- [ ] 16.6 支持 skill curation：candidate、security scan、approval gate、visibility filter、usage store、promoter。
- [ ] 16.7 将现有 Macaca skill governance service 作为主边界，framework 仅提供 harness adapter。
- [ ] 16.8 增加 skill load tests、repository unavailable tests、curation policy tests。

### Phase 17：Harness subagents and background tasks

- [ ] 17.1 对齐 subagent declaration、spec loader、factory、manager。
- [ ] 17.2 支持 workspace subagent specs。
- [ ] 17.3 支持 `agent_send` 同步委派。
- [ ] 17.4 支持 `agent_spawn` 后台任务。
- [ ] 17.5 支持 task repository：default/workspace/remote Agent Protocol。
- [ ] 17.6 支持后台任务状态反向 system reminder。
- [ ] 17.7 子 agent event stream gap 要用 Macaca event source 字段补齐，避免上游当前 gap 影响 UI/trace。
- [ ] 17.8 增加 subagent stream forwarding tests、background task resume tests。

### Phase 18：Protocol and extension adapters

- [ ] 18.1 A2A client/server 对齐 AgentCard、message、task、artifact、well-known resolver、Nacos resolver。
- [ ] 18.2 AG-UI adapter 从 `AgentEvent` 转换，不直接依赖 agent internals。
- [ ] 18.3 Agent Protocol adapter 走 gateway/protocol service。
- [ ] 18.4 Chat Completions Web adapter 从 `AgentEvent` 生成 OpenAI-compatible chunks。
- [ ] 18.5 Studio/Telemetry adapter 只消费 sanitized trace/event。
- [ ] 18.6 Training adapter 作为 optional ecosystem service，不进入 framework core。
- [ ] 18.7 Scheduler/RocketMQ/Nacos/Higress 均作为 infrastructure service/provider。
- [ ] 18.8 增加 protocol conversion golden tests。

### Phase 19：RAG and long-term memory compatibility

- [ ] 19.1 上游 2.0 中 RAG/LongTermMemory 仍处于推进/兼容状态，Macaca 不应把 deprecated core RAG 作为新主路径。
- [ ] 19.2 保留 RAG compat adapter：Knowledge、RetrieveConfig、RAGMode、GenericRAGMiddleware。
- [ ] 19.3 新主路径走 Macaca Context/Memory/Retrieval services。
- [ ] 19.4 Bailian/Dify/RAGFlow/Haystack/Mem0/ReMe 作为 provider adapters。
- [ ] 19.5 增加 deprecated API warning tests 和 service boundary tests。

### Phase 20：Runtime-host serviceization

- [ ] 20.1 在 `macaca-runtime-host` 构造 default framework provider。
- [ ] 20.2 给 framework service 增加 descriptor、health、snapshot、commands、structured errors。
- [ ] 20.3 每个 framework service call 必带 trace context。
- [ ] 20.4 policy/resource/entitlement/metering decorator 在 provider 外层执行。
- [ ] 20.5 provider absent 时返回 unavailable，不 crash、不假成功。
- [ ] 20.6 Web/CLI 不得再直接构造 framework provider。
- [ ] 20.7 更新 Route C dependency gates。

### Phase 21：消费者兼容迁移

- [ ] 21.1 保留当前 `Agent::reply`、`Hook`、`SessionStore`、`Pipeline` 兼容入口，标记 deprecated。
- [ ] 21.2 给 web framework runner 切到 `AgentRuntimeProvider.stream_events`。
- [ ] 21.3 给 task planner/review 切到 stable `AgentCallCommand`。
- [ ] 21.4 给 goal pipeline 切到 stable `AgentEvent` 和 `AgentCallResult`。
- [ ] 21.5 给 A2A/AG-UI/Chat Completions adapters 切到 event conversion。
- [ ] 21.6 每迁移一个消费者，增加一组 compatibility tests。
- [ ] 21.7 消费者迁移完成后再删除 deprecated 直连路径。

### Phase 22：验证与能力等价

- [ ] 22.1 建立 AgentScope 2.0 doc capability matrix，并要求 `equivalent/delegated` 覆盖率 100%。
- [ ] 22.2 建立 upstream examples parity tests：quickstart、ReAct、MCP、tool、permission、streamEvents、session、Harness workspace、skill、subagent、Plan Mode。
- [ ] 22.3 建立 Macaca OS acceptance gates：
  - YAML/WASM/GenUI apps still run。
  - `/api/chat/v2` session creation/recovery 不回退。
  - task board session isolation 不回退。
  - trace/audit replay 保持可重放。
  - optional provider absent 返回 structured unavailable。
  - no provider/app/model/driver/gateway name hardcoding below app/provider layer。
  - logs/snapshots sanitized。
- [ ] 22.4 运行：
  - `cargo check -p macaca-framework`
  - `cargo test -p macaca-framework`
  - `cargo check -p macaca-runtime-host -p macaca-web -p macaca-integration-tests`
  - `cargo test -p macaca-integration-tests -- --nocapture`
  - dependency-boundary tests
  - license header tests
  - OpenSpec strict validation

## 7. 升级跟随机制

为降低未来 AgentScope 2.x/3.x 升级冲击，建立以下机制：

- Upstream snapshot：每次升级记录上游 git commit/tag、doc hash、module inventory、API inventory。
- Capability matrix：`docs/agentscope-java-compatibility-matrix.md` 持续维护。
- Contract tests：Macaca stable ABI tests 不随上游变更，provider adapter tests 随上游补齐。
- Adapter versioning：`FrameworkDescriptor.upstream_version` 与 `FrameworkDescriptor.contract_version` 分离。
- Feature gates：新 upstream 能力先进入 provider feature，不直接改变 stable ABI。
- Compatibility facade：旧 Macaca 消费者通过 facade 获得同等行为，内部 adapter 自行处理 upstream API 差异。
- Deprecation window：至少两阶段，先 bridge + warn，再迁移消费者，再删除。
- Boundary gates：CI 禁止 shell/sdk/kernel 依赖 provider implementation。
- License gates：所有 derived/adapted source 强制 Apache-2.0 header。

## 8. 建议的最终目录形态

```text
crates/runtime/
├── macaca-framework-contract/
│   └── src/
│       ├── agent/
│       ├── context/
│       ├── event/
│       ├── message/
│       ├── model/
│       ├── session/
│       ├── tool/
│       └── provider.rs
├── macaca-framework-agentscope2/
│   └── src/
│       ├── react/
│       ├── middleware/
│       ├── harness/
│       ├── mcp/
│       ├── plan/
│       ├── compatibility/
│       └── provider.rs
└── macaca-framework/
    └── src/lib.rs   # Transitional facade/re-export during migration
```

如果暂时不拆 crate，至少先在当前 crate 内形成同等模块边界：

```text
crates/runtime/macaca-framework/src/
├── contract/
├── provider/agentscope2/
├── compatibility/v1/
├── message/
├── event/
├── context/
├── session/
├── middleware/
├── react/
├── tool/
├── mcp/
├── harness/
├── protocol/
└── service_bridge/
```

## 9. 风险与控制

| 风险 | 控制 |
|---|---|
| 1:1 迁移范围过大 | 按 phase + OpenSpec tasks 切片，每片可验证、可回滚 |
| AgentScope 细节泄漏给消费者 | stable ABI + provider adapter，消费者只见 Macaca command/event |
| framework 变成 OS 语义 owner | 所有 serviceized 能力走 service boundary，framework 只编排 |
| Harness 文件/沙箱绕过 policy | filesystem/sandbox 必须作为 service/driver provider，side effect 前 policy |
| 事件系统影响 Web/SSE | 先实现 event accumulator 和 golden tests，再迁移 SSE |
| 旧调用路径双轨长期存在 | deprecation window 后用 dependency gate 删除 direct path |
| 上游 Apache-2.0 合规遗漏 | license header gate + notice 文档 + derived source inventory |
| 大文件继续膨胀 | Phase 2 先拆模块，CI 加 500 行红线检查 |

## 10. 第一批应立即执行的任务

1. 创建 OpenSpec change `upgrade-framework-to-agentscope2`。
2. 建立 `docs/agentscope-java-compatibility-matrix.md`。
3. 建立 source inventory 脚本，输出 AgentScope Java 2.0 package/class/method/module 清单。
4. 先拆 `message.rs`、`event`、`react_agent.rs` 三个核心模块，为 2.0 message/event/streamEvents 做准备。
5. 引入 `RuntimeContext`、`AgentEvent`、`MiddlewareBase` 三个核心 contract，但先不迁移所有消费者。
6. 用 fake model + fake tool 写 ReAct 2.0 parity tests。
7. 在 runtime-host 增加 framework provider descriptor/health/snapshot 骨架。
8. 加 license header gate 和 derived source notice。

以上 8 项完成后，再进入完整 Harness 改写。这样可以先稳定“可替换 framework ABI”，再持续补齐 AgentScope 2.0 的全部工程化能力。
