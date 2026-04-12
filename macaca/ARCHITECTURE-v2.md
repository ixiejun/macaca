# Macaca OS 完整技术架构文档

> **Status:** This file is a non-canonical deep reference / source draft. For the canonical system-definition document, read [`docs/SYSTEM_OVERVIEW.md`](docs/SYSTEM_OVERVIEW.md). This file still mixes current / intended / planned material and should not be treated as the primary architecture contract.

## 1. 系统定位

Macaca OS 是一个 **Agent 操作系统**，类比关系：
- Linux 管理进程 → Macaca OS 管理 Agent
- Linux 运行应用程序 → Macaca OS 运行 Application
- Linux 有系统调用 → Macaca OS 有 Agent 能力接口

核心设计原则：**任何软件都可以通过 Driver 被 Agent 操控**。

---

## 2. 架构全景图

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                              用户交互层 (User Interfaces)                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────────┐ │
│  │   Web UI    │  │  Telegram   │  │   Discord   │  │  其他 IM (Slack, WeChat...)  │ │
│  │  (Next.js)  │  │   Bot       │  │    Bot      │  │       (Gateway 扩展)         │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────────────┬──────────────┘ │
│         └─────────────────┴─────────────────┴──────────────────────┘                │
│                                    │                                                │
│                              HTTP / WebSocket                                        │
└────────────────────────────────────┼────────────────────────────────────────────────┘
                                     │
┌────────────────────────────────────┼────────────────────────────────────────────────┐
│                              平台服务层 (Platform Services)                          │
│                                    │                                                │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                         macaca-web (Web 服务器)                              │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐  │   │
│  │  │  REST API    │  │  SSE Stream  │  │    Chat      │  │  Session Store │  │   │
│  │  │  /api/apps   │  │/agents/stream│  │  /api/chat   │  │   (redb)       │  │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘  └────────────────┘  │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                       macaca-gateway (IM 网关)                               │   │
│  │     ┌─────────────┐      ┌─────────────┐      ┌─────────────────────────┐   │   │
│  │     │   Gateway   │◄────►│  Telegram   │      │   EventHandler (可插拔)  │   │   │
│  │     │   Manager   │◄────►│   Adapter   │      │  ┌─────────────────────┐  │   │   │
│  │     │             │◄────►│  Discord    │      │  │ TaskRequestHandler  │  │   │   │
│  │     │             │      │   Adapter   │      │  │ StatusQueryHandler  │  │   │   │
│  │     └─────────────┘      └─────────────┘      │  │ CommandHandler      │  │   │   │
│  │                                               │  └─────────────────────┘  │   │   │
│  │  事件类型: TaskRequest, StatusQuery, UserReply, Command                     │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────────┘
                                     │
