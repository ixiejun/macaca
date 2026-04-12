## Phase 1: 核心原语 (Core Primitives)

- [ ] 1.1 创建 `macaca-framework` crate 骨架（Cargo.toml、lib.rs、mod 声明）
- [ ] 1.2 实现 `ContentBlock` enum（Text/Thinking/ToolUse/ToolResult/Image/Audio/Video）+ serde 序列化
- [ ] 1.3 实现 `MsgContent`（String | Vec<ContentBlock>）+ `Msg` 结构体（id/name/content/role/metadata/timestamp）
- [ ] 1.4 实现 `Msg` 便捷构造方法（user/assistant/system/tool_result）和内容提取方法（get_text/get_tool_calls/strip_thinking）
- [ ] 1.5 实现 `StateModule` trait（state_dict/load_state_dict）+ 手动实现宏占位
- [ ] 1.6 定义核心标识类型（AgentId/SessionId）复用 macaca-proto 已有类型
- [ ] 1.7 cargo check + 单元测试：Msg 序列化往返、ContentBlock 类型判别、strip_thinking 正确性
- [ ] 1.8 **里程碑**: `macaca-framework` 可作为依赖被其他 crate 引用

## Phase 2: Agent 抽象 (Agent Abstraction)

- [ ] 2.1 定义 `Agent` trait（reply/observe/interrupt/name/id）
- [ ] 2.2 实现 `Hook` trait（pre_reply/post_reply/pre_observe/post_observe）
- [ ] 2.3 实现 `HookRegistry`（实例级 + 全局级 hook 注册/执行链）
- [ ] 2.4 实现 `HookedAgent<A: Agent>` wrapper（自动注入 hook 链到 reply/observe）
- [ ] 2.5 实现 `UserAgent`（从 stdin 或可插拔 InputSource 获取用户输入）
- [ ] 2.6 cargo test：Hook 执行顺序、HookedAgent 透明包装、UserAgent mock 输入

## Phase 3: Model & Formatter (模型与格式化)

- [ ] 3.1 定义 `ChatModel` trait（chat/chat_stream/name）
- [ ] 3.2 实现 `ChatResponse` 结构体（content: Vec<ContentBlock>, usage, id）
- [ ] 3.3 实现 `ChatUsage`（input_tokens/output_tokens/duration）
- [ ] 3.4 定义 `Formatter` trait（format: &[Msg] → Vec<Value>, parse_response: Value → ChatResponse）
- [ ] 3.5 实现 `OpenAiFormatter`（适配 OpenAI/兼容 API 的消息格式）
- [ ] 3.6 实现 `DashScopeFormatter`（适配 DashScope 的消息格式差异）
- [ ] 3.7 实现 `AnthropicFormatter`（适配 Anthropic 的消息格式）
- [ ] 3.8 实现 `LlmProviderAdapter`：将 `macaca_llm::LlmProvider` 适配为 `ChatModel`
- [ ] 3.9 cargo test：Formatter 格式化正确性、多提供商消息往返、Adapter 集成测试

## Phase 4: Memory 记忆系统

- [ ] 4.1 定义 `WorkingMemory` trait（add/get/delete/delete_by_mark/update_mark/size/clear/update_summary/get_with_summary）
- [ ] 4.2 实现 `TaggedMsg`（Msg + Vec<String> marks）
- [ ] 4.3 实现 `InMemoryWorkingMemory`（Vec<TaggedMsg> 存储 + 标签过滤）
- [ ] 4.4 实现 `CompressionConfig`（trigger_threshold/target_tokens/keep_recent/summary_model）
- [ ] 4.5 实现 `compress_memory()`（调用 LLM 生成结构化摘要，替换旧消息）
- [ ] 4.6 定义 `LongTermMemory` trait（record/retrieve + 工具注册接口）
- [ ] 4.7 实现 `LongTermMemoryMode`（StaticControl/AgentControl/Both）
- [ ] 4.8 为 `InMemoryWorkingMemory` 实现 `StateModule`（序列化/恢复记忆内容）
- [ ] 4.9 cargo test：标签添加/过滤/删除、压缩触发条件、StateModule 往返、记忆大小限制

## Phase 5: Tool 工具系统

