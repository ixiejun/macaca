# Design: macaca-app 渐进式 Builder / Strategy / Factory / Composite 重构

## Context

`macaca-app` 现在是 Agent OS 的 application 声明式边界。它负责：

- 读取 manifest 与 agent source
- 将 manifest 装配成运行中的 `LoadedApp` / `AppRuntime`
- 为 workflow/coordinator 生成默认 prompt
- 提供应用级入口、状态和 agent 配置解释

当前实现可运行，但多个职责在 `runtime.rs` 和 `workflow.rs` 中仍然耦合较紧。随着 driver、tool policy、framework primitive、skills、MCP 能力持续抽象化，如果不先在 `macaca-app` 建立更清晰的构造与策略边界，未来会继续把 application-specific 规则写回 prompt 文本或启动流程里。

本设计严格遵守：

- 每次只做一个小切片
- 行为 1:1 还原
- 旧 API 先委托，不能一次性删除
- 应用差异必须通过 manifest/capability/tool policy/strategy 表达，而不是硬编码到单个 app

## Goals

- 将 runtime 装配从运行时管理逻辑中分离出来。
- 将 workflow prompt 从大段硬编码字符串拆成稳定模板与策略注入点。
- 为 application startup 建立可扩展的装配工厂边界。
- 为 application-level capability 聚合建立结构化内部表示。
- 保持现有应用可运行语义与 prompt 默认行为不变。

## Non-Goals

- 不新增 manifest 字段。
- 不改变 application registry 和 web runtime 的外部接口。
- 不在本 change 中迁移 traced agent construction。
- 不改变 `WorkflowEngine` 当前负责“生成 prompt 而非直接执行 workflow”的事实。
- 不直接修改上层 coordinator/planner 的调度策略。

## Proposed Design

### 1. AppRuntimeBuilder

将 `AppRuntime::start_app*` 中的构造步骤显式拆为：

- manifest resolve
- validation
- agent config assemble
- runtime load object build

第一阶段 builder 只承接现有逻辑，不改变 public 入口。

```rust
pub struct AppRuntimeBuilder {
    manifest: AppManifest,
    base_dir: PathBuf,
}

impl AppRuntimeBuilder {
    pub fn new(manifest: AppManifest, base_dir: impl Into<PathBuf>) -> Self { ... }

    pub fn validate(&self) -> Result<(), AppConfigError> { ... }

    pub fn resolve_agent_configs(&self) -> Result<Vec<AgentConfig>, AppConfigError> { ... }

    pub fn assemble_loaded_app(
        self,
        agent_ids: Vec<AgentId>,
    ) -> Result<LoadedApp, AppConfigError> { ... }
}
```

兼容策略：

- `AppRuntime::start_app_from_file` 和 `AppRuntime::start_app` 保留
- 内部逐步改为委托 builder
- 旧错误语义保持一致

### 2. WorkflowPromptParts

把 `WorkflowEngine::default_workflow_prompt()` 和 `default_assistant_prompt()` 的大段字符串拆成稳定片段，避免未来每次改一个规则都去拼接整段文本。

```rust
pub struct WorkflowPromptParts {
    pub role: String,
    pub constraints: String,
    pub tools: String,
    pub handoff: String,
}
```

第一阶段只做内部表示，不直接改变最终字符串输出。

### 3. WorkflowPromptStrategy + Template Method

`WorkflowEngine` 保留为门面对象，但 prompt 生成改为：

- 固定骨架由 template method 负责
- 具体 tools / handoff / driver selection 提示由 strategy 决定

```rust
pub trait WorkflowPromptStrategy: Send + Sync {
    fn render_tools(&self, ctx: &WorkflowPromptContext) -> String;
    fn render_handoff_rules(&self, ctx: &WorkflowPromptContext) -> String;
}

pub struct DefaultWorkflowPromptStrategy;
```

兼容策略：

- `DefaultWorkflowPromptStrategy` 首轮输出必须与当前默认 prompt 等价
- `WorkflowEngine::build_system_prompt` 仍然是调用入口

### 4. ApplicationRuntimeFactory

为 application startup 提供工厂边界，收拢 loader/runtime/workflow 的装配职责。

```rust
pub trait ApplicationRuntimeFactory: Send + Sync {
    fn build_runtime_builder(
        &self,
        manifest: AppManifest,
        base_dir: PathBuf,
    ) -> AppRuntimeBuilder;
}
```

第一阶段不要求外部调用点都改为工厂，只先把默认工厂立起来，让 `AppRuntime` 的内部依赖明确。

### 5. Application Capability Composite

为了后续让 driver、skill、tool policy、manifest 能力来源都可解释，增加 application-level capability tree：

```rust
pub enum AppCapabilityNode {
    Leaf(CapabilityRef),
    Group {
        source: AppCapabilitySource,
        children: Vec<AppCapabilityNode>,
    },
}
```

兼容策略：

- 对外仍然保留 legacy capability 列表输出
- 只在内部保存来源信息

## Migration Order

1. 给 `AppRuntime` 和 `WorkflowEngine` 增加当前行为锁定测试。
2. 引入 `AppRuntimeBuilder`，让 `start_app*` 内部委托。
3. 抽出 `WorkflowPromptParts`，保证字符串输出不变。
4. 引入 `WorkflowPromptStrategy`，默认实现保持完全兼容。
5. 把 driver/tool 选择规则收敛到 capability/provider 输入层，去掉 prompt 中对单个 driver 的硬编码依赖。
6. 引入 `ApplicationRuntimeFactory` 和 capability composite。
7. 用 `FULLSTACK-AUTODEV` / `NEWSROOM-AUTOWRITER` fixture 验证 runtime 和 prompt 输出一致。

## Risks / Trade-offs

- 风险：prompt 生成逻辑即使字面小改动，也可能影响 planner/coordinator 行为  
  缓解：在切 strategy 前先加 snapshot，默认实现逐字兼容。

- 风险：runtime builder 拆分后，错误路径可能变化  
  缓解：锁定 `start_app*` 的错误与成功测试，不改变 public error surface。

- 风险：capability composite 过早暴露到外部接口  
  缓解：首轮只内部引入，对外仍 flatten 成旧结构。

## Verification

- `cargo test -p macaca-app`
- `cargo check -p macaca-app`
- 如 public API 影响调用侧，再运行 workspace `cargo check`
- 为以下对象运行 GitNexus impact 后再实施：
  - `AppRuntime`
  - `AppLoader`
  - `WorkflowEngine`
  - `build_system_prompt`