┌────────────────────────────────────┼────────────────────────────────────────────────┐
│                         核心内核层 (Core Kernel)                                     │
│                                    │                                                │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      macaca-kernel (Agent 内核)                              │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────────────┐  │   │
│  │  │   Kernel    │  │AgentRegistry│  │  Scheduler  │  │  StatusTracker     │  │   │
│  │  │  (中央协调)  │  │ (Agent注册表) │  │  (调度器)    │  │  (状态追踪)         │  │   │
│  │  │             │  │             │  │             │  │                    │  │   │
│  │  │ • 注册Agent │  │ • 存储Agent │  │ • 任务分配   │  │ • 运行时状态        │  │   │
│  │  │ • 执行Agent │  │ • 生命周期   │  │ • 负载均衡   │  │ • 活动追踪          │  │   │
│  │  │ • 状态管理  │  │ • 元数据     │  │ • 优先级    │  │ • 历史记录          │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └────────────────────┘  │   │
│  │                                                                               │   │
│  │  关键方法:                                                                    │   │
│  │    - register_agent()    → 注册新Agent                                       │   │
│  │    - execute_agent()     → 执行Agent（会更新状态）                            │   │
│  │    - update_agent_activity() → 更新Agent活动状态                              │   │
│  │    - list_agent_statuses()   → 获取所有Agent状态                              │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      macaca-app (Application 运行时)                          │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────────────┐  │   │
│  │  │ AppRegistry │  │ AppRuntime  │  │ AppLoader   │  │ WorkflowEngine     │  │   │
│  │  │ (应用注册表) │  │ (生命周期)   │  │ (配置加载)   │  │ (工作流引擎)        │  │   │
│  │  │             │  │             │  │             │  │                    │  │   │
│  │  │ • 发现应用   │  │ • 启动应用   │  │ • 解析YAML   │  │ • 执行工作流        │  │   │
│  │  │ • 管理配置   │  │ • 停止应用   │  │ • 加载Persona│  │ • 步骤编排          │  │   │
│  │  │ • 版本控制   │  │ • 状态管理   │  │ • 读取Skills │  │ • 条件分支          │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └────────────────────┘  │   │
│  │                                                                               │   │
│  │  Application Layer 支持:                                                      │   │
│  │    - L1: 原生应用 (Rust代码)                                                  │   │
│  │    - L2: WASM 应用 (任何可编译为WASM的语言)                                    │   │
│  │    - L3: 声明式应用 (YAML/JSON配置)                                           │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      macaca-runtime (Agentic 运行时)                          │   │
│  │  ┌────────────────────────────────────────────────────────────────────────┐ │   │
│  │  │                        AgenticLoop (核心循环)                             │ │   │
│  │  │                                                                         │ │   │
│  │  │   ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐           │ │   │
│  │  │   │  Call   │───►│  Parse  │───►│ Execute │───►│  Feed   │────────┐  │ │   │
│  │  │   │   LLM   │    │  Tools  │    │  Tools  │    │  Back   │        │  │ │   │
│  │  │   └─────────┘    └─────────┘    └─────────┘    └─────────┘        │  │ │   │
│  │  │        ▲                                            │              │  │ │   │
│  │  │        └────────────────────────────────────────────┘              │  │ │   │
│  │  │                      (循环直到完成或达到最大迭代)                      │  │ │   │
│  │  └────────────────────────────────────────────────────────────────────────┘ │   │
│  │                                                                             │   │
│  │  配置:                                                                      │   │
│  │    - max_iterations: 25 (最大LLM往返次数)                                    │   │
│  │    - tool_timeout: 60s (工具执行超时)                                        │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────────┘
                                     │
