## Phase 1: Adapter 桥接层

- [x] 1.1 在 `macaca-framework` 中新建 `src/adapter.rs` 模块
- [x] 1.2 实现 `LlmProviderAdapter`：将 `macaca_llm::LlmProvider` 适配为 `ChatModel` trait
  - `chat()`: framework JSON messages → `LlmMessage` → `provider.chat()` → `LlmResponse` → framework `ChatResponse`
  - 自动检测 provider 类型选择合适的 Formatter（OpenAI/DashScope/Anthropic）
- [x] 1.3 实现 `LegacyToolHandler`：将 `macaca_tools::Tool` 包装为 `ToolHandler` trait
  - `execute()`: 调用 `tool.execute(args)` → 转为 `ToolResponse`
  - `schema()`: 从 `ToolDefinition` 提取 JSON Schema
- [x] 1.4 实现 `ToolSetBridge::from_tool_set()`：批量注册所有 `macaca_tools::Tool` 到 `Toolkit`
- [x] 1.5 在 `macaca-framework/Cargo.toml` 添加 `macaca-llm` 和 `macaca-tools` 为可选依赖（feature = "macaca-compat"）
- [x] 1.6 cargo test：MockLlmProvider → LlmProviderAdapter → ChatModel 调用往返正确
- [x] 1.7 cargo test：Tool 注册 + 调用通过 Adapter 正确执行
- [x] 1.8 实现 `SingleToolAdapter`：将单个 `macaca_tools::Tool` 包装为 `ToolHandler`（per-agent 工具用）

## Phase 2: Framework Agent 工厂

- [x] 2.1 新建 `macaca-web/src/framework_runner.rs`
- [x] 2.2 实现 `FrameworkRunner::build_agent()`：根据 persona 目录构建 ReActAgent
  - 加载 `IDENTITY.md` → system prompt
  - 创建 `LlmProviderAdapter`（从 `state.llm`）
  - 构建 `Toolkit`（base tools via ToolSetBridge + per-agent todo tools via SingleToolAdapter）
  - 创建 `InMemoryWorkingMemory`
  - 注入 workspace 路径到 system prompt
- [x] 2.3 实现 `FrameworkRunner::build_coordinator()`：构建带 pause/resume 支持的 Coordinator agent
  - 通过 `PauseOnGoalMiddleware` 在 `create_goal` 工具调用后暂停
  - 通过 `resume_rx` 在目标完成后恢复
- [x] 2.4 实现 `SseEmitterHook`：通过 Hook 将 ReActAgent 事件桥接到 SSE channel
  - `pre_reply` → emit "thinking"
  - `post_reply` → emit "content" / "done"
- [x] 2.5 实现 `SseToolMiddleware`：通过 ToolMiddleware 将工具调用/结果桥接到 SSE
  - `before` → emit "tool_call"
  - `after` → emit "tool_result"
- [x] 2.6 在 `lib.rs` 添加 `pub mod framework_runner;`
- [x] 2.7 cargo check 通过
- [x] 2.8 将 `state.tools` 从 `Box<dyn ToolSet>` 改为 `Arc<dyn ToolSet>`（消除 unsafe 代码）

## Phase 3: Coordinator 迁移（可选引擎）

- [x] 3.1 添加 `ChatRequest.engine: Option<String>` 字段（"framework" | "legacy"，默认 "legacy"）
- [x] 3.2 实现 `post_chat_v2()`：基于 ReActAgent 的 SSE 流
  - 使用 `FrameworkRunner::build_coordinator()` 构建 agent
  - 注入 `SseEmitterHook` + `SseToolMiddleware` + `PauseOnGoalMiddleware`
  - 注册到 `active_sessions`（复用现有 pause/resume 基础设施）
  - 调用 `agent.reply(user_msg)` → 通过 hook 发射 SSE 事件
- [x] 3.3 注册 `/api/chat/v2` 路由到 `post_chat_v2`
- [x] 3.4 session 保存逻辑（post_chat_v2 完成后写入 StoredSession）
- [x] 3.5 cargo check 通过

## Phase 4: Worker 执行迁移

- [x] 4.1 在 `loop_manager.rs` 的 WorkerLoop TaskClaimed 消费者中替换 delegate_task + poll 为 framework 执行路径
- [x] 4.2 使用 `FrameworkRunner::build_agent()` 构建 worker ReActAgent（含 session context）
- [x] 4.3 直接调用 `agent.reply(task_prompt)` 替代 `delegate_task` + 结果轮询
  - 成功 → `board.submit_for_review(task_id, output)`
  - AgentError → `board.mark_failed(task_id, error)` 或 reset to Pending（瞬态错误）
- [x] 4.4 保留 agent status 更新（Working → Idle）
- [x] 4.5 保留 run_trace 事件发射
- [x] 4.6 同步修复 RetryTask 路径（同样使用 ReActAgent）
- [x] 4.7 添加 30 分钟超时保护（tokio::time::timeout）
- [x] 4.8 cargo check 通过

## Phase 5: Planner 分解/审查迁移

- [x] 5.1 在 GoalReady 消费者中使用 `FrameworkRunner::build_agent("planner")` 构建 planner agent
- [x] 5.2 直接调用 `planner.reply(decompose_prompt)` 替代 `delegate_task`
  - 成功 → planner 的 create_todo 工具调用自动创建子任务
  - 错误 → 记录日志（而非静默忽略）
- [x] 5.3 在 ReviewNeeded 消费者中同样使用 ReActAgent 执行审查
- [x] 5.4 在 NeedsMoreWork 消费者中使用 ReActAgent 创建后续任务
- [x] 5.5 cargo check 通过

## Phase 6: 验证 + 编译

- [x] 6.1 全 workspace `cargo check` 通过
- [x] 6.2 macaca-framework 测试通过（231 tests, 0 failures）
- [x] 6.3 macaca-web 测试通过（13/14, 1 pre-existing failure unrelated to migration）
- [x] 6.4 更新 tasks.md 所有任务标记完成
- [ ] 6.5 E2E 测试：`POST /api/chat/v2` 返回正确 SSE 流
- [ ] 6.6 E2E 测试：create_goal → PlanLoop 分解 → Worker 执行 → PlanLoop 审查 → 完成
- [ ] 6.7 将 `ChatRequest.engine` 默认值从 "legacy" 改为 "framework"（待 E2E 验证后）
- [ ] 6.8 标记 `run_agentic_stream` 系列函数为 `#[deprecated]`（待切换默认后）
