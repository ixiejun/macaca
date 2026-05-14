# Macaca 架构参考

## Crate 目录（20 个 crate）

| Crate | 用途 | 关键类型 | 完成度 | 问题 |
|-------|------|----------|--------|------|
| **macaca-proto** | 共享类型、配置、错误 | `AgentId`, `TaskId`, `TodoItem`, `LlmMessage`, `ToolCall`, `MacacaConfig` | 100% | `orchestration.rs:243` 硬编码 agent 枚举 |
| **macaca-persist** | redb KV 存储、EventLog、快照管理 | `RedbStore`, `PersistStore` trait, `EventLog` | 100% | 无 |
| **macaca-llm** | LLM 抽象、多提供商路由、降级 | `LlmProvider` trait, `LlmRouter`, `ResilientLlmWrapper`, `CostTracker`, `RateLimiter` | 100% | 定价表硬编码 |
| **macaca-runtime** | AgenticLoop、上下文窗口、循环检测 | `AgenticLoop`, `PausableAgenticLoop`, `ContextWindowManager`, `LoopDetector`, `PermissionChecker` | 100% | 3 个 run 变体 60% 重复 |
| **macaca-task** | TaskBoard、PlanLoop、WorkerLoop、分解器 | `TaskBoard`, `TaskSpace`, `TodoStore`, `PlanLoop`, `WorkerLoop`, `LlmDecomposer`, `TaskScheduler` | 100% | `TaskTracker` 可能是死代码；`TaskQueue` 与 kernel 重叠 |
| **macaca-kernel** | 执行器、Fork 管理、审计、告警 | `Kernel`, `ApplicationExecutor`, `ForkManager`, `ExecutionQueue`, `AuditLogger`, `AlertManager` | 100% | 重复 TaskId/DelegatedTask |
| **macaca-tools** | 内置工具、编排工具、Todo 工具 | `Tool` trait, `ToolSet` trait, `DelegateTaskTool`, `CreateGoalTool`, `FileWriteTool`, `ShellTool` | 100% | 无 |
| **macaca-web** | Axum HTTP、SSE、agent runner、状态管理 | `AppState`, `WebAgentRunner`, `ActiveSession` | 100% | **routes.rs 4,993 行**；30+ coordinator 硬编码 |
| **macaca-app** | 应用模型、加载器、工作流引擎 | `AppManifest`, `AppLoader`, `AppRuntime`, `WorkflowEngine` | 95% | L2 WASM 存根 |
| **macaca-cli** | CLI 入口 | `Cli`, `Commands` | 100% | 无 |
| **macaca-gateway** | IM 网关（Telegram/Discord） | `ImAdapter` trait, `TelegramAdapter` | 30% | **未接入**服务启动 |
| **macaca-agent** | Agent trait、状态机 | `Agent` trait, `BasicAgent`, `AgentStateMachine` | 100% | 无 |
| **macaca-memory** | 三层记忆系统（会话/文件/向量） | `MemoryManager`, `SessionMemory`, `FileMemory` | 50% | **未接入** agent 执行路径 |
| **macaca-ipc** | 进程间通信 | `MessageSender` trait, `LocalBus`, `NatsBus` | 40% | **未接入** |
| **macaca-mcp** | MCP 客户端 | `McpClient`, `McpToolAdapter` | 20% | **未接入**工具加载 |
| **macaca-sdk** | 声明式 Agent SDK | `AgentConfig`, `AgentPersona`, `DeclarativeAgent` | 100% | 无 |
| **macaca-skill** | 技能发现与注册 | `SkillRegistry`, `SkillCatalog`, `SkillTool` | 100% | discovery.rs 有 dead_code |
| **macaca-driver** | 驱动框架 | `SoftwareDriver` trait, `DriverRegistry` | 100% | 无 |
| **macaca-driver-claude-code** | Claude Code CLI 驱动 | `ClaudeCodeDriver`, `ClaudeCodeConfig` | 100% | `dangerously_skip_permissions()` |
| **macaca-integration-tests** | 跨 crate 集成测试 | `ScriptedLlm` | 80% | 覆盖良好 |