┌────────────────────────────────────┼────────────────────────────────────────────────┐
│                         能力扩展层 (Capability Layer)                                │
│                                    │                                                │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      macaca-driver (Driver 框架)                              │   │
│  │                                                                               │   │
│  │  SoftwareDriver Trait (软件驱动接口):                                         │   │
│  │    fn manifest() -> DriverManifest     // 驱动元数据                          │   │
│  │    async fn initialize()               // 初始化连接                          │   │
│  │    fn tools() -> Vec<Box<dyn Tool>>    // 暴露能力为工具                       │   │
│  │    async fn health_check()             // 健康检查                            │   │
│  │    async fn shutdown()                 // 优雅关闭                            │   │
│  │                                                                               │   │
│  │  Driver 类型:                                                                 │   │
│  │    - CliSubprocess    // 命令行程序 (如 Claude Code CLI)                      │   │
│  │    - RestApi          // REST/GraphQL API                                    │   │
│  │    - UiAutomation     // UI自动化 (AppleScript, Accessibility)                │   │
│  │    - FileIpc          // 文件/IPC通信                                        │   │
│  │    - McpProtocol      // MCP 协议                                            │   │
│  │                                                                               │   │
│  │  内置 Drivers:                                                                │   │
│  │    ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐     │   │
│  │    │  ShellDriver    │  │ FilesystemDriver│  │  ClaudeCodeDriver       │     │   │
│  │    │  (shell命令)     │  │  (文件操作)      │  │  (Claude Code CLI)      │     │   │
│  │    └─────────────────┘  └─────────────────┘  └─────────────────────────┘     │   │
│  │                                                                               │   │
│  │  Driver Registry: 动态发现、加载、管理 Driver                                  │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      macaca-tools (工具系统)                                  │   │
│  │                                                                               │   │
│  │  Tool Trait:                                                                  │   │
│  │    fn name() -> &str                    // 工具名称                          │   │
│  │    fn description() -> &str             // 功能描述                          │   │
│  │    fn parameters() -> Value            // JSON Schema参数定义                 │   │
│  │    async fn execute(args) -> Result    // 执行逻辑                           │   │
│  │                                                                               │   │
│  │  ToolSet: 工具集合，支持从多个 Driver 聚合工具                                  │   │
│  │                                                                               │   │
│  │  内置工具: file_read, file_write, shell, web_search, code_search...           │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      macaca-memory (记忆系统 - 可插拔)                          │   │
│  │                                                                               │   │
│  │  三层记忆架构:                                                                 │   │
│  │    ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐     │   │
│  │    │   Session Layer │  │   File Layer    │  │     Vector Layer        │     │   │
│  │    │   (会话级内存)   │  │   (持久化存储)   │  │     (向量检索)          │     │   │
│  │    │                 │  │                 │  │                         │     │   │
│  │    │ • 短期记忆       │  │ • 长期存储       │  │ • 语义搜索              │     │   │
│  │    │ • 自动过期       │  │ • 结构化存储     │  │ • 相似度匹配            │     │   │
│  │    │ • 快速访问       │  │ • 跨会话保留     │  │ • 嵌入向量              │     │   │
│  │    └─────────────────┘  └─────────────────┘  └─────────────────────────┘     │   │
│  │                                                                               │   │
│  │  可插拔组件:                                                                   │   │
│  │    - EmbeddingProvider: DashScope, OpenAI, 或自定义                            │   │
│  │    - VectorStore: Milvus, InMemoryVectorStore, 或自定义                        │   │
│  │                                                                               │   │
│  │  自动检索: auto_retrieve(TaskContext) -> 相关记忆                              │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      macaca-llm (LLM 抽象层)                                  │   │
│  │                                                                               │   │
│  │  LlmProvider Trait:                                                           │   │
│  │    async fn chat(messages, options) -> LlmResponse                           │   │
│  │                                                                               │   │
│  │  支持提供商:                                                                   │   │
│  │    - OpenAI (GPT-4, GPT-3.5)                                                  │   │
│  │    - Anthropic (Claude)                                                       │   │
│  │    - DashScope (通义千问)                                                      │   │
│  │    - DeepSeek / Ollama (OpenAI兼容)                                           │   │
│  │                                                                               │   │
│  │  功能:                                                                         │   │
│  │    - 工具调用 (Function Calling)                                              │   │
│  │    - Token 使用追踪                                                            │   │
│  │    - 速率限制                                                                  │   │
│  │    - 成本估算                                                                  │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      macaca-ipc (进程间通信)                                   │   │
│  │                                                                               │   │
│  │  通信模式:                                                                     │   │
│  │    - Pub/Sub: 主题订阅发布                                                    │   │
│  │    - P2P: Agent 间直接通信                                                    │   │
│  │    - Request/Reply: 同步请求响应                                              │   │
│  │                                                                               │   │
│  │  后端支持: NATS (默认), 或自定义实现                                           │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      macaca-mcp (MCP 协议支持)                                │   │
│  │                                                                               │   │
│  │  MCP (Model Context Protocol) 适配器:                                         │   │
│  │    - 连接 MCP 服务器                                                          │   │
│  │    - 将 MCP 工具转换为 Macaca Tool                                            │   │
│  │    - 支持 stdio 和 SSE 传输                                                   │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────────┘
                                     │
┌────────────────────────────────────┼────────────────────────────────────────────────┐
│                         开发工具层 (Developer Tools)                                 │
│                                    │                                                │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      macaca-sdk (Agent SDK)                                   │   │
│  │                                                                               │   │
│  │  开发方式支持:                                                                 │   │
│  │                                                                               │   │
│  │  1. 声明式开发 (L3 - 推荐):                                                    │   │
│  │     - YAML/TOML 配置 Agent                                                    │   │
│  │     - personas/ 目录定义角色提示词                                            │   │
│  │     - skills/ 目录定义技能                                                    │   │
│  │     - workflows/ 目录定义工作流                                               │   │
│  │                                                                               │   │
│  │  2. Rust 原生开发 (L1):                                                        │   │
│  │     - 实现 Agent trait                                                        │   │
│  │     - 直接调用 Kernel API                                                     │   │
│  │                                                                               │   │
│  │  3. WASM 开发 (L2 - 计划中):                                                   │   │
│  │     - 任何可编译为 WASM 的语言                                                │   │
│  │     - WASM 运行时沙箱                                                        │   │
│  │                                                                               │   │
│  │  SDK 组件:                                                                     │   │
│  │    - AgentBuilder: 流畅的 Agent 构建 API                                      │   │
│  │    - AgentConfig: 配置解析 (YAML/TOML)                                        │   │
│  │    - AgentPersona: 角色/人格管理                                              │   │
│  │    - register_from_config(): 从配置注册 Agent                                 │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
│                                                                                     │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      macaca-cli (命令行工具)                                  │   │
│  │                                                                               │   │
│  │  命令:                                                                         │   │
│  │    macaca run      → 启动内核和网关                                           │   │
│  │    macaca web      → 启动 Web 服务器                                          │   │
│  │    macaca agents   → 列出所有 Agent                                           │   │
│  │    macaca status   → 显示系统状态                                             │   │
│  │    macaca version  → 显示版本                                                 │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────────┘
                                     │
