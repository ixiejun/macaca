# Change: Implement macaca-framework — Rust 版 AgentScope 完整实现

## Why

Macaca Agent OS 在 Goal-Task 全链路（create_goal → 分解 → 执行 → 审查 → 完成）上花费了 2 周仍无法稳定跑通，根本原因是**缺少一个成熟的底层 Agent 框架**。当前各 crate 分散实现了 Agent 生命周期的各个片段（`macaca-runtime` 做循环、`macaca-task` 做任务、`macaca-tools` 做工具、`macaca-memory` 做记忆），但缺少统一的抽象层来协调它们。

AgentScope（阿里巴巴达摩院开源）是一个经过生产验证的 Agent 框架，提供了完整的 Agent 抽象体系：

1. **StateModule 自描述序列化** — 所有组件自动追踪状态、支持序列化/恢复，解决了 Macaca 中状态散落在 redb/内存/AppState 中的问题
2. **ContentBlock 类型系统** — 7 种消息块类型（Text/ToolUse/ToolResult/Thinking/Image/Audio/Video），比 Macaca 当前的字符串消息更有表达力
3. **ReActAgent 核心循环** — 统一的推理-行动循环，内置记忆压缩、工具分组、规划笔记本，取代 Macaca 散布在 `agentic_loop.rs` + `chat_orchestrator.rs` 中的 ad-hoc 逻辑
4. **Formatter 层分离** — 将消息格式化从模型调用中独立出来，解决 Macaca 在 `openai_compatible.rs` 中硬编码格式的问题
5. **标签化记忆系统** — 支持标签过滤、LLM 压缩摘要、多后端，取代 Macaca 的简单 token 截断
6. **Pipeline 编排原语** — Sequential/Fanout/MsgHub，提供比 PlanLoop/WorkerLoop 更灵活的多 Agent 协作模式
7. **PlanNotebook** — 单 Agent 内部规划能力，注册为工具让 LLM 自主使用，与 OS 层 TaskBoard 互补
8. **Hook 系统** — 系统化的 pre/post 钩子注入，替代 Macaca 的 ad-hoc 回调

## What Changes

在 `macaca/crates/` 下新建 `macaca-framework` crate，完整移植 AgentScope 的核心架构到 Rust。

### Phase 1: 核心原语 (Core Primitives)
- `Msg` 消息类型 + 7 种 `ContentBlock` enum
- `StateModule` trait + derive macro 自描述序列化
- `AgentId`/`SessionId` 标识类型

### Phase 2: Agent 抽象 (Agent Abstraction)
- `Agent` trait（reply/observe/interrupt 接口）
- Hook 系统（pre/post 钩子注入，类 trait wrapper 模式）
- `ReActAgent` — 完整的推理-行动循环实现
- `UserAgent` — 用户输入代理

### Phase 3: Model & Formatter (模型与格式化)
- `ChatModel` trait — 统一 LLM 调用抽象
- `ChatResponse` — 统一模型响应（含流式）
- `Formatter` trait — 消息格式转换层
- 内置 Formatter：OpenAI/DashScope/Anthropic

### Phase 4: Memory 记忆系统
- `WorkingMemory` trait — 短期工作记忆（标签系统）
- `InMemoryWorkingMemory` — 内存实现
- `LongTermMemory` trait — 长期记忆抽象
- `CompressionConfig` — LLM 驱动的记忆压缩

### Phase 5: Tool 工具系统
- `Toolkit` — 工具注册/发现/执行
- `ToolGroup` — 工具分组 + 动态激活
- `ToolMiddleware` trait — 工具执行中间件链
- `ToolResponse` — 结构化工具响应（支持流式）
- MCP 客户端集成

### Phase 6: Pipeline 编排
- `SequentialPipeline` — 串行编排
- `FanoutPipeline` — 扇出编排（并发/顺序）
- `MsgHub` — 消息广播（多 Agent 对话）
- `Pipeline` trait — 可扩展编排接口

### Phase 7: Plan 规划系统
- `Plan`/`SubTask` 数据模型（状态机）
- `PlanNotebook` — 注册为工具集的规划能力
- Hint 系统 — 根据规划状态自动生成引导

### Phase 8: Session 持久化
- `Session` trait — 跨会话状态持久化
- 与现有 `macaca-persist` (redb) 集成

### Phase 9: Tracing 可观测性
- OpenTelemetry 集成
- Agent/Model/Tool 级别 span 追踪
- 结构化属性记录

### Phase 10: 集成层
- 适配现有 `macaca-llm` 的 `LlmProvider` trait
- 适配现有 `macaca-tools` 的 `ToolSet` trait
- 替换 `macaca-runtime` 的 `AgenticLoop` 为 `ReActAgent`
- 为 `macaca-web` 提供新的 Agent 执行入口

### Phase 11（原）→ Phase 12: Goal-Task 链路重建
（编号顺延，见 tasks.md）

### Phase 11: A2A 协议 (Agent-to-Agent)
- `AgentCard` — Agent 服务描述（name/url/capabilities/skills）
- `A2AAgent` — 远程 Agent 客户端（将远程 A2A 服务包装为本地 Agent trait）
- `A2AFormatter` — AgentScope Msg ↔ A2A Message 双向转换
- `AgentCardResolver` trait — 服务发现（File/WellKnown HTTP/注册中心）
- `A2AServer` — 将本地 Agent 暴露为 A2A HTTP/SSE 端点（基于 axum）
- A2A Task 生命周期管理（submitted → working → completed/failed）

## Explicit Non-Goal

- **不实现实时语音** — RealtimeAgent/TTS 暂不在范围
- **不实现模型微调** — tune/tuner 模块不移植
- **不实现评估框架** — evaluate 模块后续单独处理
- **不替换 macaca-task** — OS 层 TaskBoard/PlanLoop/WorkerLoop 保留，macaca-framework 提供底层 Agent 能力

## Impact

- **新增 crate**: `macaca-framework`（预计 ~8,000-12,000 行 Rust）
- **修改 crate**: `macaca-runtime`（AgenticLoop 适配）、`macaca-web`（Agent 执行入口 + A2A Server 端点）、`macaca-llm`（Formatter 适配）
- **Cargo.toml**: workspace 成员新增
- **不影响**: HTTP API、前端、持久化 schema、PlanLoop/WorkerLoop 行为