## 依赖图

```
macaca-proto（基础 — 无内部依赖）
  ├── macaca-persist
  ├── macaca-llm
  ├── macaca-agent
  ├── macaca-sdk
  ├── macaca-ipc
  └── macaca-memory

macaca-tools（依赖 proto, persist, task）
macaca-runtime（依赖 proto, llm, tools）
macaca-task（依赖 proto, persist, llm）
macaca-kernel（依赖 proto, agent, llm, tools, persist）
macaca-app（依赖 proto, sdk, kernel, agent）
macaca-web（依赖全部 — 集成层）
macaca-cli（依赖 web）
```

## 层级分离

| 层级 | 位置 | 内容 |
|------|------|------|
| OS 底座 | `macaca/crates/macaca-*` | 通用调度、执行、持久化、LLM 抽象 |
| 应用配置 | `examples/apps/{name}/personas/` | Agent 身份、工具、路由策略 |
| 用户配置 | `config/default.toml` | 端口、模型、API 密钥、预算 |

## PlanLoop/WorkerLoop 生命周期

### 启动
- `ensure_plan_and_worker_loops()` 调用时机：
  - `post_chat`（首次聊天请求时）
  - 服务启动时（为每个已注册应用自动启动）
- 创建：1 个 PlanLoop + N 个 WorkerLoop（每个 worker agent 一个）
- 在 `plan_loop_handles` / `worker_loop_handles` 中存储 shutdown handle

### TERMINATE
- 设置 shutdown flag
- **从 map 中移除** handle（允许重启）
- 移除 PlanLoop waker
- 取消所有非终态任务 + 目标

### 重启
- 下次 `post_chat` 或 `create_goal` 触发 `ensure_plan_and_worker_loops`
- `already = false`（handle 已移除）→ 创建全新循环

## WorkerLoop 消费者中的任务委派

```
WorkerEvent::TaskClaimed → 构建 prompt → 更新 agent 状态为 Working
  → executor.delegate_task() → 每 3 秒轮询结果
  → 成功? → submit_for_review → 唤醒 PlanLoop
  → 失败（任务错误）? → mark_failed
  → 失败（LLM/委派错误）? → 重置为 Pending（重试）
  → 更新 agent 状态为 Idle
```

## AgenticLoop 内部结构

`macaca-runtime/src/agentic_loop.rs` 中的三个变体：
1. `run()` — 基础循环，无事件，无暂停
2. `run_with_events()` — 每步 emit `AgentExecutionEvent`
3. `run_with_pause()` — 事件 + `PausableAgenticLoop` 暂停/恢复

每次迭代：
1. 用 messages + 工具定义调用 LLM
2. 如果响应包含 tool_calls → 执行每个工具 → 追加结果 → 继续
3. 如果无 tool_calls → 最终响应 → 退出
4. 检查 max_iterations / 循环检测器 / 取消信号

## 技术债务优先级

### P0 — 架构风险
- `routes.rs` 4,993 行 → 拆分为 chat_orchestrator、loop_manager、sse、session
- AppState 27 字段 → 分组为 PersistenceState、LoopState、SessionState、AppConfig
- 30+ 处 "coordinator" 硬编码 → 从 manifest 读取 entry_agent

### P1 — 重复代码
- `TaskId` 在 proto 和 kernel 两处定义（不同类型，同名）
- `DelegatedTask` 在 proto 和 kernel 两处定义（不同字段）
- AgenticLoop 3 个变体 60% 共享代码

### P2 — 未接入模块
- macaca-memory（agent 执行时无记忆检索）
- macaca-ipc（LocalBus 未使用）
- macaca-mcp（未在工具加载管线中）
- macaca-gateway（服务器未启动）

### P3 — 死代码
- `TaskTracker`（macaca-task，已被 TodoStore/TaskBoard 取代）
- `TaskQueue`（macaca-task，与 kernel ExecutionQueue 重叠）
- `Message.tsx`（前端，已被 ConversationTurn 替换）
- `renderValue()` 在 ConversationTurn 和 DelegatedAgentTrace 中重复