┌────────────────────────────────────┼────────────────────────────────────────────────┐
│                         协议层 (Protocol Layer)                                      │
│                                    │                                                │
│  ┌─────────────────────────────────────────────────────────────────────────────┐   │
│  │                      macaca-proto (核心协议)                                  │   │
│  │                                                                               │   │
│  │  核心类型:                                                                     │   │
│  │    - AgentId, TaskId, ApplicationId, MemoryId                                │   │
│  │    - AgentState: Created, Running, Suspended, Terminated                     │   │
│  │    - AgentActivity: Idle, Thinking, ExecutingTool, Waiting, Error            │   │
│  │    - Task, TaskRequest, TaskResult                                           │   │
│  │    - MemoryEntry, MemoryLayer                                                │   │
│  │    - LlmMessage, LlmResponse, ToolCall                                       │   │
│  │    - GatewayEvent                                                            │   │
│  │                                                                               │   │
│  │  错误处理: MacacaError                                                        │   │
│  │  配置: KernelConfig, LlmConfig                                                │   │
│  └─────────────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. 核心流程详解

### 3.1 Application 启动流程

```
用户: macaca web
    ↓
macaca-web: start_server()
    ↓
1. 加载配置 (config/default.toml)
2. 初始化 LLM Provider
3. 初始化 Kernel
4. 初始化 AppRuntime
    ↓
AppRegistry.discover_apps() → 扫描标准目录
    - ~/.macaca/apps/
    - ./apps/
    - <executable_dir>/../apps/
    ↓
对于每个 app.yaml:
    AppRuntime.start_app(manifest, base_dir, kernel)
        ↓
    AppLoader.resolve_agent_configs(manifest, base_dir)
        - 读取 personas/
        - 读取 agent 配置
        ↓
    对于每个 agent_config:
        register_from_config(kernel, config)
            ↓
        AgentBuilder.from_config(config).build_with_manifest()
            ↓
        kernel.register_agent(Box::new(agent), manifest)
            ↓
        AgentRegistry.register(agent, manifest)
        StatusTracker.register(id, name)
        StatusTracker.update_state(&id, AgentState::Running)
    ↓
LoadedApp { manifest, agent_ids, status: Running }
```

### 3.2 用户请求处理流程（当前问题）

**当前实现（问题）**：
```
用户发送消息
    ↓
POST /api/chat
    ↓
post_chat() 处理
    ↓
直接调用 llm.chat(messages, options)  ← 问题：绕过 Agent 系统
    ↓
返回 SSE 流
    ↓
Agent 状态始终是 Idle（因为没有执行 Agent）
```

**正确架构应该**：
```
用户发送消息
    ↓
POST /api/apps/{id}/invoke 或 /api/chat
    ↓
路由到 Coordinator Agent
    ↓
kernel.execute_agent(coordinator_id)
    ↓
StatusTracker.set_thinking(coordinator_id)
    ↓
Coordinator Agent 分析意图
    ├── 简单聊天 → 直接回复
    └── 复杂任务 → 触发 Workflow
              ↓
        WorkflowEngine.execute()
              ↓
        按步骤调度各个 Agent
              ↓
        每个 Agent 执行时更新状态
              ↓
        返回 SSE 流（包含状态更新）
```

### 3.3 Agent 执行流程