- [ ] 5.1 定义 `ToolHandler` trait（execute/execute_streaming）
- [ ] 5.2 定义 `ToolMiddleware` trait（before/after）
- [ ] 5.3 实现 `RegisteredTool`（name/description/schema/handler/group/preset_args）
- [ ] 5.4 实现 `ToolGroup`（名称 + 工具名列表 + active 状态）
- [ ] 5.5 实现 `ToolResponse`（content: Vec<ContentBlock>, metadata, stream/is_last/is_interrupted）
- [ ] 5.6 实现 `Toolkit` 核心（register/unregister/call_tool/get_definitions/update_groups）
- [ ] 5.7 实现 `Toolkit` 中间件链执行（before → handler → after）
- [ ] 5.8 实现 `Toolkit` 工具分组动态激活/停用（set_extended_model 结构化输出）
- [ ] 5.9 实现 `ToolSetAdapter`：将 `macaca_tools::ToolSet` 适配为 `Toolkit`
- [ ] 5.10 实现 MCP 工具注册（从 `macaca-mcp` 的 MCP 客户端注册工具到 Toolkit）
- [ ] 5.11 cargo test：工具注册/调用、分组激活/停用、中间件链、preset_args 注入、Adapter 集成

## Phase 6: ReActAgent 实现

- [ ] 6.1 实现 `ReActAgent` 结构体（model/formatter/toolkit/memory/long_term_memory/plan_notebook/compression/max_iters）
- [ ] 6.2 实现 `ReActAgent::reply()`（记忆 → 检索 → 推理-行动循环 → 总结）
- [ ] 6.3 实现 `ReActAgent::_reasoning()`（构建 prompt → formatter → model.chat → 处理响应）
- [ ] 6.4 实现 `ReActAgent::_acting()`（toolkit.call_tool → 结果存入记忆 → 构建 ToolResultBlock）
- [ ] 6.5 实现流式输出支持（reply 返回 Stream<Msg> 变体）
- [ ] 6.6 实现中断支持（CancellationToken → interrupt → handle_interrupt）
- [ ] 6.7 实现 `ReActAgent` 的 `StateModule`（序列化 memory/toolkit/plan_notebook）
- [ ] 6.8 cargo test：使用 MockChatModel 测试完整 ReAct 循环、工具调用链、中断恢复、最大迭代限制

## Phase 7: Pipeline 编排

- [ ] 7.1 定义 `Pipeline` trait（run: Msg → Msg）
- [ ] 7.2 实现 `SequentialPipeline`（串行执行 Agent 链）
- [ ] 7.3 实现 `FanoutPipeline`（扇出，支持并发/顺序模式）
- [ ] 7.4 实现 `MsgHub`（多 Agent 消息广播，自动剥离 ThinkingBlock，subscriber 管理）
- [ ] 7.5 实现 `stream_pipeline_messages()`（从 Pipeline 执行中流式消费消息）
- [ ] 7.6 cargo test：Sequential 链式传递、Fanout 并发执行、MsgHub 广播+Thinking剥离

## Phase 8: Plan 规划系统

- [ ] 8.1 实现 `Plan` 数据模型（id/name/description/subtasks/state/outcome + 状态机）
- [ ] 8.2 实现 `SubTask` 数据模型（name/description/state/outcome + 顺序约束）
- [ ] 8.3 实现 `PlanNotebook`（current_plan/historical_plans + 状态管理方法）
- [ ] 8.4 实现 `PlanNotebook::register_tools()`（create_plan/revise_plan/update_subtask/finish_subtask/finish_plan/view）
- [ ] 8.5 实现 `DefaultPlanToHint`（根据规划状态自动生成引导消息）
- [ ] 8.6 实现 `PlanNotebook` 的 `StateModule`
- [ ] 8.7 cargo test：Plan 状态转换、SubTask 顺序约束、工具注册正确性、Hint 生成逻辑

## Phase 9: Session & Tracing

- [ ] 9.1 定义 `Session` trait（save_state/load_state）
- [ ] 9.2 实现 `RedbSession`：基于 macaca-persist 的 redb 后端
- [ ] 9.3 实现 Agent + Session 集成（save_session_state/load_session_state 自动调用 StateModule）
- [ ] 9.4 集成 `tracing` crate + `tracing-opentelemetry`（Agent/Model/Tool 级别 span）
- [ ] 9.5 实现 `#[trace_reply]` / `#[trace_reasoning]` / `#[trace_tool]` 属性宏（自动 span 创建）
- [ ] 9.6 cargo test：Session 保存/恢复往返、StateModule 递归序列化、Tracing span 创建

