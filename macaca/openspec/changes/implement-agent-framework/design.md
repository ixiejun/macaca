## Context

Macaca Agent OS 需要一个底层 Agent 框架来统一 Agent 生命周期管理。AgentScope（Python）提供了成熟的参考架构。本设计将其核心概念映射到 Rust 惯用模式，同时与现有 macaca-* crates 良好集成。

## Goals / Non-Goals

**Goals:**
- 完整移植 AgentScope 核心架构（非简化版）
- Rust 惯用设计：trait、enum、derive macro、async/await
- 与现有 macaca-llm/macaca-tools/macaca-persist 集成而非替代
- 支持 Send + Sync + 'static（适合 tokio 多线程运行时）
- 零成本抽象：不用 trait object 时无运行时开销

**Non-Goals:**
- 不做 Python FFI 或直接翻译 Python 代码
- 不引入新的 LLM 提供商实现（复用 macaca-llm）
- 不替换 OS 层的 TaskBoard/PlanLoop/WorkerLoop

## Decisions

### Decision 1: ContentBlock 类型系统

**选择**: Rust enum + serde 标签联合

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    Text(TextBlock),
    Thinking(ThinkingBlock),
    ToolUse(ToolUseBlock),
    ToolResult(ToolResultBlock),
    Image(ImageBlock),
    Audio(AudioBlock),
    Video(VideoBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Msg {
    pub id: String,
    pub name: String,
    pub content: MsgContent,  // String | Vec<ContentBlock>
    pub role: Role,
    pub metadata: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MsgContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}
```

**替代**: 用 `serde_json::Value` 做动态类型 → 放弃编译期类型安全
**替代**: 只用 String → 无法表达多模态和工具调用

### Decision 2: Agent trait 设计

**选择**: 异步 trait + 关联类型

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    /// 生成回复（核心方法）
    async fn reply(&self, msg: Msg) -> Result<Msg>;
    
    /// 接收消息但不生成回复
    async fn observe(&self, msg: Msg) -> Result<()> {
        // 默认实现：存入记忆
        Ok(())
    }
    
    /// 中断当前执行
    async fn interrupt(&self, msg: Msg) -> Result<()> {
        Ok(())
    }
    
    /// Agent 名称
    fn name(&self) -> &str;
    
    /// Agent ID
    fn id(&self) -> &AgentId;
}
```

**Hook 注入**: 通过 wrapper 类型而非 metaclass

```rust
pub struct HookedAgent<A: Agent> {
    inner: A,
    pre_reply_hooks: Vec<Box<dyn Hook>>,
    post_reply_hooks: Vec<Box<dyn Hook>>,
}

impl<A: Agent> Agent for HookedAgent<A> {
    async fn reply(&self, msg: Msg) -> Result<Msg> {
        let msg = self.run_pre_hooks(msg).await?;
        let result = self.inner.reply(msg).await?;
        self.run_post_hooks(result).await
    }
}
```

**替代**: 用 macro 注入 → 编译期固定，无法运行时动态添加 hook

### Decision 3: StateModule — derive macro 序列化

**选择**: proc-macro derive + trait

```rust
pub trait StateModule: Send + Sync {
    fn state_dict(&self) -> serde_json::Value;
    fn load_state_dict(&mut self, state: serde_json::Value) -> Result<()>;
}

// 用法：
#[derive(StateModule)]
pub struct MyAgent {
    #[state]
    memory: InMemoryWorkingMemory,
    #[state]
    toolkit: Toolkit,
    // 不标记 #[state] 的字段不参与序列化
    llm: Arc<dyn ChatModel>,
}
```

**替代**: 直接用 `#[derive(Serialize)]` → 无法选择性序列化、无法处理 `Arc<dyn Trait>` 等非序列化字段

### Decision 4: Formatter 层

**选择**: 独立 trait，与 ChatModel 解耦

```rust
#[async_trait]
pub trait Formatter: Send + Sync {
    /// 将内部 Msg 列表转换为 LLM API 所需的消息格式
    fn format(&self, msgs: &[Msg]) -> Vec<serde_json::Value>;
    
    /// 将 LLM API 响应转换为内部 ChatResponse
    fn parse_response(&self, raw: serde_json::Value) -> Result<ChatResponse>;
}
```

内置实现：`OpenAiFormatter`、`DashScopeFormatter`、`AnthropicFormatter`

**与 macaca-llm 集成**: `ChatModel` trait 包装 `macaca_llm::LlmProvider`，通过 `Formatter` 做转换

```rust
pub struct LlmProviderAdapter {
    provider: Arc<dyn LlmProvider>,
    formatter: Box<dyn Formatter>,
}

impl ChatModel for LlmProviderAdapter {
    async fn chat(&self, msgs: Vec<Msg>, options: &ChatOptions) -> Result<ChatResponse> {
        let formatted = self.formatter.format(&msgs);
        let llm_msgs = /* convert formatted to LlmMessage */;
        let response = self.provider.chat(llm_msgs, &llm_options).await?;
        self.formatter.parse_response(response)
    }
}
```

### Decision 5: Memory 标签系统

**选择**: 消息附带标签集合

```rust
pub struct TaggedMsg {
    pub msg: Msg,
    pub marks: Vec<String>,
}

#[async_trait]
pub trait WorkingMemory: StateModule + Send + Sync {
    async fn add(&mut self, msg: Msg, marks: Vec<String>);
    async fn get(&self, mark: Option<&str>, exclude_mark: Option<&str>) -> Vec<&TaggedMsg>;
    async fn delete(&mut self, msg_id: &str);
    async fn delete_by_mark(&mut self, mark: &str);
    async fn update_mark(&mut self, msg_ids: &[String], old_mark: &str, new_mark: &str);
    async fn size(&self) -> usize;
    async fn clear(&mut self);
    
    /// 更新压缩摘要（LLM 生成的历史摘要）
    async fn update_summary(&mut self, summary: Msg);
    /// 获取带可选摘要前置的记忆
    async fn get_with_summary(&self) -> Vec<Msg>;
}
```

### Decision 6: Toolkit — 工具注册与分组

**选择**: 注册表模式 + 分组 + 中间件

```rust
pub struct Toolkit {
    tools: HashMap<String, RegisteredTool>,
    groups: HashMap<String, ToolGroup>,
    middlewares: Vec<Box<dyn ToolMiddleware>>,
}

pub struct RegisteredTool {
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,      // JSON Schema
    pub handler: Box<dyn ToolHandler>,
    pub group: String,                   // "basic" 默认组
    pub preset_args: serde_json::Value,  // 预设参数（不暴露给 LLM）
}

#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResponse>;
    /// 流式执行（默认委托给 execute）
    async fn execute_streaming(&self, args: serde_json::Value) -> Result<Pin<Box<dyn Stream<Item = ToolResponse>>>>;
}

#[async_trait]
pub trait ToolMiddleware: Send + Sync {
    async fn before(&self, name: &str, args: &mut serde_json::Value) -> Result<()>;
    async fn after(&self, name: &str, response: &mut ToolResponse) -> Result<()>;
}
```

**与 macaca-tools 集成**: `ToolSetAdapter` 将 `macaca_tools::ToolSet` 包装为 `Toolkit`

### Decision 7: Pipeline 编排

**选择**: trait + 函数式组合

```rust
#[async_trait]
pub trait Pipeline: Send + Sync {
    async fn run(&self, msg: Msg) -> Result<Msg>;
}

pub struct SequentialPipeline {
    agents: Vec<Arc<dyn Agent>>,
}

pub struct FanoutPipeline {
    agents: Vec<Arc<dyn Agent>>,
    concurrent: bool,
}

pub struct MsgHub {
    participants: Vec<Arc<dyn Agent>>,
}
```

`MsgHub` 通过内部 `broadcast::channel` 实现消息广播，自动剥离 `ThinkingBlock`。

### Decision 8: PlanNotebook — 工具化规划

**选择**: 将规划能力注册为一组工具

```rust
pub struct PlanNotebook {
    current_plan: Option<Plan>,
    historical_plans: Vec<Plan>,
}

impl PlanNotebook {
    /// 注册规划工具到 Toolkit
    pub fn register_tools(&self, toolkit: &mut Toolkit) {
        toolkit.register("create_plan", ...);
        toolkit.register("revise_plan", ...);
        toolkit.register("update_subtask_state", ...);
        toolkit.register("finish_subtask", ...);
        toolkit.register("finish_plan", ...);
    }
    
    /// 根据当前规划状态生成 hint 消息
    pub fn hint(&self) -> Option<Msg> { ... }
}
```

`Plan` 状态机: `Todo → InProgress → Done | Abandoned`
`SubTask` 状态机: `Todo → InProgress → Done | Abandoned`

约束：同一时间只允许一个 SubTask 处于 InProgress。

### Decision 9: A2A 协议 (Agent-to-Agent)

**选择**: 完整实现 Google A2A 协议的 Rust 版本，基于 axum 提供 Server 端。

```rust
/// Agent 服务描述卡片（用于服务发现）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub name: String,
    pub url: String,
    pub version: String,
    pub description: Option<String>,
    pub capabilities: AgentCapabilities,
    pub default_input_modes: Vec<String>,
    pub default_output_modes: Vec<String>,
    pub skills: Vec<AgentSkill>,
}

/// A2A 消息（协议级别，区别于框架内部 Msg）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AMessage {
    pub message_id: String,
    pub role: A2ARole,  // User | Agent
    pub parts: Vec<A2APart>,
    pub context_id: Option<String>,
}

/// A2A 消息内容部分（多态）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum A2APart {
    Text { text: String },
    File { file: A2AFile },
    Data { data: serde_json::Value },
}

/// A2A 异步任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2ATask {
    pub id: String,
    pub context_id: Option<String>,
    pub status: A2ATaskStatus,
    pub artifacts: Vec<A2AArtifact>,
}

/// 任务状态机: Submitted → Working → Completed | Failed | Canceled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum A2ATaskState {
    Submitted,
    Working,
    Completed,
    Failed,
    Canceled,
}
```

**客户端**: `A2AAgent` 实现 `Agent` trait，将远程 A2A 服务包装为本地 Agent

```rust
pub struct A2AAgent {
    card: AgentCard,
    client: reqwest::Client,
    formatter: A2AFormatter,
}

impl Agent for A2AAgent {
    async fn reply(&self, msg: Msg) -> Result<Msg> {
        let a2a_msg = self.formatter.to_a2a(&msg)?;
        let response = self.client.post(&self.card.url)
            .json(&SendMessageRequest { message: a2a_msg })
            .send().await?;
        // 支持 SSE 流式响应
        self.formatter.from_a2a(response).await
    }
}
```

**服务端**: `A2AServer` 将本地 `Agent` 暴露为 A2A HTTP/SSE 端点

```rust
pub struct A2AServer {
    card: AgentCard,
    agent: Arc<dyn Agent>,
    formatter: A2AFormatter,
}

impl A2AServer {
    /// 构建 axum Router
    pub fn router(self) -> Router {
        Router::new()
            .route("/.well-known/agent.json", get(Self::agent_card))
            .route("/a2a/message/send", post(Self::send_message))
            .route("/a2a/message/stream", post(Self::send_message_stream))
            .route("/a2a/task/:id", get(Self::get_task))
    }
}
```

**服务发现**: `AgentCardResolver` trait + 多后端

```rust
#[async_trait]
pub trait AgentCardResolver: Send + Sync {
    async fn resolve(&self) -> Result<AgentCard>;
}

pub struct FileCardResolver { path: PathBuf }
pub struct WellKnownCardResolver { base_url: String, client: reqwest::Client }
```

**Formatter**: `A2AFormatter` 双向转换 Msg ↔ A2AMessage

```rust
pub struct A2AFormatter;

impl A2AFormatter {
    /// AgentScope Msg → A2A Message
    pub fn to_a2a(&self, msg: &Msg) -> Result<A2AMessage>;
    /// A2A Message → AgentScope Msg  
    pub fn from_a2a_message(&self, name: &str, msg: A2AMessage) -> Result<Msg>;
    /// A2A Task → AgentScope Msg list
    pub fn from_a2a_task(&self, name: &str, task: A2ATask) -> Result<Vec<Msg>>;
}
```

**替代**: 用 gRPC 而非 HTTP/SSE → 增加复杂度，A2A 标准协议使用 HTTP
**替代**: 直接用 macaca-ipc 做 Agent 通信 → 不兼容跨服务场景

### Decision 10: Crate 依赖关系（原 Decision 9）

```
macaca-framework (新)
  ├── depends on: macaca-proto (类型共享)
  ├── depends on: macaca-persist (Session 持久化)
  ├── optional: macaca-llm (LlmProvider 适配)
  ├── optional: macaca-tools (ToolSet 适配)
  └── optional: macaca-mcp (MCP 客户端)

macaca-runtime (修改)
  └── depends on: macaca-framework (使用 ReActAgent 替代 AgenticLoop)

macaca-web (修改)  
  └── depends on: macaca-framework (Agent 执行入口)
```

### Decision 10: ReActAgent 核心循环（替代 AgenticLoop）

```rust
pub struct ReActAgent {
    name: String,
    id: AgentId,
    sys_prompt: String,
    model: Arc<dyn ChatModel>,
    formatter: Box<dyn Formatter>,
    toolkit: Toolkit,
    memory: Box<dyn WorkingMemory>,
    long_term_memory: Option<Box<dyn LongTermMemory>>,
    plan_notebook: Option<PlanNotebook>,
    compression: Option<CompressionConfig>,
    max_iters: usize,
    hooks: HookRegistry,
}

impl Agent for ReActAgent {
    async fn reply(&self, msg: Msg) -> Result<Msg> {
        self.memory.add(msg, vec![]);
        self.retrieve_long_term_memory().await;
        
        for i in 0..self.max_iters {
            self.compress_if_needed().await?;
            let response = self.reasoning(i).await?;
            
            if response.tool_calls().is_empty() {
                return Ok(response.into_msg());
            }
            
            for tool_call in response.tool_calls() {
                self.acting(tool_call).await?;
            }
        }
        
        self.summarize().await
    }
}
```

## Risks / Trade-offs

| Risk | Impact | Mitigation |
|------|--------|------------|
| 工作量巨大（~10K 行 Rust） | 开发周期长 | 分 10 个 Phase，每个独立可验证 |
| 与现有 crate 接口不兼容 | 集成困难 | 通过 Adapter 模式桥接，逐步替换 |
| Rust 的 async trait 限制 | 性能/灵活性 | 使用 `async-trait` crate，待 Rust 原生支持后迁移 |
| StateModule derive macro 复杂 | 维护成本 | Phase 1 先用手动实现，后续补 proc-macro |
| 过度设计风险 | 复杂度膨胀 | 严格遵循 AgentScope 已验证的抽象，不自创 |

## Migration Plan

渐进式迁移，不做 big-bang 切换：

1. **Phase 1-7**: 实现 macaca-framework 核心（独立 crate，不影响现有系统）
2. **Phase 8-9**: 添加 Adapter 层（macaca-llm → ChatModel、macaca-tools → Toolkit）
3. **Phase 10**: 在 macaca-web 中提供 `ReActAgent` 作为**可选**执行引擎，与现有 `AgenticLoop` 并存
4. **后续**: 逐步将 `AgenticLoop` 的调用点迁移到 `ReActAgent`，最终废弃 `macaca-runtime` 的循环实现