```
kernel.execute_agent(agent_id)
    ↓
StatusTracker.set_thinking(agent_id, "executing agent")
    ↓
AgentRegistry.get(agent_id) → 获取 Agent
    ↓
agent.run(llm, tools, services)
    ↓
如果是 DeclarativeAgent:
    AgenticLoop.run()
        ↓
    循环:
        1. llm.chat() → 获取响应
        2. 如果有 tool_calls:
           - StatusTracker.set_executing_tool(agent_id, tool_name)
           - 执行工具
           - 将结果反馈给 LLM
           - 继续循环
        3. 如果没有 tool_calls:
           - 返回最终结果
    ↓
StatusTracker.set_idle(agent_id)
```

### 3.4 状态追踪流程

```
Agent 状态更新:
    ↓
StatusTracker.update_activity(agent_id, activity)
    ↓
内部 HashMap<AgentId, AgentRuntimeStatus>
    ↓
更新: activity, updated_at
    ↓
SSE Stream 检测变化
    ↓
推送到前端
    ↓
前端 AgentPanel 更新显示
```

---

## 4. 可插拔架构设计

### 4.1 记忆系统可插拔

```rust
// 核心 Trait
trait MemoryStore: Send + Sync {
    async fn store(&self, entry: MemoryEntry) -> Result<MemoryId>;
    async fn retrieve(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>>;
}

trait EmbeddingProvider: Send + Sync {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
}

trait VectorStore: Send + Sync {
    async fn upsert(&self, id: &str, vector: Vec<f32>, payload: Value) -> Result<()>;
    async fn search(&self, vector: Vec<f32>, limit: usize) -> Result<Vec<VectorSearchResult>>;
}

// 可插拔实现
MemoryManager<V: VectorStore, E: EmbeddingProvider> {
    session: SessionMemory,
    file: FileMemory,
    vector: Option<V>,      // 可替换: Milvus, InMemoryVectorStore
    embedding: Option<E>,   // 可替换: DashScope, OpenAI, MockEmbedding
}
```

### 4.2 LLM 提供商可插拔

```rust
trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn chat(&self, messages: Vec<LlmMessage>, options: &LlmOptions) -> Result<LlmResponse>;
}

// 实现: OpenAiProvider, AnthropicProvider, DashScopeProvider, etc.
```

### 4.3 Driver 可插拔

```rust
trait SoftwareDriver: Send + Sync {
    fn manifest(&self) -> &DriverManifest;
    async fn initialize(&mut self) -> Result<()>;
    fn tools(&self) -> Vec<Box<dyn Tool>>;
    async fn health_check(&self) -> Result<bool>;
    async fn shutdown(&mut self) -> Result<()>;
}

// 内置: ShellDriver, FilesystemDriver
// 扩展: ClaudeCodeDriver, FigmaDriver, BrowserDriver, etc.
```

### 4.4 Gateway 可插拔

```rust
trait ImAdapter: Send + Sync {
    fn name(&self) -> &str;
    async fn start(&self, handler: Arc<dyn EventHandler>) -> Result<()>;
    async fn send_message(&self, channel_id: &str, content: &str) -> Result<()>;
    async fn stop(&self) -> Result<()>;
}

// 实现: TelegramAdapter, DiscordAdapter, SlackAdapter, etc.
```

### 4.5 Agent 上下文编排可插拔

通过 `AgentServices` 注入：
```rust
pub struct AgentServices {
    pub memory: Option<Box<dyn MemoryService>>,
    pub ipc: Option<Box<dyn IpcService>>,
    pub persist: Option<Box<dyn PersistService>>,
}
```

用户可以自定义：
- `MemoryService`: 自定义记忆存储和检索逻辑
- `IpcService`: 自定义 Agent 间通信方式
- `PersistService`: 自定义状态持久化

---

## 5. Application 开发方式

### 5.1 L3: 声明式开发（推荐）

```yaml
# app.yaml
id: fullstack-autodev
name: Fullstack AutoDev
version: 1.0.0
layer: L3Declarative

agents:
  - id: coordinator
    persona: personas/coordinator.md
    capabilities:
      - name: task_classification
      - name: workflow_orchestration

  - id: architect
    persona: personas/architect.md
    capabilities:
      - name: system_design
      - name: tech_stack_selection

workflows:
  sdd:
    description: Spec-Driven Development
    steps:
      - name: analyze
        agent: coordinator
        prompt_template: analyze_request.md
      - name: design
        agent: architect
        prompt_template: create_design.md
        depends_on: [analyze]
```