## Phase 10: 集成与验证

- [ ] 10.1 在 macaca-web 中添加 `macaca-framework` 依赖
- [ ] 10.2 实现 `FrameworkAgentRunner`：基于 `ReActAgent` 的 Agent 执行器（与现有 `WebAgentRunner` 并存）
- [ ] 10.3 通过 LlmProviderAdapter + ToolSetAdapter 桥接现有 macaca-llm 和 macaca-tools
- [ ] 10.4 添加 HTTP 端点切换参数（`?engine=framework` vs 默认 legacy）
- [ ] 10.5 cargo check + cargo test 全 workspace 通过
- [ ] 10.6 集成测试：使用 ScriptedLlm 测试 ReActAgent → 工具调用 → 响应 全链路
- [ ] 10.7 E2E 验证：通过 HTTP API 使用 framework 引擎执行对话
- [ ] 10.8 **里程碑**: macaca-framework 可作为 macaca-web 的可选执行引擎使用

## Phase 11: A2A 协议 (Agent-to-Agent)

- [ ] 11.1 实现 A2A 核心数据类型：`AgentCard`、`AgentCapabilities`、`AgentSkill`（服务描述）
- [ ] 11.2 实现 A2A 消息类型：`A2AMessage`、`A2APart`（Text/File/Data）、`A2ARole`
- [ ] 11.3 实现 A2A 任务类型：`A2ATask`、`A2ATaskState`（Submitted→Working→Completed/Failed）、`A2AArtifact`
- [ ] 11.4 实现 `A2AFormatter`：Msg → A2AMessage 正向转换（ContentBlock → A2APart 映射）
- [ ] 11.5 实现 `A2AFormatter`：A2AMessage/A2ATask → Msg 反向转换（A2APart → ContentBlock 映射）
- [ ] 11.6 定义 `AgentCardResolver` trait + `FileCardResolver`（本地 JSON 文件加载）
- [ ] 11.7 实现 `WellKnownCardResolver`（HTTP GET `/.well-known/agent.json`）
- [ ] 11.8 实现 `A2AAgent`：实现 `Agent` trait，通过 HTTP 调用远程 A2A 服务（支持 SSE 流式响应）
- [ ] 11.9 实现 `A2AAgent` 的 observe() 消息缓冲（本地缓存，下次 reply 时合并发送）
- [ ] 11.10 实现 `A2AServer`：将本地 `Arc<dyn Agent>` 暴露为 axum HTTP/SSE 端点
- [ ] 11.11 实现 `A2AServer` 路由：`/.well-known/agent.json`（GET）、`/a2a/message/send`（POST）、`/a2a/message/stream`（POST SSE）
- [ ] 11.12 实现 A2A Task 生命周期管理（in-memory task store, 状态转换, artifact 收集）
- [ ] 11.13 在 macaca-web 中集成 A2AServer：可选挂载 A2A 端点到现有 axum Router
- [ ] 11.14 cargo test：A2AFormatter 双向转换正确性、AgentCard 序列化/反序列化、A2AAgent mock 调用、A2AServer 端点响应
- [ ] 11.15 集成测试：本地启动 A2AServer + A2AAgent 客户端，完成跨进程 Agent 对话

## Phase 12: Goal-Task 链路重建（基于 framework）

- [ ] 12.1 实现 `PlanNotebook` 与 macaca-task `TaskBoard` 的桥接（PlanNotebook 的 subtask 映射到 TodoItem）
- [ ] 12.2 重写 PlanLoop 消费者：使用 ReActAgent（而非 raw delegate_task）执行分解和审查
- [ ] 12.3 重写 WorkerLoop 消费者：使用 ReActAgent 执行任务（带 session context）
- [ ] 12.4 实现 Goal 状态机扩展（Pending → Decomposing → InProgress → Evaluating → Completed/Failed）
- [ ] 12.5 添加 delegate_task 错误处理 + Worker 超时保护
- [ ] 12.6 E2E 验证：create_goal → 分解 → 执行 → 审查 → 完成 全链路稳定运行