### 5.2 L1: Rust 原生开发

```rust
use macaca_agent::Agent;
use macaca_proto::{AgentId, AgentOutput, AgentState, Capability};

pub struct MyCustomAgent {
    id: AgentId,
    state: AgentState,
}

#[async_trait]
impl Agent for MyCustomAgent {
    fn id(&self) -> AgentId { self.id }
    fn capabilities(&self) -> &[Capability] { /* ... */ }
    fn state(&self) -> AgentState { self.state }

    async fn run(&self, llm: &dyn LlmProvider, tools: &dyn ToolSet, services: &AgentServices) -> Result<AgentOutput> {
        // 自定义逻辑
    }
}

// 注册
kernel.register_agent(Box::new(MyCustomAgent::new()), manifest).await?;
```

### 5.3 L2: WASM 开发（计划中）

```rust
// 任何可编译为 WASM 的语言
// WASI 接口
// 沙箱执行
```

---

## 6. 当前架构问题与修复方案

### 6.1 问题 1: Chat 绕过 Agent 系统

**症状**: Agent 状态始终显示 IDLE

**原因**: `post_chat` 直接调用 `llm.chat()`，不经过 kernel 的 agent 执行

**修复方案**:
1. 修改 Chat 路由，通过 Coordinator Agent 处理请求
2. Coordinator 分析意图，决定直接回复或触发工作流
3. 使用 `AgenticLoop` 执行 Agent

### 6.2 问题 2: Agent 状态更新不完整

**症状**: 只有 execute_agent 会更新状态，Agent 内部活动没有细粒度状态

**原因**: Agent 内部调用 LLM/工具时，没有通知 StatusTracker

**修复方案**:
1. 在 `AgenticLoop` 中添加状态更新钩子
2. 或者通过回调函数让 Agent 报告状态变化

### 6.3 问题 3: Workflow 引擎未集成

**症状**: `app.yaml` 中定义了 workflow，但 chat 没有使用

**原因**: WorkflowEngine 未实现或未集成到 chat 流程

**修复方案**:
1. 实现 WorkflowEngine
2. Chat 请求触发 workflow 执行
3. 每个 workflow 步骤对应 Agent 执行

---

## 7. 关键文件索引

| 组件 | 关键文件 | 说明 |
|------|----------|------|
| Proto | `crates/macaca-proto/src/types.rs` | 核心类型定义 |
| Kernel | `crates/macaca-kernel/src/kernel.rs` | Kernel 实现 |
| Kernel | `crates/macaca-kernel/src/status.rs` | 状态追踪 |
| Agent | `crates/macaca-agent/src/agent.rs` | Agent Trait |
| App | `crates/macaca-app/src/runtime.rs` | App 运行时 |
| App | `crates/macaca-app/src/workflow.rs` | 工作流引擎 |
| Runtime | `crates/macaca-runtime/src/agentic_loop.rs` | Agentic 循环 |
| Web | `crates/macaca-web/src/routes.rs` | API 路由 |
| Web | `crates/macaca-web/src/state.rs` | 应用状态 |
| SDK | `crates/macaca-sdk/src/builder.rs` | Agent 构建器 |
| Driver | `crates/macaca-driver/src/driver.rs` | Driver Trait |
| Memory | `crates/macaca-memory/src/manager.rs` | 记忆管理器 |
| Gateway | `crates/macaca-gateway/src/gateway.rs` | 网关管理 |
| Tools | `crates/macaca-tools/src/lib.rs` | 工具系统 |
| LLM | `crates/macaca-llm/src/provider.rs` | LLM Provider |

---

## 8. 总结

Macaca OS 是一个设计完善的 Agent 操作系统：

1. **分层架构**: 从底层协议到用户交互，每层职责清晰
2. **可插拔设计**: 记忆、LLM、Driver、Gateway 都可替换
3. **多语言支持**: 声明式(YAML)、Rust原生、WASM(计划中)
4. **多平台接入**: Web、Telegram、Discord 等 IM
5. **完整生命周期**: Agent 注册、调度、执行、状态追踪

当前的主要问题是 Chat 流程没有正确使用 Agent 系统，需要修复以：
- 正确显示 Agent 状态
- 支持 Workflow 编排
- 实现真正的 Agent 协作
